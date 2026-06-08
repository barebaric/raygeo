# raygeo

[![PyPI](https://img.shields.io/pypi/v/raygeo.svg)](https://pypi.org/project/raygeo/)
[![CI](https://github.com/barebaric/raygeo/actions/workflows/ci.yml/badge.svg)](https://github.com/barebaric/raygeo/actions/workflows/ci.yml)

A high-performance 2D/3D geometry library for Python, built in Rust with PyO3.

raygeo provides vector path construction, polygon boolean operations, curve
fitting, path transformations, and geometric queries — all backed by a native
Rust extension.

## Installation

```
pip install raygeo
```

Requires Python 3.10+ and a compatible platform (Linux, Windows, macOS Intel,
or macOS Apple Silicon). Pre-compiled wheels are available on
[PyPI](https://pypi.org/project/raygeo/).

## Quick Start

### Building Paths

The `Geometry` class is the core abstraction. It stores a vector path as a
sequence of move, line, arc, and cubic Bezier commands:

```python
from raygeo import Geometry

# Create a 10x10 square
g = Geometry()
g.move_to(0, 0)
g.line_to(10, 0)
g.line_to(10, 10)
g.line_to(0, 10)
g.close_path()

print(g.area())    # 100.0
print(g.rect())    # (0.0, 0.0, 10.0, 10.0)
print(g.is_closed())  # True
```

You can also create paths from point lists:

```python
triangle = Geometry.from_points([(0, 0), (10, 0), (5, 8.66)])
```

### Accessing Raw Data

Internally, geometry data is stored as an `(N, 8)` NumPy float64 array.
Each row represents one command:

```python
import numpy as np
from raygeo import COL_TYPE, COL_X, COL_Y, CMD_TYPE_LINE

data = g.data  # numpy array, shape (N, 8)
print(data[:, COL_X])  # all x coordinates
```

The 8 columns are:

| Column               | Index | Description                              |
| -------------------- | ----- | ---------------------------------------- |
| `COL_TYPE`           | 0     | Command type (move/line/arc/bezier)      |
| `COL_X`              | 1     | X coordinate                             |
| `COL_Y`              | 2     | Y coordinate                             |
| `COL_Z`              | 3     | Z coordinate                             |
| `COL_I` / `COL_C1X`  | 4     | Arc center offset X / Bezier control 1 X |
| `COL_J` / `COL_C1Y`  | 5     | Arc center offset Y / Bezier control 1 Y |
| `COL_CW` / `COL_C2X` | 6     | Arc clockwise flag / Bezier control 2 X  |
| — / `COL_C2Y`        | 7     | (unused for arcs) / Bezier control 2 Y   |

### Arcs and Bezier Curves

```python
g = Geometry()
g.move_to(0, 0)
g.arc_to(10, 0, i=5, j=0, clockwise=False)  # semicircular arc
g.close_path()

# Bezier curves
g2 = Geometry()
g2.move_to(0, 0)
g2.bezier_to(10, 0, c1x=3, c1y=5, c2x=7, c2y=5)

# Convert arcs to Bezier curves (for non-uniform scaling)
g3 = Geometry()
g3.move_to(0, 0)
g3.arc_to_as_bezier(10, 0, i=5, j=0)
g3.upgrade_to_scalable()
```

### Path Analysis

```python
print(g.distance())       # total path length
print(g.area())           # signed enclosed area
print(g.rect())           # bounding box (x_min, y_min, x_max, y_max)
print(g.is_closed())      # path closure check
print(g.segments())       # split into sub-paths

# Find closest point on path
result = g.find_closest_point(5, 5)  # (segment_index, distance, (px, py))

# Point and tangent at parameter t on a segment
pos, tangent = g.get_point_and_tangent_at(segment_index=0, t=0.5)
```

### Transformations

```python
import numpy as np
from raygeo import Geometry

g = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])

# Offset (grow/shrink)
grown = g.grow(2.0)   # offset outward by 2 units
shrunk = g.grow(-1.0)  # offset inward by 1 unit

# Affine transform (4x4 matrix)
matrix = [
    [1, 0, 0, 5],  # translate x by 5
    [0, 1, 0, 3],  # translate y by 3
    [0, 0, 1, 0],
    [0, 0, 0, 1],
]
g.transform(matrix)

# Map geometry into a frame
mapped = g.map_to_frame(
    origin=(0, 0),
    p_width=(100, 0),
    p_height=(0, 100),
)

g.flip_x()  # negate all x coordinates
g.flip_y()  # negate all y coordinates
```

### Contour Operations

```python
# Split into separate closed contours
contours = g.split_into_contours()

# Split into disconnected components
components = g.split_into_components()

# Separate holes from solids
inner, outer = g.split_inner_and_outer_contours()

# Remove shared edges between sub-paths
outer_only = g.remove_inner_edges()
```

### Polygon Operations

The `geo.shape.polygon` submodule provides polygon-specific operations powered by Clipper2:

```python
from raygeo import Geometry
from raygeo.geo.shape.polygon import (
    get_polygon_area,
    get_polygon_bounds,
    offset_polygon,
    get_polygons_union,
    get_polygons_intersection,
    get_polygons_difference,
    is_point_inside_polygon,
    polygons_intersect,
    get_polygon_convex_hull,
)

square = [(0, 0), (10, 0), (10, 10), (0, 10)]
circle_approx = [(5 + 5 * math.cos(a), 5 + 5 * math.sin(a))
                 for a in [i * math.pi / 20 for i in range(40)]]

get_polygon_area(square)                # 100.0
get_polygon_bounds(square)              # (0.0, 0.0, 10.0, 10.0)
is_point_inside_polygon((5, 5), square) # True

# Boolean operations
union = get_polygons_union([square, circle_approx])
intersection = get_polygons_intersection([square, circle_approx])
difference = get_polygons_difference([square], [circle_approx])

# Offset
inflated = offset_polygon(square, 2.0)

# NumPy variants are also available (suffixed with _numpy)
import numpy as np
sq_np = np.array(square)
get_polygon_area(sq_np)  # also works with numpy arrays
```

### Curve Fitting

```python
from raygeo import Geometry

# Simplify a path
g.simplify(tolerance=0.1)

# Convert curves to line segments
g.linearize(tolerance=0.01)

# Fit arcs and beziers to linear data
g.fit_arcs(tolerance=0.5)
g.fit_curves(tolerance=0.5, beziers=True, arcs=True)

# Convert geometry to polygons
polygons = g.to_polygons(tolerance=0.01)
```

### Self-Intersection Detection

```python
g.has_self_intersections()          # check for self-intersections
g.intersects_with(other_geometry)   # check intersection with another geometry
g.encloses(other_geometry)          # check if this fully encloses another
```

### Serialization

```python
# Dump to dict (JSON-safe)
data = g.dump()

# Load from dict
g2 = Geometry.load(data)

# Pickle support (via __reduce_ex__)
import pickle
g3 = pickle.loads(pickle.dumps(g))
```

## Documentation

Full API documentation is generated from the source type stubs and published
with the [RayForge Developer Docs](https://rayforge.org/docs/developer/raygeo-api/raygeo).

## Development

### Prerequisites

- [Rust toolchain](https://rustup.rs) (latest stable)
- Python 3.10+
- [maturin](https://www.maturin.rs) (`pip install maturin`)
- Node.js (only needed for `make lint-python`, which runs pyright via npx)

### Quick Start

```bash
# Create and activate a virtual environment (Unix)
python -m venv .venv
source .venv/bin/activate   # on Windows: .venv\Scripts\activate

# Install build tool and build the extension
pip install maturin pytest
make dev                   # builds Rust extension and installs into venv

# Run tests
make test

# Full check (lint + test)
make check
```

### Available Make Targets

| Target           | Description                              |
| ---------------- | ---------------------------------------- |
| `make dev`       | Build and install into the active venv   |
| `make build`     | Build release wheel to `dist/`           |
| `make test`      | Run pytest                               |
| `make lint`      | Lint Rust + Python (including pyright)   |
| `make format`    | Auto-format Rust + Python               |
| `make check`     | Lint + test                              |
| `make stubs`     | Regenerate `.pyi` type stubs             |
| `make visual`    | Launch Streamlit visual test playground  |

### Visual Testing

The `make visual` target launches an interactive Streamlit app for
exploring geometry operations visually. Install the optional deps first:

```bash
pip install -e ".[visual]"
make visual
```

## License

MIT
