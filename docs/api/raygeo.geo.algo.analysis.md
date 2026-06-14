---
title: raygeo.geo.algo.analysis
sidebar_label: raygeo.geo.algo.analysis
sidebar_position: 4
---

Path analysis utilities for inspecting and cleaning geometry data.

Provides functions for removing duplicate points from point sequences, extracting subpath vertices,
computing subpath/geometry area, and determining path winding order.

## Functions

### `get_area()`

`get_area(geometry: geo.Geometry) -> float`

Compute the total unsigned area enclosed by the geometry.

Sums all subpaths (outer + inner). Returns 0 for empty or open geometry.

**Returns:** Total unsigned area.

| Parameter    | Type           | Description                    |
| ------------ | -------------- | ------------------------------ |
| `geometry`   | `geo.Geometry` | Geometry to compute area from. |
| _Returns_    | `float`        |                                |
| _Complexity_ |                | O(n) time, O(1) space          |

### `get_path_winding_order()`

`get_path_winding_order(geometry: geo.Geometry, start_cmd_index: int) -> str`

Determine the winding order of a subpath.

**Returns:** `"ccw"`, `"cw"`, or `"unknown"`.

| Parameter         | Type           | Description                    |
| ----------------- | -------------- | ------------------------------ |
| `geometry`        | `geo.Geometry` | Geometry to analyze.           |
| `start_cmd_index` | `int`          | Index of the starting command. |
| _Returns_         | `str`          |                                |
| _Complexity_      |                | O(n) time, O(1) space          |

### `get_subpath_area()`

`get_subpath_area(geometry: geo.Geometry, start_cmd_index: int) -> float`

Compute the signed area of a subpath using the shoelace formula.

Positive area is CCW, negative is CW. Returns 0 for unclosed subpaths.

**Returns:** Signed area.

| Parameter         | Type           | Description                    |
| ----------------- | -------------- | ------------------------------ |
| `geometry`        | `geo.Geometry` | Geometry to compute area from. |
| `start_cmd_index` | `int`          | Index of the starting command. |
| _Returns_         | `float`        |                                |
| _Complexity_      |                | O(n) time, O(1) space          |

### `get_subpath_vertices()`

`get_subpath_vertices(geometry: geo.Geometry, start_cmd_index: int) -> list[tuple[float, float]]`

Extract vertices from a subpath starting at the given command index.

Linearizes arcs and beziers into vertex sequences.

**Returns:** List of (x, y) vertices.

| Parameter         | Type                        | Description                                                                                     |
| ----------------- | --------------------------- | ----------------------------------------------------------------------------------------------- |
| `geometry`        | `geo.Geometry`              | Geometry to extract vertices from.                                                              |
| `start_cmd_index` | `int`                       | Index of the starting command.                                                                  |
| _Returns_         | `list[tuple[float, float]]` |                                                                                                 |
| _Complexity_      |                             | O(n + m) time, O(m) space where n is the number of commands and m the number of output vertices |

### `remove_duplicates()`

`remove_duplicates(points: collections.abc.Sequence[types.Point]) -> types.Polygon`

Remove duplicate points from a sequence.

**Returns:** List of unique points.

| Parameter    | Type                                    | Description                |
| ------------ | --------------------------------------- | -------------------------- |
| `points`     | `collections.abc.Sequence[types.Point]` | Sequence of (x, y) points. |
| _Returns_    | `types.Polygon`                         |                            |
| _Complexity_ |                                         | O(n) time, O(n) space      |
