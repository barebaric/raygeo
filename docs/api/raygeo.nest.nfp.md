---
title: raygeo.nest.nfp
sidebar_label: raygeo.nest.nfp
sidebar_position: 29
---

No-Fit Polygon calculation for nesting algorithms.

Provides functions for computing No-Fit Polygons (NFP) using Minkowski sums, both for convex and
general polygon pairs.

## Functions

### `nfp_convex_fast()`

```python
nfp_convex_fast(
    static_poly: Sequence[tuple[float, float]],
    orbiting: Sequence[tuple[float, float]],
) -> list[list[tuple[float, float]]]
```

Fast NFP for convex polygon pairs.

| Parameter     | Type                              | Description                        |
| ------------- | --------------------------------- | ---------------------------------- |
| `static_poly` | `Sequence[tuple[float, float]]`   | Static polygon as points.          |
| `orbiting`    | `Sequence[tuple[float, float]]`   | Orbiting polygon as points.        |
| _Returns_     | `list[list[tuple[float, float]]]` | List of NFP polygons.              |
| _Complexity_  |                                   | O(n + m) for convex polygon pairs. |

### `nfp_minkowski()`

```python
nfp_minkowski(
    static_poly: Sequence[tuple[float, float]],
    orbiting: Sequence[tuple[float, float]],
) -> list[list[tuple[float, float]]]
```

General NFP using Minkowski sum with Clipper union.

| Parameter     | Type                              | Description                           |
| ------------- | --------------------------------- | ------------------------------------- |
| `static_poly` | `Sequence[tuple[float, float]]`   | Static polygon as points.             |
| `orbiting`    | `Sequence[tuple[float, float]]`   | Orbiting polygon as points.           |
| _Returns_     | `list[list[tuple[float, float]]]` | List of NFP polygons.                 |
| _Complexity_  |                                   | O(n \* m) where n, m = vertex counts. |

### `no_fit_polygon()`

```python
no_fit_polygon(
    static_poly: Sequence[types.Point],
    orbiting: Sequence[types.Point],
) -> list[types.Polygon]
```

Compute the No-Fit Polygon (NFP) for two polygons.

| Parameter     | Type                    | Description                                             |
| ------------- | ----------------------- | ------------------------------------------------------- |
| `static_poly` | `Sequence[types.Point]` | Static polygon as (x, y) points.                        |
| `orbiting`    | `Sequence[types.Point]` | Orbiting polygon as (x, y) points.                      |
| _Returns_     | `list[types.Polygon]`   | List of NFP polygons.                                   |
| _Complexity_  |                         | O(n \* m) where n, m = vertex counts of input polygons. |

### `normalize_polygon()`

```python
normalize_polygon(
    poly: Sequence[types.Point],
) -> tuple[types.Polygon, float, float]
```

Shift a polygon so its bounding box minimum is at (0, 0).

| Parameter    | Type                                 | Description                               |
| ------------ | ------------------------------------ | ----------------------------------------- |
| `poly`       | `Sequence[types.Point]`              | Input polygon as (x, y) points.           |
| _Returns_    | `tuple[types.Polygon, float, float]` | (normalized_polygon, offset_x, offset_y). |
| _Complexity_ |                                      | O(n) where n = vertex count.              |

### `polygon_to_key()`

```python
polygon_to_key(poly: Sequence[types.Point]) -> list[tuple[int, int]]
```

Convert a polygon to a rounded integer key for caching.

| Parameter    | Type                    | Description                            |
| ------------ | ----------------------- | -------------------------------------- |
| `poly`       | `Sequence[types.Point]` | Input polygon as (x, y) points.        |
| _Returns_    | `list[tuple[int, int]]` | List of rounded (x, y) integer tuples. |
| _Complexity_ |                         | O(n) where n = vertex count.           |
