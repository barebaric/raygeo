# `raygeo.shape` — Shape Primitives

Shape query submodules for arcs, Bezier curves, circles, lines, polygons,
rectangles, and points. All submodules are re-exported from `raygeo.shape`:

```python
from raygeo.shape.arc import get_arc_bounds
from raygeo.shape.bezier import get_bezier_point_at
from raygeo.shape.polygon import get_polygon_area
from raygeo.shape import is_point_inside_polygon  # also works
```

---

## `raygeo.shape.arc`

Arc shape queries — bounds, angles, linearisation, and containment.

### `get_arc_bounds(start, end, center, clockwise) -> Rect`

Compute the 2D bounding box of a circular arc.

### `get_arc_direction(center, start, mouse) -> bool`

Determine arc direction from center, start, and mouse point. Returns `True`
for clockwise. Useful for interactive tools.

### `get_arc_closest_point(arc_cmd, start_pos, x, y) -> Optional[Tuple[float, Point, float]]`

Find closest point on an arc to query position `(x, y)`.
Returns `(distance, (px, py), t)` or `None`.

`arc_cmd` can be an 8-element list or an object with `end`,
`center_offset`, and `clockwise` attributes.

### `get_arc_midpoint(start, end, center, clockwise) -> Point`

Compute the midpoint on a circular arc.

### `get_arc_angles(start, end, center, clockwise) -> Tuple[float, float, float]`

Compute start angle, end angle, and sweep in radians.

### `does_arc_intersect_rect(arc_start, arc_end, arc_center, clockwise, rect) -> bool`

Check whether an arc intersects an axis-aligned rectangle.

### `does_arc_intersect_circle(..., circle_center, circle_radius) -> bool`

Check whether an arc intersects a circle. Full signature:
`does_arc_intersect_circle(arc_start, arc_end, arc_center, clockwise,
circle_center, circle_radius)`.

Check whether an arc intersects a circle.

### `is_arc_clockwise(points, center) -> bool`

Determine winding direction of an arc from sample points.

### `is_arc_inside_polygons(arc_start, arc_end, arc_center, clockwise, polygons) -> bool`

Check whether an arc lies entirely inside a set of polygons. Polygons can be
lists of `(x, y)` tuples or `(N, 2)` NumPy arrays.

### `is_angle_between(angle, start, end, clockwise) -> bool`

Check whether `angle` lies within the arc sweep from `start` to `end`.

### `normalize_angle(angle) -> float`

Normalize angle into `[0, 2π)`.

### `linearize_arc(arc_cmd, start_point, resolution=0.1) -> List[Tuple[Point3D, Point3D]]`

Convert an arc into line segments. Returns list of
`((x1,y1,z1), (x2,y2,z2))` segments.

### `linearize_arc_from_array(data, start_point, max_seg_length) -> List[List[float]]`

Linearise an arc from a raw 8-element command row.
Returns list of `[x1, y1, z1, x2, y2, z2]`.

---

## `raygeo.shape.bezier`

Cubic Bezier curve operations — evaluation, splitting, bounding,
linearisation, and containment.

### `get_bezier_point_at(p0, p1, p2, p3, t) -> Point`

Evaluate a cubic Bezier at parameter `t`. Only x, y coordinates used.

### `split_bezier(p0, p1, p2, p3, t) -> Tuple[CubicBezier, CubicBezier]`

Split a cubic Bezier at parameter `t` using de Casteljau's algorithm.
Each half is `((p0, c1, c2, p1))`.

### `get_bezier_bounds(p0, p1, p2, p3) -> Rect`

Compute the 2D bounding box.

### `get_bezier_rect_intersections(p0, p1, p2, p3, rect) -> List[float]`

Find parameter values where a Bezier intersects a rectangle.
Returns `t` values in `[0, 1]`.

### `clip_bezier_with_rect(p0, p1, p2, p3, rect) -> List[CubicBezier]`

Clip a cubic Bezier to a rectangle. Returns one or more Bezier curves.

### `convert_cubic_bezier_to_quadratic(p0, p1, p2, p3) -> Tuple[Point, Point, Point]`

Approximate a cubic Bezier with a quadratic one.
Returns `(start, control, end)`.

