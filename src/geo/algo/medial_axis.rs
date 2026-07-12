use prof_macros::prof;

use std::collections::{HashMap, HashSet, VecDeque};
use std::panic::{self, AssertUnwindSafe};

use spade::handles::FixedVertexHandle;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

use crate::geo::shape::point::get_circumcenter;
use crate::geo::shape::polygon::{
    clean_polygon, is_point_in_polygon, resample_polygon,
};
use crate::types::{Point, Polygon};

type Cdt = ConstrainedDelaunayTriangulation<Point2<f64>>;

#[derive(Clone, Copy, Debug)]
pub struct MaNode {
    pub point: Point,
    pub clearance: f64,
}

#[derive(Clone, Debug)]
pub struct MaBranch {
    pub nodes: Vec<usize>,
    pub points: Vec<Point>,
    pub clearances: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct MedialAxis {
    pub nodes: Vec<MaNode>,
    pub edges: Vec<(usize, usize)>,
    pub root: usize,
    pub branches: Vec<MaBranch>,
    /// Depth of each node from `root` (root has depth 0).
    pub(crate) depth: Vec<u32>,
    /// Binary-lifting ancestor table: `up[k][i]` = 2^k-th ancestor of node `i`.
    /// `up[0][i]` is the parent, `up[k][i] = i` when 2^k ≥ depth[i].
    pub(crate) up: Vec<Vec<u32>>,
}

impl MedialAxis {
    /// Build depth and binary-lifting ancestor table from a parent array.
    pub(crate) fn build_lca_cache(
        parent_idx: &[usize],
        root: usize,
    ) -> (Vec<u32>, Vec<Vec<u32>>) {
        let n = parent_idx.len();
        let mut depth = vec![0u32; n];
        for i in 0..n {
            if i == root || parent_idx[i] == usize::MAX {
                continue;
            }
            // Walk up to compute depth, caching as we go.
            let mut cur = i;
            let mut d = 0u32;
            while cur != root && depth[cur] == 0 {
                if parent_idx[cur] == usize::MAX {
                    break;
                }
                cur = parent_idx[cur];
                d += 1;
            }
            d += depth[cur];
            // Fill depths along the path we walked.
            cur = i;
            while cur != root && depth[cur] == 0 {
                depth[cur] = d;
                d = d.wrapping_sub(1);
                cur = parent_idx[cur];
                if cur == usize::MAX {
                    break;
                }
            }
        }

        let max_k = (usize::BITS - n.leading_zeros()) as usize;
        let mut up = vec![vec![root as u32; n]; max_k];
        for i in 0..n {
            up[0][i] = if parent_idx[i] == usize::MAX || i == root {
                i as u32
            } else {
                parent_idx[i] as u32
            };
        }
        for k in 1..max_k {
            for i in 0..n {
                up[k][i] = up[k - 1][up[k - 1][i] as usize];
            }
        }

        (depth, up)
    }

