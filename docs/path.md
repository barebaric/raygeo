# `raygeo.path` — Path-Level Operations

Path-level operations on `Geometry` objects and raw command arrays. Many
functions accept a `data` argument — a list of 8-element float lists (the same
layout as `Geometry.data`).

```python
from raygeo.path import (
    PyCommand,
    grow_geometry,
    split_into_contours,
    reverse_contour,
    normalize_winding_orders,
)
```

## `PyCommand`

Typed view over a single geometry command row. Use `isinstance` checks
to dispatch:

```python
for cmd in geometry.iter_typed_commands():
    if isinstance(cmd, PyCommand.Move):
        print("Move to", cmd.end)
    elif isinstance(cmd, PyCommand.Line):
        print("Line to", cmd.end)
    elif isinstance(cmd, PyCommand.Arc):
        print("Arc to", cmd.end, "offset", cmd.center_offset)
    elif isinstance(cmd, PyCommand.Bezier):
        print("Bezier to", cmd.end, "ctrl", cmd.control1, cmd.control2)
```

### Variants

| Variant            | Attributes                                                |
| ------------------ | --------------------------------------------------------- |
| `PyCommand.Move`   | `end: Point3D`                                            |
| `PyCommand.Line`   | `end: Point3D`                                            |
| `PyCommand.Arc`    | `end: Point3D`, `center_offset: Point`, `clockwise: bool` |
| `PyCommand.Bezier` | `end: Point3D`, `control1: Point`, `control2: Point`      |

## Geometry Operations

### `grow_geometry(geometry, offset) -> Geometry`

Offset a geometry outward (positive) or inward (negative).
Returns a new `Geometry`.

### `split_into_contours(geometry) -> List[Geometry]`

Split into individual closed contours.

### `split_into_components(geometry) -> List[Geometry]`

Split into disconnected components separated by move-to commands.

### `reverse_contour(contour) -> Geometry`

Reverse the winding direction of a closed contour. Returns a new `Geometry`.

### `split_inner_and_outer_contours(contours) -> Tuple[List[Geometry], List[Geometry]]`

Partition contours into inner (hole) and outer groups.
Returns `(inner, outer)`.

### `normalize_winding_orders(contours) -> List[Geometry]`

Normalize so outer contours are CCW and inner are CW.

### `filter_to_external_contours(contours) -> List[Geometry]`

Keep only the outermost contours, discarding holes.

### `remove_inner_edges(geometry) -> Geometry`

Remove edges shared between adjacent sub-paths.

### `close_all_contours(geometry) -> Geometry`

Ensure all sub-paths are closed.

### `close_geometry_gaps(geometry, tolerance) -> Geometry`

Close small gaps between adjacent path segments.

### `does_enclose(container, content) -> bool`

Check whether `container` fully encloses `content`.
Raises `RuntimeError` on failure.

### `get_valid_contours_data(contour_geometries) -> List[dict]`

Extract validated contour data. Each dict has keys `"geo"` (Geometry),
`"vertices"` (list of (x, y)), `"is_closed"` (bool), and
`"original_index"` (int).

## Raw Array Functions

### `get_bounding_rect_from_array(data) -> Tuple[float, float, float, float]`

Compute the 2D bounding box from raw command rows.
Returns `(x_min, y_min, x_max, y_max)`.

### `get_total_distance_from_array(data) -> float`

Compute total path length from raw command rows.

### `get_subpath_vertices_from_array(data, subpath_index) -> List[Point]`

Get vertices of a specific sub-path.

### `get_subpath_area_from_array(data, subpath_index) -> float`

Compute signed area of a specific sub-path.

### `get_area_from_array(data) -> float`

Compute total signed area of all sub-paths.

### `get_path_winding_order_from_array(data, start_cmd_index) -> str`

Determine winding order. Returns `"cw"`, `"ccw"`, or `"unknown"`.

### `get_point_tangent_at(data, row_index, t) -> Optional[Tuple[Point, Point]]`

