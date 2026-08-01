---
title: raygeo.geo.shape.polygon
sidebar_label: raygeo.geo.shape.polygon
---

## CornerType

Which corner type to find in \[`find_polygon_corners`\].

- `CornerType.Convex`: convex corners (interior angle < 180°).
- `CornerType.Concave`: concave / reflex corners (default).

## JoinStyle

Corner join style for polygon offset operations.

- `JoinStyle.Miter`: Extends edges until they meet (default).
- `JoinStyle.Round`: Adds a circular arc at the corner.
- `JoinStyle.Square`: Extends edges by the offset distance.

## Functions

### `apply_minimum_curvature()`

```python
apply_minimum_curvature(
    polygon: Sequence[types.Point],
    r_min: float,
) -> list[types.Polygon]
```

Fillet tight internal corners to a minimum radius.

Offsets inward by `r_min` (Miter), then outward by `r_min` (Round). Acts as a high-pass curvature
filter — sharp corners are rounded to exactly `r_min` while the overall shape is preserved.

| Parameter    | Type                    | Description                       |
| ------------ | ----------------------- | --------------------------------- |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.         |
| `r_min`      | `float`                 | Minimum allowed curvature radius. |
| _Returns_    | `list[types.Polygon]`   | Filleted polygon(s).              |
| _Complexity_ |                         | O(n)                              |

![Minimum curvature fillet applied to a triangle](images/geo-shape-polygon-min-curvature.png)

*Minimum curvature fillet applied to a triangle*

### `clean_polygon()`

```python
clean_polygon(
    polygon: Sequence[types.Point],
    tolerance: Optional[float] = None,
) -> Optional[types.Polygon]
```

Clean a polygon by removing near-duplicate points.

| Parameter    | Type                      | Description                           |
| ------------ | ------------------------- | ------------------------------------- |
| `polygon`    | `Sequence[types.Point]`   | Input polygon as (x, y) points.       |
| `tolerance`  | `Optional[float] = None`  | Distance tolerance for deduplication. |
| _Returns_    | `Optional[types.Polygon]` | Cleaned polygon or None.              |
| _Complexity_ |                           | O(n)                                  |

![ removes near-duplicate vertices](images/geo-shape-polygon-clean-polygon.png)

*`clean_polygon` removes near-duplicate vertices*

### `do_polygons_intersect()`

```python
do_polygons_intersect(
    p1: Sequence[types.Point],
    p2: Sequence[types.Point],
    min_area: float = 0,
) -> bool
```

Check if two polygons intersect.

| Parameter    | Type                    | Description                          |
| ------------ | ----------------------- | ------------------------------------ |
| `p1`         | `Sequence[types.Point]` | First polygon as (x, y) points.      |
| `p2`         | `Sequence[types.Point]` | Second polygon as (x, y) points.     |
| `min_area`   | `float = 0`             | Minimum intersection area threshold. |
| _Returns_    | `bool`                  | True if polygons intersect.          |
| _Complexity_ |                         | O(n * m)                             |

### `does_path_sweep_intersect_polygon()`

```python
does_path_sweep_intersect_polygon(
    path: Sequence[types.Point],
    radius: float,
    obstacles: Sequence[types.Polygon],
) -> bool
```

Check if a disk swept along a path intersects any obstacle polygon.

Returns True when the Minkowski sweep of a disk of *radius* along *path* intersects any polygon in
*obstacles*.

| Parameter    | Type                      | Description                                |
| ------------ | ------------------------- | ------------------------------------------ |
| `path`       | `Sequence[types.Point]`   | Open polyline as (x, y) points.            |
| `radius`     | `float`                   | Disk radius.                               |
| `obstacles`  | `Sequence[types.Polygon]` | List of obstacle polygons.                 |
| _Returns_    | `bool`                    | True if any obstacle intersects the sweep. |
| _Complexity_ |                           | O(n * m)                                   |

![Tests whether the Minkowski sweep of a disk along a polyline intersects any obstacle polygon](images/geo-shape-polygon-path-sweep-intersect.png)

*Tests whether the Minkowski sweep of a disk along a polyline intersects any obstacle polygon*

### `does_polygon_enclose_circle()`

```python
does_polygon_enclose_circle(
    center: types.Point,
    radius: float,
    polygon: Sequence[types.Point],
) -> bool
```

Check if a polygon fully encloses a circle.

Uses a conservative fast check: the polygon's AABB must contain the circle's AABB, and the circle
center must be inside the polygon.

| Parameter    | Type                    | Description                                    |
| ------------ | ----------------------- | ---------------------------------------------- |
| `center`     | `types.Point`           | Circle center (x, y).                          |
| `radius`     | `float`                 | Circle radius.                                 |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.                      |
| _Returns_    | `bool`                  | True if the polygon fully encloses the circle. |
| _Complexity_ |                         | O(n)                                           |

### `find_entry_edges()`

```python
find_entry_edges(
    polygon: Sequence[tuple[float, float]],
    boundaries: Sequence[Sequence[tuple[float, float]]],
    dist_tol: float = 1,
) -> list[int]
```

| Parameter    | Type                                      | Description |
| ------------ | ----------------------------------------- | ----------- |
| `polygon`    | `Sequence[tuple[float, float]]`           |             |
| `boundaries` | `Sequence[Sequence[tuple[float, float]]]` |             |
| `dist_tol`   | `float = 1`                               |             |
| _Returns_    | `list[int]`                               |             |

