---
title: raygeo.geo.algo.nest2d.placement
sidebar_label: raygeo.geo.algo.nest2d.placement
sidebar_position: 24
---

Placement search for nesting algorithms.

Provides candidate generation strategies (bottom-left, grid, perimeter), position search, and
high-level nesting orchestration.

## Functions

### `calculate_fitness()`

```python
calculate_fitness(
    polygon_groups: Sequence[Sequence[numpy.ndarray]],
    rotations: Sequence[float],
    sheet_indices: Sequence[int],
    *,
    num_parts: int = 0,
) -> float
```

Calculate fitness score for a set of placements.

Lower is better. Returns infinity if no placements or zero area.

| Parameter        | Type                                | Description                                   |
| ---------------- | ----------------------------------- | --------------------------------------------- |
| `polygon_groups` | `Sequence[Sequence[numpy.ndarray]]` | Polygons for each placement.                  |
| `rotations`      | `Sequence[float]`                   | Rotation angle in degrees for each placement. |
| `sheet_indices`  | `Sequence[int]`                     | 0-based sheet index for each placement.       |
| `num_parts`      | `int`                               | Total number of parts (some may be unplaced). |
| _Returns_        | `float`                             | Fitness score (lower is better).              |
| _Complexity_     |                                     | O(n) where n = number of placements.          |

### `filter_candidates_multi_resolution()`

```python
filter_candidates_multi_resolution(
    candidates: list[tuple[float, float]],
    ifp_bounds: tuple[float, float, float, float],
    min_dist: float,
) -> list[tuple[float, float]]
```

Remove candidates that are too close together.

| Parameter    | Type                                | Description                                    |
| ------------ | ----------------------------------- | ---------------------------------------------- |
| `candidates` | `list[tuple[float, float]]`         | List of (x, y) candidate positions.            |
| `ifp_bounds` | `tuple[float, float, float, float]` | IFP bounding box (min_x, min_y, max_x, max_y). |
| `min_dist`   | `float`                             | Minimum allowed distance between candidates.   |
| _Returns_    | `list[tuple[float, float]]`         | Filtered list of (x, y) positions.             |
| _Complexity_ |                                     | O(n log n) for spatial distance filtering.     |

### `find_valid_position()`

```python
find_valid_position(
    ifp_polygons: Sequence[Sequence[tuple[float, float]]],
    part_polygons: Sequence[Sequence[tuple[float, float]]],
    part_hulls: Sequence[Sequence[tuple[float, float]]],
    placed_polys_list: Sequence[Sequence[Sequence[tuple[float, float]]]],
    placed_hulls_list: Sequence[Sequence[Sequence[tuple[float, float]]]],
    grid: spatial_grid2d.SpatialGrid,
    sheet_world_offset: tuple[float, float],
    spacing: float = 1,
    min_area: float = 1,
    curve_tolerance: float = 0.5,
) -> tuple[float, float] | None
```

Find a valid position: heuristic search first, NFP fallback second.

Supports hull-based collision detection and sheet world offsets.

| Parameter            | Type                                                | Description                                         |
| -------------------- | --------------------------------------------------- | --------------------------------------------------- |
| `ifp_polygons`       | `Sequence[Sequence[tuple[float, float]]]`           | IFP polygons (valid placement region).              |
| `part_polygons`      | `Sequence[Sequence[tuple[float, float]]]`           | Part polygons to place.                             |
| `part_hulls`         | `Sequence[Sequence[tuple[float, float]]]`           | Convex hulls for collision (may be empty).          |
| `placed_polys_list`  | `Sequence[Sequence[Sequence[tuple[float, float]]]]` | Already-placed parts, each a list of polygons.      |
| `placed_hulls_list`  | `Sequence[Sequence[Sequence[tuple[float, float]]]]` | Hulls of already-placed parts, each a list.         |
| `grid`               | `spatial_grid2d.SpatialGrid`                        | SpatialGrid for fast neighbor lookup.               |
| `sheet_world_offset` | `tuple[float, float]`                               | (offset_x, offset_y) for this sheet.                |
| `spacing`            | `float = 1`                                         | Minimum spacing between parts.                      |
| `min_area`           | `float = 1`                                         | Minimum overlap area (clipper coords).              |
| `curve_tolerance`    | `float = 0.5`                                       | Curve tolerance for distance filtering.             |
| _Returns_            | `tuple[float, float] &#124; None`                   | (x, y) position or None.                            |
| _Complexity_         |                                                     | O(n \* m) for candidate search with overlap checks. |

### `find_valid_position_nfp()`

