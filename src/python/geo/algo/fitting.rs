pyo3_stub_gen::module_doc!("raygeo.geo.algo.fitting", "{}", MODULE_DOC_FITTING);

pub(crate) const MODULE_DOC_FITTING: &str = "\
Curve and primitive fitting algorithms.

Provides functions for fitting arcs, lines, circles, and beziers to
point sequences. Includes recursive fitting with primitives, polyline
linearization, and evaluating fitting quality (line and arc deviation).
";

use super::super::flex_point::PyPoint3D;
use super::super::Geometry;
use crate::geo::algo::fitting::{
    are_points_collinear, create_arc_cmd, fit_circle_to_3_points,
    fit_circle_to_points, fit_points_recursive, fit_points_with_primitives,
    flatten_to_points, get_polyline_arc_deviation, get_polyline_line_deviation,
    linearize_geometry, project_circle_center_to_bisector,
};
use crate::Geometry as CoreGeometry;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "fitting")?;
    m.setattr("__doc__", MODULE_DOC_FITTING)?;

    register_functions!(
        m,
        are_points_collinear_py,
        fit_circle_to_3_points_py,
        fit_circle_to_points_py,
        project_circle_center_to_bisector_py,
        flatten_to_points_py,
        linearize_geometry_py,
        create_line_cmd_py,
        create_arc_cmd_py,
        fit_points_recursive_py,
        fit_points_with_primitives_py,
        get_polyline_line_deviation_py,
        get_polyline_arc_deviation_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    from collections.abc import Sequence
    import raygeo.geo.types

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
    import raygeo.geo.types

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
    import raygeo.geo.types

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
    import raygeo.geo.types

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
    import raygeo.geo
    import raygeo.geo.types

    def flatten_to_points(
        geometry: geo.Geometry,
        tolerance: float,
    ) -> list[list[types.Point3D]]:
        """Flatten curves into linear segments.

        :param geometry: Geometry to flatten.
        :param tolerance: Flattening tolerance.
        :returns: List of flattened point segments.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "flatten_to_points")]
fn flatten_to_points_py(
    geometry: &Geometry,
    tolerance: f64,
) -> Vec<Vec<(f64, f64, f64)>> {
    flatten_to_points(geometry.inner.data(), tolerance)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo

    def linearize_geometry(
        geometry: geo.Geometry,
        tolerance: float,
    ) -> geo.Geometry:
        """Linearize geometry data into line segments.

        :param geometry: Geometry to linearize.
        :param tolerance: Linearization tolerance.
        :returns: Linearized Geometry.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "linearize_geometry")]
fn linearize_geometry_py(geometry: &Geometry, tolerance: f64) -> Geometry {
    let mut result = geometry.inner.copy();
    result.data = linearize_geometry(&result.data, tolerance);
    Geometry { inner: result }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo
    import raygeo.geo.types

    def create_line_cmd(
        end_point: types.Point3D,
    ) -> geo.Line:
        """Create a line command from an end point.

        :param end_point: End point (x, y, z).
        :returns: A Line command.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "create_line_cmd")]
fn create_line_cmd_py(end_point: PyPoint3D) -> super::super::geometry::PyLine {
    super::super::geometry::PyLine {
        end: (end_point.0, end_point.1, end_point.2),
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo
    import raygeo.geo.types

    def create_arc_cmd(
        end: types.Point3D,
        center: types.Point,
        start: types.Point3D,
    ) -> geo.Arc:
        """Create an arc command.

        :param end: End point (x, y, z).
        :param center: Center offset (dx, dy).
        :param start: Start point (x, y, z).
        :returns: An Arc command.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "create_arc_cmd")]
fn create_arc_cmd_py(
    end: PyPoint3D,
    center: (f64, f64),
    start: PyPoint3D,
) -> super::super::geometry::PyArc {
    let cmd = create_arc_cmd(
        (end.0, end.1, end.2),
        center,
        (start.0, start.1, start.2),
    );
    match cmd {
        crate::types::Command::Arc {
            end,
            center_offset,
            clockwise,
        } => super::super::geometry::PyArc {
            end,
            center_offset,
            clockwise,
        },
        _ => unreachable!(),
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo
    import raygeo.geo.types

    def fit_points_recursive(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float,
        start_idx: int,
        end_idx: int,
    ) -> geo.Geometry:
        """Recursively fit points with line and arc primitives.

        :param points: Sequence of 3D points to fit.
        :param tolerance: Fitting tolerance.
        :param start_idx: Start index in the points array.
        :param end_idx: End index in the points array.
        :returns: Geometry of fitted commands.
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
) -> Geometry {
    let core = CoreGeometry {
        data: fit_points_recursive(&points, tolerance, start_idx, end_idx),
        last_move_to: (0.0, 0.0, 0.0),
        uniform_scalable: true,
    };
    Geometry { inner: core }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo
    import raygeo.geo.types

    def fit_points_with_primitives(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float,
    ) -> geo.Geometry:
        """Fit a polyline of points with arc and line primitives.

        :param points: Sequence of 3D points to fit.
        :param tolerance: Fitting tolerance.
        :returns: Geometry of fitted commands.
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_points_with_primitives")]
fn fit_points_with_primitives_py(
    points: Vec<(f64, f64, f64)>,
    tolerance: f64,
) -> Geometry {
    let core = CoreGeometry {
        data: fit_points_with_primitives(&points, tolerance),
        last_move_to: (0.0, 0.0, 0.0),
        uniform_scalable: true,
    };
    Geometry { inner: core }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

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
    import raygeo.geo.types

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
