---
title: raygeo.geo.algo.interp
sidebar_label: raygeo.geo.algo.interp
sidebar_position: 14
---

Segment interpolation utilities for parameter-based point projection, clipping, and scanline data
slicing along 3D line segments.

## Functions

### `barycentric_interpolate()`

```python
barycentric_interpolate(
    p: tuple[float, float],
    va: tuple[float, float],
    vb: tuple[float, float],
    vc: tuple[float, float],
    ua: float,
    ub: float,
    uc: float,
) -> float
```

Interpolate a scalar field at a point inside a triangle.

Given triangle vertices (va, vb, vc) with scalar values (ua, ub, uc), returns the linearly
interpolated value at point p using barycentric coordinates.

| Parameter    | Type                  | Description                                                                                         |
| ------------ | --------------------- | --------------------------------------------------------------------------------------------------- |
| `p`          | `tuple[float, float]` | Query point (x, y).                                                                                 |
| `va`         | `tuple[float, float]` | First triangle vertex (x, y).                                                                       |
| `vb`         | `tuple[float, float]` | Second triangle vertex (x, y).                                                                      |
| `vc`         | `tuple[float, float]` | Third triangle vertex (x, y).                                                                       |
| `ua`         | `float`               | Scalar value at vertex a.                                                                           |
| `ub`         | `float`               | Scalar value at vertex b.                                                                           |
| `uc`         | `float`               | Scalar value at vertex c.                                                                           |
| _Returns_    | `float`               | Interpolated scalar value. Outside the triangle, the barycentric coordinates are clamped to [0, 1]. |
| _Complexity_ |                       | O(1) time, O(1) space                                                                               |

### `barycentric_weights()`

```python
barycentric_weights(
    p: tuple[float, float],
    va: tuple[float, float],
    vb: tuple[float, float],
    vc: tuple[float, float],
) -> tuple[float, float, float]
```

Compute raw barycentric coordinates for a point in a triangle.

Returns (r, s, t) where r is the weight for va, s for vb, t for vc. Weights are unclamped — the
point is strictly inside (or on the boundary of) the triangle iff all three are in [0, 1].

| Parameter    | Type                         | Description                                     |
| ------------ | ---------------------------- | ----------------------------------------------- |
| `p`          | `tuple[float, float]`        | Query point (x, y).                             |
| `va`         | `tuple[float, float]`        | First triangle vertex (x, y).                   |
| `vb`         | `tuple[float, float]`        | Second triangle vertex (x, y).                  |
| `vc`         | `tuple[float, float]`        | Third triangle vertex (x, y).                   |
| _Returns_    | `tuple[float, float, float]` | Tuple (r, s, t) of raw barycentric coordinates. |
| _Complexity_ |                              | O(1) time, O(1) space                           |

### `compute_segment_delta()`

```python
compute_segment_delta(
    start: tuple[float, float, float],
    end: tuple[float, float, float],
) -> tuple[float, float, float, float]
```

Compute delta vector and squared length between two 3D points.

| Parameter    | Type                                | Description               |
| ------------ | ----------------------------------- | ------------------------- |
| `start`      | `tuple[float, float, float]`        | Starting point (x, y, z). |
| `end`        | `tuple[float, float, float]`        | Ending point (x, y, z).   |
| _Returns_    | `tuple[float, float, float, float]` | (dx, dy, dz, len_sq).     |
| _Complexity_ |                                     | O(1) time, O(1) space     |

### `compute_t_range()`

```python
compute_t_range(
    origin: tuple[float, float, float],
    new_start: tuple[float, float, float],
    new_end: tuple[float, float, float],
    delta: tuple[float, float, float, float],
) -> tuple[float, float]
```

Compute parameter range (t_start, t_end) for a clipped sub-segment.

| Parameter    | Type                                | Description                               |
| ------------ | ----------------------------------- | ----------------------------------------- |
| `origin`     | `tuple[float, float, float]`        | Start of original segment (x, y, z).      |
| `new_start`  | `tuple[float, float, float]`        | Start of clipped sub-segment (x, y, z).   |
| `new_end`    | `tuple[float, float, float]`        | End of clipped sub-segment (x, y, z).     |
| `delta`      | `tuple[float, float, float, float]` | Segment delta from compute_segment_delta. |
| _Returns_    | `tuple[float, float]`               | (t_start, t_end) in [0, 1].               |
| _Complexity_ |                                     | O(1) time, O(1) space                     |

### `project_t_along_segment()`

```python
project_t_along_segment(
    origin: tuple[float, float, float],
    point: tuple[float, float, float],
    delta: tuple[float, float, float, float],
) -> float
```

Project a point onto a line segment, returning t in [0, 1].

| Parameter    | Type                                | Description                               |
| ------------ | ----------------------------------- | ----------------------------------------- |
| `origin`     | `tuple[float, float, float]`        | Start of segment (x, y, z).               |
| `point`      | `tuple[float, float, float]`        | Point to project (x, y, z).               |
| `delta`      | `tuple[float, float, float, float]` | Segment delta from compute_segment_delta. |
| _Returns_    | `float`                             | Parameter t clamped to [0, 1].            |
| _Complexity_ |                                     | O(1) time, O(1) space                     |

### `slice_scanline_data()`

```python
slice_scanline_data(data: list[int], t_start: float, t_end: float) -> list[int]
```

Slice a scanline power array by parameter range [t_start, t_end).

| Parameter    | Type        | Description                                                   |
| ------------ | ----------- | ------------------------------------------------------------- |
| `data`       | `list[int]` | Full scanline power values.                                   |
| `t_start`    | `float`     | Start parameter in [0, 1].                                    |
| `t_end`      | `float`     | End parameter in [0, 1].                                      |
| _Returns_    | `list[int]` | Sliced power values.                                          |
| _Complexity_ |             | O(n) time, O(n) space where n is the length of the data slice |

### `solve_quadratic()`

```python
solve_quadratic(
    a: float,
    b: float,
    c: float,
) -> tuple[float | None, float | None]
```

Solve quadratic equation a x^2 + b x + c = 0.

| Parameter    | Type                                          | Description                                |
| ------------ | --------------------------------------------- | ------------------------------------------ |
| `a`          | `float`                                       | Quadratic coefficient.                     |
| `b`          | `float`                                       | Linear coefficient.                        |
| `c`          | `float`                                       | Constant term.                             |
| _Returns_    | `tuple[float &#124; None, float &#124; None]` | (root1, root2), each None if no real root. |
| _Complexity_ |                                               | O(1) time, O(1) space                      |