![ finds narrow-passage edges not collinear with pocket boundary (in red)](images/geo-shape-polygon-find-entry-edges.png)

*`find_entry_edges` finds narrow-passage edges not collinear with pocket boundary (in red)*

### `find_polygon_corners()`

```python
find_polygon_corners(
    polygon: Sequence[tuple[float, float]],
    corner_type: CornerType = CornerType.Concave,
    threshold_deg: float = 90,
) -> list[tuple[int, float]]
```

Find corners of a polygon matching *corner_type*.

Returns a list of (vertex_index, interior_angle_deg) for each vertex whose interior angle is at
least *threshold_deg*. Winding is auto-detected from the signed area.

| Parameter       | Type                              | Description                                            |
| --------------- | --------------------------------- | ------------------------------------------------------ |
| `polygon`       | `Sequence[tuple[float, float]]`   | Polygon vertices (closed or open; treated as closed).  |
| `corner_type`   | `CornerType = CornerType.Concave` | `CornerType.Concave` (default) or `CornerType.Convex`. |
| `threshold_deg` | `float = 90`                      | Minimum interior angle in degrees (default 90).        |
| _Returns_       | `list[tuple[int, float]]`         | List of (vertex_index, interior_angle_deg) tuples.     |

![ labels convex (circle) and concave (square) vertices with interior angles](images/geo-shape-polygon-find-polygon-corners.png)

*`find_polygon_corners` labels convex (circle) and concave (square) vertices with interior angles*

### `flip_polygon()`

```python
flip_polygon(
    polygon: Sequence[types.Point],
    flip_h: bool,
    flip_v: bool,
) -> types.Polygon
```

Flip a polygon horizontally and/or vertically.

| Parameter    | Type                    | Description                   |
| ------------ | ----------------------- | ----------------------------- |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.     |
| `flip_h`     | `bool`                  | Whether to flip horizontally. |
| `flip_v`     | `bool`                  | Whether to flip vertically.   |
| _Returns_    | `types.Polygon`         | Flipped polygon.              |
| _Complexity_ |                         | O(n)                          |

### `flip_polygon_numpy()`

```python
flip_polygon_numpy(
    polygon: numpy.NDArray,
    flip_h: bool,
    flip_v: bool,
) -> numpy.NDArray
```

Flip a polygon from numpy array.

| Parameter    | Type            | Description                     |
| ------------ | --------------- | ------------------------------- |
| `polygon`    | `numpy.NDArray` | Polygon as a 2D numpy array.    |
| `flip_h`     | `bool`          | Whether to flip horizontally.   |
| `flip_v`     | `bool`          | Whether to flip vertically.     |
| _Returns_    | `numpy.NDArray` | Flipped polygon as numpy array. |
| _Complexity_ |                 | O(n)                            |

### `flip_polygons()`

```python
flip_polygons(
    polygons: Sequence[types.Polygon],
    flip_h: bool,
    flip_v: bool,
) -> list[types.Polygon]
```

Flip multiple polygons.

| Parameter    | Type                      | Description                   |
| ------------ | ------------------------- | ----------------------------- |
| `polygons`   | `Sequence[types.Polygon]` | List of polygons to flip.     |
| `flip_h`     | `bool`                    | Whether to flip horizontally. |
| `flip_v`     | `bool`                    | Whether to flip vertically.   |
| _Returns_    | `list[types.Polygon]`     | Flipped polygons.             |
| _Complexity_ |                           | O(n * m)                      |

### `flip_polygons_numpy()`

```python
flip_polygons_numpy(
    polygons: Sequence[numpy.NDArray],
    flip_h: bool,
    flip_v: bool,
) -> list[numpy.NDArray]
```

Flip polygons from numpy arrays.

| Parameter    | Type                      | Description                   |
| ------------ | ------------------------- | ----------------------------- |
| `polygons`   | `Sequence[numpy.NDArray]` | List of 2D numpy arrays.      |
| `flip_h`     | `bool`                    | Whether to flip horizontally. |
| `flip_v`     | `bool`                    | Whether to flip vertically.   |
| _Returns_    | `list[numpy.NDArray]`     | List of flipped numpy arrays. |
| _Complexity_ |                           | O(n * m)                      |

### `get_circle_polygon()`

```python
get_circle_polygon(
    center: types.Point,
    radius: float,
    n: int = 64,
) -> types.Polygon
```

Approximate a circle as an n-gon polygon.

| Parameter    | Type            | Description                       |
| ------------ | --------------- | --------------------------------- |
| `center`     | `types.Point`   | Centre point (x, y).              |
| `radius`     | `float`         | Circle radius.                    |
| `n`          | `int = 64`      | Number of sides (default 64).     |
| _Returns_    | `types.Polygon` | Polygon as list of (x, y) points. |
| _Complexity_ |                 | O(n)                              |

![ approximates a circle as an n-sided polygon](images/geo-shape-polygon-circle-polygon.png)

*`get_circle_polygon` approximates a circle as an n-sided polygon*

### `get_miter_offset_intersection()`

```python
get_miter_offset_intersection(
    v: types.Point,
    off_a: types.Point,
    dir_a: types.Point,
    off_b: types.Point,
    dir_b: types.Point,
) -> types.Point
```

Intersect two offset lines at a vertex for miter join.

Line A: `v + off_a + t * dir_a` Line B: `v + off_b + s * dir_b`

Returns the intersection point. When the lines are nearly parallel falls back to `v + off_a`.