    /// Compute the Medial Axis Transform of a planar domain via Delaunay
    /// circumcenters.
    ///
    /// * `outer` — outer boundary polygon (CCW).
    /// * `holes` — inner hole polygons (CW), may be empty.
    /// * `min_clearance` — minimum clearance; nodes with smaller radius are
    ///   pruned.
    /// * `sampling_spacing` — target spacing for boundary-sampling and
    ///   Steiner grid.  Smaller values = denser mesh ≈ finer MAT.
    #[prof]
    pub fn compute(
        outer: &[Point],
        holes: &[Vec<Point>],
        min_clearance: f64,
        sampling_spacing: f64,
    ) -> Result<MedialAxis, String> {
        if outer.len() < 3 {
            return Err("outer boundary must have at least 3 vertices".into());
        }
        if sampling_spacing <= 0.0 {
            return Err("sampling_spacing must be positive".into());
        }

        let (triangles, vertices) =
            build_sampled_cdt(outer, holes, sampling_spacing)?;
        if triangles.is_empty() {
            return Err("CDT produced no triangles".into());
        }

        let mut nodes = Vec::new();
        let mut tri_to_node = vec![None; triangles.len()];

        for (ti, tri) in triangles.iter().enumerate() {
            let pts = [vertices[tri[0]], vertices[tri[1]], vertices[tri[2]]];

            let (center, radius) = get_circumcenter(pts[0], pts[1], pts[2]);
            if radius < 0.0 {
                continue;
            }

            if !is_point_in_polygon(center, &outer.to_vec()) {
                continue;
            }
            let mut in_hole = false;
            for hole in holes {
                if is_point_in_polygon(center, &hole.to_vec()) {
                    in_hole = true;
                    break;
                }
            }
            if in_hole {
                continue;
            }

            if radius < min_clearance {
                continue;
            }

            let idx = nodes.len();
            nodes.push(MaNode {
                point: center,
                clearance: radius,
            });
            tri_to_node[ti] = Some(idx);
        }

        if nodes.is_empty() {
            return Err("no valid medial axis nodes found — try smaller \
                 sampling_spacing or smaller min_clearance"
                .into());
        }

        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut edge_map: HashMap<(usize, usize), (usize, usize)> =
            HashMap::new();

        for (ti, tri) in triangles.iter().enumerate() {
            let Some(ni) = tri_to_node[ti] else {
                continue;
            };
            for e in 0..3 {
                let a = tri[e];
                let b = tri[(e + 1) % 3];
                let key = if a < b { (a, b) } else { (b, a) };

                if let Some(&(other_ti, _)) = edge_map.get(&key) {
                    if let Some(nj) = tri_to_node[other_ti] {
                        adj[ni].push(nj);
                        adj[nj].push(ni);
                    }
                    edge_map.remove(&key);
                } else {
                    edge_map.insert(key, (ti, e));
                }
            }
        }

        for a in &mut adj {
            a.sort();
            a.dedup();
        }

        let root = nodes
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.clearance.partial_cmp(&b.1.clearance).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        let mut parent = vec![usize::MAX; nodes.len()];
        let mut visited = vec![false; nodes.len()];
        let mut queue = VecDeque::new();
        visited[root] = true;
        queue.push_back(root);
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    parent[v] = u;
                    queue.push_back(v);
                }
            }
        }

        let edges: Vec<(usize, usize)> = parent
            .iter()
            .enumerate()
            .filter(|(_, &p)| p != usize::MAX)
            .map(|(c, &p)| (p, c))
            .collect();

        let branches = contract_to_branches(&adj, &parent, root, &nodes);

        let (depth, up) = Self::build_lca_cache(&parent, root);

        Ok(MedialAxis {
            nodes,
            edges,
            root,
            branches,
            depth,
            up,
        })
    }

    /// Find the lowest common ancestor of two nodes using binary lifting.
    pub fn find_lca(&self, mut a: usize, mut b: usize) -> usize {
        let max_k = self.up.len();

        // Lift deeper node up to the same depth.  If a node's depth is 0
        // (disconnected component) the binary lifting below naturally
        // handles it since up[k][x] == x for such nodes.
        if self.depth[a] > self.depth[b] {
            let mut diff = self.depth[a] - self.depth[b];
            let mut k = 0;
            while diff > 0 {
                if diff & 1 != 0 {
                    a = self.up[k][a] as usize;
                }
                diff >>= 1;
                k += 1;
            }
        } else {
            let mut diff = self.depth[b].wrapping_sub(self.depth[a]);
            let mut k = 0;
            while diff > 0 {
                if diff & 1 != 0 {
                    b = self.up[k][b] as usize;
                }
                diff >>= 1;
                k += 1;
            }
        }

        // Binary search for LCA.  Self-looped (disconnected) nodes
        // stay in place and will mismatch — we detect that below.
        if a != b {
            for k in (0..max_k).rev() {
                if self.up[k][a] != self.up[k][b] {
                    a = self.up[k][a] as usize;
                    b = self.up[k][b] as usize;
                }
            }
            a = self.up[0][a] as usize;
        }
        a
    }

    /// Find a path between two nodes using binary-lifting LCA.
    pub fn path_between_indices(
        &self,
        from_idx: usize,
        to_idx: usize,
    ) -> Option<Vec<Point>> {
        if from_idx == to_idx {
            return Some(vec![self.nodes[from_idx].point]);
        }

        let lca = self.find_lca(from_idx, to_idx);

        // Walk from `from_idx` up to LCA, collecting nodes.
        let mut from_to_lca: Vec<usize> = Vec::new();
        let mut cur = from_idx;
        while cur != lca {
            from_to_lca.push(cur);
            cur = self.up[0][cur] as usize;
            if cur == from_to_lca[from_to_lca.len() - 1] {
                // Self-loop — reached top of a disconnected component
                // without meeting the other node.
                return None;
            }
        }
        from_to_lca.push(lca);

        // Walk from `to_idx` up to LCA (excluding LCA, it's already added).
        let mut to_rev: Vec<usize> = Vec::new();
        cur = to_idx;
        while cur != lca {
            to_rev.push(cur);
            cur = self.up[0][cur] as usize;
            if cur == to_rev[to_rev.len() - 1] {
                return None;
            }
        }

        from_to_lca.extend(to_rev.into_iter().rev());
        Some(
            from_to_lca
                .into_iter()
                .map(|i| self.nodes[i].point)
                .collect(),
        )
    }

    /// Build a cleared mask for all nodes.  Returns a `Vec<bool>` where
    /// `cleared_mask[i]` is true iff `nodes[i]` lies inside at least one
    /// polygon in `cleared`.  Pass this mask to
    /// [`path_between_indices_cleared`] to avoid per-node per-polygon
    /// iteration on every call.
    #[prof]
    pub fn build_cleared_mask(&self, cleared: &[Polygon]) -> Vec<bool> {
        self.nodes
            .iter()
            .map(|n| {
                cleared
                    .iter()
                    .any(|poly| is_point_in_polygon(n.point, poly))
            })
            .collect()
    }

    /// Walk up parent pointers from `idx` until a cleared node is found.
    fn first_cleared_ancestor(
        &self,
        mut idx: usize,
        is_cleared: &[bool],
    ) -> Option<usize> {
        while !is_cleared[idx] {
            let parent = self.up[0][idx] as usize;
            if parent == idx {
                return None;
            }
            idx = parent;
        }
        Some(idx)
    }

    /// Collect node indices from `idx` up to `meeting`, skipping uncleared
    /// nodes by jumping to the nearest cleared ancestor.
    fn collect_cleared_path_to_meeting(
        &self,
        mut idx: usize,
        meeting: usize,
        is_cleared: &[bool],
    ) -> Option<Vec<usize>> {
        if idx == meeting {
            return Some(vec![meeting]);
        }
        if !is_cleared[idx] {
            idx = self.first_cleared_ancestor(idx, is_cleared)?;
            if idx == meeting {
                return Some(vec![meeting]);
            }
        }
        let mut nodes = Vec::new();
        loop {
            let prev = idx;
            nodes.push(idx);
            idx = self.up[0][idx] as usize;
            if idx == prev {
                // Self-loop — disconnected component.
                return None;
            }
            if idx == meeting {
                break;
            }
            if !is_cleared[idx] {
                idx = self.first_cleared_ancestor(idx, is_cleared)?;
                if idx == meeting {
                    nodes.push(idx);
                    return Some(nodes);
                }
            }
        }
        nodes.push(meeting);
        Some(nodes)
    }

    /// Find a path between two points, visiting only MAT nodes inside at
    /// least one `cleared` polygon.  Uncleared nodes on the tree path are
    /// skipped by walking up to the nearest cleared ancestor.  This avoids
    /// the O(N·P) upfront cost of trimming the entire tree.
    #[prof]
    pub fn path_between_cleared(
        &self,
        from: Point,
        to: Point,
        cleared: &[Polygon],
    ) -> Option<Vec<Point>> {
        let from_idx = self.nearest_node(from)?;
        let to_idx = self.nearest_node(to)?;
        let mask = self.build_cleared_mask(cleared);
        self.path_between_indices_cleared(from_idx, to_idx, &mask)
    }

    /// Like `path_between_indices` but only visits cleared nodes, using a
    /// pre-computed [`build_cleared_mask`].
    #[prof]
    pub fn path_between_indices_cleared(
        &self,
        from_idx: usize,
        to_idx: usize,
        is_cleared: &[bool],
    ) -> Option<Vec<Point>> {
        if from_idx == to_idx {
            if is_cleared[from_idx] {
                return Some(vec![self.nodes[from_idx].point]);
            }
            return None;
        }

        let from = self.first_cleared_ancestor(from_idx, is_cleared)?;
        let to = self.first_cleared_ancestor(to_idx, is_cleared)?;
        let lca = self.find_lca(from, to);
        let meeting = self.first_cleared_ancestor(lca, is_cleared)?;

        let mut path =
            self.collect_cleared_path_to_meeting(from, meeting, is_cleared)?;
        let to_side =
            self.collect_cleared_path_to_meeting(to, meeting, is_cleared)?;

        path.extend(to_side.into_iter().rev().skip(1));
        Some(path.into_iter().map(|i| self.nodes[i].point).collect())
    }

    /// Find the nearest MAT node index to a point via linear scan.
    pub fn nearest_node(&self, pt: Point) -> Option<usize> {
        if self.nodes.is_empty() {
            return None;
        }
        self.nodes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (a.point - pt).length_squared();
                let db = (b.point - pt).length_squared();
                crate::utils::sort_f64(da, db)
            })
            .map(|(i, _)| i)
    }

    /// Find a path between `from` and `to` using the Medial Axis tree.
    #[prof]
    pub fn path_between(&self, from: Point, to: Point) -> Option<Vec<Point>> {
        let from_idx = self.nearest_node(from)?;
        let to_idx = self.nearest_node(to)?;
        self.path_between_indices(from_idx, to_idx)
    }

    /// Return a new `MedialAxis` containing only nodes whose positions
    /// fall inside at least one of the given polygons.
    ///
    /// Branches are discarded (not needed for routing).  Routing through
    /// a trimmed MAT ensures travel paths only go through already-cleared
    /// territory.
    #[prof]
    pub fn trim_to_polygons(&self, polygons: &[Vec<Point>]) -> MedialAxis {
        let mut old_to_new: Vec<Option<usize>> = vec![None; self.nodes.len()];
        let mut new_nodes: Vec<MaNode> = Vec::new();

        for (old_i, node) in self.nodes.iter().enumerate() {
            if polygons
                .iter()
                .any(|poly| is_point_in_polygon(node.point, poly))
            {
                old_to_new[old_i] = Some(new_nodes.len());
                new_nodes.push(*node);
            }
        }

        let mut new_edges: Vec<(usize, usize)> = Vec::new();
        for &(a, b) in &self.edges {
            if let (Some(na), Some(nb)) = (old_to_new[a], old_to_new[b]) {
                if na != nb {
                    new_edges.push((na, nb));
                }
            }
        }

        let root = new_nodes
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.clearance
                    .partial_cmp(&b.clearance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Build adjacency from new_edges, then BFS from root to produce a
        // proper parent array (avoiding nodes whose ancestor chain was
        // truncated by trimming).
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); new_nodes.len()];
        for &(a, b) in &new_edges {
            adj[a].push(b);
            adj[b].push(a);
        }
        let mut parent_idx = vec![usize::MAX; new_nodes.len()];
        let mut queue = VecDeque::new();
        parent_idx[root] = root; // temporary — makes visited work
        queue.push_back(root);
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if parent_idx[v] == usize::MAX {
                    parent_idx[v] = u;
                    queue.push_back(v);
                }
            }
        }
        parent_idx[root] = usize::MAX;
        let (depth, up) = Self::build_lca_cache(&parent_idx, root);

        MedialAxis {
            nodes: new_nodes,
            edges: new_edges,
            root,
            branches: Vec::new(),
            depth,
            up,
        }
    }
}

