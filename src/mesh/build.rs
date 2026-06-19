use std::collections::HashMap;

use spade::handles::FixedVertexHandle;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

use crate::geo::shape::polygon::{
    is_point_in_polygon, offset_polygon_with_style as offset_poly, JoinStyle,
};
use crate::types::Point;

use super::types::{BoundaryTag, Triangle, TriangleMesh};

type Cdt = ConstrainedDelaunayTriangulation<Point2<f64>>;

fn to_spade(p: &Point) -> Point2<f64> {
    Point2::new(p.x, p.y)
}

pub fn build_triangle_mesh(
    outer: &[Point],
    holes: &[Vec<Point>],
    tool_radius: f64,
    min_angle: f64,
) -> Result<TriangleMesh, String> {
    if outer.len() < 3 {
        return Err("outer boundary must have at least 3 vertices".into());
    }
    for (i, hole) in holes.iter().enumerate() {
        if hole.len() < 3 {
            return Err(format!("hole {} must have at least 3 vertices", i));
        }
    }

    let offset_result = if tool_radius.abs() > 1e-12 {
        offset_poly(&outer.to_vec(), -tool_radius, JoinStyle::Miter)
    } else {
        Vec::new()
    };
    let outer_poly = if offset_result.is_empty() {
        outer.to_vec()
    } else {
        offset_result
            .into_iter()
            .next()
            .unwrap_or_else(|| outer.to_vec())
    };

    let mut cdt = Cdt::new();
    let mut vidx_map: HashMap<FixedVertexHandle, usize> = HashMap::new();
    let mut vertices: Vec<Point> = Vec::new();
    let mut boundary_tags: Vec<BoundaryTag> = Vec::new();

    let register_vertex = |cdt: &mut Cdt,
                           vidx_map: &mut HashMap<FixedVertexHandle, usize>,
                           vertices: &mut Vec<Point>,
                           boundary_tags: &mut Vec<BoundaryTag>,
                           p: &Point,
                           tag: BoundaryTag|
     -> FixedVertexHandle {
        let handle = cdt.insert(to_spade(p)).unwrap();
        vidx_map.insert(handle, vertices.len());
        vertices.push(*p);
        boundary_tags.push(tag);
        handle
    };

    let outer_handles: Vec<FixedVertexHandle> = outer_poly
        .iter()
        .map(|p| {
            register_vertex(
                &mut cdt,
                &mut vidx_map,
                &mut vertices,
                &mut boundary_tags,
                p,
                BoundaryTag::Outer,
            )
        })
        .collect();
    for i in 0..outer_handles.len() {
        let j = (i + 1) % outer_handles.len();
        cdt.add_constraint(outer_handles[i], outer_handles[j]);
    }

    for hole in holes {
        let mut hole_handles: Vec<FixedVertexHandle> = Vec::new();
        for p in hole {
            let h = register_vertex(
                &mut cdt,
                &mut vidx_map,
                &mut vertices,
                &mut boundary_tags,
                p,
                BoundaryTag::Inner,
            );
            hole_handles.push(h);
        }
        for i in 0..hole_handles.len() {
            let j = (i + 1) % hole_handles.len();
            cdt.add_constraint(hole_handles[i], hole_handles[j]);
        }
    }

    insert_steiner_points(
        &mut cdt,
        &outer_poly,
        holes,
        &mut vidx_map,
        &mut vertices,
        &mut boundary_tags,
        min_angle,
    );

    let mut triangles: Vec<Triangle> = Vec::new();
    for face in cdt.inner_faces() {
        let vs = face.vertices();
        let idx0 = vidx_map.get(&vs[0].fix()).copied();
        let idx1 = vidx_map.get(&vs[1].fix()).copied();
        let idx2 = vidx_map.get(&vs[2].fix()).copied();
        if let (Some(i), Some(j), Some(k)) = (idx0, idx1, idx2) {
            let cx = (vertices[i].x + vertices[j].x + vertices[k].x) / 3.0;
            let cy = (vertices[i].y + vertices[j].y + vertices[k].y) / 3.0;
            if !is_point_in_polygon(Point::new(cx, cy), &outer_poly) {
                continue;
            }
            let mut inside = true;
            for hole in holes {
                if is_point_in_polygon(Point::new(cx, cy), hole) {
                    inside = false;
                    break;
                }
            }
            if inside {
                triangles.push([i, j, k]);
            }
        }
    }

    let adjacency = build_adjacency(&triangles);

    Ok(TriangleMesh {
        vertices,
        triangles,
        adjacency,
        boundary_tags,
    })
}

