pyo3_stub_gen::module_doc!("raygeo.geo.algo", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Geometric algorithms for path processing.

This module provides algorithms that operate on geometry paths and point
sequences. It covers several categories of geometric processing:

Clipping — intersect and clip line segments against rectangles and
polygon regions. Also includes coordinate conversion between floating
point and Clipper's integer grid system for boolean-accuracy clipping.

Fitting — reconstruct curves (arcs, lines, beziers) from unordered
point sequences. Includes recursive primitive fitting, circle fitting,
polyline linearization, and deviation analysis to evaluate fit quality.

Minkowski sums — compute Minkowski sums, convolutions, and no-fit
polygons for 2D toolpath generation, nesting, and packing algorithms.

Simplification — reduce the number of points in a polyline while
preserving shape within a tolerance (Ramer-Douglas-Peucker).

Smoothing — apply Gaussian kernel smoothing to polylines with
configurable corner-angle thresholds to preserve sharp features.
";

pyo3_stub_gen::module_doc!("raygeo.geo.algo.analysis", "{}", MODULE_DOC_ANALYSIS);

pub(crate) const MODULE_DOC_ANALYSIS: &str = "\
Path analysis utilities for inspecting and cleaning geometry data.

Provides functions for removing duplicate points from point sequences
and extracting individual points from path data.
";

pyo3_stub_gen::module_doc!("raygeo.geo.algo.clipping", "{}", MODULE_DOC_CLIPPING);

pub(crate) const MODULE_DOC_CLIPPING: &str = "\
Line and polygon clipping operations.

Provides functions for clipping line segments against rectangles and
polygon regions, as well as converting between float and Clipper
integer coordinate systems.
";

pyo3_stub_gen::module_doc!("raygeo.geo.algo.fitting", "{}", MODULE_DOC_FITTING);

pub(crate) const MODULE_DOC_FITTING: &str = "\
Curve and primitive fitting algorithms.

Provides functions for fitting arcs, lines, circles, and beziers to
point sequences. Includes recursive fitting with primitives, polyline
linearization, and evaluating fitting quality (line and arc deviation).
";

pyo3_stub_gen::module_doc!("raygeo.geo.algo.minkowski", "{}", MODULE_DOC_MINKOWSKI);

pub(crate) const MODULE_DOC_MINKOWSKI: &str = "\
Minkowski sum operations for 2D polygon toolpath generation.

Provides convolution of point sequences and segments, Minkowski sums
for convex polygons, and no-fit polygon / inner fit polygon calculations
used in nesting and packing algorithms.
";

pyo3_stub_gen::module_doc!("raygeo.geo.algo.simplify", "{}", MODULE_DOC_SIMPLIFY);

pub(crate) const MODULE_DOC_SIMPLIFY: &str = "\
Polyline simplification using the Ramer-Douglas-Peucker algorithm.

Reduces the number of points in a polyline while preserving the overall
shape within a given tolerance.
";

pyo3_stub_gen::module_doc!("raygeo.geo.algo.smooth", "{}", MODULE_DOC_SMOOTH);

pub(crate) const MODULE_DOC_SMOOTH: &str = "\
Polyline smoothing using Gaussian kernels.

Provides Gaussian kernel computation and circular/linear polyline
smoothing with configurable corner angle thresholds to preserve
sharp features.
";

pyo3_stub_gen::module_doc!("raygeo.geo.algo.overcut", "{}", MODULE_DOC_OVERCUT);

pub(crate) const MODULE_DOC_OVERCUT: &str = "\
Overcut operations for closed contours.

Extends closed contours past their start point to ensure complete
cuts through the material, particularly useful in laser cutting
where the laser may not fully penetrate at the start/end point.
";

use super::flex_point::{
    extract_polygon, extract_polygons, int_poly_to_points, poly_to_points,
    PyPoint2D, PyPoint3D,
};
use super::Geometry;
use crate::geo::algo::clipping::{
    clip_line_segment_with_polygons, clip_line_segment_with_rect,
    subtract_polygons_from_line_segment,
};
use crate::geo::algo::fitting::{
    are_points_collinear, create_arc_cmd, create_line_cmd,
    fit_circle_to_3_points, fit_circle_to_points, fit_points_recursive,
    fit_points_with_primitives, flatten_to_points, get_polyline_arc_deviation,
    get_polyline_line_deviation, linearize_geometry,
    project_circle_center_to_bisector,
};
use crate::geo::algo::minkowski::{
    calculate_input_scale, convolve_point_sequences, convolve_two_segments,
    get_inner_fit_polygon, get_no_fit_polygon,
    get_polygon_minkowski_sum_convex,
};
use crate::geo::algo::overcut::apply_overcut;
use crate::geo::algo::simplify::simplify_polyline;
use crate::geo::algo::smooth::{
    compute_gaussian_kernel, resample_polyline, smooth_circularly,
    smooth_polyline, smooth_sub_segment,
};
use crate::Segment3D;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

const CLIPPER_SCALE: i64 = 10_000_000;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let algo_mod = PyModule::new(py, "algo")?;
    algo_mod.setattr("__doc__", MODULE_DOC)?;

    let minkowski_mod = PyModule::new(py, "minkowski")?;
    minkowski_mod.setattr("__doc__", MODULE_DOC_MINKOWSKI)?;
    let simplify_mod = PyModule::new(py, "simplify")?;
    simplify_mod.setattr("__doc__", MODULE_DOC_SIMPLIFY)?;
    let clipping_mod = PyModule::new(py, "clipping")?;
    clipping_mod.setattr("__doc__", MODULE_DOC_CLIPPING)?;
    let smooth_mod = PyModule::new(py, "smooth")?;
    smooth_mod.setattr("__doc__", MODULE_DOC_SMOOTH)?;

    let overcut_mod = PyModule::new(py, "overcut")?;
    overcut_mod.setattr("__doc__", MODULE_DOC_OVERCUT)?;

    clipping_mod.add_function(wrap_pyfunction!(
        clip_line_segment_py,
        clipping_mod.clone()
    )?)?;
    clipping_mod.add_function(wrap_pyfunction!(
        clip_line_segment_to_regions_py,
        clipping_mod.clone()
    )?)?;
    clipping_mod.add_function(wrap_pyfunction!(
        subtract_polygons_from_line_segment_py,
        clipping_mod.clone()
    )?)?;
    clipping_mod
        .add_function(wrap_pyfunction!(to_clipper_py, clipping_mod.clone())?)?;
    clipping_mod.add_function(wrap_pyfunction!(
        from_clipper_py,
        clipping_mod.clone()
    )?)?;

    minkowski_mod.add_function(wrap_pyfunction!(
        minkowski_sum_convex_py,
        minkowski_mod.clone()
    )?)?;
    minkowski_mod.add_function(wrap_pyfunction!(
        get_inner_fit_polygon_py,
        minkowski_mod.clone()
    )?)?;
    minkowski_mod.add_function(wrap_pyfunction!(
        get_no_fit_polygon_py,
        minkowski_mod.clone()
    )?)?;
    minkowski_mod.add_function(wrap_pyfunction!(
        calculate_input_scale_py,
        minkowski_mod.clone()
    )?)?;
    minkowski_mod.add_function(wrap_pyfunction!(
        convolve_two_segments_py,
        minkowski_mod.clone()
    )?)?;
    minkowski_mod.add_function(wrap_pyfunction!(
        convolve_point_sequences_py,
        minkowski_mod.clone()
    )?)?;

    simplify_mod.add_function(wrap_pyfunction!(
        simplify_polyline_py,
        simplify_mod.clone()
    )?)?;

    smooth_mod.add_function(wrap_pyfunction!(
        compute_gaussian_kernel_py,
        smooth_mod.clone()
    )?)?;
    smooth_mod.add_function(wrap_pyfunction!(
        smooth_circularly_py,
        smooth_mod.clone()
    )?)?;
    smooth_mod.add_function(wrap_pyfunction!(
        smooth_polyline_algo_py,
        smooth_mod.clone()
    )?)?;
    smooth_mod.add_function(wrap_pyfunction!(
        smooth_sub_segment_py,
        smooth_mod.clone()
    )?)?;
    smooth_mod.add_function(wrap_pyfunction!(
        resample_polyline_py,
        smooth_mod.clone()
    )?)?;

    overcut_mod.add_function(wrap_pyfunction!(
        apply_overcut_py,
        overcut_mod.clone()
    )?)?;

    let fitting_mod = PyModule::new(py, "fitting")?;
    fitting_mod.setattr("__doc__", MODULE_DOC_FITTING)?;
    fitting_mod.add_function(wrap_pyfunction!(
        are_points_collinear_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        fit_circle_to_3_points_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        fit_circle_to_points_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        project_circle_center_to_bisector_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        flatten_to_points_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        linearize_geometry_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        create_line_cmd_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        create_arc_cmd_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        fit_points_recursive_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        fit_points_with_primitives_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        get_polyline_line_deviation_py,
        fitting_mod.clone()
    )?)?;
    fitting_mod.add_function(wrap_pyfunction!(
        get_polyline_arc_deviation_py,
        fitting_mod.clone()
    )?)?;

    let analysis_mod = PyModule::new(py, "analysis")?;
    analysis_mod.setattr("__doc__", MODULE_DOC_ANALYSIS)?;
    analysis_mod.add_function(wrap_pyfunction!(
        remove_duplicates_py,
        analysis_mod.clone()
    )?)?;

    algo_mod.add_submodule(&minkowski_mod)?;
    algo_mod.add_submodule(&simplify_mod)?;
    algo_mod.add_submodule(&clipping_mod)?;
    algo_mod.add_submodule(&smooth_mod)?;
    algo_mod.add_submodule(&overcut_mod)?;
    algo_mod.add_submodule(&fitting_mod)?;
    algo_mod.add_submodule(&analysis_mod)?;

    m.add_submodule(&algo_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo", &algo_mod)?;
    sys_modules.set_item("raygeo.geo.algo.analysis", &analysis_mod)?;
    sys_modules.set_item("raygeo.geo.algo.clipping", &clipping_mod)?;
    sys_modules.set_item("raygeo.geo.algo.fitting", &fitting_mod)?;
    sys_modules.set_item("raygeo.geo.algo.minkowski", &minkowski_mod)?;
    sys_modules.set_item("raygeo.geo.algo.simplify", &simplify_mod)?;
    sys_modules.set_item("raygeo.geo.algo.smooth", &smooth_mod)?;
    sys_modules.set_item("raygeo.geo.algo.overcut", &overcut_mod)?;

    Ok(())
}

type Point = (f64, f64);

fn to_data_array(data: Vec<Vec<f64>>) -> Vec<[f64; 8]> {
    data.into_iter()
        .map(|r| {
            let mut a = [0.0; 8];
            let len = r.len().min(8);
            a[..len].copy_from_slice(&r[..len]);
            a
        })
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    from collections.abc import Sequence
    from raygeo.geo import types

    def are_points_collinear(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float = 1e-6,
    ) -> bool:
        """Check if three or more points are collinear within tolerance.

        :param points: Sequence of 3D points.
        :param tolerance: Collinearity tolerance.
        :returns: True if points are collinear.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "are_points_collinear")]
#[pyo3(signature = (points, tolerance=1e-6))]
fn are_points_collinear_py(
    points: Vec<(f64, f64, f64)>,
    tolerance: f64,
) -> bool {
    are_points_collinear(&points, tolerance)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    from raygeo.geo import types

    def fit_circle_to_3_points(
        p1: types.Point2DOr3D,
        p2: types.Point2DOr3D,
        p3: types.Point2DOr3D,
    ) -> typing.Optional[tuple[types.Point, float]]:
        """Fit a circle to three points.

        :param p1: First point (x, y) or (x, y, z).
        :param p2: Second point (x, y) or (x, y, z).
        :param p3: Third point (x, y) or (x, y, z).
        :returns: Tuple of (center, radius) or None.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_circle_to_3_points")]
fn fit_circle_to_3_points_py(
    p1: PyPoint3D,
    p2: PyPoint3D,
    p3: PyPoint3D,
) -> Option<((f64, f64), f64)> {
    fit_circle_to_3_points(
        (p1.0, p1.1, p1.2),
        (p2.0, p2.1, p2.2),
        (p3.0, p3.1, p3.2),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    from raygeo.geo import types

    def fit_circle_to_points(
        points: collections.abc.Sequence[types.Point3D],
    ) -> typing.Optional[tuple[types.Point, float, float]]:
        """Fit a circle to a set of points.

        :param points: Sequence of 3D points to fit.
        :returns: Tuple of (center, radius, error) or None.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_circle_to_points")]
fn fit_circle_to_points_py(
    points: Vec<(f64, f64, f64)>,
) -> Option<((f64, f64), f64, f64)> {
    fit_circle_to_points(&points)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    from raygeo.geo import types

    def project_circle_center_to_bisector(
        p1: types.Point2DOr3D,
        p2: types.Point2DOr3D,
        center: types.Point,
    ) -> types.Point:
        """Project a circle center onto the perpendicular bisector of two points.

        :param p1: First point (x, y) or (x, y, z).
        :param p2: Second point (x, y) or (x, y, z).
        :param center: Circle center to project.
        :returns: Projected center point (x, y).
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "project_circle_center_to_bisector")]
fn project_circle_center_to_bisector_py(
    p1: PyPoint3D,
    p2: PyPoint3D,
    center: (f64, f64),
) -> (f64, f64) {
    project_circle_center_to_bisector(
        (p1.0, p1.1, p1.2),
        (p2.0, p2.1, p2.2),
        center,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def flatten_to_points(
        data: collections.abc.Sequence[collections.abc.Sequence[float]],
        tolerance: float,
    ) -> list[list[types.Point3D]]:
        """Flatten curves into linear segments.

        :param data: Array of command data.
        :param tolerance: Flattening tolerance.
        :returns: List of flattened point segments.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "flatten_to_points")]
fn flatten_to_points_py(
    data: Vec<Vec<f64>>,
    tolerance: f64,
) -> Vec<Vec<(f64, f64, f64)>> {
    let arr = to_data_array(data);
    flatten_to_points(&arr, tolerance)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def linearize_geometry(
        data: collections.abc.Sequence[collections.abc.Sequence[float]],
        tolerance: float,
    ) -> list[list[float]]:
        """Linearize geometry data into line segments.

        :param data: Array of command data.
        :param tolerance: Linearization tolerance.
        :returns: List of linearized segment rows.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "linearize_geometry")]
fn linearize_geometry_py(data: Vec<Vec<f64>>, tolerance: f64) -> Vec<Vec<f64>> {
    let arr = to_data_array(data);
    linearize_geometry(&arr, tolerance)
        .into_iter()
        .map(|r| r.to_vec())
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import types

    def create_line_cmd(
        end_point: types.Point3D,
    ) -> list[float]:
        """Create a line command array from an end point.

        :param end_point: End point (x, y, z).
        :returns: Line command array (8 floats).
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "create_line_cmd")]
fn create_line_cmd_py(end_point: PyPoint3D) -> Vec<f64> {
    create_line_cmd((end_point.0, end_point.1, end_point.2)).to_vec()
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import types

    def create_arc_cmd(
        end: types.Point3D,
        center: types.Point,
        start: types.Point3D,
    ) -> list[float]:
        """Create an arc command array.

        :param end: End point (x, y, z).
        :param center: Center offset (dx, dy).
        :param start: Start point (x, y, z).
        :returns: Arc command array (8 floats).
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "create_arc_cmd")]
fn create_arc_cmd_py(
    end: PyPoint3D,
    center: (f64, f64),
    start: PyPoint3D,
) -> Vec<f64> {
    create_arc_cmd((end.0, end.1, end.2), center, (start.0, start.1, start.2))
        .to_vec()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def fit_points_recursive(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float,
        start_idx: int,
        end_idx: int,
    ) -> list[list[float]]:
        """Recursively fit points with line and arc primitives.

        :param points: Sequence of 3D points to fit.
        :param tolerance: Fitting tolerance.
        :param start_idx: Start index in the points array.
        :param end_idx: End index in the points array.
        :returns: List of fitted command rows.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_points_recursive")]
fn fit_points_recursive_py(
    points: Vec<(f64, f64, f64)>,
    tolerance: f64,
    start_idx: usize,
    end_idx: usize,
) -> Vec<Vec<f64>> {
    fit_points_recursive(&points, tolerance, start_idx, end_idx)
        .into_iter()
        .map(|r| r.to_vec())
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def fit_points_with_primitives(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float,
    ) -> list[list[float]]:
        """Fit a polyline of points with arc and line primitives.

        :param points: Sequence of 3D points to fit.
        :param tolerance: Fitting tolerance.
        :returns: List of fitted command rows.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_points_with_primitives")]
fn fit_points_with_primitives_py(
    points: Vec<(f64, f64, f64)>,
    tolerance: f64,
) -> Vec<Vec<f64>> {
    fit_points_with_primitives(&points, tolerance)
        .into_iter()
        .map(|r| r.to_vec())
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def get_polyline_line_deviation(
        points: collections.abc.Sequence[types.Point3D],
        start: int,
        end: int,
    ) -> tuple[float, int]:
        """Get the maximum line deviation for a segment of a polyline.

        :param points: Sequence of 3D points.
        :param start: Start index.
        :param end: End index.
        :returns: Tuple of (max_deviation, index_of_max).
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "get_polyline_line_deviation")]
fn get_polyline_line_deviation_py(
    points: Vec<(f64, f64, f64)>,
    start: usize,
    end: usize,
) -> (f64, usize) {
    get_polyline_line_deviation(&points, start, end)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def get_polyline_arc_deviation(
        points: collections.abc.Sequence[types.Point3D],
        center: types.Point,
        radius: float,
    ) -> float:
        """Get the maximum arc deviation for a set of points.

        :param points: Sequence of 3D points.
        :param center: Arc center (x, y).
        :param radius: Arc radius.
        :returns: Maximum deviation from the arc.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "get_polyline_arc_deviation")]
fn get_polyline_arc_deviation_py(
    points: Vec<(f64, f64, f64)>,
    center: (f64, f64),
    radius: f64,
) -> f64 {
    get_polyline_arc_deviation(&points, center, radius)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

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
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "resample_polyline")]
fn resample_polyline_py(
    points: Vec<(f64, f64, f64)>,
    max_segment_length: f64,
    is_closed: bool,
) -> Vec<(f64, f64, f64)> {
    resample_polyline(&points, max_segment_length, is_closed)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    from raygeo.geo import types

    def clip_line_segment_with_rect(
        p1: types.Point3D,
        p2: types.Point3D,
        rect: types.Rect,
    ) -> typing.Optional[tuple[types.Point3D, types.Point3D]]:
        """Clip a line segment with a rectangle.

        :param p1: Start point of the line segment.
        :param p2: End point of the line segment.
        :param rect: Clipping rectangle (x_min, y_min, x_max, y_max).
        :returns: Clipped segment or None if fully outside.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "clip_line_segment_with_rect")]
fn clip_line_segment_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    rect: (f64, f64, f64, f64),
) -> Option<Segment3D> {
    clip_line_segment_with_rect(p1, p2, rect)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def subtract_polygons_from_line_segment(
        p1: types.Point3D,
        p2: types.Point3D,
        regions: collections.abc.Sequence[collections.abc.Sequence[types.Point]],
    ) -> list[tuple[types.Point3D, types.Point3D]]:
        """Subtract polygon regions from a line segment.

        :param p1: Start point of the line segment.
        :param p2: End point of the line segment.
        :param regions: List of polygon regions to subtract.
        :returns: List of remaining segments after subtraction.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "subtract_polygons_from_line_segment")]