fn build_sampled_cdt(
    outer: &[Point],
    holes: &[Vec<Point>],
    spacing: f64,
) -> Result<(Vec<[usize; 3]>, Vec<Point>), String> {
    // Clean outer boundary with clipper to resolve self-intersections
    let outer_vec: Vec<Point> = outer.to_vec();
    let cleaned_outer = clean_polygon(&outer_vec, 0.0)
        .ok_or_else(|| "failed to clean outer boundary".to_string())?;
    let outer_samples = resample_polygon(&cleaned_outer, spacing);

    // Clean holes and discard any that extend outside the outer boundary or
    // fail to clean.
    let mut cleaned_holes: Vec<Vec<Point>> = Vec::new();
    for hole in holes {
        if hole.len() < 3 {
            continue;
        }
        let hole_vec: Vec<Point> = hole.to_vec();
        let cleaned = match clean_polygon(&hole_vec, 0.0) {
            Some(p) if p.len() >= 3 => p,
            _ => continue,
        };
        let samples = resample_polygon(&cleaned, spacing);
        // Skip holes whose vertices lie outside the outer boundary
        if samples
            .iter()
            .all(|p| is_point_in_polygon(*p, &cleaned_outer))
        {
            cleaned_holes.push(samples);
        }
    }

    // Build the CDT inside a panic-safe boundary so that degenerate
    // geometry (edge intersections, zero-length constraints) is caught
    // and falls back to a hole-less triangulation.
    let (triangles, vertices) =
        match try_build_cdt(&outer_samples, &cleaned_holes) {
            Ok(result) => result,
            Err(err) => {
                eprintln!(
                "raygeo: CDT with {} holes failed ({}), retrying without holes",
                cleaned_holes.len(),
                err
            );
                try_build_cdt(&outer_samples, &[])?
            }
        };

    if triangles.is_empty() {
        return Err("CDT produced no triangles".into());
    }
    Ok((triangles, vertices))
}

