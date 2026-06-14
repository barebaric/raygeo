---
title: raygeo.nest.placement
sidebar_label: raygeo.nest.placement
sidebar_position: 29
---

Placement search for nesting algorithms.

Provides candidate generation strategies (bottom-left, grid, perimeter), position search, and
high-level nesting orchestration.

## Functions

### `calculate_fitness()`

`calculate_fitness(polygon_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]], rotations: collections.abc.Sequence[float], sheet_indices: collections.abc.Sequence[int], *, num_parts: int = 0) -> float`

Calculate fitness score for a set of placements.

Lower is better. Returns infinity if no placements or zero area.

**Returns:** Fitness score (lower is better).

| Parameter        | Type                                                                | Description                                   |
| ---------------- | ------------------------------------------------------------------- | --------------------------------------------- |
| `polygon_groups` | `collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]]` | Polygons for each placement.                  |
| `rotations`      | `collections.abc.Sequence[float]`                                   | Rotation angle in degrees for each placement. |
| `sheet_indices`  | `collections.abc.Sequence[int]`                                     | 0-based sheet index for each placement.       |
| `num_parts`      | `int`                                                               | Total number of parts (some may be unplaced). |
| _Returns_        | `float`                                                             |                                               |

### `filter_candidates_multi_resolution()`

`filter_candidates_multi_resolution(candidates: list[tuple[float, float]], ifp_bounds: tuple[float, float, float, float], min_dist: float) -> list[tuple[float, float]]`

Remove candidates that are too close together.

**Returns:** Filtered list of (x, y) positions.

| Parameter    | Type                                | Description                                    |
| ------------ | ----------------------------------- | ---------------------------------------------- |
| `candidates` | `list[tuple[float, float]]`         | List of (x, y) candidate positions.            |
| `ifp_bounds` | `tuple[float, float, float, float]` | IFP bounding box (min_x, min_y, max_x, max_y). |
| `min_dist`   | `float`                             | Minimum allowed distance between candidates.   |
| _Returns_    | `list[tuple[float, float]]`         |                                                |

### `find_valid_position()`

`find_valid_position(ifp_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], part_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], part_hulls: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], placed_polys_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]], placed_hulls_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]], grid: spatial_grid.SpatialGrid, sheet_world_offset: tuple[float, float], spacing: float = 1, min_area: float = 1, curve_tolerance: float = 0.5) -> tuple[float, float] | None`

Find a valid position: heuristic search first, NFP fallback second.

Supports hull-based collision detection and sheet world offsets.

**Returns:** (x, y) position or None.

| Parameter            | Type                                                                                                | Description                                    |
| -------------------- | --------------------------------------------------------------------------------------------------- | ---------------------------------------------- | --- |
| `ifp_polygons`       | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | IFP polygons (valid placement region).         |
| `part_polygons`      | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | Part polygons to place.                        |
| `part_hulls`         | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | Convex hulls for collision (may be empty).     |
| `placed_polys_list`  | `collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]]` | Already-placed parts, each a list of polygons. |
| `placed_hulls_list`  | `collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]]` | Hulls of already-placed parts, each a list.    |
| `grid`               | `spatial_grid.SpatialGrid`                                                                          | SpatialGrid for fast neighbor lookup.          |
| `sheet_world_offset` | `tuple[float, float]`                                                                               | (offset_x, offset_y) for this sheet.           |
| `spacing`            | `float = 1`                                                                                         | Minimum spacing between parts.                 |
| `min_area`           | `float = 1`                                                                                         | Minimum overlap area (clipper coords).         |
| `curve_tolerance`    | `float = 0.5`                                                                                       | Curve tolerance for distance filtering.        |
| _Returns_            | `tuple[float, float]                                                                                | None`                                          |     |

### `find_valid_position_nfp()`

`find_valid_position_nfp(ifp_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], part_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], part_hulls: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], placed_polys_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]], placed_hulls_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]], grid: spatial_grid.SpatialGrid, sheet_world_offset: tuple[float, float], spacing: float = 1, min_area: float = 1, curve_tolerance: float = 0.5) -> tuple[float, float] | None`

