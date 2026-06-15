---
title: raygeo.nest.spatial_grid
sidebar_label: raygeo.nest.spatial_grid
sidebar_position: 31
---

Grid-based spatial index for fast overlap queries.

Divides the 2D plane into fixed-size cells and associates each inserted item with the cells its
bounding box touches.

## SpatialGrid

### `clear()`

`clear() -> None`

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `insert()`

`insert(index: int, bbox: Sequence[float]) -> None`

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `index`   | `int`             |             |
| `bbox`    | `Sequence[float]` |             |
| _Returns_ | `None`            |             |

### `query()`

`query(bbox: Sequence[float]) -> list[int]`

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `bbox`    | `Sequence[float]` |             |
| _Returns_ | `list[int]`       |             |