| Parameter | Type          | Description                      |
| --------- | ------------- | -------------------------------- |
| `v`       | `types.Point` | Vertex point (x, y).             |
| `off_a`   | `types.Point` | Offset from *v* along line A.    |
| `dir_a`   | `types.Point` | Unit direction vector of line A. |
| `off_b`   | `types.Point` | Offset from *v* along line B.    |
| `dir_b`   | `types.Point` | Unit direction vector of line B. |
| _Returns_ | `types.Point` | Intersection point (x, y).       |

### `get_point_line_distance()`

```python
get_point_line_distance(
    point: types.Point,
    line_start: types.Point,
    line_end: types.Point,
) -> float
```

Compute the distance from a point to a line.

| Parameter    | Type          | Description              |
| ------------ | ------------- | ------------------------ |
| `point`      | `types.Point` | Point (x, y).            |
| `line_start` | `types.Point` | Line start point (x, y). |
| `line_end`   | `types.Point` | Line end point (x, y).   |
| _Returns_    | `float`       | Perpendicular distance.  |
| _Complexity_ |               | O(1)                     |

### `get_polygon_area()`

```python
get_polygon_area(polygon: Sequence[types.Point]) -> float
```

Get the unsigned area of a polygon.

| Parameter    | Type                    | Description               |
| ------------ | ----------------------- | ------------------------- |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points. |
| _Returns_    | `float`                 | Unsigned area.            |
| _Complexity_ |                         | O(n)                      |

### `get_polygon_boundary_distance()`

```python
get_polygon_boundary_distance(
    a: Sequence[tuple[float, float]],
    b: Sequence[tuple[float, float]],
) -> float
```

Minimum midpoint-to-segment distance between the boundaries of two polygons.

Uses segment midpoints rather than raw segment-segment distance to avoid false positives from
polygons that merely touch at a shared vertex.

| Parameter    | Type                            | Description                      |
| ------------ | ------------------------------- | -------------------------------- |
| `a`          | `Sequence[tuple[float, float]]` | First polygon as (x, y) points.  |
| `b`          | `Sequence[tuple[float, float]]` | Second polygon as (x, y) points. |
| _Returns_    | `float`                         | Minimum boundary distance.       |
| _Complexity_ |                                 | O(n * m)                         |

### `get_polygon_bounds()`

```python
get_polygon_bounds(polygon: Sequence[types.Point]) -> types.Rect
```

Get the bounding rectangle of a polygon.

| Parameter    | Type                    | Description                                         |
| ------------ | ----------------------- | --------------------------------------------------- |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.                           |
| _Returns_    | `types.Rect`            | Bounding rectangle as (x_min, y_min, x_max, y_max). |
| _Complexity_ |                         | O(n)                                                |

### `get_polygon_centroid()`

```python
get_polygon_centroid(polygon: Sequence[types.Point]) -> types.Point
```

Get the centroid of a polygon.

| Parameter    | Type                    | Description               |
| ------------ | ----------------------- | ------------------------- |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points. |
| _Returns_    | `types.Point`           | Centroid point (x, y).    |
| _Complexity_ |                         | O(n)                      |

![ computes the geometric center](images/geo-shape-polygon-centroid.png)

*`get_polygon_centroid` computes the geometric center*

### `get_polygon_closest_point()`

```python
get_polygon_closest_point(
    polygon: Sequence[types.Point],
    x: float,
    y: float,
) -> tuple[float, tuple[float, float], float] | None
```

Find the closest point on a polygon boundary to (x, y).

| Parameter | Type                                                   | Description                                            |
| --------- | ------------------------------------------------------ | ------------------------------------------------------ |
| `polygon` | `Sequence[types.Point]`                                | Polygon as (x, y) points.                              |
| `x`       | `float`                                                | X coordinate.                                          |
| `y`       | `float`                                                | Y coordinate.                                          |
| _Returns_ | `tuple[float, tuple[float, float], float] &#124; None` | (t, (cx, cy), distance_squared) or None if degenerate. |

![ finds the nearest boundary point to a given coordinate](images/geo-shape-polygon-polygon-closest-point.png)

*`get_polygon_closest_point` finds the nearest boundary point to a given coordinate*

### `get_polygon_convex_hull()`

```python
get_polygon_convex_hull(polygon: Sequence[types.Point]) -> types.Polygon
```

Get the convex hull of a polygon.

| Parameter    | Type                    | Description                    |
| ------------ | ----------------------- | ------------------------------ |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.      |
| _Returns_    | `types.Polygon`         | Convex hull as list of points. |
| _Complexity_ |                         | O(n log n)                     |

![ wraps polygon in convex hull](images/geo-shape-polygon-convex-hull.png)

*`get_polygon_convex_hull` wraps polygon in convex hull*

### `get_polygon_edges()`

```python
get_polygon_edges(
    polygon: Sequence[types.Point],
) -> list[tuple[types.Point, types.Point]]
```

Get the edges of a polygon.

| Parameter    | Type                                    | Description                         |
| ------------ | --------------------------------------- | ----------------------------------- |
| `polygon`    | `Sequence[types.Point]`                 | Polygon as (x, y) points.           |
| _Returns_    | `list[tuple[types.Point, types.Point]]` | List of ((x1, y1), (x2, y2)) edges. |
| _Complexity_ |                                         | O(n)                                |

### `get_polygon_from_points()`

```python
get_polygon_from_points(
    points: Sequence[types.Point],
    tolerance: Optional[float] = None,
) -> Optional[types.Polygon]
```

Convert a run of points into a cleaned polygon.

