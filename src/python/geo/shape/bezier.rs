pyo3_stub_gen::module_doc!("raygeo.geo.shape.bezier", "{}", MODULE_DOC_BEZIER);

pub(crate) const MODULE_DOC_BEZIER: &str = "\
Cubic bezier curve queries and conversions.

Provides point evaluation at a parameter t, splitting into two halves,
bounding rectangle computation, flattening to line segments (both
fixed-step and adaptive subdivision), rectangle clipping, flatness
testing, perpendicular distance measurement, and conversion from
cubic to quadratic form.
";

use super::super::flex_point::{extract_polygons, PyPoint2D, PyPoint3D};
use crate::geo::shape::bezier::{
    clip_bezier_with_rect, convert_cubic_bezier_to_quadratic, flatten_bezier,
    get_bezier_bounds, get_bezier_flatness_sq, get_bezier_length,
    get_bezier_point_at, get_bezier_rect_intersections,
    get_perpendicular_dist_sq, is_bezier_flat, is_bezier_inside_polygons,
    linearize_bezier, linearize_bezier_adaptive, linearize_bezier_segment,
    split_bezier,
};
use crate::types::{Point, Point3D, Rect};
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "bezier")?;
    m.setattr("__doc__", MODULE_DOC_BEZIER)?;

    register_functions!(
        m,
        get_bezier_point_at_py,
        split_bezier_py,
        get_bezier_bounds_py,
        get_bezier_rect_intersections_py,
        clip_bezier_with_rect_py,
        convert_cubic_bezier_to_quadratic_py,
        is_bezier_flat_py,
        is_bezier_inside_polygons_py,
        linearize_bezier_py,
        linearize_bezier_adaptive_py,
        linearize_bezier_segment_py,
        flatten_bezier_py,
        get_bezier_flatness_sq_py,
        get_perpendicular_dist_sq_py,
        get_bezier_length_py,
    );

    shape_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_point_at(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        t: float,
    ) -> types.Point:
        """Get a point on a cubic bezier at parameter t.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param t: Parameter value (0..1).
        :returns: Point on the bezier curve (x, y).
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_point_at")]
fn get_bezier_point_at_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    t: f64,
) -> Point {
    get_bezier_point_at(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
        t,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def split_bezier(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        t: float,
    ) -> tuple[
        tuple[types.Point, types.Point, types.Point, types.Point],
        tuple[types.Point, types.Point, types.Point, types.Point],
    ]:
        """Split a cubic bezier at parameter t.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param t: Split parameter (0..1).
        :returns: Two bezier curves (left, right).
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "split_bezier")]
#[allow(clippy::type_complexity)]
fn split_bezier_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    t: f64,
) -> ((Point, Point, Point, Point), (Point, Point, Point, Point)) {
    let (left, right) = split_bezier(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
        t,
    );
    (
        (left.0, left.1, left.2, left.3),
        (right.0, right.1, right.2, right.3),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_bounds(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
    ) -> types.Rect:
        """Get the bounding rectangle of a cubic bezier.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_bounds")]
fn get_bezier_bounds_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
) -> (f64, f64, f64, f64) {
    let r = get_bezier_bounds(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
    );
    (r.0, r.1, r.2, r.3)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_rect_intersections(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        rect: types.Rect,
    ) -> list[float]:
        """Get intersection t-values of a bezier with a rectangle.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param rect: Rectangle (x_min, y_min, x_max, y_max).
        :returns: List of t-values where the bezier intersects.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_rect_intersections")]
fn get_bezier_rect_intersections_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    rect: (f64, f64, f64, f64),
) -> Vec<f64> {
    get_bezier_rect_intersections(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
        Rect(rect.0, rect.1, rect.2, rect.3),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def clip_bezier_with_rect(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        rect: types.Rect,
    ) -> list[tuple[types.Point, types.Point, types.Point, types.Point]]:
        """Clip a cubic bezier with a rectangle.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param rect: Clipping rectangle (x_min, y_min, x_max, y_max).
        :returns: List of bezier segments inside the rectangle.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "clip_bezier_with_rect")]
fn clip_bezier_with_rect_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    rect: (f64, f64, f64, f64),
) -> Vec<(Point, Point, Point, Point)> {
    clip_bezier_with_rect(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
        Rect(rect.0, rect.1, rect.2, rect.3),
    )
    .into_iter()
    .map(|c| (c.0, c.1, c.2, c.3))
    .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def convert_cubic_bezier_to_quadratic(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
    ) -> tuple[types.Point, types.Point, types.Point]:
        """Convert a cubic bezier to a quadratic bezier.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :returns: Quadratic bezier (p0, p1, p2).
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "convert_cubic_bezier_to_quadratic")]
fn convert_cubic_bezier_to_quadratic_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
) -> (Point, Point, Point) {
    convert_cubic_bezier_to_quadratic(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def is_bezier_flat(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        tolerance_sq: float,
    ) -> bool:
        """Test whether a cubic bezier curve is flat enough to approximate with a line segment.

        Uses a chord-distance flatness test. For non-degenerate curves (p0 != p3)
        it checks whether both control points lie within tolerance_sq of the chord
        line. For degenerate curves (p0 approx p3) it checks the control point
        distances from the start point.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param tolerance_sq: Squared tolerance for flatness.
        :returns: True if the curve is flat enough.
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "is_bezier_flat")]
fn is_bezier_flat_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    tolerance_sq: f64,
) -> bool {
    is_bezier_flat(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
        tolerance_sq,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def is_bezier_inside_polygons(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        polygons: typing.Any,
    ) -> bool:
        """Check if a bezier curve is inside a set of polygons.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param polygons: List of polygons to check against.
        :returns: True if the bezier is inside all polygons.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "is_bezier_inside_polygons")]
