//! 1D root-finding methods: bisection, secant, Illinois.
//!
//! Each solver returns (root, status, iteration_count).  The status tells
//! the caller whether convergence was achieved, the bracket was invalid,
//! or the iteration limit was reached.

/// Convergence / diagnostic status of a root-finding attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootStatus {
    /// `|f(root)| ≤ tol`
    Converged,
    /// The initial bracket does not straddle zero (`f(lo)·f(hi) > 0`)
    NoBracket,
    /// `max_iter` reached without convergence
    MaxIter,
    /// The iterate produced a non-finite value (NaN or ±∞)
    Diverged,
}

/// Bisection: guaranteed convergence for a valid bracket, linear rate
/// O(log((hi−lo)/tol)).
pub fn bisect<F: Fn(f64) -> f64>(
    f: F,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, RootStatus, usize) {
    let mut f_lo = f(lo);
    let f_hi = f(hi);
    if !f_lo.is_finite() || !f_hi.is_finite() {
        return (f64::NAN, RootStatus::Diverged, 0);
    }
    if f_lo == 0.0 {
        return (lo, RootStatus::Converged, 0);
    }
    if f_hi == 0.0 {
        return (hi, RootStatus::Converged, 0);
    }
    if f_lo.signum() == f_hi.signum() {
        return (f64::NAN, RootStatus::NoBracket, 0);
    }

    for iter in 1..=max_iter {
        let mid = (lo + hi) * 0.5;
        let f_mid = f(mid);
        if !f_mid.is_finite() {
            return (mid, RootStatus::Diverged, iter);
        }
        if f_mid == 0.0 || (hi - lo) * 0.5 <= tol {
            return (mid, RootStatus::Converged, iter);
        }
        if f_lo.signum() != f_mid.signum() {
            hi = mid;
        } else {
            lo = mid;
            f_lo = f_mid;
        }
    }
    let root = (lo + hi) * 0.5;
    (root, RootStatus::MaxIter, max_iter)
}

/// Secant: superlinear convergence when it works, may diverge.
///
/// Uses `x0` and `x1` as initial guesses (they need NOT bracket the root).
pub fn secant<F: Fn(f64) -> f64>(
    f: F,
    mut x0: f64,
    mut x1: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, RootStatus, usize) {
    let mut f0 = f(x0);
    if !f0.is_finite() {
        return (x0, RootStatus::Diverged, 0);
    }
    if f0 == 0.0 {
        return (x0, RootStatus::Converged, 0);
    }

    for iter in 1..=max_iter {
        let f1 = f(x1);
        if !f1.is_finite() {
            return (x1, RootStatus::Diverged, iter);
        }
        if f1 == 0.0 {
            return (x1, RootStatus::Converged, iter);
        }
        let denom = f1 - f0;
        if denom.abs() < f64::EPSILON {
            return (x1, RootStatus::Diverged, iter);
        }
        let x2 = x1 - f1 * (x1 - x0) / denom;
        if !x2.is_finite() {
            return (x1, RootStatus::Diverged, iter);
        }
        if (x2 - x1).abs() <= tol {
            return (x2, RootStatus::Converged, iter);
        }
        x0 = x1;
        f0 = f1;
        x1 = x2;
    }
    (x1, RootStatus::MaxIter, max_iter)
}

/// Illinois (safeguarded false-position): like secant but maintains a
/// bracket and uses the modified regula falsi trick to avoid stagnation.
///
/// Requires `f(lo)·f(hi) < 0`.
pub fn illinois<F: Fn(f64) -> f64>(
    f: F,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, RootStatus, usize) {
    let mut f_lo = f(lo);
    let mut f_hi = f(hi);
    if !f_lo.is_finite() || !f_hi.is_finite() {
        return (f64::NAN, RootStatus::Diverged, 0);
    }
    if f_lo == 0.0 {
        return (lo, RootStatus::Converged, 0);
    }
    if f_hi == 0.0 {
        return (hi, RootStatus::Converged, 0);
    }
    if f_lo.signum() == f_hi.signum() {
        return (f64::NAN, RootStatus::NoBracket, 0);
    }

    let mut side = 0i8;
    for iter in 1..=max_iter {
        let x = hi - f_hi * (hi - lo) / (f_hi - f_lo);
        if !x.is_finite() {
            return (x, RootStatus::Diverged, iter);
        }
        let fx = f(x);
        if !fx.is_finite() {
            return (x, RootStatus::Diverged, iter);
        }
        if fx == 0.0 || (hi - lo).abs() <= tol || fx.abs() <= tol {
            return (x, RootStatus::Converged, iter);
        }
        if fx.signum() == f_lo.signum() {
            lo = x;
            f_lo = fx;
            side = -1;
        } else {
            hi = x;
            f_hi = fx;
            if side == 1 {
                f_lo *= 0.5;
            }
            side = 1;
        }
    }
    let root = (lo + hi) * 0.5;
    (root, RootStatus::MaxIter, max_iter)
}

