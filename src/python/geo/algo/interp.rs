pyo3_stub_gen::module_doc!("raygeo.geo.algo.interp", "{}", MODULE_DOC_INTERP);

pub(crate) const MODULE_DOC_INTERP: &str = "\
Segment interpolation utilities for parameter-based point projection,
clipping, and scanline data slicing along 3D line segments.
";

use crate::geo::algo::interp::{
    barycentric_interpolate, compute_segment_delta_3d, compute_t_range,
    get_barycentric_weights, project_t_along_segment, slice_scanline_data,
    solve_quadratic,
};
use crate::python::geo::flex_point::tuple_to_point3d;
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
        barycentric_interpolate_py,
        barycentric_weights_py,
    );

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.interp", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    def compute_segment_delta_3d(
        start: tuple[float, float, float],
        end: tuple[float, float, float],
    ) -> tuple[float, float, float, float]:
        """Compute delta vector and squared length between two 3D points.

        :param start: Starting point (x, y, z).
        :param end: Ending point (x, y, z).
        :returns: (dx, dy, dz, len_sq).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "compute_segment_delta_3d")]
fn compute_segment_delta_py(
    start: (f64, f64, f64),
    end: (f64, f64, f64),
) -> (f64, f64, f64, f64) {
    let d = compute_segment_delta_3d(
        tuple_to_point3d(start),
        tuple_to_point3d(end),
    );
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
        :param delta: Segment delta from compute_segment_delta_3d.
        :returns: Parameter t clamped to [0, 1].
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "project_t_along_segment")]
fn project_t_along_segment_py(
    origin: (f64, f64, f64),
    point: (f64, f64, f64),
    delta: (f64, f64, f64, f64),
) -> f64 {
    let d = crate::geo::algo::interp::SegmentDelta {
        dx: delta.0,
        dy: delta.1,
        dz: delta.2,
        len_sq: delta.3,
    };
    project_t_along_segment(
        tuple_to_point3d(origin),
        tuple_to_point3d(point),
        &d,
    )
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
        :param delta: Segment delta from compute_segment_delta_3d.
        :returns: (t_start, t_end) in [0, 1].
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "compute_t_range")]
fn compute_t_range_py(
    origin: (f64, f64, f64),
    new_start: (f64, f64, f64),
    new_end: (f64, f64, f64),
    delta: (f64, f64, f64, f64),
) -> (f64, f64) {
    let d = crate::geo::algo::interp::SegmentDelta {
        dx: delta.0,
        dy: delta.1,
        dz: delta.2,
        len_sq: delta.3,
    };
    compute_t_range(
        tuple_to_point3d(origin),
        tuple_to_point3d(new_start),
        tuple_to_point3d(new_end),
        &d,
    )
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
        :complexity: O(n) time, O(n) space where n is the length of the data slice
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
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "solve_quadratic")]
fn solve_quadratic_py(a: f64, b: f64, c: f64) -> (Option<f64>, Option<f64>) {
    solve_quadratic(a, b, c)
}

#[gen_stub_pyfunction(
    python = r#"
    def barycentric_interpolate(
        p: tuple[float, float],
        va: tuple[float, float],
        vb: tuple[float, float],
        vc: tuple[float, float],
        ua: float,
        ub: float,
        uc: float,
    ) -> float:
        """Interpolate a scalar field at a point inside a triangle.

        Given triangle vertices (va, vb, vc) with scalar values
        (ua, ub, uc), returns the linearly interpolated value at point p
        using barycentric coordinates.

        :param p: Query point (x, y).
        :param va: First triangle vertex (x, y).
        :param vb: Second triangle vertex (x, y).
        :param vc: Third triangle vertex (x, y).
        :param ua: Scalar value at vertex a.
        :param ub: Scalar value at vertex b.
        :param uc: Scalar value at vertex c.
        :returns: Interpolated scalar value. Outside the triangle, the
            barycentric coordinates are clamped to [0, 1].
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "barycentric_interpolate")]
#[pyo3(signature = (p, va, vb, vc, ua, ub, uc))]
fn barycentric_interpolate_py(
    p: (f64, f64),
    va: (f64, f64),
    vb: (f64, f64),
    vc: (f64, f64),
    ua: f64,
    ub: f64,
    uc: f64,
) -> f64 {
    barycentric_interpolate(
        crate::geo::types::Point::new(p.0, p.1),
        crate::geo::types::Point::new(va.0, va.1),
        crate::geo::types::Point::new(vb.0, vb.1),
        crate::geo::types::Point::new(vc.0, vc.1),
        ua,
        ub,
        uc,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    def get_barycentric_weights(
        p: tuple[float, float],
        va: tuple[float, float],
        vb: tuple[float, float],
        vc: tuple[float, float],
    ) -> tuple[float, float, float]:
        """Compute raw barycentric coordinates for a point in a triangle.

        Returns (r, s, t) where r is the weight for va, s for vb, t for vc.
        Weights are unclamped — the point is strictly inside (or on the
        boundary of) the triangle iff all three are in [0, 1].

        :param p: Query point (x, y).
        :param va: First triangle vertex (x, y).
        :param vb: Second triangle vertex (x, y).
        :param vc: Third triangle vertex (x, y).
        :returns: Tuple (r, s, t) of raw barycentric coordinates.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.interp"
)]
#[pyfunction(name = "get_barycentric_weights")]
#[pyo3(signature = (p, va, vb, vc))]
fn barycentric_weights_py(
    p: (f64, f64),
    va: (f64, f64),
    vb: (f64, f64),
    vc: (f64, f64),
) -> (f64, f64, f64) {
    get_barycentric_weights(
        crate::geo::types::Point::new(p.0, p.1),
        crate::geo::types::Point::new(va.0, va.1),
        crate::geo::types::Point::new(vb.0, vb.1),
        crate::geo::types::Point::new(vc.0, vc.1),
    )
}