fn is_bezier_inside_polygons_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    polygons: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    let polygons_2d = extract_polygons(polygons)?;
    Ok(is_bezier_inside_polygons(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
        &polygons_2d,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def linearize_bezier(
        p0: types.Point3D,
        p1: types.Point3D,
        p2: types.Point3D,
        p3: types.Point3D,
        num_steps: int,
    ) -> list[tuple[types.Point3D, types.Point3D]]:
        """Linearize a bezier into line segments.

        :param p0: Start control point (x, y, z).
        :param p1: First control point (x, y, z).
        :param p2: Second control point (x, y, z).
        :param p3: End control point (x, y, z).
        :param num_steps: Number of linearization steps.
        :returns: List of (p1, p2) segment pairs.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "linearize_bezier")]
#[allow(clippy::type_complexity)]
fn linearize_bezier_py(
    p0: PyPoint3D,
    p1: PyPoint3D,
    p2: PyPoint3D,
    p3: PyPoint3D,
    num_steps: usize,
) -> Vec<(Point3D, Point3D)> {
    linearize_bezier(
        Point3D(p0.0, p0.1, p0.2),
        Point3D(p1.0, p1.1, p1.2),
        Point3D(p2.0, p2.1, p2.2),
        Point3D(p3.0, p3.1, p3.2),
        num_steps,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def linearize_bezier_adaptive(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        tolerance_sq: float,
        max_subdivisions: int = 20,
    ) -> types.Polygon:
        """Adaptively linearize a bezier curve.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param tolerance_sq: Squared tolerance for subdivision.
        :param max_subdivisions: Maximum recursion depth.
        :returns: List of linearized points (x, y).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "linearize_bezier_adaptive")]
#[pyo3(signature = (p0, p1, p2, p3, tolerance_sq, max_subdivisions=20))]
fn linearize_bezier_adaptive_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    tolerance_sq: f64,
    max_subdivisions: usize,
) -> Vec<Point> {
    linearize_bezier_adaptive(
        Point(p0.0, p0.1),
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        Point(p3.0, p3.1),
        tolerance_sq,
        max_subdivisions,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def linearize_bezier_segment(
        p0: types.Point3D,
        p1: types.Point3D,
        p2: types.Point3D,
        p3: types.Point3D,
        tolerance: float = 0.1,
    ) -> list[types.Point3D]:
        """Linearize a single bezier segment.

        :param p0: Start control point (x, y, z).
        :param p1: First control point (x, y, z).
        :param p2: Second control point (x, y, z).
        :param p3: End control point (x, y, z).
        :param tolerance: Linearization tolerance.
        :returns: List of linearized points (x, y, z).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "linearize_bezier_segment")]
#[pyo3(signature = (p0, p1, p2, p3, tolerance=0.1))]
fn linearize_bezier_segment_py(
    p0: PyPoint3D,
    p1: PyPoint3D,
    p2: PyPoint3D,
    p3: PyPoint3D,
    tolerance: f64,
) -> Vec<Point3D> {
    linearize_bezier_segment(
        Point3D(p0.0, p0.1, p0.2),
        Point3D(p1.0, p1.1, p1.2),
        Point3D(p2.0, p2.1, p2.2),
        Point3D(p3.0, p3.1, p3.2),
        Some(tolerance),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def flatten_bezier(
        p0: types.Point3D,
        p1: types.Point3D,
        p2: types.Point3D,
        p3: types.Point3D,
        tolerance: float,
        max_subdivisions: int,
        pts: list,
    ) -> None:
        """Flatten a bezier curve into points.

        :param p0: Start control point (x, y, z).
        :param p1: First control point (x, y, z).
        :param p2: Second control point (x, y, z).
        :param p3: End control point (x, y, z).
        :param tolerance: Flattening tolerance.
        :param max_subdivisions: Maximum recursion depth.
        :param pts: Output list to append points to.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "flatten_bezier")]
