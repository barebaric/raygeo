use crate::geo::algo::interp::barycentric_interpolate;
use crate::geo::algo::intersect::get_ray_line_intersection;
use crate::geo::types::{Point, Point3D};

use super::gradient::compute_gradient_field;
use super::types::{BoundaryTag, TriangleMesh};

const SUBDIVIDE_LENGTH: f64 = 0.2;

pub fn trace_spiral(
    mesh: &TriangleMesh,
    u_field: &[f64],
    step_over: f64,
    start_point: Option<Point>,
) -> Result<Vec<Point3D>, String> {
    if u_field.len() != mesh.vertices.len() {
        return Err(format!(
            "u_field length {} does not match vertex count {}",
            u_field.len(),
            mesh.vertices.len()
        ));
    }
    if step_over <= 0.0 {
        return Err("step_over must be positive".into());
    }

    let gradient = compute_gradient_field(mesh, u_field)?;

    let (start_pt, start_tri, start_entry_edge) = match start_point {
        Some(pt) => {
            let ti = find_triangle_containing(mesh, pt)
                .ok_or("start point is outside the mesh domain")?;
            (pt, ti, 3usize)
        }
        None => pick_start(mesh, u_field)?,
    };

    let mut path: Vec<Point3D> = Vec::new();
    path.push(Point3D::new(start_pt.x, start_pt.y, 0.0));

    let mut current_pos = start_pt;
    let mut current_tri = start_tri;
    let mut entry_edge = start_entry_edge;
    let max_steps = mesh.triangles.len() * 20;

    for _step in 0..max_steps {
        let g = gradient[current_tri];
        let mag = (g[0] * g[0] + g[1] * g[1]).sqrt();

        if mag < 1e-12 {
            break;
        }

        let nx = g[0] / mag;
        let ny = g[1] / mag;
        let tx = -ny;
        let ty = nx;

        let alpha = step_over * mag / (2.0 * std::f64::consts::PI);
        let dir = Point::new(tx + alpha * nx, ty + alpha * ny);

        let (exit_edge, next_pos) = match find_exit_edge(
            mesh,
            current_tri,
            current_pos,
            &dir,
            entry_edge,
        ) {
            Some(result) => result,
            None => break,
        };

        let seg_dx = next_pos.x - current_pos.x;
        let seg_dy = next_pos.y - current_pos.y;
        let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();
        let n_sub = (seg_len / SUBDIVIDE_LENGTH).ceil() as usize;
        for i in 1..n_sub {
            let t = i as f64 / n_sub as f64;
            path.push(Point3D::new(
                current_pos.x + t * seg_dx,
                current_pos.y + t * seg_dy,
                0.0,
            ));
        }
        current_pos = next_pos;

        let mut next_tri = mesh.adjacency[current_tri * 3 + exit_edge];
        let mut cut_edge_cross = false;
        if next_tri < 0 {
            if let Some((other_tri, other_ei)) =
                find_cut_edge_match(mesh, current_tri, exit_edge)
            {
                next_tri = other_tri as isize;
                entry_edge = other_ei;
                cut_edge_cross = true;
            }
        }
        if next_tri < 0 {
            break;
        }
        let next_tri = next_tri as usize;

        let next_entry = if cut_edge_cross {
            entry_edge
        } else {
            match entry_edge_of(mesh, next_tri, current_tri) {
                Some(e) => e,
                None => break,
            }
        };

        let [a, b, c] = mesh.triangles[next_tri];
        let ui = u_field[a];
        let uj = u_field[b];
        let uk = u_field[c];
        let u_at = barycentric_interpolate(
            next_pos,
            mesh.vertices[a],
            mesh.vertices[b],
            mesh.vertices[c],
            ui,
            uj,
            uk,
        );

        if u_at > 0.99 {
            path.push(Point3D::new(next_pos.x, next_pos.y, 0.0));
            break;
        }

        current_tri = next_tri;
        entry_edge = next_entry;
    }

    Ok(path)
}

