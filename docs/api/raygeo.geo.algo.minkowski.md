---
title: raygeo.geo.algo.minkowski
sidebar_label: raygeo.geo.algo.minkowski
sidebar_position: 10
---

Minkowski sum operations for 2D polygon toolpath generation.

Provides convolution of point sequences and segments, Minkowski sums for convex polygons, and no-fit
polygon / inner fit polygon calculations used in nesting and packing algorithms.

## Functions

### `calculate_input_scale()`

`calculate_input_scale(polygons: collections.abc.Sequence[collections.abc.Sequence[types.Point]], max_int: int = 2147483647) -> float`

Calculate the optimal input scale for clipper operations.

**Returns:** Optimal scale factor.

| Parameter    | Type                                                              | Description                        |
| ------------ | ----------------------------------------------------------------- | ---------------------------------- |
| `polygons`   | `collections.abc.Sequence[collections.abc.Sequence[types.Point]]` | List of polygons to scale.         |
| `max_int`    | `int = 2147483647`                                                | Maximum integer value for Clipper. |
| _Returns_    | `float`                                                           |                                    |
| _Complexity_ |                                                                   | O(n) time, O(1) space              |

### `convolve_point_sequences()`

`convolve_point_sequences(seq_a: collections.abc.Sequence[tuple[float, float]], seq_b: collections.abc.Sequence[tuple[float, float]]) -> list[list[tuple[float, float]]]`

Convolve two sequences of points.

**Returns:** Convolved point sequences.

| Parameter    | Type                                            | Description                   |
| ------------ | ----------------------------------------------- | ----------------------------- |
| `seq_a`      | `collections.abc.Sequence[tuple[float, float]]` | First sequence of points.     |
| `seq_b`      | `collections.abc.Sequence[tuple[float, float]]` | Second sequence of points.    |
| _Returns_    | `list[list[tuple[float, float]]]`               |                               |
| _Complexity_ |                                                 | O(n _ m) time, O(n _ m) space |

### `convolve_two_segments()`

`convolve_two_segments(a1: tuple[float, float], a2: tuple[float, float], b1: tuple[float, float], b2: tuple[float, float]) -> list[tuple[float, float]]`

Convolve two line segments.

**Returns:** Convolved point sequence.

| Parameter    | Type                        | Description               |
| ------------ | --------------------------- | ------------------------- |
| `a1`         | `tuple[float, float]`       | Start point of segment A. |
| `a2`         | `tuple[float, float]`       | End point of segment A.   |
| `b1`         | `tuple[float, float]`       | Start point of segment B. |
| `b2`         | `tuple[float, float]`       | End point of segment B.   |
| _Returns_    | `list[tuple[float, float]]` |                           |
| _Complexity_ |                             | O(1) time, O(1) space     |

### `get_inner_fit_polygon()`

`get_inner_fit_polygon(outer: collections.abc.Sequence[types.Point], inner: collections.abc.Sequence[types.Point]) -> list[types.Polygon]`

Compute the inner fit polygon (no-fit polygon for nesting).

**Returns:** Inner fit polygon.

| Parameter    | Type                                    | Description                     |
| ------------ | --------------------------------------- | ------------------------------- |
| `outer`      | `collections.abc.Sequence[types.Point]` | Outer polygon as (x, y) points. |
| `inner`      | `collections.abc.Sequence[types.Point]` | Inner polygon as (x, y) points. |
| _Returns_    | `list[types.Polygon]`                   |                                 |
| _Complexity_ |                                         | O(n \* m) time, O(n + m) space  |

### `get_no_fit_polygon()`

`get_no_fit_polygon(subject: collections.abc.Sequence[types.Point], tool: collections.abc.Sequence[types.Point]) -> list[types.Polygon]`

Compute the no-fit polygon for two 2D polygons.

**Returns:** No-fit polygon.

| Parameter    | Type                                    | Description                       |
| ------------ | --------------------------------------- | --------------------------------- |
| `subject`    | `collections.abc.Sequence[types.Point]` | Subject polygon as (x, y) points. |
| `tool`       | `collections.abc.Sequence[types.Point]` | Tool polygon as (x, y) points.    |
| _Returns_    | `list[types.Polygon]`                   |                                   |
| _Complexity_ |                                         | O(n \* m) time, O(n + m) space    |

### `get_polygon_minkowski_sum_convex()`

`get_polygon_minkowski_sum_convex(poly_a: collections.abc.Sequence[tuple[float, float]], poly_b: collections.abc.Sequence[tuple[float, float]]) -> list[list[tuple[float, float]]]`

Compute the Minkowski sum of two convex polygons.

**Returns:** Minkowski sum as list of polygons.

| Parameter    | Type                                            | Description                      |
| ------------ | ----------------------------------------------- | -------------------------------- |
| `poly_a`     | `collections.abc.Sequence[tuple[float, float]]` | First convex polygon as points.  |
| `poly_b`     | `collections.abc.Sequence[tuple[float, float]]` | Second convex polygon as points. |
| _Returns_    | `list[list[tuple[float, float]]]`               |                                  |
| _Complexity_ |                                                 | O(n + m) time, O(n + m) space    |

![Minkowski sum of two convex polygons](images/minkowski-sum.png)

_Minkowski sum of two convex polygons_
