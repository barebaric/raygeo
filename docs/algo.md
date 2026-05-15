# `raygeo.algo` — Algorithms

Algorithm submodules for clipping, fitting, Minkowski sums, simplification,
and smoothing. All submodules are re-exported from `raygeo.algo`:

```python
from raygeo.algo import clip_line_segment_with_polygons
from raygeo.algo import fit_circle_to_3_points
from raygeo.algo import get_no_fit_polygon
from raygeo.algo import simplify_polyline
from raygeo.algo import smooth_polyline
```

---

## `raygeo.algo.clipping`

Line-segment clipping against rectangles and polygons.

### `clip_line_segment_with_rect(p1, p2, rect) -> Optional[Tuple[Point3D, Point3D]]`

Clip a 3D line segment to a 2D axis-aligned rectangle. Returns the clipped
segment or `None` if entirely outside.

### `subtract_polygons_from_line_segment(p1, p2, regions) -> List[Tuple[Point3D, Point3D]]`

Subtract polygon regions from a line segment. Returns only portions **not**
inside any polygon.

### `clip_line_segment_with_polygons(p1, p2, regions) -> List[Tuple[Point3D, Point3D]]`

Keep only portions of a segment that fall inside at least one polygon.
Z coordinates are linearly interpolated.

---

## `raygeo.algo.fitting`

Curve fitting — circle fitting, Bezier conversion, and primitive extraction
from point sequences.

### Circle Fitting

| Function                            | Signature                                          | Description              |
| ----------------------------------- | -------------------------------------------------- | ------------------------ |
| `fit_circle_to_3_points`            | `(p1, p2, p3) -> Optional[Tuple[Point, float]]`    | Fit circle through 3 pts |
| `fit_circle_to_points`              | `(points) -> Optional[Tuple[Point, float, float]]` | Least-squares fit        |
| `project_circle_center_to_bisector` | `(p1, p2, center) -> Point`                        | Project onto bisector    |

### Point Analysis

### `are_points_collinear(points, tolerance=1e-6) -> bool`

Check whether all points lie on approximately the same line.

### Curve Fitting

| Function                      | Signature                                        | Description              |
| ----------------------------- | ------------------------------------------------ | ------------------------ |
| `fit_points_with_primitives`  | `(points, tolerance) -> List[List[float]]`       | Fit lines, arcs, Beziers |
| `fit_points_recursive`        | `(points, tol, start, end) -> List[List[float]]` | Recursive curve fitting  |
| `get_polyline_line_deviation` | `(points, start, end) -> Tuple[float, int]`      | Max deviation from line  |
| `get_polyline_arc_deviation`  | `(points, center, radius) -> float`              | RMS deviation from arc   |

### Command Construction & Conversion

| Function                            | Signature                                       | Description         |
| ----------------------------------- | ----------------------------------------------- | ------------------- |
| `create_line_cmd`                   | `(end_point) -> List[float]`                    | Build a line-to row |
| `create_arc_cmd`                    | `(end, center, start) -> List[float]`           | Build an arc-to row |
| `convert_arc_to_beziers_from_array` | `(start, end, offset, cw) -> List[List[float]]` | Arc to Bezier rows  |

### Linearisation

| Function             | Signature                                  | Description             |
| -------------------- | ------------------------------------------ | ----------------------- |
| `flatten_to_points`  | `(data, tolerance) -> List[List[Point3D]]` | Command array to points |
| `linearize_geometry` | `(data, tolerance) -> List[List[float]]`   | Curves to line segments |

---

## `raygeo.algo.minkowski`

Minkowski sum and No-Fit Polygon algorithms for 2D packing/nesting.

### `get_polygon_minkowski_sum_convex(poly_a, poly_b) -> List[List[Tuple[int, int]]]`

Minkowski sum of two **convex** integer polygons.

### `get_inner_fit_polygon(outer, inner) -> List[List[Point]]`

Compute the Inner Fit Polygon (IFP) — all valid placements of `inner`
fully inside `outer`.

### `get_no_fit_polygon(subject, tool) -> List[List[Point]]`

Compute the No-Fit Polygon (NFP) — all relative positions where `tool`
touches but doesn't overlap `subject`.

### `calculate_input_scale(polygons, max_int=2147483647) -> float`

Calculate scale factor for integer-based computations.

### `convolve_two_segments(a1, a2, b1, b2) -> List[Tuple[int, int]]`

Convolve two integer line segments (Minkowski sum of edges).

### `convolve_point_sequences(seq_a, seq_b) -> List[List[Tuple[int, int]]]`

Convolve two integer point sequences.

---

## `raygeo.algo.simplify`

Polyline simplification using the Douglas-Peucker algorithm.

### `simplify_polyline(points, tolerance) -> List[Tuple[float, ...]]`

Simplify a 2D polyline. Points are projected to z=0 for computation.

### `simplify_polyline_to_array(data, tolerance) -> List[List[float]]`

Simplify a polyline stored as a 2D array. Extra columns beyond x, y are
preserved in the output.

---

## `raygeo.algo.smooth`

Polyline smoothing algorithms — Gaussian kernels, circular smoothing, and
sub-segment smoothing.

### `compute_gaussian_kernel(amount) -> Tuple[List[float], float]`

Build a normalized Gaussian convolution kernel with `2 * amount + 1`
elements. Returns `(kernel, sigma)`.

### `smooth_circularly(points, kernel) -> List[Point3D]`

Smooth a closed polyline using circular convolution (wraps around).

### `smooth_polyline(points, amount, corner_angle_threshold, is_closed=None) -> List[Point3D]`

Smooth with corner preservation. Corners sharper than
`corner_angle_threshold` radians are left un-smoothed.

### `smooth_sub_segment(points, kernel) -> List[Point3D]`

Smooth an open sub-segment (no wrapping).

### `resample_polyline(points, max_segment_length, is_closed) -> List[Point3D]`

Resample so no segment exceeds `max_segment_length`. New points inserted
by linear interpolation.