### `is_bezier_inside_polygons(p0, p1, p2, p3, polygons) -> bool`

Check whether a cubic Bezier lies entirely inside polygons.

### `linearize_bezier(p0, p1, p2, p3, num_steps) -> List[Tuple[Point3D, Point3D]]`

Uniformly linearise into `num_steps` line segments.

### `linearize_bezier_adaptive(p0, p1, p2, p3, tolerance_sq, max_subdivisions=20) -> List[Point]`

Adaptively linearise a 2D Bezier using recursive subdivision.

### `linearize_bezier_from_array(bezier_row, start_point, max_seg_length) -> List[List[float]]`

Linearise from a raw 8-element command row.

### `linearize_bezier_segment(p0, p1, p2, p3, tolerance=0.1) -> List[Point3D]`

Adaptive linearisation of a 3D Bezier segment.

### `flatten_bezier(p0, p1, p2, p3, tolerance, max_subdivisions, pts) -> None`

Flatten a 3D Bezier by appending points to an existing list
(mutates in place).

### `bezier_flatness_sq(a, b, c, d) -> float`

Compute squared flatness — how close the curve is to a straight line.
Zero means perfectly flat.

### `perp_dist_sq(pt, origin, vx, vy, vz=0.0, norm_sq=0.0) -> float`

Squared perpendicular distance from `pt` to a line defined by `origin`
and direction `(vx, vy, vz)`.

---

## `raygeo.shape.circle`

Circle shape queries — intersections, containment, and projection.

### `get_circle_circle_intersections(c1, r1, c2, r2) -> List[Point]`

Compute intersection points of two circles. Returns 0, 1, or 2 points.

### `is_circle_inside_rect(center, radius, rect) -> bool`

Check whether a circle is entirely inside a rectangle.

### `does_circle_intersect_rect(center, radius, rect) -> bool`

Check whether a circle intersects or touches a rectangle.

### `line_segment_intersects_circle(p1, p2, circle_center, circle_radius) -> bool`

Check whether a line segment intersects a circle.

### `project_point_onto_circle(point, center, radius) -> Optional[Point]`

Project a point onto the circumference. Returns `None` if point coincides
with center.

---

## `raygeo.shape.line`

Line and line-segment queries — intersections, closest points, and distances.

### `get_line_line_intersection(p1, p2, p3, p4) -> Optional[Point]`

Intersection of two infinite lines. Returns `None` if parallel.

### `get_line_segment_intersection(p1, p2, p3, p4) -> Optional[Point]`

Intersection of two finite segments. Returns `None` if they don't intersect.

### `get_line_closest_point(line_p1, line_p2, x, y) -> Point`

Closest point on an infinite line to query position.

### `get_line_segment_closest_point(seg_p1, seg_p2, x, y) -> Tuple[float, Point, float]`

Closest point on a segment. Returns `(distance, (px, py), t)`.

### `get_point_line_distance(point, line_p1, line_p2) -> float`

Perpendicular distance from a point to an infinite line.

### `is_point_on_line_segment(point, seg_p1, seg_p2) -> bool`

Check whether a point lies on a line segment.

### `does_line_segment_intersect_rect(p1, p2, rect) -> bool`

Check whether a segment intersects a rectangle.

### `does_line_segment_intersect_circle(p1, p2, circle_center, circle_radius) -> bool`

Check whether a segment intersects a circle.

### `get_line_segment_polygon_intersections(p1, p2, polygon) -> List[float]`

Parametric intersection values of a segment with polygon edges.

---

## `raygeo.shape.point`

Single-point utility functions.

### `midpoint(p1, p2) -> Point3D`

Compute the midpoint between two points. Accepts 2D or 3D tuples.

---

## `raygeo.shape.polygon`

Polygon operations — area, bounds, Boolean ops, transforms, and
NumPy-accelerated variants.

### Basic Properties

