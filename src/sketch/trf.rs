//! Trust-region-reflective least squares (no bounds, exact SVD path).
//!
//! A faithful port of SciPy's `least_squares(method="trf")` inner
//! algorithm for unconstrained problems, including the More-Sorensen
//! trust-region subproblem solver and the termination criteria, so
//! that convergence behaviour matches the reference implementation.

use nalgebra::{DMatrix, DVector, SVD};

const EPS: f64 = f64::EPSILON;

/// Result of the trust-region-reflective solve.
#[derive(Clone, Debug)]
pub struct TrfResult {
    /// Solution vector.
    pub x: Vec<f64>,
    /// Final cost `0.5 * ||f||^2`.
    pub cost: f64,
    /// Termination status (`> 0` means success, mirroring SciPy:
    /// 1 gtol, 2 ftol, 3 xtol, 4 both, 0 max_nfev reached).
    pub status: i32,
    /// Number of residual function evaluations.
    pub nfev: usize,
}

/// Residual function: state vector -> residuals.
pub type ResidualFn<'a> = Box<dyn FnMut(&[f64]) -> Vec<f64> + 'a>;

/// Jacobian provider: state vector -> row-major (m x n) matrix.
pub type JacobianFn<'a> = Box<dyn FnMut(&[f64]) -> Vec<f64> + 'a>;

/// The residual function and Jacobian provider closures used by the
/// solver.
pub struct TrfProblem<'a> {
    /// Residual function.
    pub fun: ResidualFn<'a>,
    /// Jacobian provider.
    pub jac: JacobianFn<'a>,
    /// Number of residuals the Jacobian provides (m).
    pub m: usize,
}

