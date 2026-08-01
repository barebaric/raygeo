//! Python bindings for 3D polygon operations.

use super::super::flex_point::{
    edge_pairs3d_to_tuples, point3d_to_tuple, points3d_to_tuples, PyPoint3D,
};
use crate::geo::shape::polygon3d::{
    deduplicate_polyline_3d, fillet_polyline_3d, flip_polygon_3d,
    flip_polygons_3d, get_polygon_area_3d, get_polygon_bounds_3d,
    get_polygon_centroid_3d, get_polygon_convex_hull_3d, get_polygon_edges_3d,
    get_polygon_group_bounds_3d, get_polygon_perimeter_3d,
    get_polygon_signed_area_3d, get_polygons_closest_point_3d,
    get_polygons_difference_3d, get_polygons_group_difference_3d,
    get_polygons_group_intersection_3d, get_polygons_intersection_3d,
    get_polygons_union_3d, get_polyline_end_tangent_3d, offset_polygon_3d,
    offset_polyline_3d, resample_polyline_3d, rotate_polygon_3d,
    rotate_polygons_3d, scale_polygon_3d, translate_polygon_3d,
    translate_polygons_3d, walk_along_polygon_3d, walk_along_polyline_3d,
};
use crate::geo::types::Point3D;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use super::super::types::Edge3D;

fn extract_polygon3d(ob: &Bound<'_, PyAny>) -> PyResult<Vec<Point3D>> {
    let mut points = Vec::new();
    for item in ob.try_iter()? {
        let item = item?;
        let (x, y, z) = item.extract::<(f64, f64, f64)>()?;
        points.push(Point3D::new(x, y, z));
    }
    Ok(points)
}