```python
find_valid_position_nfp(
    ifp_polygons: Sequence[Sequence[tuple[float, float]]],
    part_polygons: Sequence[Sequence[tuple[float, float]]],
    part_hulls: Sequence[Sequence[tuple[float, float]]],
    placed_polys_list: Sequence[Sequence[Sequence[tuple[float, float]]]],
    placed_hulls_list: Sequence[Sequence[Sequence[tuple[float, float]]]],
    grid: spatial_grid2d.SpatialGrid,
    sheet_world_offset: tuple[float, float],
    spacing: float = 1,
    min_area: float = 1,
    curve_tolerance: float = 0.5,
) -> tuple[float, float] | None
```

Find a valid position using NFP-based region subtraction.

Computes No-Fit Polygons for nearby placed parts and subtracts them from the IFP to identify viable
placement regions.

| Parameter            | Type                                                | Description                                            |
| -------------------- | --------------------------------------------------- | ------------------------------------------------------ |
| `ifp_polygons`       | `Sequence[Sequence[tuple[float, float]]]`           | IFP polygons (valid placement region).                 |
| `part_polygons`      | `Sequence[Sequence[tuple[float, float]]]`           | Part polygons to place.                                |
| `part_hulls`         | `Sequence[Sequence[tuple[float, float]]]`           | Convex hulls for collision (may be empty).             |
| `placed_polys_list`  | `Sequence[Sequence[Sequence[tuple[float, float]]]]` | Already-placed parts, each a list of polygons.         |
| `placed_hulls_list`  | `Sequence[Sequence[Sequence[tuple[float, float]]]]` | Hulls of already-placed parts, each a list.            |
| `grid`               | `spatial_grid2d.SpatialGrid`                        | SpatialGrid for fast neighbor lookup.                  |
| `sheet_world_offset` | `tuple[float, float]`                               | (offset_x, offset_y) for this sheet.                   |
| `spacing`            | `float = 1`                                         | Minimum spacing between parts.                         |
| `min_area`           | `float = 1`                                         | Minimum overlap area (clipper coords).                 |
| `curve_tolerance`    | `float = 0.5`                                       | Curve tolerance for distance filtering.                |
| _Returns_            | `tuple[float, float] &#124; None`                   | (x, y) position or None.                               |
| _Complexity_         |                                                     | O(n \* m) for NFP construction per nearby placed part. |

### `find_valid_position_scored()`

```python
find_valid_position_scored(
    ifp_polygons: Sequence[Sequence[tuple[float, float]]],
    part_polygons: Sequence[Sequence[tuple[float, float]]],
    part_hulls: Sequence[Sequence[tuple[float, float]]],
    placed_polys_list: Sequence[Sequence[Sequence[tuple[float, float]]]],
    placed_hulls_list: Sequence[Sequence[Sequence[tuple[float, float]]]],
    grid: spatial_grid2d.SpatialGrid,
    sheet_world_offset: tuple[float, float],
    spacing: float = 1,
    min_area: float = 1,
    curve_tolerance: float = 0.5,
) -> tuple[float, float] | None
```

Find a valid position using heuristic candidate search.

Uses IFP vertices, bottom-left sweep, grid, placed polygon vertices, and perimeter candidates. Falls
back to NFP-region candidates. Scores candidates and picks the best valid one.

| Parameter            | Type                                                | Description                                    |
| -------------------- | --------------------------------------------------- | ---------------------------------------------- |
| `ifp_polygons`       | `Sequence[Sequence[tuple[float, float]]]`           | IFP polygons (valid placement region).         |
| `part_polygons`      | `Sequence[Sequence[tuple[float, float]]]`           | Part polygons to place.                        |
| `part_hulls`         | `Sequence[Sequence[tuple[float, float]]]`           | Convex hulls for collision (may be empty).     |
| `placed_polys_list`  | `Sequence[Sequence[Sequence[tuple[float, float]]]]` | Already-placed parts, each a list of polygons. |
| `placed_hulls_list`  | `Sequence[Sequence[Sequence[tuple[float, float]]]]` | Hulls of already-placed parts, each a list.    |
| `grid`               | `spatial_grid2d.SpatialGrid`                        | SpatialGrid for fast neighbor lookup.          |
| `sheet_world_offset` | `tuple[float, float]`                               | (offset_x, offset_y) for this sheet.           |
| `spacing`            | `float = 1`                                         | Minimum spacing between parts.                 |
| `min_area`           | `float = 1`                                         | Minimum overlap area (clipper coords).         |
| `curve_tolerance`    | `float = 0.5`                                       | Curve tolerance for distance filtering.        |
| _Returns_            | `tuple[float, float] &#124; None`                   | (x, y) position or None.                       |
| _Complexity_         |                                                     | O(n \* m) for scored candidate search.         |

### `generate_bottom_left_candidates()`

```python
generate_bottom_left_candidates(
    ifp_bounds: tuple[float, float, float, float],
    part_bounds: tuple[float, float, float, float],
    spacing: float,
) -> list[tuple[float, float]]
```

Generate candidate positions scanning from bottom-left.