| Parameter    | Type                      | Description                                                                           |
| ------------ | ------------------------- | ------------------------------------------------------------------------------------- |
| `points`     | `Sequence[types.Point]`   | Vertices as (x, y) points.                                                            |
| `tolerance`  | `Optional[float] = None`  | Cleaning tolerance.                                                                   |
| _Returns_    | `Optional[types.Polygon]` | Cleaned polygon or the raw points if cleaning fails, or None for fewer than 3 points. |
| _Complexity_ |                           | O(n)                                                                                  |

### `get_polygon_group_bounds()`

```python
get_polygon_group_bounds(polygons: Sequence[types.Polygon]) -> types.Rect
```

Get the bounding rectangle of a group of polygons.

| Parameter    | Type                      | Description                                         |
| ------------ | ------------------------- | --------------------------------------------------- |
| `polygons`   | `Sequence[types.Polygon]` | List of polygons.                                   |
| _Returns_    | `types.Rect`              | Bounding rectangle as (x_min, y_min, x_max, y_max). |
| _Complexity_ |                           | O(n * m)                                            |

![ all polygons within a rect](images/geo-shape-polygon-group-bounds.png)

*`get_polygon_group_bounds` all polygons within a rect*

### `get_polygon_heading_at()`

```python
get_polygon_heading_at(
    polygon: list[tuple[float, float]],
    vertex: tuple[float, float],
) -> float
```

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| `polygon` | `list[tuple[float, float]]` |             |
| `vertex`  | `tuple[float, float]`       |             |
| _Returns_ | `float`                     |             |

![ draws outward-facing heading arrows at each vertex of a CCW polygon.](images/geo-shape-polygon-polygon-heading-at.png)

*`get_polygon_heading_at` draws outward-facing heading arrows at each vertex of a CCW polygon.*

### `get_polygon_perimeter()`

```python
get_polygon_perimeter(polygon: Sequence[types.Point]) -> float
```

Get the perimeter of a polygon.

| Parameter    | Type                    | Description               |
| ------------ | ----------------------- | ------------------------- |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points. |
| _Returns_    | `float`                 | Perimeter length.         |
| _Complexity_ |                         | O(n)                      |

### `get_polygon_signed_area()`

```python
get_polygon_signed_area(polygon: Sequence[types.Point]) -> float
```

Get the signed area of a polygon.

| Parameter    | Type                    | Description                                      |
| ------------ | ----------------------- | ------------------------------------------------ |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.                        |
| _Returns_    | `float`                 | Signed area (positive for CCW, negative for CW). |
| _Complexity_ |                         | O(n)                                             |

### `get_polygon_vertex_centroid()`

```python
get_polygon_vertex_centroid(
    polygon: Sequence[tuple[float, float]],
) -> tuple[float, float]
```

Arithmetic mean of polygon vertices (vertex-average centroid).

Unlike **get_polygon_centroid** (area-weighted shoelace centroid), this is useful for concave
polygons where the area centroid lies outside the boundary.

| Parameter    | Type                            | Description                     |
| ------------ | ------------------------------- | ------------------------------- |
| `polygon`    | `Sequence[tuple[float, float]]` | Polygon as (x, y) points.       |
| _Returns_    | `tuple[float, float]`           | Vertex-average centroid (x, y). |
| _Complexity_ |                                 | O(n)                            |

### `get_polygons_closest_point()`

```python
get_polygons_closest_point(
    polygons: Sequence[types.Polygon],
    x: float,
    y: float,
) -> tuple[int, float, tuple[float, float], float] | None
```

Find the closest point on any polygon in a list to (x, y).

| Parameter  | Type                                                        | Description                                             |
| ---------- | ----------------------------------------------------------- | ------------------------------------------------------- |
| `polygons` | `Sequence[types.Polygon]`                                   | List of polygons as (x, y) points.                      |
| `x`        | `float`                                                     | X coordinate.                                           |
| `y`        | `float`                                                     | Y coordinate.                                           |
| _Returns_  | `tuple[int, float, tuple[float, float], float] &#124; None` | (polygon_index, t, (cx, cy), distance_squared) or None. |

### `get_polygons_difference()`

```python
get_polygons_difference(
    poly1: Sequence[types.Point],
    poly2: Sequence[types.Point],
) -> list[types.Polygon]
```

Get the difference of two polygons.

| Parameter    | Type                    | Description                     |
| ------------ | ----------------------- | ------------------------------- |
| `poly1`      | `Sequence[types.Point]` | First polygon as (x, y) points. |
| `poly2`      | `Sequence[types.Point]` | Second polygon to subtract.     |
| _Returns_    | `list[types.Polygon]`   | Difference polygon(s).          |
| _Complexity_ |                         | O(n log n)                      |

![Polygon difference](images/geo-shape-polygon-boolean-difference.png)

*Polygon difference*

### `get_polygons_group_difference()`

```python
get_polygons_group_difference(
    subject: Sequence[types.Polygon],
    clip: Sequence[types.Polygon],
) -> list[types.Polygon]
```

Subtract clip polygons from subject polygons.

| Parameter    | Type                      | Description                |
| ------------ | ------------------------- | -------------------------- |
| `subject`    | `Sequence[types.Polygon]` | Subject polygons.          |
| `clip`       | `Sequence[types.Polygon]` | Clip polygons to subtract. |
| _Returns_    | `list[types.Polygon]`     | Difference polygon(s).     |
| _Complexity_ |                           | O(n log n)                 |

### `get_polygons_group_intersection()`