Find a valid position using NFP-based region subtraction.

Computes No-Fit Polygons for nearby placed parts and subtracts them from the IFP to identify viable
placement regions.

**Returns:** (x, y) position or None.

| Parameter            | Type                                                                                                | Description                                    |
| -------------------- | --------------------------------------------------------------------------------------------------- | ---------------------------------------------- | --- |
| `ifp_polygons`       | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | IFP polygons (valid placement region).         |
| `part_polygons`      | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | Part polygons to place.                        |
| `part_hulls`         | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | Convex hulls for collision (may be empty).     |
| `placed_polys_list`  | `collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]]` | Already-placed parts, each a list of polygons. |
| `placed_hulls_list`  | `collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]]` | Hulls of already-placed parts, each a list.    |
| `grid`               | `spatial_grid.SpatialGrid`                                                                          | SpatialGrid for fast neighbor lookup.          |
| `sheet_world_offset` | `tuple[float, float]`                                                                               | (offset_x, offset_y) for this sheet.           |
| `spacing`            | `float = 1`                                                                                         | Minimum spacing between parts.                 |
| `min_area`           | `float = 1`                                                                                         | Minimum overlap area (clipper coords).         |
| `curve_tolerance`    | `float = 0.5`                                                                                       | Curve tolerance for distance filtering.        |
| _Returns_            | `tuple[float, float]                                                                                | None`                                          |     |

### `find_valid_position_scored()`

`find_valid_position_scored(ifp_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], part_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], part_hulls: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], placed_polys_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]], placed_hulls_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]], grid: spatial_grid.SpatialGrid, sheet_world_offset: tuple[float, float], spacing: float = 1, min_area: float = 1, curve_tolerance: float = 0.5) -> tuple[float, float] | None`

Find a valid position using heuristic candidate search.

Uses IFP vertices, bottom-left sweep, grid, placed polygon vertices, and perimeter candidates. Falls
back to NFP-region candidates. Scores candidates and picks the best valid one.

**Returns:** (x, y) position or None.

| Parameter            | Type                                                                                                | Description                                    |
| -------------------- | --------------------------------------------------------------------------------------------------- | ---------------------------------------------- | --- |
| `ifp_polygons`       | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | IFP polygons (valid placement region).         |
| `part_polygons`      | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | Part polygons to place.                        |
| `part_hulls`         | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | Convex hulls for collision (may be empty).     |
| `placed_polys_list`  | `collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]]` | Already-placed parts, each a list of polygons. |
| `placed_hulls_list`  | `collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]]` | Hulls of already-placed parts, each a list.    |
| `grid`               | `spatial_grid.SpatialGrid`                                                                          | SpatialGrid for fast neighbor lookup.          |
| `sheet_world_offset` | `tuple[float, float]`                                                                               | (offset_x, offset_y) for this sheet.           |
| `spacing`            | `float = 1`                                                                                         | Minimum spacing between parts.                 |
| `min_area`           | `float = 1`                                                                                         | Minimum overlap area (clipper coords).         |
| `curve_tolerance`    | `float = 0.5`                                                                                       | Curve tolerance for distance filtering.        |
| _Returns_            | `tuple[float, float]                                                                                | None`                                          |     |

### `generate_bottom_left_candidates()`

`generate_bottom_left_candidates(ifp_bounds: tuple[float, float, float, float], part_bounds: tuple[float, float, float, float], spacing: float) -> list[tuple[float, float]]`

Generate candidate positions scanning from bottom-left.

**Returns:** List of (x, y) candidate positions.

| Parameter     | Type                                | Description                                     |
| ------------- | ----------------------------------- | ----------------------------------------------- |
| `ifp_bounds`  | `tuple[float, float, float, float]` | IFP bounding box (min_x, min_y, max_x, max_y).  |
| `part_bounds` | `tuple[float, float, float, float]` | Part bounding box (min_x, min_y, max_x, max_y). |
| `spacing`     | `float`                             | Minimum spacing between parts.                  |
| _Returns_     | `list[tuple[float, float]]`         |                                                 |

