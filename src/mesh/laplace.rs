use nalgebra::DVector;
use nalgebra_sparse::coo::CooMatrix;
use nalgebra_sparse::csr::CsrMatrix;

use super::types::{BoundaryTag, TriangleMesh};

pub fn solve_laplace(
    mesh: &TriangleMesh,
    max_iter: Option<usize>,
    tolerance: Option<f64>,
) -> Result<Vec<f64>, String> {
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

    let u_vec = solve_conjugate_gradient(&k_mat, &rhs, tolerance, max_iter);

    Ok(u_vec.as_slice().to_vec())
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