```python
get_polygons_group_intersection(
    subject: Sequence[types.Polygon],
    clip: Sequence[types.Polygon],
) -> list[types.Polygon]
```

Intersect two groups of polygons (subject & clip).

| Parameter    | Type                      | Description              |
| ------------ | ------------------------- | ------------------------ |
| `subject`    | `Sequence[types.Polygon]` | Subject polygons.        |
| `clip`       | `Sequence[types.Polygon]` | Clip polygons.           |
| _Returns_    | `list[types.Polygon]`     | Intersection polygon(s). |
| _Complexity_ |                           | O(n log n)               |

### `get_polygons_intersection()`

```python
get_polygons_intersection(
    poly1: Sequence[types.Point],
    poly2: Sequence[types.Point],
) -> list[types.Polygon]
```

Get the intersection of two polygons.

| Parameter    | Type                    | Description                      |
| ------------ | ----------------------- | -------------------------------- |
| `poly1`      | `Sequence[types.Point]` | First polygon as (x, y) points.  |
| `poly2`      | `Sequence[types.Point]` | Second polygon as (x, y) points. |
| _Returns_    | `list[types.Polygon]`   | Intersection polygon(s).         |
| _Complexity_ |                         | O(n log n)                       |

![Polygon intersection](images/geo-shape-polygon-boolean-intersection.png)

*Polygon intersection*

### `get_polygons_union()`

```python
get_polygons_union(polygons: Sequence[types.Polygon]) -> list[types.Polygon]
```

Get the union of multiple polygons.

| Parameter    | Type                      | Description                |
| ------------ | ------------------------- | -------------------------- |
| `polygons`   | `Sequence[types.Polygon]` | List of polygons to union. |
| _Returns_    | `list[types.Polygon]`     | Union polygon(s).          |
| _Complexity_ |                           | O(n log n)                 |

![Polygon union](images/geo-shape-polygon-boolean-union.png)

*Polygon union*

### `get_polyline_swept_polygon()`

```python
get_polyline_swept_polygon(
    path: Sequence[types.Point],
    radius: float,
) -> list[types.Polygon]
```

Compute the Minkowski sum of a polyline path with a disk.

Returns a single polygon covering the swept area — the union of segment-wide rectangular strips
capped with half-circles at the first and last endpoints.

| Parameter    | Type                    | Description                     |
| ------------ | ----------------------- | ------------------------------- |
| `path`       | `Sequence[types.Point]` | Open polyline as (x, y) points. |
| `radius`     | `float`                 | Offset radius.                  |
| _Returns_    | `list[types.Polygon]`   | A single swept polygon.         |
| _Complexity_ |                         | O(n)                            |

![ computes the Minkowski sum of a polyline path with a disk](images/geo-shape-polygon-polyline-swept.png)

*`get_polyline_swept_polygon` computes the Minkowski sum of a polyline path with a disk*

### `get_segment_swept_polygon()`

```python
get_segment_swept_polygon(
    a: types.Point,
    b: types.Point,
    radius: float,
) -> list[types.Polygon]
```

Compute the swept area of a line segment with a given radius.

Returns a rectangle (the Minkowski sum of the segment with a disk of *radius*) plus two disks at the
endpoints. Useful for toolpath clearance tracking and roughing simulation.

| Parameter    | Type                  | Description                                  |
| ------------ | --------------------- | -------------------------------------------- |
| `a`          | `types.Point`         | Start point (x, y).                          |
| `b`          | `types.Point`         | End point (x, y).                            |
| `radius`     | `float`               | Offset radius.                               |
| _Returns_    | `list[types.Polygon]` | List of polygons (rectangle + two end-caps). |
| _Complexity_ |                       | O(n)                                         |

![ computes the swept area of a line segment with a given radius](images/geo-shape-polygon-segment-swept.png)

*`get_segment_swept_polygon` computes the swept area of a line segment with a given radius*

### `get_signed_boundary_distance()`

```python
get_signed_boundary_distance(
    point: tuple[float, float],
    polygons: Sequence[Sequence[tuple[float, float]]],
) -> float
```

Signed perpendicular distance from point to nearest polygon boundary.

Positive = outside all polygons, Negative = inside any polygon, Zero = exactly on a boundary.

| Parameter  | Type                                      | Description           |
| ---------- | ----------------------------------------- | --------------------- |
| `point`    | `tuple[float, float]`                     | Query point `(x, y)`. |
| `polygons` | `Sequence[Sequence[tuple[float, float]]]` | List of polygons.     |
| _Returns_  | `float`                                   | Signed distance (mm). |

![Signed distance around a square. Red = outside (+), blue = inside (-), black contour = boundary.](images/geo-shape-polygon-signed-boundary-distance-field.png)

*Signed distance around a square. Red = outside (+), blue = inside (-), black contour = boundary.*

### `is_almost_equal()`

```python
is_almost_equal(a: float, b: float, tolerance: Optional[float] = None) -> bool
```

Check if two floats are almost equal.

| Parameter    | Type                     | Description           |
| ------------ | ------------------------ | --------------------- |
| `a`          | `float`                  | First float.          |
| `b`          | `float`                  | Second float.         |
| `tolerance`  | `Optional[float] = None` | Comparison tolerance. |
| _Returns_    | `bool`                   | True if               |
| _Complexity_ |                          | O(1)                  |

### `is_path_confined_to_boundary()`

```python
is_path_confined_to_boundary(
    path: Sequence[types.Point],
    boundary: Sequence[types.Point],
    clearance: float,
) -> bool
```