Get position and tangent at parameter `t` on a command row.
Returns `((px, py), (tx, ty))` or `None`.

### `get_outward_normal_at_from_array(data, row_index, t) -> Optional[Point]`

Get outward normal at parameter `t`. Returns `(nx, ny)` or `None`.

## Intersection Checks

### `check_self_intersection(data, fail_on_t_junction) -> bool`

Check whether raw command array self-intersects. Pass `None` for `data`
to return `False`.

### `check_intersection(data1, data2, fail_on_t_junction) -> bool`

Check whether two raw command arrays intersect.

### `check_self_intersection_from_array(data, fail_on_t_junction) -> bool`

Like `check_self_intersection` but requires non-optional data.

### `check_intersection_from_array(data1, data2, fail_on_t_junction) -> bool`

Like `check_intersection` but requires non-optional data.

## Optimization & Fitting

### `optimize_path_from_array(data, tolerance, fit_arcs) -> NDArray`

Optimize by fitting curves and simplifying. Returns `(N, 8)` NumPy array.

### `fit_arcs(data, tolerance, progress_callback=None) -> Optional[List[List[float]]]`

Fit circular arcs to linear segments. Pass `None` for `data` to return
`None`.

### `fit_curves(data, tolerance, preserve_beziers, preserve_arcs) -> NDArray`

Fit curves (Beziers and/or arcs) to linear command data.

### `remove_duplicate_segments(data, tolerance=1e-6) -> Optional[NDArray]`

Remove duplicate segments. Returns `(N, 8)` NumPy array or `None`.

## Linearisation & Flattening

### `linearize_geometry(data, tolerance) -> NDArray`

Convert all curves to line segments. Returns `(N, 8)` NumPy array.

### `flatten_to_points(data, tolerance) -> List[List[Point3D]]`

Flatten curves to point sequences per sub-path.

## Command Construction

### `create_line_cmd(end_point) -> List[float]`

Build a single line-to command row (8-element list).

### `create_arc_cmd(end, center, start) -> List[float]`

Build a single arc-to command row.

### `convert_arc_to_beziers_from_array(start, end, center_offset, clockwise) -> List[List[float]]`

Convert an arc to cubic Bezier command rows.

## Transformation

### `apply_affine_transform_to_array(data, matrix) -> NDArray`

Apply a 4x4 affine transform to raw command rows. Returns `(N, 8)` NumPy
array.

### `map_geometry_to_frame(geometry, origin, p_width, p_height, ...) -> Geometry`

Map a geometry into a rectangular frame with optional anchoring.

**Parameters:**

- `origin` — Frame origin `(x, y)`.
- `p_width` — `(source_width, target_width)`.
- `p_height` — `(source_height, target_height)`.
- `anchor_y`, `anchor_x` — Optional anchor positions.
- `stable_src_height`, `stable_src_width` — Source dimensions to keep stable.

### `extract_overcut_rows(data, max_length) -> Optional[NDArray]`

Extract overcut rows from command data.

## Utility Functions

### `get_angle_at_vertex(p0, p1, p2) -> float`

Compute angle at middle vertex `p1` in radians.

### `remove_duplicates(points) -> List[Point]`

Remove consecutive duplicate 2D points.

### `is_clockwise(points) -> bool`

Check whether a polygon winds clockwise.

### `is_closed(commands, tolerance=1e-6) -> bool`

Check whether a raw command array forms a closed path.

### `_are_points_equal(p1, p2, tolerance) -> bool`

Check if two 3D points are within tolerance of each other.

### `_get_segment_key(data, index, _tolerance) -> Optional[Tuple[str, ...]]`

Extract a hashable segment key for deduplication.

### `_are_segments_equal(key1, key2, tolerance) -> bool`

Compare two segment keys for equality within tolerance.

### `_partial_segment_from_row(row, start_point, t) -> Optional[List[float]]`

Compute a partial segment from start_point to parameter `t`.

### `_segment_length_from_row(row, start_point) -> float`

Compute the length of a single segment.
