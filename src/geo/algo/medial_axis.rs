use std::collections::{HashMap, HashSet, VecDeque};

use spade::handles::FixedVertexHandle;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

use crate::geo::shape::polygon::is_point_in_polygon;
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

/// Compute the Medial Axis Transform of a planar domain via Delaunay
/// circumcenters.
///
/// * `outer` — outer boundary polygon (CCW).
/// * `holes` — inner hole polygons (CW), may be empty.
/// * `tool_radius` — minimum clearance; nodes with smaller radius are pruned.
/// * `sampling_spacing` — target spacing for boundary-sampling and Steiner
///   grid.  Smaller values = denser mesh = finer MAT ≈ `step_over × 0.5`.
pub fn compute_medial_axis(
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
        return Err(
            "no valid medial axis nodes found — try smaller sampling_spacing \
             or larger tool_radius"
                .into(),
        );
    }

    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut edge_map: HashMap<(usize, usize), (usize, usize)> = HashMap::new();

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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn circumcenter(a: Point, b: Point, c: Point) -> (Point, f64) {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-30 {
        return (Point::new(0.0, 0.0), -1.0);
    }
    let a2 = a.x * a.x + a.y * a.y;
    let b2 = b.x * b.x + b.y * b.y;
    let c2 = c.x * c.x + c.y * c.y;
    let ux = (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d;
    let uy = (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d;
    let center = Point::new(ux, uy);
    let r = ((center.x - a.x).powi(2) + (center.y - a.y).powi(2)).sqrt();
    (center, r)
}

fn build_sampled_cdt(
    outer: &[Point],
    holes: &[Vec<Point>],
    spacing: f64,
) -> Result<(Vec<[usize; 3]>, Vec<Point>), String> {
    let mut cdt = Cdt::new();
    let mut vertices: Vec<Point> = Vec::new();
    let mut vidx_map: HashMap<FixedVertexHandle, usize> = HashMap::new();

    let insert_pt = |cdt: &mut Cdt,
                     vidx_map: &mut HashMap<FixedVertexHandle, usize>,
                     vertices: &mut Vec<Point>,
                     p: Point|
     -> FixedVertexHandle {
        let h = cdt.insert(Point2::new(p.x, p.y)).unwrap();
        vidx_map.insert(h, vertices.len());
        vertices.push(p);
        h
    };

    // Sample outer boundary densely and insert as constraint
    let outer_samples = sample_polygon(outer, spacing);
    let outer_handles: Vec<_> = outer_samples
        .iter()
        .map(|p| insert_pt(&mut cdt, &mut vidx_map, &mut vertices, *p))
        .collect();
    for i in 0..outer_handles.len() {
        let j = (i + 1) % outer_handles.len();
        cdt.add_constraint(outer_handles[i], outer_handles[j]);
    }

    // Sample holes and insert as constraints
    for hole in holes {
        if hole.len() < 3 {
            continue;
        }
        let hole_samples = sample_polygon(hole, spacing);
        let hole_handles: Vec<_> = hole_samples
            .iter()
            .map(|p| insert_pt(&mut cdt, &mut vidx_map, &mut vertices, *p))
            .collect();
        for i in 0..hole_handles.len() {
            let j = (i + 1) % hole_handles.len();
            cdt.add_constraint(hole_handles[i], hole_handles[j]);
        }
    }

    // NO interior Steiner points — the MAT requires every triangle vertex
    // to lie on the boundary (circumradius = distance to boundary).

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
            let cx = (vertices[a].x + vertices[b].x + vertices[c].x) / 3.0;
            let cy = (vertices[a].y + vertices[b].y + vertices[c].y) / 3.0;
            let centroid = Point::new(cx, cy);
            if !is_point_in_polygon(centroid, &outer.to_vec()) {
                continue;
            }
            let mut inside_hole = false;
            for hole in holes {
                if is_point_in_polygon(centroid, &hole.to_vec()) {
                    inside_hole = true;
                    break;
                }
            }
            if inside_hole {
                continue;
            }
            triangles.push([a, b, c]);
        }
    }

    Ok((triangles, vertices))
}

fn sample_polygon(poly: &[Point], spacing: f64) -> Vec<Point> {
    if poly.is_empty() {
        return vec![];
    }
    let mut result = Vec::new();
    for i in 0..poly.len() {
        let j = (i + 1) % poly.len();
        let dx = poly[j].x - poly[i].x;
        let dy = poly[j].y - poly[i].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 {
            result.push(poly[i]);
            continue;
        }
        let n = (len / spacing).ceil() as usize;
        for k in 0..n {
            let t = k as f64 / n as f64;
            result.push(Point::new(poly[i].x + t * dx, poly[i].y + t * dy));
        }
    }
    result
}

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