| Function                   | Signature                 | Description                     |
| -------------------------- | ------------------------- | ------------------------------- |
| `get_polygon_area`         | `(polygon) -> float`      | Signed area (positive = CCW)    |
| `get_polygon_signed_area`  | `(polygon) -> float`      | Signed area                     |
| `get_polygon_perimeter`    | `(polygon) -> float`      | Perimeter length                |
| `get_polygon_bounds`       | `(polygon) -> Rect`       | Axis-aligned bounding box       |
| `get_polygon_group_bounds` | `(polygons) -> Rect`      | Bounding box for multiple polys |
| `get_polygon_centroid`     | `(polygon) -> Point`      | Centroid                        |
| `is_polygon_convex`        | `(polygon) -> bool`       | Convexity check                 |
| `get_polygon_convex_hull`  | `(polygon) -> Polygon`    | Convex hull (CCW order)         |
| `get_polygon_edges`        | `(polygon) -> List[Edge]` | Edge pairs                      |

### Point & Polygon Tests

| Function                  | Signature                        | Description        |
| ------------------------- | -------------------------------- | ------------------ |
| `is_point_inside_polygon` | `(point, polygon) -> bool`       | Point-in-polygon   |
| `point_line_distance`     | `(pt, start, end) -> float`      | Perpendicular dist |
| `polygons_intersect`      | `(p1, p2, min_area=0.0) -> bool` | Intersection test  |

### Boolean Operations

| Function                    | Signature                        | Description            |
| --------------------------- | -------------------------------- | ---------------------- |
| `get_polygons_union`        | `(polygons) -> List[Polygon]`    | Union                  |
| `get_polygons_intersection` | `(p1, p2) -> List[Polygon]`      | Intersection           |
| `get_polygons_difference`   | `(p1, p2) -> List[Polygon]`      | Difference p1 \ p2     |
| `offset_polygon`            | `(polygon, offset) -> List[...]` | Inflate (+)/deflate(-) |

### Transforms

| Function             | Signature                                   | Description            |
| -------------------- | ------------------------------------------- | ---------------------- |
| `flip_polygon`       | `(polygon, flip_h, flip_v) -> Polygon`      | Flip h/v               |
| `flip_polygons`      | `(polygons, flip_h, flip_v) -> List[...]`   | Flip multiple          |
| `rotate_polygon`     | `(polygon, angle) -> Polygon`               | Rotate (radians)       |
| `rotate_polygons`    | `(polygons, angle) -> List[...]`            | Rotate multiple        |
| `scale_polygon`      | `(polygon, scale, scale_y=None) -> Polygon` | Scale (separate x/y)   |
| `translate_polygon`  | `(polygon, dx, dy) -> Polygon`              | Translate              |
| `translate_polygons` | `(polygons, dx, dy) -> List[...]`           | Translate multiple     |
| `translate_bounds`   | `(bounds, dx, dy) -> Rect`                  | Translate bounding box |

### Utility

| Function             | Signature                             | Description            |
| -------------------- | ------------------------------------- | ---------------------- |
| `clean_polygon`      | `(poly, tol=1e-6) -> Optional[...]`   | Remove duplicate verts |
| `is_almost_equal`    | `(a, b, tol=1e-9) -> bool`            | Float equality check   |
| `normalize_polygons` | `(polygons) -> (List[...],f, f)`      | Shift to positive quad |
| `to_clipper_numpy`   | `(polygon, scale=10M) -> List[Tuple]` | Convert to Clipper int |

### NumPy Variants

All have the same functionality as their pure-Python counterparts but
accept/return `(N, 2)` float64 NumPy arrays:

`polygon_area_numpy`, `polygon_bounds_numpy`, `polygon_perimeter_numpy`,
`polygon_group_bounds_numpy`, `flip_polygon_numpy`, `flip_polygons_numpy`,
`normalize_polygons_numpy`, `point_in_polygon_numpy`,
`polygons_intersect_numpy`, `rotate_polygon_numpy`,
`rotate_polygons_numpy`, `translate_polygon_numpy`,
`translate_polygons_numpy`

---

## `raygeo.shape.rect`

Rectangle shape queries — point containment and overlap tests.

### `is_point_inside_rect(point, rect) -> bool`

Check whether a point is inside (or on the boundary of) a rectangle.

### `does_rect_contain_rect(outer, inner) -> bool`

Check whether `outer` fully contains `inner`.

### `does_rect_intersect_rect(r1, r2) -> bool`

Check whether two rectangles overlap (touching counts as intersection).