/// Bisection with iteration history.  Returns `(root, status, iters, estimates)`
/// where `estimates` contains the midpoint of each iteration.
pub fn bisect_tracked<F: Fn(f64) -> f64>(
    f: F,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, RootStatus, usize, Vec<f64>) {
    let mut estimates = Vec::new();
    let mut f_lo = f(lo);
    let f_hi = f(hi);
    if !f_lo.is_finite() || !f_hi.is_finite() {
        return (f64::NAN, RootStatus::Diverged, 0, estimates);
    }
    if f_lo == 0.0 {
        return (lo, RootStatus::Converged, 0, estimates);
    }
    if f_hi == 0.0 {
        return (hi, RootStatus::Converged, 0, estimates);
    }
    if f_lo.signum() == f_hi.signum() {
        return (f64::NAN, RootStatus::NoBracket, 0, estimates);
    }
    for iter in 1..=max_iter {
        let mid = (lo + hi) * 0.5;
        estimates.push(mid);
        let f_mid = f(mid);
        if !f_mid.is_finite() {
            return (mid, RootStatus::Diverged, iter, estimates);
        }
        if f_mid == 0.0 || (hi - lo) * 0.5 <= tol {
            return (mid, RootStatus::Converged, iter, estimates);
        }
        if f_lo.signum() != f_mid.signum() {
            hi = mid;
        } else {
            lo = mid;
            f_lo = f_mid;
        }
    }
    (lo, RootStatus::MaxIter, max_iter, estimates)
}

/// Secant with iteration history.  Returns `(root, status, iters, estimates)`
/// where `estimates` contains each iterate.
pub fn secant_tracked<F: Fn(f64) -> f64>(
    f: F,
    mut x0: f64,
    mut x1: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, RootStatus, usize, Vec<f64>) {
    let mut estimates = vec![x0, x1];
    let mut f0 = f(x0);
    if !f0.is_finite() {
        return (x0, RootStatus::Diverged, 0, estimates);
    }
    if f0 == 0.0 {
        return (x0, RootStatus::Converged, 1, estimates);
    }
    for iter in 1..=max_iter {
        let f1 = f(x1);
        if !f1.is_finite() {
            return (x1, RootStatus::Diverged, iter, estimates);
        }
        if f1 == 0.0 {
            return (x1, RootStatus::Converged, iter, estimates);
        }
        let denom = f1 - f0;
        if denom.abs() < f64::EPSILON {
            return (x1, RootStatus::Diverged, iter, estimates);
        }
        let x2 = x1 - f1 * (x1 - x0) / denom;
        if !x2.is_finite() {
            return (x1, RootStatus::Diverged, iter, estimates);
        }
        estimates.push(x2);
        if (x2 - x1).abs() <= tol {
            return (x2, RootStatus::Converged, iter, estimates);
        }
        x0 = x1;
        f0 = f1;
        x1 = x2;
    }
    (x1, RootStatus::MaxIter, max_iter, estimates)
}

/// Illinois with iteration history.  Returns `(root, status, iters, estimates)`
/// where `estimates` contains the estimate of each iteration.
pub fn illinois_tracked<F: Fn(f64) -> f64>(
    f: F,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, RootStatus, usize, Vec<f64>) {
    let mut estimates = Vec::new();
    let mut f_lo = f(lo);
    let mut f_hi = f(hi);
    if !f_lo.is_finite() || !f_hi.is_finite() {
        return (f64::NAN, RootStatus::Diverged, 0, estimates);
    }
    if f_lo == 0.0 {
        return (lo, RootStatus::Converged, 0, estimates);
    }
    if f_hi == 0.0 {
        return (hi, RootStatus::Converged, 0, estimates);
    }
    if f_lo.signum() == f_hi.signum() {
        return (f64::NAN, RootStatus::NoBracket, 0, estimates);
    }
    let mut side = 0i8;
    for iter in 1..=max_iter {
        let x = hi - f_hi * (hi - lo) / (f_hi - f_lo);
        if !x.is_finite() {
            return (x, RootStatus::Diverged, iter, estimates);
        }
        estimates.push(x);
        let fx = f(x);
        if !fx.is_finite() {
            return (x, RootStatus::Diverged, iter, estimates);
        }
        if fx == 0.0 || (hi - lo).abs() <= tol || fx.abs() <= tol {
            return (x, RootStatus::Converged, iter, estimates);
        }
        if fx.signum() == f_lo.signum() {
            lo = x;
            f_lo = fx;
            side = -1;
        } else {
            hi = x;
            f_hi = fx;
            if side == 1 {
                f_lo *= 0.5;
            }
            side = 1;
        }
    }
    ((lo + hi) * 0.5, RootStatus::MaxIter, max_iter, estimates)
}