fn subtract_polygons_from_line_segment_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    regions: &Bound<'_, PyAny>,
) -> PyResult<Vec<Segment3D>> {
    let regions = extract_polygons(regions)?;
    Ok(subtract_polygons_from_line_segment(p1, p2, &regions))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def clip_line_segment_with_polygons(
        p1: types.Point3D,
        p2: types.Point3D,
        regions: collections.abc.Sequence[collections.abc.Sequence[types.Point]],
    ) -> list[tuple[types.Point3D, types.Point3D]]:
        """Clip line segments that fall within polygon regions.

        :param p1: Start point of the line segment.
        :param p2: End point of the line segment.
        :param regions: Polygon regions to clip against.
        :returns: List of clipped segments.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "clip_line_segment_with_polygons")]
fn clip_line_segment_to_regions_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    regions: &Bound<'_, PyAny>,
) -> PyResult<Vec<Segment3D>> {
    let regions = extract_polygons(regions)?;
    Ok(clip_line_segment_with_polygons(p1, p2, &regions))
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import types

    def to_clipper(
        polygon: types.Polygon,
        scale: int = 10000000,
    ) -> list[tuple[int, int]]:
        """Convert a polygon to Clipper coordinates.

        :param polygon: Input polygon as a list of (x, y) points.
        :param scale: Scale factor for integer conversion.
        :returns: Polygon with integer coordinates for Clipper.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "to_clipper")]
