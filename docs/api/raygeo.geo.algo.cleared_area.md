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