### `generate_grid_candidates()`

`generate_grid_candidates(ifp_bounds: tuple[float, float, float, float], part_bounds: tuple[float, float, float, float], spacing: float) -> list[tuple[float, float]]`

Generate candidate positions in a grid pattern.

**Returns:** List of (x, y) candidate positions.

| Parameter     | Type                                | Description                                     |
| ------------- | ----------------------------------- | ----------------------------------------------- |
| `ifp_bounds`  | `tuple[float, float, float, float]` | IFP bounding box (min_x, min_y, max_x, max_y).  |
| `part_bounds` | `tuple[float, float, float, float]` | Part bounding box (min_x, min_y, max_x, max_y). |
| `spacing`     | `float`                             | Grid spacing.                                   |
| _Returns_     | `list[tuple[float, float]]`         |                                                 |

### `generate_perimeter_candidates()`

`generate_perimeter_candidates(placed_groups: collections.abc.Sequence[collections.abc.Sequence[list[tuple[float, float]]]], part_bounds: tuple[float, float, float, float], spacing: float) -> list[tuple[float, float]]`

Generate edge-aligned candidates around placed parts.

For each placed part (group of polygons), produces 8 positions: right-bottom, right-top,
left-bottom, left-top, above-left, above-right, below-left, below-right.

**Returns:** List of (x, y) candidate positions.

| Parameter       | Type                                                                            | Description                                     |
| --------------- | ------------------------------------------------------------------------------- | ----------------------------------------------- |
| `placed_groups` | `collections.abc.Sequence[collections.abc.Sequence[list[tuple[float, float]]]]` | List of placed parts, each a list of polygons.  |
| `part_bounds`   | `tuple[float, float, float, float]`                                             | Part bounding box (min_x, min_y, max_x, max_y). |
| `spacing`       | `float`                                                                         | Minimum spacing between parts.                  |
| _Returns_       | `list[tuple[float, float]]`                                                     |                                                 |

### `place_parts()`

`place_parts(part_polys: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]], part_hulls: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]], sheet_polys: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]], sheet_offsets: collections.abc.Sequence[tuple[float, float]], rotations: collections.abc.Sequence[float], flips_h: collections.abc.Sequence[bool], flips_v: collections.abc.Sequence[bool], spacing: float = 1, min_area: float = 1, curve_tolerance: float = 0.5) -> list[dict]`

Place as many parts as possible onto sheets.

Supports combined IFP for multi-polygon parts, hull-based collision detection, world-space offsets
per sheet, gravity post-processing, and fitness calculation.

Parts are sorted by area (largest first). For each part, the best sheet and position are selected.
After all parts are placed, gravity is applied per sheet.

**Returns:** List of dicts, one per sheet, with keys: placements, sheet_index, unused_part_indices,
fitness.

| Parameter         | Type                                                                                                | Description                                        |
| ----------------- | --------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `part_polys`      | `collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]]` | List of parts, each a list of polygon point lists. |
| `part_hulls`      | `collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]]` | List of hull groups per part (may be empty).       |
| `sheet_polys`     | `collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]`                           | List of sheet polygons.                            |
| `sheet_offsets`   | `collections.abc.Sequence[tuple[float, float]]`                                                     | World-space offset (x, y) for each sheet.          |
| `rotations`       | `collections.abc.Sequence[float]`                                                                   | Rotation angle (degrees) for each part.            |
| `flips_h`         | `collections.abc.Sequence[bool]`                                                                    | Horizontal flip flag per part.                     |
| `flips_v`         | `collections.abc.Sequence[bool]`                                                                    | Vertical flip flag per part.                       |
| `spacing`         | `float = 1`                                                                                         | Minimum spacing between parts.                     |
| `min_area`        | `float = 1`                                                                                         | Minimum overlap area (clipper coords).             |
| `curve_tolerance` | `float = 0.5`                                                                                       | Curve tolerance for distance filtering.            |
| _Returns_         | `list[dict]`                                                                                        |                                                    |
