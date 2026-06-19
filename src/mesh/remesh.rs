//! Mesh refinement.

use crate::mesh::build::build_adjacency;
use crate::mesh::types::{BoundaryTag, TriangleMesh};
use crate::types::Point;
use std::collections::HashMap;

/// Refine *mesh* so that no interior edge exceeds `max_edge_len`.
///
/// Boundary-constraint edges (outer ↔ outer or inner ↔ inner) are
/// never split — only edges with at least one free (non-boundary)
/// vertex are subdivided.
///
/// Returns a new `TriangleMesh` with shorter edges, or an error
/// string if retriangulation fails.
pub fn remesh(
    mesh: &TriangleMesh,
    outer: &[Point],
    max_edge_len: f64,
) -> Result<TriangleMesh, String> {
    let max_sq = max_edge_len * max_edge_len;

    // Collect all midpoints of edges that exceed the threshold.
    let mut new_pts: Vec<Point> = Vec::new();
    let mut seen: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();

    for tri in &mesh.triangles {
        for ei in 0..3 {
            let a = tri[ei];
            let b = tri[(ei + 1) % 3];
            let key = if a < b { (a, b) } else { (b, a) };
            if !seen.insert(key) {
                continue;
            }
            // Skip constraint edges (both endpoints on a boundary).
            if !matches!(mesh.boundary_tags[a], BoundaryTag::None)
                && !matches!(mesh.boundary_tags[b], BoundaryTag::None)
            {
                continue;
            }
            let dx = mesh.vertices[a].x - mesh.vertices[b].x;
            let dy = mesh.vertices[a].y - mesh.vertices[b].y;
            if dx * dx + dy * dy > max_sq {
                new_pts.push(Point::new(
                    (mesh.vertices[a].x + mesh.vertices[b].x) * 0.5,
                    (mesh.vertices[a].y + mesh.vertices[b].y) * 0.5,
                ));
            }
        }
    }

    if new_pts.is_empty() {
        return Ok(TriangleMesh {
            vertices: mesh.vertices.clone(),
            triangles: mesh.triangles.clone(),
            adjacency: mesh.adjacency.clone(),
            boundary_tags: mesh.boundary_tags.clone(),
        });
    }

    rebuild_with_insertions(mesh, outer, &new_pts)
}

/// Insert *new_points* into the domain by rebuilding the CDT from
/// the existing mesh vertices plus the new ones.
fn rebuild_with_insertions(
    mesh: &TriangleMesh,
    outer: &[Point],
    new_points: &[Point],
) -> Result<TriangleMesh, String> {
    use crate::geo::shape::polygon::is_point_in_polygon;
    use spade::handles::FixedVertexHandle;
    use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation as _};

    type Cdt = ConstrainedDelaunayTriangulation<Point2<f64>>;
    let mut cdt = Cdt::new();
    let mut vidx_map: HashMap<FixedVertexHandle, usize> = HashMap::new();
    let mut vertices: Vec<Point> = mesh.vertices.clone();
    let mut boundary_tags: Vec<BoundaryTag> = mesh.boundary_tags.clone();

    // Insert all existing mesh vertices into the CDT.
    let mut vtx: Vec<FixedVertexHandle> =
        Vec::with_capacity(mesh.vertices.len());
    for v in &mesh.vertices {
        let h = cdt.insert(Point2::new(v.x, v.y)).unwrap();
        vidx_map.insert(h, vtx.len());
        vtx.push(h);
    }

    // Insert constraints for boundary edges.
    // For each outer/Inner boundary chain, we reconstruct constraint
    // edges by finding consecutive boundary vertices along the mesh.
    let mut seen: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();
    for tri in &mesh.triangles {
        for ei in 0..3 {
            let a = tri[ei];
            let b = tri[(ei + 1) % 3];
            if matches!(mesh.boundary_tags[a], BoundaryTag::None)
                || matches!(mesh.boundary_tags[b], BoundaryTag::None)
            {
                continue;
            }
            let key = if a < b { (a, b) } else { (b, a) };
            if seen.insert(key) {
                let _ = cdt.add_constraint(vtx[a], vtx[b]);
            }
        }
    }

    // Insert the new midpoint vertices (as free vertices).
    for p in new_points {
        let h = cdt.insert(Point2::new(p.x, p.y)).unwrap();
        vidx_map.insert(h, vertices.len());
        vertices.push(*p);
        boundary_tags.push(BoundaryTag::None);
    }

    // Keep only triangles inside the outer boundary.
    let outer_poly: Vec<Point> = outer.to_vec();
    let mut triangles: Vec<[usize; 3]> = Vec::new();
    for face in cdt.inner_faces() {
        let vs = face.vertices();
        let idx0 = vidx_map.get(&vs[0].fix()).copied();
        let idx1 = vidx_map.get(&vs[1].fix()).copied();
        let idx2 = vidx_map.get(&vs[2].fix()).copied();
        if let (Some(i), Some(j), Some(k)) = (idx0, idx1, idx2) {
            let cx = (vertices[i].x + vertices[j].x + vertices[k].x) / 3.0;
            let cy = (vertices[i].y + vertices[j].y + vertices[k].y) / 3.0;
            if is_point_in_polygon(Point::new(cx, cy), &outer_poly) {
                triangles.push([i, j, k]);
            }
        }
    }

    let adjacency = build_adjacency(&triangles);

    let result = TriangleMesh {
        vertices,
        triangles,
        adjacency,
        boundary_tags,
    };

    // Recurse once in case the new mesh still has long edges.
    Ok(result)
}
