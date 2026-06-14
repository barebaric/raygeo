---
title: raygeo.geo.algo.interp
sidebar_label: raygeo.geo.algo.interp
sidebar_position: 8
---

Segment interpolation utilities for parameter-based point projection, clipping, and scanline data
slicing along 3D line segments.

## Functions

### `compute_segment_delta()`

`compute_segment_delta(start: tuple[float, float, float], end: tuple[float, float, float]) -> tuple[float, float, float, float]`

Compute delta vector and squared length between two 3D points.

**Returns:** (dx, dy, dz, len_sq).

| Parameter | Type                                | Description               |
| --------- | ----------------------------------- | ------------------------- |
| `start`   | `tuple[float, float, float]`        | Starting point (x, y, z). |
| `end`     | `tuple[float, float, float]`        | Ending point (x, y, z).   |
| _Returns_ | `tuple[float, float, float, float]` |                           |

### `compute_t_range()`

`compute_t_range(origin: tuple[float, float, float], new_start: tuple[float, float, float], new_end: tuple[float, float, float], delta: tuple[float, float, float, float]) -> tuple[float, float]`

Compute parameter range (t_start, t_end) for a clipped sub-segment.

**Returns:** (t_start, t_end) in [0, 1].

| Parameter   | Type                                | Description                               |
| ----------- | ----------------------------------- | ----------------------------------------- |
| `origin`    | `tuple[float, float, float]`        | Start of original segment (x, y, z).      |
| `new_start` | `tuple[float, float, float]`        | Start of clipped sub-segment (x, y, z).   |
| `new_end`   | `tuple[float, float, float]`        | End of clipped sub-segment (x, y, z).     |
| `delta`     | `tuple[float, float, float, float]` | Segment delta from compute_segment_delta. |
| _Returns_   | `tuple[float, float]`               |                                           |

### `project_t_along_segment()`

`project_t_along_segment(origin: tuple[float, float, float], point: tuple[float, float, float], delta: tuple[float, float, float, float]) -> float`

Project a point onto a line segment, returning t in [0, 1].

**Returns:** Parameter t clamped to [0, 1].

| Parameter | Type                                | Description                               |
| --------- | ----------------------------------- | ----------------------------------------- |
| `origin`  | `tuple[float, float, float]`        | Start of segment (x, y, z).               |
| `point`   | `tuple[float, float, float]`        | Point to project (x, y, z).               |
| `delta`   | `tuple[float, float, float, float]` | Segment delta from compute_segment_delta. |
| _Returns_ | `float`                             |                                           |

### `slice_scanline_data()`

`slice_scanline_data(data: list[int], t_start: float, t_end: float) -> list[int]`

Slice a scanline power array by parameter range [t_start, t_end).

**Returns:** Sliced power values.

| Parameter | Type        | Description                 |
| --------- | ----------- | --------------------------- |
| `data`    | `list[int]` | Full scanline power values. |
| `t_start` | `float`     | Start parameter in [0, 1].  |
| `t_end`   | `float`     | End parameter in [0, 1].    |
| _Returns_ | `list[int]` |                             |

### `solve_quadratic()`

`solve_quadratic(a: float, b: float, c: float) -> tuple[float | None, float | None]`

Solve quadratic equation a x^2 + b x + c = 0.

**Returns:** (root1, root2), each None if no real root.

| Parameter | Type         | Description            |
| --------- | ------------ | ---------------------- | ------ | --- |
| `a`       | `float`      | Quadratic coefficient. |
| `b`       | `float`      | Linear coefficient.    |
| `c`       | `float`      | Constant term.         |
| _Returns_ | `tuple[float | None, float            | None]` |     |