/// Solves an unconstrained nonlinear least-squares problem with the
/// trust-region-reflective algorithm (exact SVD trust-region solver,
/// unit variable scaling), mirroring
/// `scipy.optimize.least_squares(method="trf")` without bounds.
pub fn solve_trf_no_bounds(
    problem: TrfProblem<'_>,
    x0: &[f64],
    ftol: f64,
    xtol: f64,
    gtol: f64,
    max_nfev: usize,
) -> TrfResult {
    let n = x0.len();
    let m = problem.m;
    let mut fun = problem.fun;
    let mut jac = problem.jac;

    let mut x = x0.to_vec();
    let mut f = fun(&x);
    let mut nfev = 1usize;

    let mut cost = 0.5 * dot(&f, &f);

    let mut j_mat = DMatrix::from_row_slice(m, n, &jac(&x));
    let mut g = compute_grad(&j_mat, &f);

    let mut delta = norm(x0);
    if delta == 0.0 {
        delta = 1.0;
    }

    let mut alpha = 0.0f64;
    let mut termination_status: Option<i32> = None;

    loop {
        let g_norm = g.iter().fold(0.0f64, |acc, v| acc.max(v.abs()));
        if g_norm < gtol {
            termination_status = Some(1);
        }
        if termination_status.is_some() || nfev == max_nfev {
            break;
        }

        // Exact solver path with unit scaling: J_h = J, g_h = g.
        // nalgebra stores V transposed (like LAPACK); restore the
        // SciPy convention `V = VT.T` (n x k, orthonormal columns).
        let svd = SVD::new(j_mat.clone(), true, true);
        let u = svd.u.expect("U requested");
        let v_t = svd.v_t.expect("V requested");
        let v = v_t.transpose();
        let s = svd.singular_values;
        let uf = u.tr_mul(&DVector::from_vec(f.clone()));

        let mut actual_reduction = -1.0f64;
        let mut x_new = x.clone();
        let mut f_new = f.clone();
        let mut cost_new = cost;
        while actual_reduction <= 0.0 && nfev < max_nfev {
            let (step_h, alpha_new) =
                solve_lsq_trust_region(n, m, &uf, &s, &v, delta, Some(alpha));
            alpha = alpha_new;

            let predicted_reduction = -evaluate_quadratic(&j_mat, &g, &step_h);
            let step = step_h.clone();
            x_new = add(&x, &step);
            f_new = fun(&x_new);
            nfev += 1;

            let step_h_norm = norm(&step_h);

            if f_new.iter().any(|v| !v.is_finite()) {
                delta = 0.25 * step_h_norm;
                continue;
            }

            cost_new = 0.5 * dot(&f_new, &f_new);
            actual_reduction = cost - cost_new;

            let (delta_new, ratio) = update_tr_radius(
                delta,
                actual_reduction,
                predicted_reduction,
                step_h_norm,
                step_h_norm > 0.95 * delta,
            );

            let step_norm = norm(&step);
            termination_status = check_termination(
                actual_reduction,
                cost,
                step_norm,
                norm(&x),
                ratio,
                ftol,
                xtol,
            );
            if termination_status.is_some() {
                break;
            }

            alpha *= delta / delta_new;
            delta = delta_new;
        }

        if actual_reduction > 0.0 {
            x = x_new;
            f = f_new;
            cost = cost_new;
            j_mat = DMatrix::from_row_slice(m, n, &jac(&x));
            g = compute_grad(&j_mat, &f);
        }

        // Non-improving iterations reset the reduction (mirrors the
        // reference implementation); the loop continues.
        let _ = actual_reduction;
    }

    TrfResult {
        x,
        cost,
        status: termination_status.unwrap_or(0),
        nfev,
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn norm(a: &[f64]) -> f64 {
    dot(a, a).sqrt()
}

fn add(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
}

fn compute_grad(j: &DMatrix<f64>, f: &[f64]) -> Vec<f64> {
    let fv = DVector::from_vec(f.to_vec());
    j.tr_mul(&fv).data.as_vec().clone()
}

/// `0.5 * ||J s||^2 + g . s`.
fn evaluate_quadratic(j: &DMatrix<f64>, g: &[f64], s: &[f64]) -> f64 {
    let sv = DVector::from_vec(s.to_vec());
    let js = j * sv;
    let q: f64 = js.iter().map(|v| v * v).sum();
    let l: f64 = s.iter().zip(g.iter()).map(|(a, b)| a * b).sum();
    0.5 * q + l
}

/// Updates the trust-region radius from step quality.  Returns the
/// new radius and the actual/predicted reduction ratio.
fn update_tr_radius(
    delta: f64,
    actual_reduction: f64,
    predicted_reduction: f64,
    step_norm: f64,
    bound_hit: bool,
) -> (f64, f64) {
    let ratio = if predicted_reduction > 0.0 {
        actual_reduction / predicted_reduction
    } else if predicted_reduction == actual_reduction && actual_reduction == 0.0
    {
        1.0
    } else {
        0.0
    };

    let mut new_delta = delta;
    if ratio < 0.25 {
        new_delta = 0.25 * step_norm;
    } else if ratio > 0.75 && bound_hit {
        new_delta *= 2.0;
    }

    (new_delta, ratio)
}

/// Termination check of the nonlinear least-squares loop.  Returns
/// `Some(status)` when converged.
fn check_termination(
    d_f: f64,
    cost: f64,
    dx_norm: f64,
    x_norm: f64,
    ratio: f64,
    ftol: f64,
    xtol: f64,
) -> Option<i32> {
    let ftol_satisfied = d_f < ftol * cost && ratio > 0.25;
    let xtol_satisfied = dx_norm < xtol * (xtol + x_norm);

    if ftol_satisfied && xtol_satisfied {
        Some(4)
    } else if ftol_satisfied {
        Some(2)
    } else if xtol_satisfied {
        Some(3)
    } else {
        None
    }
}

fn phi_and_derivative(
    alpha: f64,
    suf: &DVector<f64>,
    s: &DVector<f64>,
    delta: f64,
) -> (f64, f64) {
    let denom = s.zip_map(s, |a, b| a * b).add_scalar(alpha);
    let p_norm = suf.component_div(&denom).norm();
    let phi = p_norm - delta;
    let suf_sq = suf.zip_map(suf, |a, b| a * b);
    let denom_cu = denom.zip_map(&denom, |a, _| a * a * a);
    let phi_prime = -suf_sq.component_div(&denom_cu).sum() / p_norm;
    (phi, phi_prime)
}

/// Solves the trust-region subproblem arising in least-squares
/// minimization (More's method via a single SVD, as used by MINPACK
/// and SciPy).
fn solve_lsq_trust_region(
    n: usize,
    m: usize,
    uf: &DVector<f64>,
    s: &DVector<f64>,
    v: &DMatrix<f64>,
    delta: f64,
    initial_alpha: Option<f64>,
) -> (Vec<f64>, f64) {
    // Stopping tolerance and iteration cap of the root-finding
    // procedure, mirroring the SciPy defaults.
    const RTOL: f64 = 0.01;
    const MAX_ITER: usize = 10;
    let suf = s.zip_map(uf, |a, b| a * b);

    let full_rank = if m >= n {
        s[s.len() - 1] > EPS * m as f64 * s[0]
    } else {
        false
    };

    if full_rank {
        let step = -(v * DVector::from_iterator(
            s.len(),
            (0..s.len()).map(|i| uf[i] / s[i]),
        ));
        if step.norm() <= delta {
            return (step.data.as_vec().clone(), 0.0);
        }
    }

    let suf_norm = suf.norm();
    let mut alpha_upper = suf_norm / delta;

    let mut alpha_lower = 0.0f64;
    if full_rank {
        let (phi, phi_prime) = phi_and_derivative(0.0, &suf, s, delta);
        alpha_lower = -phi / phi_prime;
    }

    let mut alpha = match initial_alpha {
        None => (0.001 * alpha_upper).max((alpha_lower * alpha_upper).sqrt()),
        Some(a) if !full_rank && a == 0.0 => {
            (0.001 * alpha_upper).max((alpha_lower * alpha_upper).sqrt())
        }
        Some(a) => a,
    };

    for _ in 0..MAX_ITER {
        if alpha < alpha_lower || alpha > alpha_upper {
            alpha =
                (0.001 * alpha_upper).max((alpha_lower * alpha_upper).sqrt());
        }

        let (phi, phi_prime) = phi_and_derivative(alpha, &suf, s, delta);

        if phi < 0.0 {
            alpha_upper = alpha;
        }

        let ratio = phi / phi_prime;
        alpha_lower = alpha_lower.max(alpha - ratio);
        alpha -= (phi + delta) * ratio / delta;

        if phi.abs() < RTOL * delta {
            break;
        }
    }

    let denom = s.zip_map(s, |a, b| a * b).add_scalar(alpha);
    let mut p = -(v * suf.component_div(&denom));

    let p_norm = p.norm();
    p *= delta / p_norm;

    (p.data.as_vec().clone(), alpha)
}