Check if a path stays within clearance of a pocket boundary.

Returns True when every vertex of *path* is inside *boundary* and no segment approaches within
*clearance* of any boundary edge.

| Parameter   | Type                    | Description                                 |
| ----------- | ----------------------- | ------------------------------------------- |
| `path`      | `Sequence[types.Point]` | Open polyline as (x, y) points.             |
| `boundary`  | `Sequence[types.Point]` | Pocket boundary polygon as (x, y) points.   |
| `clearance` | `float`                 | Minimum distance to boundary edges.         |
| _Returns_   | `bool`                  | True if path is safely inside the boundary. |

### `is_point_inside_polygon()`

```python
is_point_inside_polygon(
    point: types.Point,
    polygon: Sequence[types.Point],
) -> bool
```

Check if a point is inside a polygon.

| Parameter    | Type                    | Description                          |
| ------------ | ----------------------- | ------------------------------------ |
| `point`      | `types.Point`           | Point (x, y) to test.                |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.            |
| _Returns_    | `bool`                  | True if point is inside the polygon. |
| _Complexity_ |                         | O(n)                                 |

### `is_polygon_clockwise()`

```python
is_polygon_clockwise(points: Sequence[types.Point2DOr3D]) -> bool
```

Check if a polygon has clockwise winding.

| Parameter    | Type                          | Description                             |
| ------------ | ----------------------------- | --------------------------------------- |
| `points`     | `Sequence[types.Point2DOr3D]` | Sequence of (x, y) or (x, y, z) points. |
| _Returns_    | `bool`                        | True if the polygon is clockwise.       |
| _Complexity_ |                               | O(n)                                    |

### `is_polygon_convex()`

```python
is_polygon_convex(polygon: Sequence[types.Point]) -> bool
```

Check if a polygon is convex.

| Parameter    | Type                    | Description                    |
| ------------ | ----------------------- | ------------------------------ |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.      |
| _Returns_    | `bool`                  | True if the polygon is convex. |
| _Complexity_ |                         | O(n)                           |

### `normalize_polygons()`

```python
normalize_polygons(
    polygons: Sequence[types.Polygon],
) -> tuple[list[types.Polygon], float, float]
```

Normalize polygons (outer CCW, inner CW).

| Parameter    | Type                                       | Description                                   |
| ------------ | ------------------------------------------ | --------------------------------------------- |
| `polygons`   | `Sequence[types.Polygon]`                  | List of polygons to normalize.                |
| _Returns_    | `tuple[list[types.Polygon], float, float]` | Tuple of (normalized_polygons, min_x, min_y). |
| _Complexity_ |                                            | O(n log n)                                    |

### `normalize_polygons_numpy()`

```python
normalize_polygons_numpy(
    polygons: Sequence[numpy.NDArray],
) -> tuple[list[numpy.NDArray], float, float]
```

Normalize polygons from numpy arrays.

| Parameter    | Type                                       | Description                                 |
| ------------ | ------------------------------------------ | ------------------------------------------- |
| `polygons`   | `Sequence[numpy.NDArray]`                  | Sequence of 2D numpy arrays.                |
| _Returns_    | `tuple[list[numpy.NDArray], float, float]` | Tuple of (normalized_arrays, min_x, min_y). |
| _Complexity_ |                                            | O(n log n)                                  |

### `offset_polygon()`

```python
offset_polygon(
    polygon: Sequence[types.Point],
    offset: float,
    join_style: JoinStyle = JoinStyle.Miter,
) -> list[types.Polygon]
```

Offset (inflate/deflate) a polygon.

| Parameter    | Type                          | Description                                                 |
| ------------ | ----------------------------- | ----------------------------------------------------------- |
| `polygon`    | `Sequence[types.Point]`       | Polygon as (x, y) points.                                   |
| `offset`     | `float`                       | Offset distance (positive to inflate, negative to deflate). |
| `join_style` | `JoinStyle = JoinStyle.Miter` | Corner join style (default: `JoinStyle.Miter`).             |
| _Returns_    | `list[types.Polygon]`         | Offset polygon(s).                                          |
| _Complexity_ |                               | O(n log n)                                                  |

![Polygon offset — miter vs round vs square join styles](images/geo-shape-polygon-offset.png)

*Polygon offset — miter vs round vs square join styles*

### `point_in_polygon_numpy()`

```python
point_in_polygon_numpy(point: types.Point, polygon: numpy.NDArray) -> bool
```

Check if point is in polygon from numpy array.

| Parameter    | Type            | Description                          |
| ------------ | --------------- | ------------------------------------ |
| `point`      | `types.Point`   | Point (x, y) to test.                |
| `polygon`    | `numpy.NDArray` | Polygon as a 2D numpy array.         |
| _Returns_    | `bool`          | True if point is inside the polygon. |
| _Complexity_ |                 | O(n)                                 |

### `polygon_area_numpy()`

```python
polygon_area_numpy(polygon: numpy.NDArray) -> float
```

Get the area of a polygon from numpy array.

| Parameter    | Type            | Description                  |
| ------------ | --------------- | ---------------------------- |
| `polygon`    | `numpy.NDArray` | Polygon as a 2D numpy array. |
| _Returns_    | `float`         | Signed area.                 |
| _Complexity_ |                 | O(n)                         |

### `polygon_bounds_numpy()`

```python
polygon_bounds_numpy(polygon: numpy.NDArray) -> types.Rect
```

