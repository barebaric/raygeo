pyo3_stub_gen::module_doc!("raygeo.geo.algo.interp", "{}", MODULE_DOC_INTERP);

pub(crate) const MODULE_DOC_INTERP: &str = "\
Segment interpolation utilities for parameter-based point projection,
clipping, and scanline data slicing along 3D line segments.
";

use crate::geo::algo::interp::{
    compute_segment_delta, compute_t_range, project_t_along_segment,
    slice_scanline_data, solve_quadratic,
};
use crate::types::Point3D;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "interp")?;

    register_functions!(
        m,
        compute_segment_delta_py,
        project_t_along_segment_py,
        compute_t_range_py,
        slice_scanline_data_py,
        solve_quadratic_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    def compute_segment_delta(
        start: tuple[float, float, float],
        end: tuple[float, float, float],
    ) -> tuple[float, float, float, float]:
        """Compute delta vector and squared length between two 3D points.

        :param start: Starting point (x, y, z).
        :param end: Ending point (x, y, z).
        :returns: (dx, dy, dz, len_sq).
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "compute_segment_delta")]
fn compute_segment_delta_py(
    start: Point3D,
    end: Point3D,
) -> (f64, f64, f64, f64) {
    let d = compute_segment_delta(start, end);
    (d.dx, d.dy, d.dz, d.len_sq)
}

#[gen_stub_pyfunction(
    python = r#"
    def project_t_along_segment(
        origin: tuple[float, float, float],
        point: tuple[float, float, float],
        delta: tuple[float, float, float, float],
    ) -> float:
        """Project a point onto a line segment, returning t in [0, 1].

        :param origin: Start of segment (x, y, z).
        :param point: Point to project (x, y, z).
        :param delta: Segment delta from compute_segment_delta.
        :returns: Parameter t clamped to [0, 1].
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "project_t_along_segment")]
fn project_t_along_segment_py(
    origin: Point3D,
    point: Point3D,
    delta: (f64, f64, f64, f64),
) -> f64 {
    let d = crate::geo::algo::interp::SegmentDelta {
        dx: delta.0,
        dy: delta.1,
        dz: delta.2,
        len_sq: delta.3,
    };
    project_t_along_segment(origin, point, &d)
}

#[gen_stub_pyfunction(
    python = r#"
    def compute_t_range(
        origin: tuple[float, float, float],
        new_start: tuple[float, float, float],
        new_end: tuple[float, float, float],
        delta: tuple[float, float, float, float],
    ) -> tuple[float, float]:
        """Compute parameter range (t_start, t_end) for a clipped sub-segment.

        :param origin: Start of original segment (x, y, z).
        :param new_start: Start of clipped sub-segment (x, y, z).
        :param new_end: End of clipped sub-segment (x, y, z).
        :param delta: Segment delta from compute_segment_delta.
        :returns: (t_start, t_end) in [0, 1].
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "compute_t_range")]
fn compute_t_range_py(
    origin: Point3D,
    new_start: Point3D,
    new_end: Point3D,
    delta: (f64, f64, f64, f64),
) -> (f64, f64) {
    let d = crate::geo::algo::interp::SegmentDelta {
        dx: delta.0,
        dy: delta.1,
        dz: delta.2,
        len_sq: delta.3,
    };
    compute_t_range(origin, new_start, new_end, &d)
}

#[gen_stub_pyfunction(
    python = r#"
    def slice_scanline_data(
        data: list[int],
        t_start: float,
        t_end: float,
    ) -> list[int]:
        """Slice a scanline power array by parameter range [t_start, t_end).

        :param data: Full scanline power values.
        :param t_start: Start parameter in [0, 1].
        :param t_end: End parameter in [0, 1].
        :returns: Sliced power values.
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "slice_scanline_data")]
fn slice_scanline_data_py(data: Vec<u8>, t_start: f64, t_end: f64) -> Vec<i32> {
    slice_scanline_data(&data, t_start, t_end)
        .into_iter()
        .map(|v| v as i32)
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    def solve_quadratic(
        a: float,
        b: float,
        c: float,
    ) -> tuple[float | None, float | None]:
        """Solve quadratic equation a x^2 + b x + c = 0.

        :param a: Quadratic coefficient.
        :param b: Linear coefficient.
        :param c: Constant term.
        :returns: (root1, root2), each None if no real root.
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "solve_quadratic")]
fn solve_quadratic_py(a: f64, b: f64, c: f64) -> (Option<f64>, Option<f64>) {
    solve_quadratic(a, b, c)
}
