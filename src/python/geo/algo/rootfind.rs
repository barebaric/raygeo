pyo3_stub_gen::module_doc!("raygeo.geo.algo.rootfind", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
1D root-finding methods: bisection, secant, Illinois.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::rootfind;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "rootfind")?;
    m.setattr(
        "__doc__",
        "1D root-finding methods: bisection, secant, Illinois.",
    )?;
    register_functions!(
        m,
        bisect_py,
        bracket_grid_py,
        secant_py,
        illinois_py,
        bisect_tracked_py,
        secant_tracked_py,
        illinois_tracked_py,
    );
    algo_mod.add_submodule(&m)?;
    Ok(())
}

fn call_f(f: &Bound<'_, PyAny>, x: f64) -> f64 {
    f.call1((x,)).unwrap().extract::<f64>().unwrap_or(f64::NAN)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def bisect(
        f: typing.Callable[[float], float],
        lo: float,
        hi: float,
        tol: float = 1e-10,
        max_iter: int = 100,
    ) -> tuple[float, str, int]:
        """Bisection root-finding.

        :param f: Function to find the root of (takes float, returns float).
        :param lo: Lower bound of the search interval.
        :param hi: Upper bound of the search interval.
        :param tol: Convergence tolerance (default 1e-10).
        :param max_iter: Maximum iterations (default 100).
        :returns: ``(root, status_string, iteration_count)``.
        """
    "#,
    module = "raygeo.geo.algo.rootfind"
)]
#[pyfunction(name = "bisect")]
fn bisect_py(
    f: Bound<'_, PyAny>,
    lo: f64,
    hi: f64,
    tol: f64,
    max_iter: usize,
) -> PyResult<(f64, String, usize)> {
    let (r, s, i) = rootfind::bisect(|x| call_f(&f, x), lo, hi, tol, max_iter);
    Ok((r, format!("{s:?}"), i))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def secant(
        f: typing.Callable[[float], float],
        x0: float,
        x1: float,
        tol: float = 1e-10,
        max_iter: int = 100,
    ) -> tuple[float, str, int]:
        """Secant root-finding.

        :param f: Function to find the root of (takes float, returns float).
        :param x0: First initial guess.
        :param x1: Second initial guess.
        :param tol: Convergence tolerance (default 1e-10).
        :param max_iter: Maximum iterations (default 100).
        :returns: ``(root, status_string, iteration_count)``.
        """
    "#,
    module = "raygeo.geo.algo.rootfind"
)]
#[pyfunction(name = "secant")]
fn secant_py(
    f: Bound<'_, PyAny>,
    x0: f64,
    x1: f64,
    tol: f64,
    max_iter: usize,
) -> PyResult<(f64, String, usize)> {
    let (r, s, i) = rootfind::secant(|x| call_f(&f, x), x0, x1, tol, max_iter);
    Ok((r, format!("{s:?}"), i))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def illinois(
        f: typing.Callable[[float], float],
        lo: float,
        hi: float,
        tol: float = 1e-10,
        max_iter: int = 100,
    ) -> tuple[float, str, int]:
        """Illinois (safeguarded false-position) root-finding.

        :param f: Function to find the root of (takes float, returns float).
        :param lo: Lower bound of the search interval.
        :param hi: Upper bound of the search interval.
        :param tol: Convergence tolerance (default 1e-10).
        :param max_iter: Maximum iterations (default 100).
        :returns: ``(root, status_string, iteration_count)``.
        """
    "#,
    module = "raygeo.geo.algo.rootfind"
)]
#[pyfunction(name = "illinois")]
fn illinois_py(
    f: Bound<'_, PyAny>,
    lo: f64,
    hi: f64,
    tol: f64,
    max_iter: usize,
) -> PyResult<(f64, String, usize)> {
    let (r, s, i) =
        rootfind::illinois(|x| call_f(&f, x), lo, hi, tol, max_iter);
    Ok((r, format!("{s:?}"), i))
}

