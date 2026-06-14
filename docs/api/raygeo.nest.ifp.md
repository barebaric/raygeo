---
title: raygeo.nest.ifp
sidebar_label: raygeo.nest.ifp
sidebar_position: 27
---

Inner-Fit Polygon (IFP) calculation for nesting algorithms.

Provides functions for computing Inner-Fit Polygons, which define the valid placement region for a
part inside a bin.

## Functions

### `build_no_go_zones()`

`build_no_go_zones(bin: collections.abc.Sequence[types.Point], part_neg: collections.abc.Sequence[types.Point]) -> list[types.Polygon]`

Build the no-go zones for a bin-part pair.

**Returns:** List of no-go zone polygons.

| Parameter  | Type                                    | Description                                |
| ---------- | --------------------------------------- | ------------------------------------------ |
| `bin`      | `collections.abc.Sequence[types.Point]` | Bin polygon as (x, y) points.              |
| `part_neg` | `collections.abc.Sequence[types.Point]` | Orbiting polygon negated as (x, y) points. |
| _Returns_  | `list[types.Polygon]`                   |                                            |

### `inner_fit_polygon()`

`inner_fit_polygon(bin: collections.abc.Sequence[types.Point], part: collections.abc.Sequence[types.Point]) -> list[types.Polygon]`

Compute the Inner-Fit Polygon (IFP) for a part inside a bin.

**Returns:** List of IFP polygons.

| Parameter | Type                                    | Description                    |
| --------- | --------------------------------------- | ------------------------------ |
| `bin`     | `collections.abc.Sequence[types.Point]` | Bin polygon as (x, y) points.  |
| `part`    | `collections.abc.Sequence[types.Point]` | Part polygon as (x, y) points. |
| _Returns_ | `list[types.Polygon]`                   |                                |

![Inner Fit Polygon showing valid placement region](images/inner-fit-polygon.png)

_Inner Fit Polygon showing valid placement region_

### `sweep_hull_for_edge()`

`sweep_hull_for_edge(p1: types.Point, p2: types.Point, part_neg: collections.abc.Sequence[types.Point]) -> types.Polygon`

Compute the convex hull sweep of part_neg along the edge p1->p2.

**Returns:** Convex hull polygon.

| Parameter  | Type                                    | Description                                |
| ---------- | --------------------------------------- | ------------------------------------------ |
| `p1`       | `types.Point`                           | First edge endpoint.                       |
| `p2`       | `types.Point`                           | Second edge endpoint.                      |
| `part_neg` | `collections.abc.Sequence[types.Point]` | Orbiting polygon negated as (x, y) points. |
| _Returns_  | `types.Polygon`                         |                                            |
