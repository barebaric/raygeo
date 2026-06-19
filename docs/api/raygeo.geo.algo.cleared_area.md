---
title: raygeo.geo.algo.cleared_area
sidebar_label: raygeo.geo.algo.cleared_area
sidebar_position: 5
---

Incremental cleared-area tracker for adaptive clearing.

Maintains a union of tool-swept polygons and provides a spatial-indexed windowed query for efficient
engagement computation.

## ClearedArea

### `expand()`

```python
expand(tool_path: Sequence[tuple[float, float]], tool_radius: float) -> None
```

| Parameter     | Type                            | Description |
| ------------- | ------------------------------- | ----------- |
| `tool_path`   | `Sequence[tuple[float, float]]` |             |
| `tool_radius` | `float`                         |             |
| _Returns_     | `None`                          |             |

### `query_window()`

```python
query_window(
    bbox: tuple[float, float, float, float],
) -> list[list[tuple[float, float]]]
```

| Parameter | Type                                | Description |
| --------- | ----------------------------------- | ----------- |
| `bbox`    | `tuple[float, float, float, float]` |             |
| _Returns_ | `list[list[tuple[float, float]]]`   |             |

### `remaining()`

```python
remaining(
    bounds: Sequence[Sequence[tuple[float, float]]],
) -> list[list[tuple[float, float]]]
```

| Parameter | Type                                      | Description |
| --------- | ----------------------------------------- | ----------- |
| `bounds`  | `Sequence[Sequence[tuple[float, float]]]` |             |
| _Returns_ | `list[list[tuple[float, float]]]`         |             |

### `total_area()`

```python
total_area() -> float
```

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| _Returns_ | `float` |             |