Get bounds of a polygon from numpy array.

| Parameter    | Type            | Description                                         |
| ------------ | --------------- | --------------------------------------------------- |
| `polygon`    | `numpy.NDArray` | Polygon as a 2D numpy array.                        |
| _Returns_    | `types.Rect`    | Bounding rectangle as (x_min, y_min, x_max, y_max). |
| _Complexity_ |                 | O(n)                                                |

### `polygon_group_bounds_numpy()`

```python
polygon_group_bounds_numpy(polygons: Sequence[numpy.NDArray]) -> types.Rect
```

Get bounds of polygon group from numpy arrays.

| Parameter    | Type                      | Description                                         |
| ------------ | ------------------------- | --------------------------------------------------- |
| `polygons`   | `Sequence[numpy.NDArray]` | Sequence of 2D numpy arrays.                        |
| _Returns_    | `types.Rect`              | Bounding rectangle as (x_min, y_min, x_max, y_max). |
| _Complexity_ |                           | O(n * m)                                            |

### `polygon_perimeter_numpy()`

```python
polygon_perimeter_numpy(polygon: numpy.NDArray) -> float
```

Get the perimeter of a polygon from numpy array.

| Parameter    | Type            | Description                  |
| ------------ | --------------- | ---------------------------- |
| `polygon`    | `numpy.NDArray` | Polygon as a 2D numpy array. |
| _Returns_    | `float`         | Perimeter length.            |
| _Complexity_ |                 | O(n)                         |

### `polygons_intersect_numpy()`

```python
polygons_intersect_numpy(
    poly1: numpy.NDArray,
    poly2: numpy.NDArray,
    min_area: float = 0,
) -> bool
```

Check if polygons intersect from numpy arrays.

| Parameter    | Type            | Description                          |
| ------------ | --------------- | ------------------------------------ |
| `poly1`      | `numpy.NDArray` | First polygon as a 2D numpy array.   |
| `poly2`      | `numpy.NDArray` | Second polygon as a 2D numpy array.  |
| `min_area`   | `float = 0`     | Minimum intersection area threshold. |
| _Returns_    | `bool`          | True if polygons intersect.          |
| _Complexity_ |                 | O(n * m)                             |

### `resample_polygon()`

```python
resample_polygon(
    polygon: Sequence[tuple[float, float]],
    spacing: float,
) -> list[tuple[float, float]]
```

Resample a closed polygon by inserting evenly-spaced points along each edge so that no segment is
longer than *spacing*.

The result is a closed polyline (last point connects back to first conceptually, but is not
duplicated).

| Parameter    | Type                            | Description                                 |
| ------------ | ------------------------------- | ------------------------------------------- |
| `polygon`    | `Sequence[tuple[float, float]]` | Polygon as (x, y) points.                   |
| `spacing`    | `float`                         | Maximum allowed segment length.             |
| _Returns_    | `list[tuple[float, float]]`     | Resampled polygon as list of (x, y) points. |
| _Complexity_ |                                 | O(n * m)                                    |

### `rotate_polygon()`

```python
rotate_polygon(polygon: Sequence[types.Point], angle: float) -> types.Polygon
```

Rotate a polygon by an angle.

| Parameter    | Type                    | Description                |
| ------------ | ----------------------- | -------------------------- |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points.  |
| `angle`      | `float`                 | Rotation angle in degrees. |
| _Returns_    | `types.Polygon`         | Rotated polygon.           |
| _Complexity_ |                         | O(n)                       |

### `rotate_polygon_numpy()`

```python
rotate_polygon_numpy(polygon: numpy.NDArray, angle: float) -> numpy.NDArray
```

Rotate a polygon from numpy array.

| Parameter    | Type            | Description                     |
| ------------ | --------------- | ------------------------------- |
| `polygon`    | `numpy.NDArray` | Polygon as a 2D numpy array.    |
| `angle`      | `float`         | Rotation angle in degrees.      |
| _Returns_    | `numpy.NDArray` | Rotated polygon as numpy array. |
| _Complexity_ |                 | O(n)                            |

### `rotate_polygons()`

```python
rotate_polygons(
    polygons: Sequence[types.Polygon],
    angle: float,
) -> list[types.Polygon]
```

Rotate multiple polygons by an angle.

| Parameter    | Type                      | Description                 |
| ------------ | ------------------------- | --------------------------- |
| `polygons`   | `Sequence[types.Polygon]` | List of polygons to rotate. |
| `angle`      | `float`                   | Rotation angle in degrees.  |
| _Returns_    | `list[types.Polygon]`     | Rotated polygons.           |
| _Complexity_ |                           | O(n * m)                    |

### `rotate_polygons_numpy()`

```python
rotate_polygons_numpy(
    polygons: Sequence[numpy.NDArray],
    angle: float,
) -> list[numpy.NDArray]
```

Rotate polygons from numpy arrays.

| Parameter    | Type                      | Description                   |
| ------------ | ------------------------- | ----------------------------- |
| `polygons`   | `Sequence[numpy.NDArray]` | Sequence of 2D numpy arrays.  |
| `angle`      | `float`                   | Rotation angle in degrees.    |
| _Returns_    | `list[numpy.NDArray]`     | List of rotated numpy arrays. |
| _Complexity_ |                           | O(n * m)                      |

### `scale_polygon()`

```python
scale_polygon(
    polygon: Sequence[types.Point],
    scale: float,
    scale_y: Optional[float] = None,
) -> types.Polygon
```

Scale a polygon.