fn to_clipper_py(
    polygon: &Bound<'_, PyAny>,
    scale: Option<i64>,
) -> PyResult<Vec<(i64, i64)>> {
    let scale = scale.unwrap_or(CLIPPER_SCALE);
    let poly = extract_polygon(polygon)?;
    Ok(poly
        .iter()
        .map(|(x, y)| ((x * scale as f64) as i64, (y * scale as f64) as i64))
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import types

    def from_clipper(
        polygon: types.IntPolygon,
        scale: int = 10000000,
    ) -> types.Polygon:
        """Convert a polygon from Clipper coordinates.

        :param polygon: Integer polygon from Clipper.
        :param scale: Scale factor used during conversion.
        :returns: Polygon with float coordinates.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "from_clipper")]
fn from_clipper_py(
    polygon: Vec<crate::python::geo::flex_point::PyIntPoint2D>,
    scale: Option<i64>,
) -> Vec<Point> {
    let scale = scale.unwrap_or(CLIPPER_SCALE) as f64;
    let poly = int_poly_to_points(polygon);
    poly.iter()
        .map(|(x, y)| (*x as f64 / scale, *y as f64 / scale))
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def get_polygon_minkowski_sum_convex(
        poly_a: collections.abc.Sequence[tuple[int, int]],
        poly_b: collections.abc.Sequence[tuple[int, int]],
    ) -> list[list[tuple[int, int]]]:
        """Compute the Minkowski sum of two convex polygons.

        :param poly_a: First convex polygon as integer points.
        :param poly_b: Second convex polygon as integer points.
        :returns: Minkowski sum as list of polygons.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "get_polygon_minkowski_sum_convex")]
