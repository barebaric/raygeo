pyo3_stub_gen::module_doc!("raygeo.geo.algo.smooth", "{}", MODULE_DOC_SMOOTH);

pub(crate) const MODULE_DOC_SMOOTH: &str = "\
Polyline smoothing using Gaussian kernels.

Provides Gaussian kernel computation and circular/linear polyline
smoothing with configurable corner angle thresholds to preserve
sharp features.
";

use super::super::flex_point::{points3d_to_tuples, PyPoint3D};
use crate::geo::algo::smooth::{
    compute_gaussian_kernel, resample_polyline, smooth_circularly, smooth_path,
    smooth_polyline, smooth_sub_segment,
};
use crate::types::{Point, Point3D};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "smooth")?;
    m.setattr("__doc__", MODULE_DOC_SMOOTH)?;

    register_functions!(
        m,
        resample_polyline_py,
        compute_gaussian_kernel_py,
        smooth_circularly_py,
        smooth_polyline_algo_py,
        smooth_sub_segment_py,
        smooth_path_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def resample_polyline(
        points: collections.abc.Sequence[types.Point3D],
        max_segment_length: float,
        is_closed: bool,
    ) -> list[types.Point3D]:
        """Resample a polyline with a maximum segment length.

        :param points: Sequence of 3D points.
        :param max_segment_length: Maximum allowed segment length.
        :param is_closed: Whether the polyline is closed.
        :returns: Resampled points.
        :complexity: O(n) time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "resample_polyline")]
fn resample_polyline_py(
    points: Vec<PyPoint3D>,
    max_segment_length: f64,
    is_closed: bool,
) -> Vec<(f64, f64, f64)> {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    let mut out = Vec::new();
    resample_polyline(&pts, max_segment_length, is_closed, &mut out);
    points3d_to_tuples(out)
}

#[gen_stub_pyfunction(
    python = r#"
    def compute_gaussian_kernel(
        amount: int,
    ) -> tuple[list[float], float]:
        """Compute a Gaussian kernel of the given size.

        :param amount: Kernel size.
        :returns: Tuple of (kernel_values, sigma).
        :complexity: O(k) time, O(k) space
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "compute_gaussian_kernel")]
fn compute_gaussian_kernel_py(amount: i32) -> (Vec<f64>, f64) {
    compute_gaussian_kernel(amount)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def smooth_circularly(
        points: collections.abc.Sequence[types.Point3D],
        kernel: collections.abc.Sequence[float],
    ) -> list[types.Point3D]:
        """Smooth a closed polyline circularly.

        :param points: Sequence of 3D points to smooth.
        :param kernel: Gaussian kernel values.
        :returns: Smoothed points.
        :complexity: O(n * k) time, O(n) space where k is the kernel size and n the number of points
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "smooth_circularly")]
fn smooth_circularly_py(
    points: Vec<PyPoint3D>,
    kernel: Vec<f64>,
) -> Vec<(f64, f64, f64)> {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    let mut out = Vec::new();
    smooth_circularly(&pts, &kernel, &mut out);
    points3d_to_tuples(out)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def smooth_polyline(
        points: collections.abc.Sequence[types.Point3D],
        amount: int,
        corner_angle_threshold: float,
        is_closed: typing.Optional[bool] = None,
    ) -> list[types.Point3D]:
        """Smooth a polyline using Gaussian smoothing.

        :param points: Sequence of 3D points to smooth.
        :param amount: Smoothing amount (kernel size).
        :param corner_angle_threshold: Angle threshold for preserving corners.
        :param is_closed: Whether the polyline is closed.
        :returns: Smoothed points.
        :complexity: O(n * k) time, O(n) space where k is the kernel size and n the number of points
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "smooth_polyline")]
#[pyo3(signature = (points, amount, corner_angle_threshold, is_closed=None))]
fn smooth_polyline_algo_py(
    points: Vec<PyPoint3D>,
    amount: i32,
    corner_angle_threshold: f64,
    is_closed: Option<bool>,
) -> Vec<(f64, f64, f64)> {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    points3d_to_tuples(smooth_polyline(
        &pts,
        amount,
        corner_angle_threshold,
        is_closed,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def smooth_sub_segment(
        points: collections.abc.Sequence[types.Point3D],
        kernel: collections.abc.Sequence[float],
    ) -> list[types.Point3D]:
        """Smooth a sub-segment of a polyline.

        :param points: Sequence of 3D points to smooth.
        :param kernel: Gaussian kernel values.
        :returns: Smoothed points.
        :complexity: O(n * k) time, O(n) space where k is the kernel size and n the number of points
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "smooth_sub_segment")]
fn smooth_sub_segment_py(
    points: Vec<PyPoint3D>,
    kernel: Vec<f64>,
) -> Vec<(f64, f64, f64)> {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    let mut out = Vec::new();
    smooth_sub_segment(&pts, &kernel, &mut out);
    points3d_to_tuples(out)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def smooth_path(
        points: collections.abc.Sequence[tuple[float, float]],
        obstacles: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        clearance: float,
        smoothing_amount: int = 50,
    ) -> list[tuple[float, float]]:
        """Smooth a polyline while avoiding obstacles.

        Two-phase constrained smoothing:

        1. **Shortcut** – greedily removes intermediate waypoints whose
           direct connection stays clear of all *obstacles* by at least
           *clearance*.
        2. **Gaussian relaxation** – iteratively applies Gaussian smoothing,
           reverting any point whose smoothed position would violate the
           clearance constraint.

        Endpoints are always preserved.

        :param points: Polyline as a list of (x, y) tuples.
        :param obstacles: List of obstacle polygons (each a list of (x, y)).
        :param clearance: Minimum distance the path must keep from obstacles.
        :param smoothing_amount: Gaussian smoothing amount 0–200 (default 50).
                                 0 applies shortcut only.
        :returns: Smoothed polyline as a list of (x, y) tuples.
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "smooth_path")]
#[pyo3(signature = (points, obstacles, clearance, smoothing_amount = 50))]
fn smooth_path_py(
    points: Vec<(f64, f64)>,
    obstacles: Vec<Vec<(f64, f64)>>,
    clearance: f64,
    smoothing_amount: i32,
) -> Vec<(f64, f64)> {
    let pts: Vec<Point> =
        points.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let obs: Vec<Vec<Point>> = obstacles
        .into_iter()
        .map(|poly| poly.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    smooth_path(&pts, &obs, clearance, smoothing_amount)
        .into_iter()
        .map(|p| (p.x, p.y))
        .collect()
}
