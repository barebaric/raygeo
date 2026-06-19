use crate::geo::algo::interp::barycentric_interpolate;
use crate::geo::algo::intersect::ray_line_intersection;
use crate::geo::algo::pde_mesh::{
    compute_gradient_field, BoundaryTag, TriangleMesh,
};
use crate::types::{Point, Point3D};

pub fn trace_spiral(
    mesh: &TriangleMesh,
    u_field: &[f64],
    step_over: f64,
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

    let (start_pt, start_tri, start_entry_edge) = pick_start(mesh, u_field)?;

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

        path.push(Point3D::new(next_pos.x, next_pos.y, 0.0));
        current_pos = next_pos;

        let next_tri = mesh.adjacency[current_tri * 3 + exit_edge];
        if next_tri < 0 {
            break;
        }
        let next_tri = next_tri as usize;

        let next_entry = match entry_edge_of(mesh, next_tri, current_tri) {
            Some(e) => e,
            None => break,
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
        if let Some(pt) = ray_line_intersection(pos, *dir, pa, pb) {
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