fn minkowski_sum_convex_py(
    poly_a: Vec<(i64, i64)>,
    poly_b: Vec<(i64, i64)>,
) -> Vec<Vec<(i64, i64)>> {
    get_polygon_minkowski_sum_convex(&poly_a, &poly_b)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def get_inner_fit_polygon(
        outer: collections.abc.Sequence[types.Point],
        inner: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Compute the inner fit polygon (no-fit polygon for nesting).

        :param outer: Outer polygon as (x, y) points.
        :param inner: Inner polygon as (x, y) points.
        :returns: Inner fit polygon.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "get_inner_fit_polygon")]
fn get_inner_fit_polygon_py(
    outer: Vec<PyPoint2D>,
    inner: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    get_inner_fit_polygon(&poly_to_points(outer), &poly_to_points(inner))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def get_no_fit_polygon(
        subject: collections.abc.Sequence[types.Point],
        tool: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Compute the no-fit polygon for two 2D polygons.

        :param subject: Subject polygon as (x, y) points.
        :param tool: Tool polygon as (x, y) points.
        :returns: No-fit polygon.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "get_no_fit_polygon")]
fn get_no_fit_polygon_py(
    subject: Vec<PyPoint2D>,
    tool: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    get_no_fit_polygon(&poly_to_points(subject), &poly_to_points(tool))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def calculate_input_scale(
        polygons: collections.abc.Sequence[collections.abc.Sequence[types.Point]],
        max_int: int = 2147483647,
    ) -> float:
        """Calculate the optimal input scale for clipper operations.

        :param polygons: List of polygons to scale.
        :param max_int: Maximum integer value for Clipper.
        :returns: Optimal scale factor.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "calculate_input_scale")]
#[pyo3(signature = (polygons, max_int=2147483647))]
fn calculate_input_scale_py(
    polygons: &Bound<'_, PyAny>,
    max_int: i64,
) -> PyResult<f64> {
    let polys = extract_polygons(polygons)?;
    Ok(calculate_input_scale(&polys, max_int))
}

#[gen_stub_pyfunction(
    python = r#"
    def convolve_two_segments(
        a1: tuple[int, int],
        a2: tuple[int, int],
        b1: tuple[int, int],
        b2: tuple[int, int],
    ) -> list[tuple[int, int]]:
        """Convolve two line segments.

        :param a1: Start point of segment A.
        :param a2: End point of segment A.
        :param b1: Start point of segment B.
        :param b2: End point of segment B.
        :returns: Convolved point sequence.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "convolve_two_segments")]
