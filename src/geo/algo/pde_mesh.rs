use std::collections::HashMap;

use spade::handles::FixedVertexHandle;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

use nalgebra::DVector;
use nalgebra_sparse::coo::CooMatrix;
use nalgebra_sparse::csr::CsrMatrix;

use crate::geo::shape::polygon::{
    is_point_in_polygon, offset_polygon as offset_poly,
};
use crate::types::Point;

pub type Triangle = [usize; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryTag {
    Outer,
    Inner,
    None,
}

#[derive(Clone, Debug)]
pub struct TriangleMesh {
    pub vertices: Vec<Point>,
    pub triangles: Vec<Triangle>,
    pub adjacency: Vec<isize>,
    pub boundary_tags: Vec<BoundaryTag>,
}

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
        offset_poly(&outer.to_vec(), -tool_radius)
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
    _min_angle: f64,
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
    let spacing = (width.min(height) * 0.1).max(width.min(height) / 20.0);

    let nx = ((width / spacing) as usize).clamp(3, 50);
    let ny = ((height / spacing) as usize).clamp(3, 50);
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

fn build_adjacency(triangles: &[Triangle]) -> Vec<isize> {
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

pub fn solve_laplace(
    mesh: &TriangleMesh,
    max_iter: Option<usize>,
    tolerance: Option<f64>,
) -> Result<Vec<f64>, String> {
    let max_iter = max_iter.unwrap_or(1000);
    let tolerance = tolerance.unwrap_or(1e-8);
    let n = mesh.vertices.len();

    // Build triple list directly instead of using Coo
    let mut triples: Vec<(usize, usize, f64)> = Vec::new();

    for tri in &mesh.triangles {
        let i = tri[0];
        let j = tri[1];
        let k = tri[2];
        let vi = mesh.vertices[i];
        let vj = mesh.vertices[j];
        let vk = mesh.vertices[k];

        let bi = vj.y - vk.y;
        let ci = vk.x - vj.x;
        let bj = vk.y - vi.y;
        let cj = vi.x - vk.x;
        let bk = vi.y - vj.y;
        let ck = vj.x - vi.x;

        let area2 = bi * cj - bj * ci;
        if area2.abs() < 1e-30 {
            continue;
        }
        let inv_area2 = 1.0 / area2.abs();

        let k_ii = (bi * bi + ci * ci) * inv_area2 * 0.5;
        let k_ij = (bi * bj + ci * cj) * inv_area2 * 0.5;
        let k_ik = (bi * bk + ci * ck) * inv_area2 * 0.5;
        let k_jj = (bj * bj + cj * cj) * inv_area2 * 0.5;
        let k_jk = (bj * bk + cj * ck) * inv_area2 * 0.5;
        let k_kk = (bk * bk + ck * ck) * inv_area2 * 0.5;

        triples.push((i, i, k_ii));
        triples.push((i, j, k_ij));
        triples.push((i, k, k_ik));
        triples.push((j, i, k_ij));
        triples.push((j, j, k_jj));
        triples.push((j, k, k_jk));
        triples.push((k, i, k_ik));
        triples.push((k, j, k_jk));
        triples.push((k, k, k_kk));
    }

    // Apply Dirichlet boundary conditions
    // For each boundary vertex i with value g: K_ii=1, K_ij=0 (j!=i), rhs_i=g
    // For interior rows j: subtract K_ji * g from rhs_j
    let mut is_boundary = vec![false; n];
    let mut boundary_value = vec![0.0_f64; n];
    for i in 0..n {
        match mesh.boundary_tags[i] {
            BoundaryTag::Outer => {
                is_boundary[i] = true;
                boundary_value[i] = 1.0;
            }
            BoundaryTag::Inner => {
                is_boundary[i] = true;
                boundary_value[i] = 0.0;
            }
            BoundaryTag::None => {}
        }
    }

    let mut rhs = DVector::from_element(n, 0.0);

    // Apply Dirichlet boundary conditions by filtering sparse entries.
    // Boundary rows become identity rows; interior rows have boundary
    // columns zeroed with RHS adjusted by -K_ij * g_j.
    // RHS adjustment is done only for upper-triangle entries (i <= j) to
    // avoid double-counting since each symmetric pair appears twice.
    let mut bdry_diag_done = vec![false; n];
    let mut filtered: Vec<(usize, usize, f64)> = Vec::new();

    for &(i, j, v) in &triples {
        if is_boundary[i] {
            if i == j && !bdry_diag_done[i] {
                filtered.push((i, i, 1.0));
                bdry_diag_done[i] = true;
            }
            if !is_boundary[j] && i <= j {
                rhs[j] -= v * boundary_value[i];
            }
        } else if is_boundary[j] {
            if j == i && !bdry_diag_done[j] {
                filtered.push((j, j, 1.0));
                bdry_diag_done[j] = true;
            }
            if i <= j {
                rhs[i] -= v * boundary_value[j];
            }
        } else {
            filtered.push((i, j, v));
        }
    }

    // Ensure all boundary vertices have a diagonal entry
    for i in 0..n {
        if is_boundary[i] && !bdry_diag_done[i] {
            filtered.push((i, i, 1.0));
            bdry_diag_done[i] = true;
        }
        if is_boundary[i] {
            rhs[i] = boundary_value[i];
        }
    }

    let mut coo = CooMatrix::new(n, n);
    for (i, j, v) in &filtered {
        coo.push(*i, *j, *v);
    }

    let k_mat = CsrMatrix::from(&coo);

    let u_vec = solve_conjugate_gradient(&k_mat, &rhs, tolerance, max_iter);

    Ok(u_vec.as_slice().to_vec())
}

pub fn compute_gradient_field(
    mesh: &TriangleMesh,
    u_field: &[f64],
) -> Result<Vec<[f64; 2]>, String> {
    if u_field.len() != mesh.vertices.len() {
        return Err(format!(
            "u_field length {} does not match vertex count {}",
            u_field.len(),
            mesh.vertices.len()
        ));
    }
    let mut gradients: Vec<[f64; 2]> = Vec::with_capacity(mesh.triangles.len());
    for tri in &mesh.triangles {
        let i = tri[0];
        let j = tri[1];
        let k = tri[2];
        let vi = mesh.vertices[i];
        let vj = mesh.vertices[j];
        let vk = mesh.vertices[k];

        let bi = vj.y - vk.y;
        let ci = vk.x - vj.x;
        let bj = vk.y - vi.y;
        let cj = vi.x - vk.x;
        let bk = vi.y - vj.y;
        let ck = vj.x - vi.x;

        let area2 = bi * cj - bj * ci;
        if area2.abs() < 1e-30 {
            gradients.push([0.0, 0.0]);
            continue;
        }
        let ui = u_field[i];
        let uj = u_field[j];
        let uk = u_field[k];

        let gx = (bi * ui + bj * uj + bk * uk) / area2;
        let gy = (ci * ui + cj * uj + ck * uk) / area2;
        gradients.push([gx, gy]);
    }
    Ok(gradients)
}

pub fn solve_laplace_with_history(
    mesh: &TriangleMesh,
    max_iter: Option<usize>,
    tolerance: Option<f64>,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let max_iter = max_iter.unwrap_or(1000);
    let tolerance = tolerance.unwrap_or(1e-8);
    let n = mesh.vertices.len();

    let mut triples: Vec<(usize, usize, f64)> = Vec::new();

    for tri in &mesh.triangles {
        let i = tri[0];
        let j = tri[1];
        let k = tri[2];
        let vi = mesh.vertices[i];
        let vj = mesh.vertices[j];
        let vk = mesh.vertices[k];

        let bi = vj.y - vk.y;
        let ci = vk.x - vj.x;
        let bj = vk.y - vi.y;
        let cj = vi.x - vk.x;
        let bk = vi.y - vj.y;
        let ck = vj.x - vi.x;

        let area2 = bi * cj - bj * ci;
        if area2.abs() < 1e-30 {
            continue;
        }
        let inv_area2 = 1.0 / area2.abs();

        let k_ii = (bi * bi + ci * ci) * inv_area2 * 0.5;
        let k_ij = (bi * bj + ci * cj) * inv_area2 * 0.5;
        let k_ik = (bi * bk + ci * ck) * inv_area2 * 0.5;
        let k_jj = (bj * bj + cj * cj) * inv_area2 * 0.5;
        let k_jk = (bj * bk + cj * ck) * inv_area2 * 0.5;
        let k_kk = (bk * bk + ck * ck) * inv_area2 * 0.5;

        triples.push((i, i, k_ii));
        triples.push((i, j, k_ij));
        triples.push((i, k, k_ik));
        triples.push((j, i, k_ij));
        triples.push((j, j, k_jj));
        triples.push((j, k, k_jk));
        triples.push((k, i, k_ik));
        triples.push((k, j, k_jk));
        triples.push((k, k, k_kk));
    }

    let mut is_boundary = vec![false; n];
    let mut boundary_value = vec![0.0_f64; n];
    for i in 0..n {
        match mesh.boundary_tags[i] {
            BoundaryTag::Outer => {
                is_boundary[i] = true;
                boundary_value[i] = 1.0;
            }
            BoundaryTag::Inner => {
                is_boundary[i] = true;
                boundary_value[i] = 0.0;
            }
            BoundaryTag::None => {}
        }
    }

    let mut rhs = DVector::from_element(n, 0.0);

    let mut bdry_diag_done = vec![false; n];
    let mut filtered: Vec<(usize, usize, f64)> = Vec::new();

    for &(i, j, v) in &triples {
        if is_boundary[i] {
            if i == j && !bdry_diag_done[i] {
                filtered.push((i, i, 1.0));
                bdry_diag_done[i] = true;
            }
            if !is_boundary[j] && i <= j {
                rhs[j] -= v * boundary_value[i];
            }
        } else if is_boundary[j] {
            if j == i && !bdry_diag_done[j] {
                filtered.push((j, j, 1.0));
                bdry_diag_done[j] = true;
            }
            if i <= j {
                rhs[i] -= v * boundary_value[j];
            }
        } else {
            filtered.push((i, j, v));
        }
    }

    for i in 0..n {
        if is_boundary[i] && !bdry_diag_done[i] {
            filtered.push((i, i, 1.0));
            bdry_diag_done[i] = true;
        }
        if is_boundary[i] {
            rhs[i] = boundary_value[i];
        }
    }

    let mut coo = CooMatrix::new(n, n);
    for (i, j, v) in &filtered {
        coo.push(*i, *j, *v);
    }

    let k_mat = CsrMatrix::from(&coo);

    let (u_vec, residuals) = solve_conjugate_gradient_with_history(
        &k_mat, &rhs, tolerance, max_iter,
    );

    Ok((u_vec.as_slice().to_vec(), residuals))
}

fn solve_conjugate_gradient_with_history(
    a: &CsrMatrix<f64>,
    b: &DVector<f64>,
    tol: f64,
    max_iter: usize,
) -> (DVector<f64>, Vec<f64>) {
    let n = b.len();
    let mut x = DVector::from_element(n, 0.0);
    let mut residuals: Vec<f64> = Vec::with_capacity(max_iter);

    let mut r = b - sparse_mat_vec_mul(a, &x);
    let mut p = r.clone();
    let mut rs_old = r.dot(&r);

    residuals.push(rs_old.sqrt());

    for _ in 0..max_iter {
        let ap = sparse_mat_vec_mul(a, &p);
        let p_dot_ap = p.dot(&ap);
        if p_dot_ap.abs() < 1e-30 {
            break;
        }
        let alpha = rs_old / p_dot_ap;
        x = &x + alpha * &p;
        r = &r - alpha * &ap;
        let rs_new = r.dot(&r);
        let res_norm = rs_new.sqrt();
        residuals.push(res_norm);
        if res_norm < tol {
            break;
        }
        p = &r + (rs_new / rs_old) * &p;
        rs_old = rs_new;
    }

    (x, residuals)
}

fn solve_conjugate_gradient(
    a: &CsrMatrix<f64>,
    b: &DVector<f64>,
    tol: f64,
    max_iter: usize,
) -> DVector<f64> {
    let n = b.len();
    let mut x = DVector::from_element(n, 0.0);

    let mut r = b - sparse_mat_vec_mul(a, &x);
    let mut p = r.clone();
    let mut rs_old = r.dot(&r);

    for _ in 0..max_iter {
        let ap = sparse_mat_vec_mul(a, &p);
        let p_dot_ap = p.dot(&ap);
        if p_dot_ap.abs() < 1e-30 {
            break;
        }
        let alpha = rs_old / p_dot_ap;
        x = &x + alpha * &p;
        r = &r - alpha * &ap;
        let rs_new = r.dot(&r);
        if rs_new.sqrt() < tol {
            break;
        }
        p = &r + (rs_new / rs_old) * &p;
        rs_old = rs_new;
    }

    x
}

fn sparse_mat_vec_mul(a: &CsrMatrix<f64>, v: &DVector<f64>) -> DVector<f64> {
    let offsets = a.row_offsets();
    let col_indices = a.col_indices();
    let values = a.values();
    let n = a.nrows();
    let mut result = DVector::from_element(n, 0.0);

    for row in 0..n {
        let start = offsets[row];
        let end = offsets[row + 1];
        let mut sum = 0.0;
        for idx in start..end {
            sum += values[idx] * v[col_indices[idx]];
        }
        result[row] = sum;
    }

    result
}
