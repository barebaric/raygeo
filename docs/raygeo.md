# `raygeo` — Top-Level Module

[![PyPI](https://img.shields.io/pypi/v/raygeo.svg)](https://pypi.org/project/raygeo/)

The top-level `raygeo` module provides the `Geometry` class, type aliases,
constants, and a small set of utility functions.

```python
import raygeo
from raygeo import Geometry, Point, Polygon, Rect
```

## Installation

```
pip install raygeo
```

## Type Aliases

| Alias         | Type                                | Description                           |
| ------------- | ----------------------------------- | ------------------------------------- |
| `Point`       | `Tuple[float, float]`               | 2D point `(x, y)`                     |
| `Point3D`     | `Tuple[float, float, float]`        | 3D point `(x, y, z)`                  |
| `Rect`        | `Tuple[float, float, float, float]` | Bounding box                          |
| `Polygon`     | `List[Tuple[float, float]]`         | 2D polygon (list of vertices)         |
| `Polygon3D`   | `List[Tuple[float, float, float]]`  | 3D polygon (list of vertices)         |
| `IntPoint`    | `Tuple[int, int]`                   | Integer 2D point                      |
| `IntPolygon`  | `List[Tuple[int, int]]`             | Integer polygon                       |
| `Edge`        | `Tuple[Point, Point]`               | Line segment `(start, end)`           |
| `CubicBezier` | `Tuple[Point, Point, Point, Point]` | Bezier control points `(p0,c1,c2,p1)` |
| `Point2DOr3D` | `Union[Point, Point3D]`             | 2D or 3D point                        |

## `Rect3D`

A `namedtuple("Rect3D", ["x_min", "x_max", "y_min", "y_max", "z_min", "z_max"])`
representing a 3D axis-aligned bounding box.

## Constants

| Constant          | Type  | Description                                               |
| ----------------- | ----- | --------------------------------------------------------- |
| `CMD_TYPE_MOVE`   | `int` | Command type for move-to (1)                              |
| `CMD_TYPE_LINE`   | `int` | Command type for line-to (2)                              |
| `CMD_TYPE_ARC`    | `int` | Command type for arc-to (3)                               |
| `CMD_TYPE_BEZIER` | `int` | Command type for Bezier curve (4)                         |
| `COL_TYPE`        | `int` | Column index for command type (0)                         |
| `COL_X`           | `int` | Column index for x coordinate (1)                         |
| `COL_Y`           | `int` | Column index for y coordinate (2)                         |
| `COL_Z`           | `int` | Column index for z coordinate (3)                         |
| `COL_I`           | `int` | Arc center x offset / Bezier ctrl 1 x (4)                 |
| `COL_J`           | `int` | Arc center y offset / Bezier ctrl 1 y (5)                 |
| `COL_CW`          | `int` | Arc clockwise flag / Bezier ctrl 2 x (6)                  |
| `COL_C1X`         | `int` | Bezier control point 1 x (4)                              |
| `COL_C1Y`         | `int` | Bezier control point 1 y (5)                              |
| `COL_C2X`         | `int` | Bezier control point 2 x (6)                              |
| `COL_C2Y`         | `int` | Bezier control point 2 y (7)                              |
| `GEO_ARRAY_COLS`  | `int` | Total columns in the data array (8)                       |
| `CLIPPER_SCALE`   | `int` | Default scale for Clipper integer conversion (10,000,000) |

All constants are also available as attributes of the `constants` namespace
object:

```python
from raygeo import constants
print(constants.COL_X)  # 1
```

## Functions

### `clip_line_segment_with_polygons(p1, p2, regions)`

Clip a 3D line segment against a set of 2D polygon regions. Returns only the
portions that fall inside at least one polygon. Z coordinates are linearly
interpolated.

```python
from raygeo import clip_line_segment_with_polygons

segments = clip_line_segment_with_polygons(
    p1=(0, 0, 0),
    p2=(10, 0, 0),
    regions=[[(2, -1), (8, -1), (8, 1), (2, 1)]],
)
# Returns: [((2.0, 0.0, 0.0), (8.0, 0.0, 0.0))]
```

**Parameters:**

- `p1` (`Point3D`) — Start of the line segment.
- `p2` (`Point3D`) — End of the line segment.
- `regions` — List of polygons. Each polygon is a list of `(x, y)` tuples
  or an `(N, 2)` NumPy array.

**Returns:** `List[Tuple[Point3D, Point3D]]`

### `is_arc_inside_polygons(arc_start, arc_end, arc_center, clockwise, polygons)`

Check whether an arc lies entirely inside a set of polygons.

**Parameters:**

- `arc_start` (`Point`) — Start point.
- `arc_end` (`Point`) — End point.
- `arc_center` (`Point`) — Center point.
- `clockwise` (`bool`) — Arc direction.
- `polygons` — List of polygons (tuples or NumPy arrays).

**Returns:** `bool`

### `is_bezier_inside_polygons(p0, p1, p2, p3, polygons)`

Check whether a cubic Bezier curve lies entirely inside a set of polygons.

**Parameters:**

- `p0`, `p1`, `p2`, `p3` (`Point`) — Bezier control points.
- `polygons` — List of polygons.

**Returns:** `bool`

### `fit_points_with_primitives(points, tolerance)`

Fit a sequence of 3D points to geometric primitives (lines, arcs,
Bezier curves).

**Parameters:**

- `points` (`List[Point3D]`) — Ordered points to fit.
- `tolerance` (`float`) — Maximum deviation from original points.

**Returns:** `List[List[float]]` — List of 8-element command rows.

### `to_clipper(polygon, scale=None)`

Convert a float polygon to integer Clipper format by scaling each vertex.

**Parameters:**

- `polygon` — List of `(x, y)` tuples or `(N, 2)` NumPy array.
- `scale` (`int`, optional) — Scale factor. Defaults to
  `CLIPPER_SCALE` (10,000,000).

**Returns:** `List[Tuple[int, int]]`

### `from_clipper(polygon, scale=None)`

Convert an integer Clipper polygon back to float format by dividing each
vertex.

**Parameters:**

- `polygon` (`List[Tuple[int, int]]`) — Integer vertices.
- `scale` (`int`, optional) — Scale factor. Defaults to `CLIPPER_SCALE`.

**Returns:** `Polygon`

## Submodules

- [`raygeo.geometry`](geometry.md) — The `Geometry` class
- [`raygeo.path`](path.md) — Path-level operations
- [`raygeo.shape`](shape.md) — Shape primitives (arcs, beziers, circles, lines, polygons)
- [`raygeo.algo`](algo.md) — Algorithms (clipping, fitting, minkowski, simplify, smooth)
