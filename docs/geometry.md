# `raygeo.geometry` — The Geometry Class

The `Geometry` class is the core abstraction of raygeo. It stores a vector path
as a sequence of move, line, arc, and cubic Bezier commands, backed by an
`(N, 8)` NumPy float64 array.

```python
from raygeo import Geometry
```

## Construction

### `Geometry()`

Create an empty geometry.

```python
g = Geometry()
```

### `Geometry.from_points(points, close=True)`

Create a geometry from a list of 2D or 3D points connected by straight lines.
The first point becomes a move-to; subsequent points are line-to commands.

```python
triangle = Geometry.from_points([(0, 0), (10, 0), (5, 8.66)])
square = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
```

**Parameters:**

- `points` (`List[Tuple[x, y]]` or `List[Tuple[x, y, z]]`) — Ordered vertices.
- `close` (`bool`) — Whether to close the path. Defaults to `True`.

**Returns:** `Geometry`

### `Geometry.load(data)` / `Geometry.from_dict(data)`

Deserialise from a dict produced by `dump()`.

### `Geometry.__len__()`

Returns the number of commands (implicitly calls `sync_to_data`).

## Drawing Commands

### `move_to(x, y, z=0.0)`

Move the pen to an absolute position without drawing.

```python
g.move_to(0, 0)
```

### `line_to(x, y, z=0.0)`

Draw a straight line to an absolute position.

```python
g.line_to(10, 0)
```

### `close_path()`

Close the current sub-path by drawing a line back to the last `move_to`
position.

```python
g.close_path()
```

### `arc_to(x, y, i=0.0, j=0.0, clockwise=True, z=0.0)`

Draw a circular arc to an absolute position. The arc center is specified as an
offset `(i, j)` relative to the current pen position.

```python
g.move_to(0, 0)
g.arc_to(10, 0, i=5, j=0, clockwise=False)  # semicircular arc
```

**Parameters:**

- `x`, `y` — End point coordinates.
- `i` — X offset from current position to arc center.
- `j` — Y offset from current position to arc center.
- `clockwise` (`bool`) — `True` for clockwise. Defaults to `True`.
- `z` — End point z coordinate.

### `bezier_to(x, y, c1x, c1y, c2x, c2y, z=0.0)`

Draw a cubic Bezier curve to an absolute position.

```python
g.move_to(0, 0)
g.bezier_to(10, 0, c1x=3, c1y=5, c2x=7, c2y=5)
```

### `arc_to_as_bezier(x, y, i, j, clockwise=True, z=0.0)`

Draw an arc by converting it to one or more cubic Bezier segments. The
resulting geometry is marked as uniformly scalable (safe for non-uniform
scaling transforms).

## Properties

### `data` (get/set)

The command data as an `(N, 8)` NumPy float64 array, or `None` when empty.
Setting replaces the entire command buffer. Pass `None` to clear.

```python
arr = g.data          # numpy array or None
g.data = new_array    # replace commands
g.data = None         # clear
```

### `last_move_to` (get/set)

The last position set by `move_to` as `(x, y, z)`.

### `uniform_scalable` (get/set)

Whether the geometry can be non-uniformly scaled without distortion. Becomes
`True` after `upgrade_to_scalable()` converts arcs to Bezier curves.

### `_pending_data` (read-only)

Commands that have not yet been flushed to `data`. Returns a list of 8-element
float lists.

## Query Methods

### `is_empty() -> bool`

Check whether the geometry contains no commands.

### `rect() -> Tuple[float, float, float, float]`

Compute the 2D axis-aligned bounding box.
Returns `(x_min, y_min, x_max, y_max)`.

```python
x_min, y_min, x_max, y_max = g.rect()
```

### `distance() -> float`

Compute the total path length (2D).

### `area() -> float`

Compute the signed enclosed area. Positive = CCW winding, negative = CW.

### `is_closed(tolerance=1e-6) -> bool`

Check whether the path returns to its start point within tolerance.

### `segments() -> List[List[Point3D]]`

Split the geometry into sub-paths. Each sub-path starts at a move-to command.

```python
for subpath in g.segments():
    print(len(subpath), "vertices")
```

### `get_command_at(index) -> Optional[Tuple]`

Retrieve a single command by index. Returns an 8-element tuple or `None` if
out of range.

### `iter_commands() -> List[Tuple]`

Return all commands as a flat list of 8-element tuples.

### `iter_typed_commands() -> List[PyCommand]`