fn find_triangle_containing(mesh: &TriangleMesh, pt: Point) -> Option<usize> {
    for (ti, tri) in mesh.triangles.iter().enumerate() {
        let a = mesh.vertices[tri[0]];
        let b = mesh.vertices[tri[1]];
        let c = mesh.vertices[tri[2]];
        if point_in_triangle(pt, a, b, c) {
            return Some(ti);
        }
    }
    None
}

fn point_in_triangle(p: Point, a: Point, b: Point, c: Point) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

fn sign(p: Point, a: Point, b: Point) -> f64 {
    (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y)
}

fn pick_start(
    mesh: &TriangleMesh,
    u_field: &[f64],
) -> Result<(Point, usize, usize), String> {
    for ti in 0..mesh.triangles.len() {
        for ei in 0..3 {
            if mesh.adjacency[ti * 3 + ei] != -1 {
                continue;
            }
            let a = mesh.triangles[ti][ei];
            let b = mesh.triangles[ti][(ei + 1) % 3];
            if mesh.boundary_tags[a] == BoundaryTag::Inner
                && mesh.boundary_tags[b] == BoundaryTag::Inner
            {
                let va = mesh.vertices[a];
                let vb = mesh.vertices[b];
                let mid = Point::new((va.x + vb.x) * 0.5, (va.y + vb.y) * 0.5);
                return Ok((mid, ti, ei));
            }
        }
    }

    let mut min_idx = 0usize;
    let mut min_val = f64::INFINITY;
    for (i, &val) in u_field.iter().enumerate() {
        if val < min_val {
            min_val = val;
            min_idx = i;
        }
    }

    for ti in 0..mesh.triangles.len() {
        let [a, b, c] = mesh.triangles[ti];
        if a == min_idx || b == min_idx || c == min_idx {
            let va = mesh.vertices[a];
            let vb = mesh.vertices[b];
            let vc = mesh.vertices[c];
            let cx = (va.x + vb.x + vc.x) / 3.0;
            let cy = (va.y + vb.y + vc.y) / 3.0;
            return Ok((Point::new(cx, cy), ti, 3));
        }
    }

    Err("mesh has no triangles".into())
}

fn find_exit_edge(
    mesh: &TriangleMesh,
    tri_idx: usize,
    pos: Point,
    dir: &Point,
    entry_edge: usize,
) -> Option<(usize, Point)> {
    let [a, b, c] = mesh.triangles[tri_idx];
    let verts = [mesh.vertices[a], mesh.vertices[b], mesh.vertices[c]];

    let mut best_t = f64::INFINITY;
    let mut best_edge = None;
    let mut best_pt = None;

    for ei in 0..3 {
        if ei < 3 && ei == entry_edge {
            continue;
        }
        let pa = verts[ei];
        let pb = verts[(ei + 1) % 3];
        if let Some(pt) = get_ray_line_intersection(pos, *dir, pa, pb) {
            let dx = pt.x - pos.x;
            let dy = pt.y - pos.y;
            let t = (dx * dx + dy * dy).sqrt();
            if t > 1e-12 && t < best_t {
                best_t = t;
                best_edge = Some(ei);
                best_pt = Some(pt);
            }
        }
    }

    best_edge.map(|e| (e, best_pt.unwrap()))
}

fn entry_edge_of(
    mesh: &TriangleMesh,
    tri_idx: usize,
    prev_tri: usize,
) -> Option<usize> {
    for ei in 0..3 {
        let nb = mesh.adjacency[tri_idx * 3 + ei];
        if nb >= 0 && nb as usize == prev_tri {
            return Some(ei);
        }
    }
    None
}

fn find_cut_edge_match(
    mesh: &TriangleMesh,
    current_tri: usize,
    exit_edge: usize,
) -> Option<(usize, usize)> {
    let a = mesh.triangles[current_tri][exit_edge];
    let b = mesh.triangles[current_tri][(exit_edge + 1) % 3];
    for ti in 0..mesh.triangles.len() {
        if ti == current_tri {
            continue;
        }
        for ei in 0..3 {
            if mesh.adjacency[ti * 3 + ei] != -1 {
                continue;
            }
            if mesh.triangles[ti][ei] == b
                && mesh.triangles[ti][(ei + 1) % 3] == a
            {
                return Some((ti, ei));
            }
        }
    }
    None
}
