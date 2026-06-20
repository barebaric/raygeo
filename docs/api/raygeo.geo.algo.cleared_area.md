---
title: raygeo.geo.algo.cleared_area
sidebar_label: raygeo.geo.algo.cleared_area
sidebar_position: 5
---

![ClearedArea tracking a simulated raster toolpath — cleared fragments shown in blue, remaining area in red](images/geo-algo-cleared-area-raster.png)

_ClearedArea tracking a simulated raster toolpath — cleared fragments shown in blue, remaining area
in red_ Incremental cleared-area tracker for adaptive clearing.

Maintains a union of tool-swept polygons and provides a spatial-indexed windowed query for efficient
engagement computation.

## ClearedArea

### `add_cleared_polygons()`

```python
add_cleared_polygons(polygons: Sequence[Sequence[tuple[float, float]]]) -> None
```

| Parameter    | Type                                      | Description                                       |
| ------------ | ----------------------------------------- | ------------------------------------------------- |
| `polygons`   | `Sequence[Sequence[tuple[float, float]]]` |                                                   |
| _Returns_    | `None`                                    |                                                   |
| _Complexity_ |                                           | O(n) where n = total vertices across all polygons |

![ClearedArea with bulk polygon insertion via ``add_cleared_polygons`` — cleared region in blue, remaining area in red](images/geo-algo-cleared-area-bulk.png)

_ClearedArea with bulk polygon insertion via `add_cleared_polygons` — cleared region in blue,
remaining area in red_

### `bites()`

```python
bites(
    step_over: float,
    valid_area: Sequence[Sequence[tuple[float, float]]],
    simplify_tol: float,
) -> list[list[tuple[float, float]]]
```

Compute the "bites" — new material reachable by expanding the current frontier outward by step_over,
clipping to valid_area, and subtracting already-cleared portions.

| Parameter      | Type                                      | Description                                            |
| -------------- | ----------------------------------------- | ------------------------------------------------------ |
| `step_over`    | `float`                                   | lateral step-over in mm                                |
| `valid_area`   | `Sequence[Sequence[tuple[float, float]]]` | list of polygons defining the valid tool-centre region |
| `simplify_tol` | `float`                                   | tolerance in mm for frontier simplification            |
| _Returns_      | `list[list[tuple[float, float]]]`         |                                                        |
| _Complexity_   |                                           | O(n log n)                                             |

![``bites`` computes the expansible material — the crescent-shaped regions of uncut material reachable by expanding the frontier by ``step_over``.](images/geo-algo-cleared-area-bites.png)

_`bites` computes the expansible material — the crescent-shaped regions of uncut material reachable
by expanding the frontier by `step_over`._

### `expand()`

```python
expand(tool_path: Sequence[tuple[float, float]], tool_radius: float) -> None
```

| Parameter     | Type                            | Description                          |
| ------------- | ------------------------------- | ------------------------------------ |
| `tool_path`   | `Sequence[tuple[float, float]]` |                                      |
| `tool_radius` | `float`                         |                                      |
| _Returns_     | `None`                          |                                      |
| _Complexity_  |                                 | O(n) where n = number of path points |

### `frontier()`

```python
frontier(simplify_tol: float) -> list[list[tuple[float, float]]]
```

Return a unioned, simplified snapshot of the current outer boundary.

| Parameter      | Type                              | Description                                 |
| -------------- | --------------------------------- | ------------------------------------------- |
| `simplify_tol` | `float`                           | tolerance in mm for polyline simplification |
| _Returns_      | `list[list[tuple[float, float]]]` |                                             |
| _Complexity_   |                                   | O(n log n)                                  |

![``frontier`` returns the outer boundary of the cleared area after merging overlapping fragments — shown in crimson.](images/geo-algo-cleared-area-frontier.png)

_`frontier` returns the outer boundary of the cleared area after merging overlapping fragments —
shown in crimson._

### `incorporate()`

```python
incorporate(
    polygons: Sequence[Sequence[tuple[float, float]]],
) -> list[list[tuple[float, float]]]
```

Add polygons, returning only the newly-added portion. Faster than add_cleared_polygons when inputs
don't overlap existing fragments (skips the full union). O(n) when inputs are disjoint from existing
fragments

| Parameter    | Type                                      | Description                                |
| ------------ | ----------------------------------------- | ------------------------------------------ |
| `polygons`   | `Sequence[Sequence[tuple[float, float]]]` |                                            |
| _Returns_    | `list[list[tuple[float, float]]]`         |                                            |
| _Complexity_ |                                           | O(n log n) worst case when union required, |

![``incorporate`` adds polygons to the cleared state while returning only the newly-covered region (shown in green).](images/geo-algo-cleared-area-incorporate.png)

_`incorporate` adds polygons to the cleared state while returning only the newly-covered region
(shown in green)._

### `query_window()`

```python
query_window(
    bbox: tuple[float, float, float, float],
) -> list[list[tuple[float, float]]]
```

| Parameter    | Type                                | Description                                                 |
| ------------ | ----------------------------------- | ----------------------------------------------------------- |
| `bbox`       | `tuple[float, float, float, float]` |                                                             |
| _Returns_    | `list[list[tuple[float, float]]]`   |                                                             |
| _Complexity_ |                                     | O(m + k) where m = number of fragments, k = output vertices |

### `remaining()`

```python
remaining(
    bounds: Sequence[Sequence[tuple[float, float]]],
) -> list[list[tuple[float, float]]]
```

| Parameter    | Type                                      | Description                                        |
| ------------ | ----------------------------------------- | -------------------------------------------------- |
| `bounds`     | `Sequence[Sequence[tuple[float, float]]]` |                                                    |
| _Returns_    | `list[list[tuple[float, float]]]`         |                                                    |
| _Complexity_ |                                           | O(n \* m) where n = bounds vertices, m = fragments |

### `total_area()`

```python
total_area() -> float
```

| Parameter    | Type    | Description |
| ------------ | ------- | ----------- |
| _Returns_    | `float` |             |
| _Complexity_ |         | O(1)        |