Return typed command objects (`PyCommand.Move`, `.Line`, `.Arc`, `.Bezier`).
See [path module](path.md#pycommand) for details.

```python
for cmd in g.iter_typed_commands():
    if isinstance(cmd, PyCommand.Move):
        print("Move to", cmd.end)
    elif isinstance(cmd, PyCommand.Line):
        print("Line to", cmd.end)
    elif isinstance(cmd, PyCommand.Arc):
        print("Arc to", cmd.end, "center offset", cmd.center_offset)
    elif isinstance(cmd, PyCommand.Bezier):
        print("Bezier to", cmd.end, "ctrl", cmd.control1, cmd.control2)
```

### `get_typed_command_at(index) -> Optional[PyCommand]`

Retrieve a typed command by index.

### `find_closest_point(x, y) -> Optional[Tuple[int, float, Point]]`

Find the closest point on the path to a query position.
Returns `(segment_index, distance, (px, py))` or `None` if empty.

### `get_point_and_tangent_at(segment_index, t) -> Optional[Tuple[Point, Point]]`

Get position and unit tangent at parameter `t` in `[0, 1]` on a segment.
Returns `((px, py), (tx, ty))` or `None` if empty.

### `get_outward_normal_at(segment_index, t) -> Optional[Point]`

Get the outward-facing unit normal at parameter `t`.
Returns `(nx, ny)` or `None` if empty.

### `has_self_intersections(fail_on_t_junction=False) -> bool`

Check whether the path self-intersects.

**Parameters:**

- `fail_on_t_junction` — If `True`, T-junctions are treated as intersections.

### `intersects_with(other) -> bool`

Check whether this geometry intersects another.

### `encloses(other) -> bool`

Check whether this geometry fully encloses another. Raises `RuntimeError` if
computation fails.

## Transformation Methods

All transformation methods return `self` for chaining unless noted otherwise.

### `transform(matrix) -> Geometry`

Apply a 4x4 affine transformation matrix in-place.

```python
import math
# Translate by (5, 3)
g.transform([
    [1, 0, 0, 5],
    [0, 1, 0, 3],
    [0, 0, 1, 0],
    [0, 0, 0, 1],
])
```

**Parameters:** `matrix` — A 4x4 list-of-lists or NumPy array.

### `grow(amount) -> Geometry`

Offset the geometry outward (positive) or inward (negative). Returns a **new**
`Geometry` (does not modify in place).

```python
inflated = g.grow(2.0)    # offset outward by 2
shrunk = g.grow(-1.0)     # offset inward by 1
```

### `flip_x() -> Geometry`

Negate all x coordinates in-place.

### `flip_y() -> Geometry`

Negate all y coordinates in-place.

### `simplify(tolerance) -> Geometry`

Simplify the path by removing redundant points.

### `linearize(tolerance) -> Geometry`

Convert arcs and Beziers to line segments.

### `fit_curves(tolerance, beziers, arcs, on_progress=None) -> Geometry`

Fit curves to linear path data.

**Parameters:**

- `tolerance` — Maximum deviation from original.
- `beziers` (`bool`) — Whether to fit Bezier curves.
- `arcs` (`bool`) — Whether to fit circular arcs.
- `on_progress` — Optional progress callback (ignored).

### `fit_arcs(tolerance) -> Geometry`

Shortcut for `fit_curves()` with only arcs enabled.

### `upgrade_to_scalable() -> Geometry`

Convert all arcs to Bezier curves so the geometry can be non-uniformly scaled.
Sets `uniform_scalable` to `True`.

### `close_gaps(tolerance) -> Geometry`

Close small gaps between adjacent segments.

### `cleanup(tolerance) -> Geometry`

Remove duplicate segments from the path.

### `append_data(rows)`

Append raw command rows from a NumPy array.

**Parameters:** `rows` — An `(N, 8)` float64 array, or `None` (no-op).

## Contour Operations

### `split_into_contours() -> List[Geometry]`

Split into individual closed contours. Each contour starts with a move-to and
ends where it began.

### `split_into_components() -> List[Geometry]`

Split into disconnected components separated by move-to commands.

### `split_inner_and_outer_contours() -> Tuple[List[Geometry], List[Geometry]]`

Partition sub-paths into inner (holes) and outer contours.
Returns `(inner, outer)`.

### `remove_inner_edges() -> Geometry`

Remove edges shared between adjacent sub-paths. Returns a new `Geometry`.

### `map_to_frame(origin, p_width, p_height, ...) -> Geometry`

Map this geometry into a rectangular frame with optional anchoring.

```python
mapped = g.map_to_frame(
    origin=(0, 0),
    p_width=(100, 0),
    p_height=(0, 100),
)
```

**Parameters:**

- `origin` (`Point`) — Frame origin `(x, y)`.
- `p_width` (`Tuple[float, float]`) — `(source_width, target_width)`.
- `p_height` (`Tuple[float, float]`) — `(source_height, target_height)`.
- `anchor_y` (`float`, optional) — Y anchor position.
- `stable_src_height` (`float`, optional) — Source height to keep stable.
- `anchor_x` (`float`, optional) — X anchor position.
- `stable_src_width` (`float`, optional) — Source width to keep stable.

### `to_polygons(tolerance=0.01) -> List[Polygon]`

Convert the geometry to a list of simple polygons. Curves are first
linearised.

## Serialization

### `dump() -> dict`

Serialise to a plain dict with keys `"last_move_to"`, `"uniform_scalable"`,
and `"commands"`. Suitable for JSON.

```python
data = g.dump()
import json
json_str = json.dumps(data)
```

### `load(data) -> Geometry` / `from_dict(data) -> Geometry`

Deserialise from a dict.

```python
g2 = Geometry.load(data)
```

The `Geometry` class also supports pickle via `__reduce_ex__`:

```python
import pickle
g3 = pickle.loads(pickle.dumps(g))
```

## Miscellaneous

### `clear()`

Remove all commands.

### `copy() -> Geometry`

Return a deep copy.

### `extend(other)`

Append all commands from another geometry.

### `sync_to_data()`

Flush pending commands to the NumPy backing store. Most methods call this
automatically.