/// Build a constrained Delaunay triangulation from outer and hole samples.
/// Catches panics from spade and returns an error instead.
fn try_build_cdt(
    outer_samples: &[Point],
    cleaned_holes: &[Vec<Point>],
) -> Result<(Vec<[usize; 3]>, Vec<Point>), String> {
    let mut cdt = Cdt::new();
    let mut vertices: Vec<Point> = Vec::new();
    // Maps from FixedVertexHandle → vertex index in `vertices`.
    let mut vidx_map: HashMap<FixedVertexHandle, usize> = HashMap::new();
    // Reverse: vertex index → FixedVertexHandle.
    let mut handles: Vec<FixedVertexHandle> = Vec::new();

    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // Insert outer boundary samples
        for p in outer_samples {
            let h = cdt.insert(Point2::new(p.x, p.y)).unwrap();
            vidx_map.insert(h, vertices.len());
            handles.push(h);
            vertices.push(*p);
        }
        // Constrain outer boundary edges
        for i in 0..outer_samples.len() {
            let j = (i + 1) % outer_samples.len();
            cdt.add_constraint(handles[i], handles[j]);
        }

        // Insert and constrain holes
        for hole_samples in cleaned_holes {
            let start = handles.len();
            for p in hole_samples {
                let h = cdt.insert(Point2::new(p.x, p.y)).unwrap();
                vidx_map.insert(h, vertices.len());
                handles.push(h);
                vertices.push(*p);
            }
            for li in 0..hole_samples.len() {
                let lj = (li + 1) % hole_samples.len();
                cdt.add_constraint(handles[start + li], handles[start + lj]);
            }
        }

        // NO interior Steiner points — the MAT requires every triangle
        // vertex to lie on the boundary.

        let mut triangles: Vec<[usize; 3]> = Vec::new();
        for face in cdt.inner_faces() {
            let vs = face.vertices();
            let h0 = vs[0].fix();
            let h1 = vs[1].fix();
            let h2 = vs[2].fix();
            let i0 = vidx_map.get(&h0).copied();
            let i1 = vidx_map.get(&h1).copied();
            let i2 = vidx_map.get(&h2).copied();
            if let (Some(a), Some(b), Some(c)) = (i0, i1, i2) {
                triangles.push([a, b, c]);
            }
        }
        (triangles, vertices)
    }));

    match result {
        Ok(ok) => Ok(ok),
        Err(_) => Err("CDT construction panicked".into()),
    }
}