fn convolve_two_segments_py(
    a1: (i64, i64),
    a2: (i64, i64),
    b1: (i64, i64),
    b2: (i64, i64),
) -> Vec<(i64, i64)> {
    convolve_two_segments(a1, a2, b1, b2)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def convolve_point_sequences(
        seq_a: collections.abc.Sequence[tuple[int, int]],
        seq_b: collections.abc.Sequence[tuple[int, int]],
    ) -> list[list[tuple[int, int]]]:
        """Convolve two sequences of points.

        :param seq_a: First sequence of integer points.
        :param seq_b: Second sequence of integer points.
        :returns: Convolved point sequences.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "convolve_point_sequences")]
fn convolve_point_sequences_py(
    seq_a: Vec<(i64, i64)>,
    seq_b: Vec<(i64, i64)>,
) -> Vec<Vec<(i64, i64)>> {
    convolve_point_sequences(&seq_a, &seq_b)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def simplify_polyline(
        points: collections.abc.Sequence[types.Point],
        tolerance: float,
    ) -> types.Polygon:
        """Simplify a polyline using the Ramer-Douglas-Peucker algorithm.

        :param points: Sequence of (x, y) points.
        :param tolerance: Simplification tolerance.
        :returns: Simplified point sequence.
        """
"#,
    module = "raygeo.geo.algo.simplify"
)]
#[pyfunction(name = "simplify_polyline")]
fn simplify_polyline_py(points: Vec<PyPoint2D>, tolerance: f64) -> Vec<Point> {
    let pts = poly_to_points(points);
    let points_3d: Vec<crate::Point3D> =
        pts.iter().map(|p| (p.0, p.1, 0.0)).collect();
    let result = simplify_polyline(&points_3d, tolerance);
    result.iter().map(|p| (p.0, p.1)).collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def remove_duplicates(points: collections.abc.Sequence[types.Point]) -> types.Polygon:
        """Remove duplicate points from a sequence.

        :param points: Sequence of (x, y) points.
        :returns: List of unique points.
        """
"#,
    module = "raygeo.geo.algo.analysis"
)]
#[pyfunction(name = "remove_duplicates")]
fn remove_duplicates_py(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    crate::geo::algo::analysis::remove_duplicates(&points)
}

#[gen_stub_pyfunction(
    python = r#"
    def compute_gaussian_kernel(
        amount: int,
    ) -> tuple[list[float], float]:
        """Compute a Gaussian kernel of the given size.

        :param amount: Kernel size.
        :returns: Tuple of (kernel_values, sigma).
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
    from raygeo.geo import types

    def smooth_circularly(
        points: collections.abc.Sequence[types.Point3D],
        kernel: collections.abc.Sequence[float],
    ) -> list[types.Point3D]:
        """Smooth a closed polyline circularly.

        :param points: Sequence of 3D points to smooth.
        :param kernel: Gaussian kernel values.
        :returns: Smoothed points.
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "smooth_circularly")]
fn smooth_circularly_py(
    points: Vec<(f64, f64, f64)>,
    kernel: Vec<f64>,
) -> Vec<(f64, f64, f64)> {
    smooth_circularly(&points, &kernel)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    from raygeo.geo import types

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
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "smooth_polyline")]
#[pyo3(signature = (points, amount, corner_angle_threshold, is_closed=None))]
fn smooth_polyline_algo_py(
    points: Vec<(f64, f64, f64)>,
    amount: i32,
    corner_angle_threshold: f64,
    is_closed: Option<bool>,
) -> Vec<(f64, f64, f64)> {
    smooth_polyline(&points, amount, corner_angle_threshold, is_closed)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    from raygeo.geo import types

    def smooth_sub_segment(
        points: collections.abc.Sequence[types.Point3D],
        kernel: collections.abc.Sequence[float],
    ) -> list[types.Point3D]:
        """Smooth a sub-segment of a polyline.

        :param points: Sequence of 3D points to smooth.
        :param kernel: Gaussian kernel values.
        :returns: Smoothed points.
        """
"#,
    module = "raygeo.geo.algo.smooth"
)]
#[pyfunction(name = "smooth_sub_segment")]
fn smooth_sub_segment_py(
    points: Vec<(f64, f64, f64)>,
    kernel: Vec<f64>,
) -> Vec<(f64, f64, f64)> {
    smooth_sub_segment(&points, &kernel)
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo import geo

    def apply_overcut(
        geometry: geo.Geometry,
        overcut: float,
    ) -> geo.Geometry:
        """Extend a closed contour past its start point.

        When laser-cutting closed contours, the laser slows down at
        corners and may not cut through completely. This function
        extends the path by ``overcut`` distance past the start point
        to ensure a clean cut.

        If the geometry is not closed, empty, or overcut is <= 0, the
        geometry is returned unchanged.

        :param geometry: The input geometry (must be closed).
        :param overcut: Distance to extend past the start point.
        :returns: A new geometry with the overcut applied.
        """
"#,
    module = "raygeo.geo.algo.overcut"
)]
#[pyfunction(name = "apply_overcut")]
fn apply_overcut_py(geometry: &Geometry, overcut: f64) -> super::Geometry {
    super::Geometry {
        inner: apply_overcut(&geometry.inner, overcut),
    }
}