| Parameter    | Type                     | Description                                |
| ------------ | ------------------------ | ------------------------------------------ |
| `polygon`    | `Sequence[types.Point]`  | Polygon as (x, y) points.                  |
| `scale`      | `float`                  | X (and Y if scale_y is None) scale factor. |
| `scale_y`    | `Optional[float] = None` | Y scale factor (optional).                 |
| _Returns_    | `types.Polygon`          | Scaled polygon.                            |
| _Complexity_ |                          | O(n)                                       |

### `to_clipper_numpy()`

```python
to_clipper_numpy(polygon: Sequence[numpy.NDArray]) -> list[tuple[int, int]]
```

Convert a numpy polygon to Clipper integer coordinates.

| Parameter    | Type                      | Description                    |
| ------------ | ------------------------- | ------------------------------ |
| `polygon`    | `Sequence[numpy.NDArray]` | Sequence of 2D numpy arrays.   |
| _Returns_    | `list[tuple[int, int]]`   | List of (x, y) integer tuples. |
| _Complexity_ |                           | O(n * m)                       |

### `translate_bounds()`

```python
translate_bounds(bounds: types.Rect, dx: float, dy: float) -> types.Rect
```

Translate a bounding rectangle.

| Parameter    | Type         | Description                                      |
| ------------ | ------------ | ------------------------------------------------ |
| `bounds`     | `types.Rect` | Bounding rectangle (x_min, y_min, x_max, y_max). |
| `dx`         | `float`      | X translation.                                   |
| `dy`         | `float`      | Y translation.                                   |
| _Returns_    | `types.Rect` | Translated bounding rectangle.                   |
| _Complexity_ |              | O(1)                                             |

### `translate_polygon()`

```python
translate_polygon(
    polygon: Sequence[types.Point],
    dx: float,
    dy: float,
) -> types.Polygon
```

Translate a polygon.

| Parameter    | Type                    | Description               |
| ------------ | ----------------------- | ------------------------- |
| `polygon`    | `Sequence[types.Point]` | Polygon as (x, y) points. |
| `dx`         | `float`                 | X translation.            |
| `dy`         | `float`                 | Y translation.            |
| _Returns_    | `types.Polygon`         | Translated polygon.       |
| _Complexity_ |                         | O(n)                      |

### `translate_polygon_numpy()`

```python
translate_polygon_numpy(
    polygon: numpy.NDArray,
    dx: float,
    dy: float,
) -> numpy.NDArray
```

Translate a polygon from numpy array.

| Parameter    | Type            | Description                        |
| ------------ | --------------- | ---------------------------------- |
| `polygon`    | `numpy.NDArray` | Polygon as a 2D numpy array.       |
| `dx`         | `float`         | X translation.                     |
| `dy`         | `float`         | Y translation.                     |
| _Returns_    | `numpy.NDArray` | Translated polygon as numpy array. |
| _Complexity_ |                 | O(n)                               |

### `translate_polygons()`

```python
translate_polygons(
    polygons: Sequence[types.Polygon],
    dx: float,
    dy: float,
) -> list[types.Polygon]
```

Translate a list of polygons.

| Parameter    | Type                      | Description                    |
| ------------ | ------------------------- | ------------------------------ |
| `polygons`   | `Sequence[types.Polygon]` | List of polygons to translate. |
| `dx`         | `float`                   | X translation.                 |
| `dy`         | `float`                   | Y translation.                 |
| _Returns_    | `list[types.Polygon]`     | Translated polygons.           |
| _Complexity_ |                           | O(n * m)                       |

### `translate_polygons_numpy()`

```python
translate_polygons_numpy(
    polygons: Sequence[numpy.NDArray],
    dx: float,
    dy: float,
) -> list[numpy.NDArray]
```

Translate polygons from numpy arrays.

| Parameter    | Type                      | Description                      |
| ------------ | ------------------------- | -------------------------------- |
| `polygons`   | `Sequence[numpy.NDArray]` | Sequence of 2D numpy arrays.     |
| `dx`         | `float`                   | X translation.                   |
| `dy`         | `float`                   | Y translation.                   |
| _Returns_    | `list[numpy.NDArray]`     | List of translated numpy arrays. |
| _Complexity_ |                           | O(n * m)                         |

### `walk_polygon_from_point()`

```python
walk_polygon_from_point(
    polygon: list[tuple[float, float]],
    start: tuple[float, float],
) -> list[tuple[int, float, float]]
```

| Parameter | Type                             | Description |
| --------- | -------------------------------- | ----------- |
| `polygon` | `list[tuple[float, float]]`      |             |
| `start`   | `tuple[float, float]`            |             |
| _Returns_ | `list[tuple[int, float, float]]` |             |

![ returns vertices in walk order from the vertex closest to a marker.](images/geo-shape-polygon-walk-polygon-from-point.png)

*`walk_polygon_from_point` returns vertices in walk order from the vertex closest to a marker.*

### `walk_polygon_vertices()`

```python
walk_polygon_vertices(
    polygon: list[tuple[float, float]],
    start_idx: int,
    forward: bool,
    stride: int = 1,
) -> list[tuple[int, float, float]]
```

| Parameter   | Type                             | Description |
| ----------- | -------------------------------- | ----------- |
| `polygon`   | `list[tuple[float, float]]`      |             |
| `start_idx` | `int`                            |             |
| `forward`   | `bool`                           |             |
| `stride`    | `int = 1`                        |             |
| _Returns_   | `list[tuple[int, float, float]]` |             |