fn extract_polygons3d(ob: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<Point3D>>> {
    let mut result = Vec::new();
    for item in ob.try_iter()? {
        let item = item?;
        result.push(extract_polygon3d(&item)?);
    }
    Ok(result)
}

fn poly3d_to_points(poly: Vec<PyPoint3D>) -> Vec<Point3D> {
    poly.into_iter()
        .map(|p| Point3D::new(p.0, p.1, p.2))
        .collect()
}

// ── Boolean & Offset (existing) ──────────────────────────────────────

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def get_polygons_union_3d(polygons: collections.abc.Sequence[types.Polygon3D]) -> list[types.Polygon3D]:
        """Compute the union of 3D polygons (XY-plane, Z preserved).

        :param polygons: List of 3D polygons.
        :returns: Union result with Z from first polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_union_3d")]
fn get_polygons_union_3d_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let polys = extract_polygons3d(polygons)?;
    let result = get_polygons_union_3d(&polys);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_intersection_3d(poly1: collections.abc.Sequence[types.Point3D], poly2: collections.abc.Sequence[types.Point3D]) -> list[types.Polygon3D]:
        """Compute the intersection of two 3D polygons (XY-plane, Z preserved).

        :param poly1: First 3D polygon.
        :param poly2: Second 3D polygon.
        :returns: Intersection result with Z from first polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_intersection_3d")]
fn get_polygons_intersection_3d_py(
    poly1: &Bound<'_, PyAny>,
    poly2: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let p1 = extract_polygon3d(poly1)?;
    let p2 = extract_polygon3d(poly2)?;
    let result = get_polygons_intersection_3d(&p1, &p2);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_difference_3d(poly1: collections.abc.Sequence[types.Point3D], poly2: collections.abc.Sequence[types.Point3D]) -> list[types.Polygon3D]:
        """Compute the difference of two 3D polygons (poly1 - poly2).

        :param poly1: Subject 3D polygon.
        :param poly2: Clip 3D polygon.
        :returns: Difference result with Z from first polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_difference_3d")]
fn get_polygons_difference_3d_py(
    poly1: &Bound<'_, PyAny>,
    poly2: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let p1 = extract_polygon3d(poly1)?;
    let p2 = extract_polygon3d(poly2)?;
    let result = get_polygons_difference_3d(&p1, &p2);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_group_intersection_3d(subject: collections.abc.Sequence[types.Polygon3D], clip: collections.abc.Sequence[types.Polygon3D]) -> list[types.Polygon3D]:
        """Group intersection of 3D polygons (subject ∩ clip).

        :param subject: Subject group of 3D polygons.
        :param clip: Clip group of 3D polygons.
        :returns: Intersection result with Z from first subject polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_group_intersection_3d")]
fn get_polygons_group_intersection_3d_py(
    subject: &Bound<'_, PyAny>,
    clip: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let subj = extract_polygons3d(subject)?;
    let clp = extract_polygons3d(clip)?;
    let result = get_polygons_group_intersection_3d(&subj, &clp);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_group_difference_3d(subject: collections.abc.Sequence[types.Polygon3D], clip: collections.abc.Sequence[types.Polygon3D]) -> list[types.Polygon3D]:
        """Group difference of 3D polygons (subject - clip).

        :param subject: Subject group of 3D polygons.
        :param clip: Clip group of 3D polygons.
        :returns: Difference result with Z from first subject polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_group_difference_3d")]
fn get_polygons_group_difference_3d_py(
    subject: &Bound<'_, PyAny>,
    clip: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let subj = extract_polygons3d(subject)?;
    let clp = extract_polygons3d(clip)?;
    let result = get_polygons_group_difference_3d(&subj, &clp);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def offset_polygon_3d(polygon: collections.abc.Sequence[types.Point3D], offset: float) -> list[types.Polygon3D]:
        """Offset (inflate/deflate) a closed 3D polygon.

        :param polygon: Input 3D polygon.
        :param offset: Offset distance (positive = grow, negative = shrink).
        :returns: Offset polygons with Z from input.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "offset_polygon_3d")]
fn offset_polygon_3d_py(
    polygon: &Bound<'_, PyAny>,
    offset: f64,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let poly = extract_polygon3d(polygon)?;
    let result = offset_polygon_3d(&poly, offset);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def offset_polyline_3d(
        polyline: collections.abc.Sequence[types.Point3D],
        distance: float,
        closed: bool = False,
    ) -> types.Polygon3D:
        """Offset a 3D polyline in true 3D (edge-plane miter).

        Unlike :func:`offset_polygon_3d` (which projects to XY, offsets, then
        lifts back), this function offsets each vertex in the local plane of
        its two adjacent edges.  This gives a *true 3D offset* suitable for
        non-planar polylines.

        Positive distance offsets to the *left* of the traversal direction.

        :param polyline: Input 3D vertices as ``(x, y, z)`` points.
        :param distance: Offset distance (positive = left, negative = right).
        :param closed: When ``True``, the polyline is treated as a closed
            ring (last vertex connects back to first).  When ``False``
            (default), the first and last vertices are offset perpendicular
            to their single edge.
        :returns: Offset polyline with the same number of vertices.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "offset_polyline_3d")]
#[pyo3(signature = (polyline, distance, closed=false))]
fn offset_polyline_3d_py(
    polyline: Vec<PyPoint3D>,
    distance: f64,
    closed: bool,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(offset_polyline_3d(
        &poly3d_to_points(polyline),
        distance,
        closed,
    ))
}

// ── 3D Analytical functions ──────────────────────────────────────────

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_perimeter_3d(
        polygon: collections.abc.Sequence[types.Point3D],
    ) -> float:
        """Get the perimeter of a 3D polygon using full 3D edge lengths.

        :param polygon: Polygon as (x, y, z) points.
        :returns: Perimeter length.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygon_perimeter_3d")]
fn get_polygon_perimeter_3d_py(polygon: Vec<PyPoint3D>) -> f64 {
    get_polygon_perimeter_3d(&poly3d_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_area_3d(
        polygon: collections.abc.Sequence[types.Point3D],
    ) -> float:
        """XY-projected area of a 3D polygon (absolute shoelace area).

        :param polygon: Polygon as (x, y, z) points.
        :returns: XY-projected area.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygon_area_3d")]
fn get_polygon_area_3d_py(polygon: Vec<PyPoint3D>) -> f64 {
    get_polygon_area_3d(&poly3d_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_signed_area_3d(
        polygon: collections.abc.Sequence[types.Point3D],
    ) -> float:
        """Signed XY-projected area of a 3D polygon (shoelace formula).

        Positive for CCW winding, negative for CW.

        :param polygon: Polygon as (x, y, z) points.
        :returns: Signed XY-projected area.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygon_signed_area_3d")]
fn get_polygon_signed_area_3d_py(polygon: Vec<PyPoint3D>) -> f64 {
    get_polygon_signed_area_3d(&poly3d_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_bounds_3d(
        polygon: collections.abc.Sequence[types.Point3D],
    ) -> types.Rect3D:
        """Get the 3D bounding box of a polygon.

        :param polygon: Polygon as (x, y, z) points.
        :returns: Bounding box as (x_min, y_min, x_max, y_max, z_min, z_max).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygon_bounds_3d")]
fn get_polygon_bounds_3d_py(
    polygon: Vec<PyPoint3D>,
) -> (f64, f64, f64, f64, f64, f64) {
    let r = get_polygon_bounds_3d(&poly3d_to_points(polygon));
    (r.min.x, r.min.y, r.max.x, r.max.y, r.min.z, r.max.z)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def get_polygon_group_bounds_3d(
        polygons: collections.abc.Sequence[types.Polygon3D],
    ) -> types.Rect3D:
        """Get the 3D bounding box of a group of polygons.

        :param polygons: List of 3D polygons.
        :returns: Bounding box as (x_min, y_min, x_max, y_max, z_min, z_max).
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygon_group_bounds_3d")]
fn get_polygon_group_bounds_3d_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<(f64, f64, f64, f64, f64, f64)> {
    let p = extract_polygons3d(polygons)?;
    let r = get_polygon_group_bounds_3d(&p);
    Ok((r.min.x, r.min.y, r.max.x, r.max.y, r.min.z, r.max.z))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_centroid_3d(
        polygon: collections.abc.Sequence[types.Point3D],
    ) -> types.Point3D:
        """Get the centroid of a 3D polygon.

        XY centroid from shoelace formula, Z from average.

        :param polygon: Polygon as (x, y, z) points.
        :returns: Centroid point (x, y, z).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygon_centroid_3d")]
fn get_polygon_centroid_3d_py(polygon: Vec<PyPoint3D>) -> (f64, f64, f64) {
    point3d_to_tuple(get_polygon_centroid_3d(&poly3d_to_points(polygon)))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_edges_3d(
        polygon: collections.abc.Sequence[types.Point3D],
    ) -> list[tuple[types.Point3D, types.Point3D]]:
        """Get the edges of a 3D polygon.

        :param polygon: Polygon as (x, y, z) points.
        :returns: List of ((x1, y1, z1), (x2, y2, z2)) edges.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygon_edges_3d")]
fn get_polygon_edges_3d_py(polygon: Vec<PyPoint3D>) -> Vec<Edge3D> {
    edge_pairs3d_to_tuples(get_polygon_edges_3d(&poly3d_to_points(polygon)))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_convex_hull_3d(
        polygon: collections.abc.Sequence[types.Point3D],
    ) -> types.Polygon3D:
        """Get the convex hull of a 3D polygon (XY-plane, Z from first vertex).

        :param polygon: Polygon as (x, y, z) points.
        :returns: Convex hull as list of (x, y, z) points.
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygon_convex_hull_3d")]
fn get_polygon_convex_hull_3d_py(
    polygon: Vec<PyPoint3D>,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(get_polygon_convex_hull_3d(&poly3d_to_points(polygon)))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polyline_end_tangent_3d(
        polyline: collections.abc.Sequence[types.Point3D],
    ) -> types.Point:
        """Normalised tangent direction at the last point of a 3D polyline.

        Returns the normalised XY direction from the second-to-last point to
        the last point.  Falls back to ``(1.0, 0.0)`` when the polyline has
        fewer than 2 points or the last edge has zero length.

        :param polyline: Polyline as (x, y, z) points.
        :returns: Normalised (dx, dy) tangent direction.
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polyline_end_tangent_3d")]
fn get_polyline_end_tangent_3d_py(polyline: Vec<PyPoint3D>) -> (f64, f64) {
    let pt = get_polyline_end_tangent_3d(&poly3d_to_points(polyline));
    (pt.x, pt.y)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def walk_along_polyline_3d(
        polyline: collections.abc.Sequence[types.Point3D],
        start: tuple[float, float, float],
        forward: bool,
        distance: float,
    ) -> types.Point3D:
        """Walk along an open 3D polyline by a given arc length from a starting point.

        Given an open polyline and a starting point on it, walk along the
        polyline segments and return the point at exactly ``distance`` units
        away.  Clamps to the nearest endpoint when the walk would exceed it.

        :param polyline: Open polyline as (x, y, z) points.
        :param start: Starting point on the polyline.
        :param forward: Walk forward (along vertex order) if True, backward if False.
        :param distance: Arc length to walk.
        :returns: Point (x, y, z) at the given distance.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "walk_along_polyline_3d")]
fn walk_along_polyline_3d_py(
    polyline: Vec<PyPoint3D>,
    start: (f64, f64, f64),
    forward: bool,
    distance: f64,
) -> (f64, f64, f64) {
    point3d_to_tuple(walk_along_polyline_3d(
        &poly3d_to_points(polyline),
        &Point3D::new(start.0, start.1, start.2),
        forward,
        distance,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def walk_along_polygon_3d(
        polygon: collections.abc.Sequence[types.Point3D],
        start: tuple[float, float, float],
        forward: bool,
        distance: float,
    ) -> types.Point3D:
        """Walk along a closed 3D polygon by a given arc length from a starting point.

        Given a closed polygon and a starting point on it, walk along the
        polygon edges and return the point at exactly ``distance`` units
        away.  The walk wraps around the polygon (unlike
        :func:`walk_along_polyline_3d` which clamps at endpoints).

        :param polygon: Closed polygon as (x, y, z) points.
        :param start: Starting point on the polygon.
        :param forward: Walk forward (along vertex order) if True, backward if False.
        :param distance: Arc length to walk.
        :returns: Point (x, y, z) at the given distance.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "walk_along_polygon_3d")]
fn walk_along_polygon_3d_py(
    polygon: Vec<PyPoint3D>,
    start: (f64, f64, f64),
    forward: bool,
    distance: f64,
) -> (f64, f64, f64) {
    point3d_to_tuple(walk_along_polygon_3d(
        &poly3d_to_points(polygon),
        &Point3D::new(start.0, start.1, start.2),
        forward,
        distance,
    ))
}

#[allow(clippy::type_complexity)]
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_closest_point_3d(
        polygons: collections.abc.Sequence[types.Polygon3D],
        x: float,
        y: float,
        z: float,
    ) -> tuple[int, float, tuple[float, float, float], float] | None:
        """Find the closest point on any 3D polygon in a list to (x, y, z).

        :param polygons: List of 3D polygons as (x, y, z) points.
        :param x: X coordinate.
        :param y: Y coordinate.
        :param z: Z coordinate.
        :returns: (polygon_index, t, (cx, cy, cz), distance_squared) or None.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_closest_point_3d")]
fn get_polygons_closest_point_3d_py(
    polygons: &Bound<'_, PyAny>,
    x: f64,
    y: f64,
    z: f64,
) -> PyResult<Option<(usize, f64, (f64, f64, f64), f64)>> {
    let polys = extract_polygons3d(polygons)?;
    let point = Point3D::new(x, y, z);
    Ok(get_polygons_closest_point_3d(&polys, point)
        .map(|(pi, t, pt, d2)| (pi, t, (pt.x, pt.y, pt.z), d2)))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def deduplicate_polyline_3d(
        polyline: collections.abc.Sequence[types.Point3D],
    ) -> types.Polygon3D:
        """Remove consecutive near-identical points from a 3D polyline.

        Points whose squared distance is less than 1e-12 are collapsed.

        :param polyline: Polyline as (x, y, z) points.
        :returns: Deduplicated polyline.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "deduplicate_polyline_3d")]
fn deduplicate_polyline_3d_py(
    polyline: Vec<PyPoint3D>,
) -> Vec<(f64, f64, f64)> {
    let mut pts = poly3d_to_points(polyline);
    deduplicate_polyline_3d(&mut pts);
    points3d_to_tuples(pts)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def fillet_polyline_3d(
        polyline: collections.abc.Sequence[types.Point3D],
        radius: float,
    ) -> types.Polygon3D:
        """Fillet corners of a 3D polyline with circular arcs.

        Each internal vertex with enough room on both adjacent edges is
        replaced by a circular arc of the given radius tangent to both
        edges.

        :param polyline: Input polyline as (x, y, z) points.
        :param radius: Fillet radius (must be > 0).
        :returns: Filleted polyline (first and last points preserved).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "fillet_polyline_3d")]
fn fillet_polyline_3d_py(
    polyline: Vec<PyPoint3D>,
    radius: f64,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(fillet_polyline_3d(&poly3d_to_points(polyline), radius))
}

// ── 3D Transform functions ───────────────────────────────────────────

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def translate_polygon_3d(
        polygon: collections.abc.Sequence[types.Point3D],
        dx: float,
        dy: float,
        dz: float = 0.0,
    ) -> types.Polygon3D:
        """Translate a 3D polygon.

        :param polygon: Polygon as (x, y, z) points.
        :param dx: X translation.
        :param dy: Y translation.
        :param dz: Z translation.
        :returns: Translated polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "translate_polygon_3d")]
#[pyo3(signature = (polygon, dx, dy, dz=0.0))]
fn translate_polygon_3d_py(
    polygon: Vec<PyPoint3D>,
    dx: f64,
    dy: f64,
    dz: f64,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(translate_polygon_3d(
        &poly3d_to_points(polygon),
        dx,
        dy,
        dz,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def translate_polygons_3d(
        polygons: collections.abc.Sequence[types.Polygon3D],
        dx: float,
        dy: float,
        dz: float = 0.0,
    ) -> list[types.Polygon3D]:
        """Translate a list of 3D polygons.

        :param polygons: List of 3D polygons.
        :param dx: X translation.
        :param dy: Y translation.
        :param dz: Z translation.
        :returns: Translated polygons.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "translate_polygons_3d")]
#[pyo3(signature = (polygons, dx, dy, dz=0.0))]
fn translate_polygons_3d_py(
    polygons: &Bound<'_, PyAny>,
    dx: f64,
    dy: f64,
    dz: f64,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let p = extract_polygons3d(polygons)?;
    Ok(translate_polygons_3d(&p, dx, dy, dz)
        .into_iter()
        .map(points3d_to_tuples)
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def scale_polygon_3d(
        polygon: collections.abc.Sequence[types.Point3D],
        scale: float,
        scale_y: typing.Optional[float] = None,
        scale_z: typing.Optional[float] = None,
    ) -> types.Polygon3D:
        """Scale a 3D polygon.

        :param polygon: Polygon as (x, y, z) points.
        :param scale: X (and Y/Z if scale_y/scale_z are None) scale factor.
        :param scale_y: Y scale factor (optional).
        :param scale_z: Z scale factor (optional).
        :returns: Scaled polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "scale_polygon_3d")]
#[pyo3(signature = (polygon, scale, scale_y=None, scale_z=None))]
fn scale_polygon_3d_py(
    polygon: Vec<PyPoint3D>,
    scale: f64,
    scale_y: Option<f64>,
    scale_z: Option<f64>,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(scale_polygon_3d(
        &poly3d_to_points(polygon),
        scale,
        scale_y,
        scale_z,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def flip_polygon_3d(
        polygon: collections.abc.Sequence[types.Point3D],
        flip_h: bool = False,
        flip_v: bool = False,
        flip_z: bool = False,
    ) -> types.Polygon3D:
        """Flip a 3D polygon horizontally, vertically, and/or along Z.

        :param polygon: Polygon as (x, y, z) points.
        :param flip_h: Whether to flip horizontally (negate X).
        :param flip_v: Whether to flip vertically (negate Y).
        :param flip_z: Whether to flip along Z (negate Z).
        :returns: Flipped polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "flip_polygon_3d")]
#[pyo3(signature = (polygon, flip_h=false, flip_v=false, flip_z=false))]
fn flip_polygon_3d_py(
    polygon: Vec<PyPoint3D>,
    flip_h: bool,
    flip_v: bool,
    flip_z: bool,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(flip_polygon_3d(
        &poly3d_to_points(polygon),
        flip_h,
        flip_v,
        flip_z,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def flip_polygons_3d(
        polygons: collections.abc.Sequence[types.Polygon3D],
        flip_h: bool = False,
        flip_v: bool = False,
        flip_z: bool = False,
    ) -> list[types.Polygon3D]:
        """Flip multiple 3D polygons.

        :param polygons: List of 3D polygons.
        :param flip_h: Whether to flip horizontally (negate X).
        :param flip_v: Whether to flip vertically (negate Y).
        :param flip_z: Whether to flip along Z (negate Z).
        :returns: Flipped polygons.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "flip_polygons_3d")]
#[pyo3(signature = (polygons, flip_h=false, flip_v=false, flip_z=false))]
fn flip_polygons_3d_py(
    polygons: &Bound<'_, PyAny>,
    flip_h: bool,
    flip_v: bool,
    flip_z: bool,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let p = extract_polygons3d(polygons)?;
    Ok(flip_polygons_3d(&p, flip_h, flip_v, flip_z)
        .into_iter()
        .map(points3d_to_tuples)
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def rotate_polygon_3d(
        polygon: collections.abc.Sequence[types.Point3D],
        angle: float,
    ) -> types.Polygon3D:
        """Rotate a 3D polygon around the Z axis (XY rotation, Z preserved).

        :param polygon: Polygon as (x, y, z) points.
        :param angle: Rotation angle in degrees.
        :returns: Rotated polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "rotate_polygon_3d")]
fn rotate_polygon_3d_py(
    polygon: Vec<PyPoint3D>,
    angle: f64,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(rotate_polygon_3d(&poly3d_to_points(polygon), angle))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def rotate_polygons_3d(
        polygons: collections.abc.Sequence[types.Polygon3D],
        angle: float,
    ) -> list[types.Polygon3D]:
        """Rotate multiple 3D polygons around the Z axis.

        :param polygons: List of 3D polygons.
        :param angle: Rotation angle in degrees.
        :returns: Rotated polygons.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "rotate_polygons_3d")]
fn rotate_polygons_3d_py(
    polygons: &Bound<'_, PyAny>,
    angle: f64,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let p = extract_polygons3d(polygons)?;
    Ok(rotate_polygons_3d(&p, angle)
        .into_iter()
        .map(points3d_to_tuples)
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def resample_polyline_3d(
        points: collections.abc.Sequence[types.Point3D],
        max_segment_length: float,
        is_closed: bool,
    ) -> list[types.Point3D]:
        """Resample a 3D polyline with a maximum segment length.

        :param points: Sequence of 3D points.
        :param max_segment_length: Maximum allowed segment length.
        :param is_closed: Whether the polyline is closed.
        :returns: Resampled 3D points.
        :complexity: O(n) time, O(n) space
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "resample_polyline_3d")]
fn resample_polyline_3d_py(
    points: Vec<PyPoint3D>,
    max_segment_length: f64,
    is_closed: bool,
) -> Vec<(f64, f64, f64)> {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    let mut out = Vec::new();
    resample_polyline_3d(&pts, max_segment_length, is_closed, &mut out);
    points3d_to_tuples(out)
}

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "polygon3d")?;

    register_functions!(
        m,
        deduplicate_polyline_3d_py,
        fillet_polyline_3d_py,
        get_polygons_closest_point_3d_py,
        get_polygons_union_3d_py,
        walk_along_polygon_3d_py,
        walk_along_polyline_3d_py,
        get_polygons_intersection_3d_py,
        get_polygons_difference_3d_py,
        get_polygons_group_intersection_3d_py,
        get_polygons_group_difference_3d_py,
        offset_polygon_3d_py,
        offset_polyline_3d_py,
        get_polygon_perimeter_3d_py,
        get_polygon_area_3d_py,
        get_polygon_signed_area_3d_py,
        get_polygon_bounds_3d_py,
        get_polygon_group_bounds_3d_py,
        get_polygon_centroid_3d_py,
        get_polygon_edges_3d_py,
        get_polygon_convex_hull_3d_py,
        get_polyline_end_tangent_3d_py,
        translate_polygon_3d_py,
        translate_polygons_3d_py,
        scale_polygon_3d_py,
        flip_polygon_3d_py,
        flip_polygons_3d_py,
        rotate_polygon_3d_py,
        rotate_polygons_3d_py,
        resample_polyline_3d_py,
    );

    shape_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.polygon3d", &m)?;

    Ok(())
}
