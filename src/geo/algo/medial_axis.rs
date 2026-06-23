use std::collections::{HashMap, HashSet, VecDeque};
use std::panic::{self, AssertUnwindSafe};

use spade::handles::FixedVertexHandle;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

use crate::geo::shape::point::circumcenter;
use crate::geo::shape::polygon::{
    clean_polygon, is_point_in_polygon, resample_polygon,
};
use crate::types::Point;

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
}

impl MedialAxis {
    /// Compute the Medial Axis Transform of a planar domain via Delaunay
    /// circumcenters.
    ///
    /// * `outer` — outer boundary polygon (CCW).
    /// * `holes` — inner hole polygons (CW), may be empty.
    /// * `tool_radius` — minimum clearance; nodes with smaller radius are
    ///   pruned.
    /// * `sampling_spacing` — target spacing for boundary-sampling and
    ///   Steiner grid.  Smaller values = denser mesh = finer MAT ≈
    ///   `step_over × 0.5`.
    pub fn compute(
        outer: &[Point],
        holes: &[Vec<Point>],
        tool_radius: f64,
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

            let (center, radius) = circumcenter(pts[0], pts[1], pts[2]);
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

            if radius < tool_radius {
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
                 sampling_spacing or larger tool_radius"
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

        Ok(MedialAxis {
            nodes,
            edges,
            root,
            branches,
        })
    }

    /// Find a path between `from` and `to` using the Medial Axis graph.
    ///
    /// Returns the node positions along the shortest-topology path
    /// (fewest edges), or `None` when the two points lie in disconnected
    /// regions of the MAT.
    pub fn path_between(&self, from: Point, to: Point) -> Option<Vec<Point>> {
        if self.nodes.is_empty() {
            return None;
        }

        let nearest = |pt: Point| -> Option<usize> {
            self.nodes
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let da = (a.point - pt).length_squared();
                    let db = (b.point - pt).length_squared();
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
        };

        let from_idx = nearest(from)?;
        let to_idx = nearest(to)?;

        if from_idx == to_idx {
            return Some(vec![self.nodes[from_idx].point]);
        }

        // Build adjacency from edges.
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
        for &(a, b) in &self.edges {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        }

        // BFS.
        let mut prev: HashMap<usize, usize> = HashMap::new();
        let mut visited: HashSet<usize> = HashSet::new();
        let mut queue: VecDeque<usize> = VecDeque::new();
        visited.insert(from_idx);
        queue.push_back(from_idx);

        while let Some(cur) = queue.pop_front() {
            if cur == to_idx {
                break;
            }
            if let Some(neighbors) = adj.get(&cur) {
                for &nb in neighbors {
                    if visited.insert(nb) {
                        prev.insert(nb, cur);
                        queue.push_back(nb);
                    }
                }
            }
        }

        if !visited.contains(&to_idx) {
            return None;
        }

        // Reconstruct.
        let mut path_idx: Vec<usize> = Vec::new();
        let mut cur = to_idx;
        path_idx.push(cur);
        while let Some(&p) = prev.get(&cur) {
            path_idx.push(p);
            cur = p;
        }
        path_idx.reverse();

        Some(path_idx.into_iter().map(|i| self.nodes[i].point).collect())
    }

    /// Return a new `MedialAxis` containing only nodes whose positions
    /// fall inside at least one of the given polygons.
    ///
    /// Branches are discarded (not needed for routing).  Routing through
    /// a trimmed MAT ensures travel paths only go through already-cleared
    /// territory.
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

        MedialAxis {
            nodes: new_nodes,
            edges: new_edges,
            root,
            branches: Vec::new(),
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
