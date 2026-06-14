---
title: raygeo.nest.collision
sidebar_label: raygeo.nest.collision
sidebar_position: 24
---

Collision detection for nesting algorithms.

Provides overlap checks with bounding-box, convex hull, and detailed polygon intersection, including
hierarchical variants for performance.

## Functions

### `any_overlap()`

`any_overlap(candidate: types.Polygon, placed: collections.abc.Sequence[types.Polygon], min_area: float = 1) -> bool`

Check if a candidate polygon overlaps any placed polygon.

**Returns:** True if any overlap detected.

| Parameter    | Type                                      | Description                                                          |
| ------------ | ----------------------------------------- | -------------------------------------------------------------------- |
| `candidate`  | `types.Polygon`                           | Candidate polygon.                                                   |
| `placed`     | `collections.abc.Sequence[types.Polygon]` | List of already-placed polygons.                                     |
| `min_area`   | `float = 1`                               | Minimum overlap area to consider (in clipper coords).                |
| _Returns_    | `bool`                                    |                                                                      |
| _Complexity_ |                                           | O(n \* m) where n = candidate vertices, m = placed polygon vertices. |

### `any_overlap_hierarchical()`

`any_overlap_hierarchical(candidate_polys: collections.abc.Sequence[numpy.ndarray], candidate_hulls: collections.abc.Sequence[numpy.ndarray], placed_polys_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]], placed_hulls_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]], min_area: float = 1) -> bool`

Hierarchical overlap: bbox -> hull -> detailed polygon.

**Returns:** True if any overlap detected.

| Parameter             | Type                                                                | Description                                       |
| --------------------- | ------------------------------------------------------------------- | ------------------------------------------------- |
| `candidate_polys`     | `collections.abc.Sequence[numpy.ndarray]`                           | Candidate polygons to check.                      |
| `candidate_hulls`     | `collections.abc.Sequence[numpy.ndarray]`                           | Convex hulls of candidate polygons.               |
| `placed_polys_groups` | `collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]]` | Groups of already-placed polygons.                |
| `placed_hulls_groups` | `collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]]` | Convex hulls of placed groups.                    |
| `min_area`            | `float = 1`                                                         | Minimum overlap area (clipper coords).            |
| _Returns_             | `bool`                                                              |                                                   |
| _Complexity_          |                                                                     | O(n \* m) with bbox/hull early-exit acceleration. |

### `any_overlap_hierarchical_grid()`

`any_overlap_hierarchical_grid(candidate_polys: collections.abc.Sequence[numpy.ndarray], candidate_hulls: collections.abc.Sequence[numpy.ndarray], placed_polys_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]], placed_hulls_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]], spatial_grid: spatial_grid.SpatialGrid, candidate_bbox: tuple[float, float, float, float], min_area: float = 1) -> bool`

| Parameter             | Type                                                                | Description                                       |
| --------------------- | ------------------------------------------------------------------- | ------------------------------------------------- |
| `candidate_polys`     | `collections.abc.Sequence[numpy.ndarray]`                           |                                                   |
| `candidate_hulls`     | `collections.abc.Sequence[numpy.ndarray]`                           |                                                   |
| `placed_polys_groups` | `collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]]` |                                                   |
| `placed_hulls_groups` | `collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]]` |                                                   |
| `spatial_grid`        | `spatial_grid.SpatialGrid`                                          | SpatialGrid for fast neighbor lookup.             |
| `candidate_bbox`      | `tuple[float, float, float, float]`                                 |                                                   |
| `min_area`            | `float = 1`                                                         |                                                   |
| _Returns_             | `bool`                                                              |                                                   |
| _Complexity_          |                                                                     | O(n \* m / k) where k = grid cell density factor. |

### `is_contained()`

`is_contained(inner: collections.abc.Sequence[types.Polygon], outer: types.Polygon) -> bool`

Check if inner polygons are fully contained within outer polygon.

**Returns:** True if all inner polygons are inside outer.

| Parameter    | Type                                      | Description                                                     |
| ------------ | ----------------------------------------- | --------------------------------------------------------------- |
| `inner`      | `collections.abc.Sequence[types.Polygon]` | List of polygons to check.                                      |
| `outer`      | `types.Polygon`                           | Outer polygon.                                                  |
| _Returns_    | `bool`                                    |                                                                 |
| _Complexity_ |                                           | O(n \* m) where n = inner polygons, m = outer polygon vertices. |