fn flatten_bezier_py(
    p0: PyPoint3D,
    p1: PyPoint3D,
    p2: PyPoint3D,
    p3: PyPoint3D,
    tolerance: f64,
    max_subdivisions: usize,
    pts: &Bound<'_, PyList>,
) -> PyResult<()> {
    let mut result = Vec::new();
    flatten_bezier(
        Point3D(p0.0, p0.1, p0.2),
        Point3D(p1.0, p1.1, p1.2),
        Point3D(p2.0, p2.1, p2.2),
        Point3D(p3.0, p3.1, p3.2),
        tolerance,
        max_subdivisions,
        &mut result,
    );
    for p in result {
        let obj = (p.0, p.1, p.2).into_pyobject(pts.py())?;
        pts.append(&obj)?;
    }
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_flatness_sq(
        a: types.Point3D,
        b: types.Point3D,
        c: types.Point3D,
        d: types.Point3D,
    ) -> float:
        """Compute the flatness squared of a cubic bezier.

        :param a: Start point (x, y, z).
        :param b: First control point (x, y, z).
        :param c: Second control point (x, y, z).
        :param d: End point (x, y, z).
        :returns: Flatness squared value.
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_flatness_sq")]
fn get_bezier_flatness_sq_py(
    a: PyPoint3D,
    b: PyPoint3D,
    c: PyPoint3D,
    d: PyPoint3D,
) -> f64 {
    get_bezier_flatness_sq(
        Point3D(a.0, a.1, a.2),
        Point3D(b.0, b.1, b.2),
        Point3D(c.0, c.1, c.2),
        Point3D(d.0, d.1, d.2),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_perpendicular_dist_sq(
        pt: types.Point3D,
        origin: types.Point3D,
        vx: float,
        vy: float,
        vz: float = 0.0,
        norm_sq: float = 0.0,
    ) -> float:
        """Compute the perpendicular distance squared.

        :param pt: Point to measure from.
        :param origin: Origin of the line.
        :param vx: X component of line direction.
        :param vy: Y component of line direction.
        :param vz: Z component of line direction.
        :param norm_sq: Precomputed squared norm (optional).
        :returns: Perpendicular distance squared.
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_perpendicular_dist_sq")]
#[pyo3(signature = (pt, origin, vx, vy, vz=0.0, norm_sq=0.0))]
fn get_perpendicular_dist_sq_py(
    pt: PyPoint3D,
    origin: PyPoint3D,
    vx: f64,
    vy: f64,
    vz: f64,
    norm_sq: f64,
) -> f64 {
    get_perpendicular_dist_sq(
        Point3D(pt.0, pt.1, pt.2),
        Point3D(origin.0, origin.1, origin.2),
        vx,
        vy,
        vz,
        norm_sq,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_length(
        p0: types.Point,
        c1: types.Point,
        c2: types.Point,
        p1: types.Point,
    ) -> float:
        """Compute the arc length of a cubic Bezier curve.

        :param p0: Start point (x, y).
        :param c1: First control point (x, y).
        :param c2: Second control point (x, y).
        :param p1: End point (x, y).
        :returns: Arc length.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_length")]
fn get_bezier_length_py(p0: Point, c1: Point, c2: Point, p1: Point) -> f64 {
    get_bezier_length(p0, c1, c2, p1)
}