/// Contract the medial-axis tree into a list of branches.
///
/// A *branch* is a maximal path whose internal nodes all have degree 2
/// (i.e. it runs between two junctions, or between a junction and a leaf).
/// Each branch is oriented so that clearances are non-decreasing along it.
fn contract_to_branches(
    adj: &[Vec<usize>],
    parent: &[usize],
    root: usize,
    nodes: &[MaNode],
) -> Vec<MaBranch> {
    let n = nodes.len();
    let mut degree = vec![0usize; n];
    for i in 0..n {
        if parent[i] != usize::MAX {
            degree[i] += 1;
            degree[parent[i]] += 1;
        }
    }
    degree[root] = degree[root].saturating_sub(1);

    let is_junction =
        |u: usize| -> bool { degree[u] != 2 || u == root || degree[u] == 0 };

    let mut branches = Vec::new();
    let mut edge_used = HashSet::new();

    for u in 0..n {
        if !is_junction(u) {
            continue;
        }
        for &v in &adj[u] {
            let key = if u < v { (u, v) } else { (v, u) };
            if edge_used.contains(&key) {
                continue;
            }

            let mut path_nodes = vec![u];
            let mut prev = u;
            let mut curr = v;

            while !is_junction(curr) {
                path_nodes.push(curr);
                let ek = if prev < curr {
                    (prev, curr)
                } else {
                    (curr, prev)
                };
                edge_used.insert(ek);

                let next = adj[curr]
                    .iter()
                    .find(|&&nb| nb != prev)
                    .copied()
                    .unwrap_or(usize::MAX);
                if next == usize::MAX {
                    break;
                }
                prev = curr;
                curr = next;
            }
            path_nodes.push(curr);

            let ek = if prev < curr {
                (prev, curr)
            } else {
                (curr, prev)
            };
            edge_used.insert(ek);

            branches.push(MaBranch {
                nodes: path_nodes,
                points: Vec::new(),
                clearances: Vec::new(),
            });
        }
    }

    for branch in &mut branches {
        branch.points = branch.nodes.iter().map(|&i| nodes[i].point).collect();
        branch.clearances =
            branch.nodes.iter().map(|&i| nodes[i].clearance).collect();
        if branch.clearances.len() >= 2
            && branch.clearances.first() < branch.clearances.last()
        {
            branch.nodes.reverse();
            branch.points.reverse();
            branch.clearances.reverse();
        }
    }

    branches
}