fn insert_steiner_points(
    cdt: &mut Cdt,
    outer: &[Point],
    holes: &[Vec<Point>],
    vidx_map: &mut HashMap<FixedVertexHandle, usize>,
    vertices: &mut Vec<Point>,
    boundary_tags: &mut Vec<BoundaryTag>,
    min_angle: f64,
) {
    let (mut min_x, mut min_y, mut max_x, mut max_y) =
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in outer {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let factor = (min_angle / 200.0).max(0.025);
    let spacing = (width.min(height) * factor).max(0.5);

    let nx = ((width / spacing) as usize).clamp(3, 200);
    let ny = ((height / spacing) as usize).clamp(3, 200);
    let dx = width / nx as f64;
    let dy = height / ny as f64;

    for i in 1..nx {
        for j in 1..ny {
            let x = min_x + i as f64 * dx;
            let y = min_y + j as f64 * dy;
            let pt = Point::new(x, y);
            if !is_point_in_polygon(pt, &outer.to_vec()) {
                continue;
            }
            let mut inside_hole = false;
            for hole in holes {
                if is_point_in_polygon(pt, hole) {
                    inside_hole = true;
                    break;
                }
            }
            if inside_hole {
                continue;
            }
            if let Ok(h) = cdt.insert(to_spade(&pt)) {
                vidx_map.insert(h, vertices.len());
                vertices.push(pt);
                boundary_tags.push(BoundaryTag::None);
            }
        }
    }
}

pub fn build_adjacency(triangles: &[Triangle]) -> Vec<isize> {
    let num_tris = triangles.len();
    let mut adj: Vec<isize> = vec![-1; num_tris * 3];
    let mut edge_map: HashMap<(usize, usize), (usize, usize)> = HashMap::new();

    for (tri_idx, tri) in triangles.iter().enumerate() {
        for local_edge in 0..3 {
            let vi = tri[local_edge];
            let vj = tri[(local_edge + 1) % 3];
            let key = if vi < vj { (vi, vj) } else { (vj, vi) };

            if let Some(&(other_tri, other_edge)) = edge_map.get(&key) {
                adj[tri_idx * 3 + local_edge] = other_tri as isize;
                adj[other_tri * 3 + other_edge] = tri_idx as isize;
                edge_map.remove(&key);
            } else {
                edge_map.insert(key, (tri_idx, local_edge));
            }
        }
    }

    adj
}

/// Build a triangle mesh with approximately uniform edge length
/// `target_edge_len`, using a refined Steiner-grid CDT.
///
/// This is a convenience wrapper around [`build_triangle_mesh`] that
/// computes the `min_angle` parameter needed to achieve the desired
/// edge size.
pub fn build_uniform_mesh(
    outer: &[Point],
    holes: &[Vec<Point>],
    tool_radius: f64,
    target_edge_len: f64,
) -> Result<TriangleMesh, String> {
    let (min_x, min_y, max_x, max_y) = outer.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(mnx, mny, mxx, mxy), p| {
            (mnx.min(p.x), mny.min(p.y), mxx.max(p.x), mxy.max(p.y))
        },
    );
    let dim = (max_x - min_x).min(max_y - min_y);
    let min_angle = (target_edge_len * 200.0 / dim).clamp(0.5, 20.0);
    build_triangle_mesh(outer, holes, tool_radius, min_angle)
}