| Parameter     | Type                                | Description                                     |
| ------------- | ----------------------------------- | ----------------------------------------------- |
| `ifp_bounds`  | `tuple[float, float, float, float]` | IFP bounding box (min_x, min_y, max_x, max_y).  |
| `part_bounds` | `tuple[float, float, float, float]` | Part bounding box (min_x, min_y, max_x, max_y). |
| `spacing`     | `float`                             | Minimum spacing between parts.                  |
| _Returns_     | `list[tuple[float, float]]`         | List of (x, y) candidate positions.             |
| _Complexity_  |                                     | O(n) where n = number of grid cells scanned.    |

### `generate_grid_candidates()`

```python
generate_grid_candidates(
    ifp_bounds: tuple[float, float, float, float],
    part_bounds: tuple[float, float, float, float],
    spacing: float,
) -> list[tuple[float, float]]
```

Generate candidate positions in a grid pattern.

| Parameter     | Type                                | Description                                     |
| ------------- | ----------------------------------- | ----------------------------------------------- |
| `ifp_bounds`  | `tuple[float, float, float, float]` | IFP bounding box (min_x, min_y, max_x, max_y).  |
| `part_bounds` | `tuple[float, float, float, float]` | Part bounding box (min_x, min_y, max_x, max_y). |
| `spacing`     | `float`                             | Grid spacing.                                   |
| _Returns_     | `list[tuple[float, float]]`         | List of (x, y) candidate positions.             |
| _Complexity_  |                                     | O(n) where n = number of grid cells.            |

### `generate_perimeter_candidates()`

```python
generate_perimeter_candidates(
    placed_groups: Sequence[Sequence[list[tuple[float, float]]]],
    part_bounds: tuple[float, float, float, float],
    spacing: float,
) -> list[tuple[float, float]]
```

Generate edge-aligned candidates around placed parts.

For each placed part (group of polygons), produces 8 positions: right-bottom, right-top,
left-bottom, left-top, above-left, above-right, below-left, below-right.

| Parameter       | Type                                            | Description                                     |
| --------------- | ----------------------------------------------- | ----------------------------------------------- |
| `placed_groups` | `Sequence[Sequence[list[tuple[float, float]]]]` | List of placed parts, each a list of polygons.  |
| `part_bounds`   | `tuple[float, float, float, float]`             | Part bounding box (min_x, min_y, max_x, max_y). |
| `spacing`       | `float`                                         | Minimum spacing between parts.                  |
| _Returns_       | `list[tuple[float, float]]`                     | List of (x, y) candidate positions.             |
| _Complexity_    |                                                 | O(n) where n = number of placed parts.          |

### `place_parts()`

```python
place_parts(
    part_polys: Sequence[Sequence[Sequence[tuple[float, float]]]],
    part_hulls: Sequence[Sequence[Sequence[tuple[float, float]]]],
    sheet_polys: Sequence[Sequence[tuple[float, float]]],
    sheet_offsets: Sequence[tuple[float, float]],
    rotations: Sequence[float],
    flips_h: Sequence[bool],
    flips_v: Sequence[bool],
    spacing: float = 1,
    min_area: float = 1,
    curve_tolerance: float = 0.5,
) -> list[dict]
```

Place as many parts as possible onto sheets.

Supports combined IFP for multi-polygon parts, hull-based collision detection, world-space offsets
per sheet, gravity post-processing, and fitness calculation.

Parts are sorted by area (largest first). For each part, the best sheet and position are selected.
After all parts are placed, gravity is applied per sheet.

| Parameter         | Type                                                | Description                                                                                     |
| ----------------- | --------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `part_polys`      | `Sequence[Sequence[Sequence[tuple[float, float]]]]` | List of parts, each a list of polygon point lists.                                              |
| `part_hulls`      | `Sequence[Sequence[Sequence[tuple[float, float]]]]` | List of hull groups per part (may be empty).                                                    |
| `sheet_polys`     | `Sequence[Sequence[tuple[float, float]]]`           | List of sheet polygons.                                                                         |
| `sheet_offsets`   | `Sequence[tuple[float, float]]`                     | World-space offset (x, y) for each sheet.                                                       |
| `rotations`       | `Sequence[float]`                                   | Rotation angle (degrees) for each part.                                                         |
| `flips_h`         | `Sequence[bool]`                                    | Horizontal flip flag per part.                                                                  |
| `flips_v`         | `Sequence[bool]`                                    | Vertical flip flag per part.                                                                    |
| `spacing`         | `float = 1`                                         | Minimum spacing between parts.                                                                  |
| `min_area`        | `float = 1`                                         | Minimum overlap area (clipper coords).                                                          |
| `curve_tolerance` | `float = 0.5`                                       | Curve tolerance for distance filtering.                                                         |
| _Returns_         | `list[dict]`                                        | List of dicts, one per sheet, with keys: placements, sheet_index, unused_part_indices, fitness. |
| _Complexity_      |                                                     | O(p \* s \* n \* m) where p = parts, s = sheets.                                                |
