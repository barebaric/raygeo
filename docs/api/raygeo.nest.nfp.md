---
title: raygeo.nest.nfp
sidebar_label: raygeo.nest.nfp
sidebar_position: 28
---

No-Fit Polygon calculation for nesting algorithms.

Provides functions for computing No-Fit Polygons (NFP) using Minkowski sums, both for convex and
general polygon pairs.

## Functions

### `nfp_convex_fast()`

`nfp_convex_fast(static_poly: collections.abc.Sequence[tuple[float, float]], orbiting: collections.abc.Sequence[tuple[float, float]]) -> list[list[tuple[float, float]]]`

Fast NFP for convex polygon pairs.

**Returns:** List of NFP polygons.

| Parameter     | Type                                            | Description                 |
| ------------- | ----------------------------------------------- | --------------------------- |
| `static_poly` | `collections.abc.Sequence[tuple[float, float]]` | Static polygon as points.   |
| `orbiting`    | `collections.abc.Sequence[tuple[float, float]]` | Orbiting polygon as points. |
| _Returns_     | `list[list[tuple[float, float]]]`               |                             |

### `nfp_minkowski()`

`nfp_minkowski(static_poly: collections.abc.Sequence[tuple[float, float]], orbiting: collections.abc.Sequence[tuple[float, float]]) -> list[list[tuple[float, float]]]`

General NFP using Minkowski sum with Clipper union.

**Returns:** List of NFP polygons.

| Parameter     | Type                                            | Description                 |
| ------------- | ----------------------------------------------- | --------------------------- |
| `static_poly` | `collections.abc.Sequence[tuple[float, float]]` | Static polygon as points.   |
| `orbiting`    | `collections.abc.Sequence[tuple[float, float]]` | Orbiting polygon as points. |
| _Returns_     | `list[list[tuple[float, float]]]`               |                             |

### `no_fit_polygon()`

`no_fit_polygon(static_poly: collections.abc.Sequence[types.Point], orbiting: collections.abc.Sequence[types.Point]) -> list[types.Polygon]`

Compute the No-Fit Polygon (NFP) for two polygons.

**Returns:** List of NFP polygons.

| Parameter     | Type                                    | Description                        |
| ------------- | --------------------------------------- | ---------------------------------- |
| `static_poly` | `collections.abc.Sequence[types.Point]` | Static polygon as (x, y) points.   |
| `orbiting`    | `collections.abc.Sequence[types.Point]` | Orbiting polygon as (x, y) points. |
| _Returns_     | `list[types.Polygon]`                   |                                    |

### `normalize_polygon()`

`normalize_polygon(poly: collections.abc.Sequence[types.Point]) -> tuple[types.Polygon, float, float]`

Shift a polygon so its bounding box minimum is at (0, 0).

**Returns:** (normalized_polygon, offset_x, offset_y).

| Parameter | Type                                    | Description                     |
| --------- | --------------------------------------- | ------------------------------- |
| `poly`    | `collections.abc.Sequence[types.Point]` | Input polygon as (x, y) points. |
| _Returns_ | `tuple[types.Polygon, float, float]`    |                                 |

### `polygon_to_key()`

`polygon_to_key(poly: collections.abc.Sequence[types.Point]) -> list[tuple[int, int]]`

Convert a polygon to a rounded integer key for caching.

**Returns:** List of rounded (x, y) integer tuples.

| Parameter | Type                                    | Description                     |
| --------- | --------------------------------------- | ------------------------------- |
| `poly`    | `collections.abc.Sequence[types.Point]` | Input polygon as (x, y) points. |
| _Returns_ | `list[tuple[int, int]]`                 |                                 |