#[gen_stub_pyfunction(
    python = r#"
    def bisect_tracked(
        f, lo: float, hi: float, tol: float = 1e-10, max_iter: int = 100
    ) -> tuple[float, str, int, list[float]]:
        """Tracked bisection root-finding.

        :param f: Function to find the root of.
        :param lo: Lower bound of the search interval.
        :param hi: Upper bound of the search interval.
        :param tol: Convergence tolerance.
        :param max_iter: Maximum number of iterations.
        :returns: ``(root, status_string, iteration_count, history)``.
        """
    "#,
    module = "raygeo.geo.algo.rootfind"
)]
#[pyfunction(name = "bisect_tracked")]
fn bisect_tracked_py(
    f: Bound<'_, PyAny>,
    lo: f64,
    hi: f64,
    tol: f64,
    max_iter: usize,
) -> PyResult<(f64, String, usize, Vec<f64>)> {
    let (r, s, i, e) =
        rootfind::bisect_tracked(|x| call_f(&f, x), lo, hi, tol, max_iter);
    Ok((r, format!("{s:?}"), i, e))
}

#[gen_stub_pyfunction(
    python = r#"
    def secant_tracked(
        f, x0: float, x1: float, tol: float = 1e-10, max_iter: int = 100
    ) -> tuple[float, str, int, list[float]]:
        """Tracked secant method root-finding.

        :param f: Function to find the root of.
        :param x0: First initial guess.
        :param x1: Second initial guess.
        :param tol: Convergence tolerance.
        :param max_iter: Maximum number of iterations.
        :returns: ``(root, status_string, iteration_count, history)``.
        """
    "#,
    module = "raygeo.geo.algo.rootfind"
)]
#[pyfunction(name = "secant_tracked")]
fn secant_tracked_py(
    f: Bound<'_, PyAny>,
    x0: f64,
    x1: f64,
    tol: f64,
    max_iter: usize,
) -> PyResult<(f64, String, usize, Vec<f64>)> {
    let (r, s, i, e) =
        rootfind::secant_tracked(|x| call_f(&f, x), x0, x1, tol, max_iter);
    Ok((r, format!("{s:?}"), i, e))
}

#[gen_stub_pyfunction(
    python = r#"
    def illinois_tracked(
        f, lo: float, hi: float, tol: float = 1e-10, max_iter: int = 100
    ) -> tuple[float, str, int, list[float]]:
        """Tracked Illinois method root-finding.

        :param f: Function to find the root of.
        :param lo: Lower bound of the search interval.
        :param hi: Upper bound of the search interval.
        :param tol: Convergence tolerance.
        :param max_iter: Maximum number of iterations.
        :returns: ``(root, status_string, iteration_count, history)``.
        """
    "#,
    module = "raygeo.geo.algo.rootfind"
)]
#[pyfunction(name = "illinois_tracked")]
fn illinois_tracked_py(
    f: Bound<'_, PyAny>,
    lo: f64,
    hi: f64,
    tol: f64,
    max_iter: usize,
) -> PyResult<(f64, String, usize, Vec<f64>)> {
    let (r, s, i, e) =
        rootfind::illinois_tracked(|x| call_f(&f, x), lo, hi, tol, max_iter);
    Ok((r, format!("{s:?}"), i, e))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def bracket_grid(
        f: typing.Callable[[float], float],
        heading: float,
        max_deflection: float,
    ) -> tuple[float, str, int]:
        """7-sample angular grid search with linear interpolation.

        Samples *f* at ``heading + max_deflection * ratio`` for
        7 ratios evenly spaced across ``[-1, -0.6, -0.2, 0, 0.2, 0.6, 1.0]``.
        When a sign change is found between adjacent samples the root is
        linearly interpolated.  Falls back to the sample with smallest
        absolute error.

        :param f: Error function *f(angle) -> error*.
        :param heading: Centre angle in radians.
        :param max_deflection: Maximum angular spread in radians.
        :returns: ``(root, status_string, sample_count)``.
        """
    "#,
    module = "raygeo.geo.algo.rootfind"
)]
#[pyfunction(name = "bracket_grid")]
fn bracket_grid_py(
    f: Bound<'_, PyAny>,
    heading: f64,
    max_deflection: f64,
) -> PyResult<(f64, String, usize)> {
    let (r, s, i) =
        rootfind::bracket_grid(heading, max_deflection, |x| call_f(&f, x));
    Ok((r, format!("{s:?}"), i))
}
