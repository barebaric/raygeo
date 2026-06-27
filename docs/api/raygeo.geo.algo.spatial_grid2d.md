---
title: raygeo.geo.algo.spatial_grid2d
sidebar_label: raygeo.geo.algo.spatial_grid2d
---

Grid-based spatial index for fast overlap queries.

Divides the 2D plane into fixed-size cells and associates each inserted item with the cells its
bounding box touches.

## SpatialGrid

A grid-based spatial index for fast overlap queries.

Divides the 2D plane into fixed-size cells and indexes items by their bounding box for efficient
overlap lookups.

### `clear()`

```python
clear() -> None
```

Remove all items from the grid.

| Parameter    | Type   | Description                    |
| ------------ | ------ | ------------------------------ |
| _Returns_    | `None` |                                |
| _Complexity_ |        | O(n) where n = number of items |

### `insert()`

```python
insert(index: int, bbox: Sequence[float]) -> None
```

Insert an item into the grid by its bounding box.

| Parameter    | Type              | Description                                  |
| ------------ | ----------------- | -------------------------------------------- |
| `index`      | `int`             | Unique identifier for the item.              |
| `bbox`       | `Sequence[float]` | `[x_min, y_min, x_max, y_max]` bounding box. |
| _Returns_    | `None`            |                                              |
| _Complexity_ |                   | O(1) amortised                               |

### `query()`

```python
query(bbox: Sequence[float]) -> list[int]
```

Query all items whose bounding box overlaps *bbox*.

| Parameter    | Type              | Description                                     |
| ------------ | ----------------- | ----------------------------------------------- |
| `bbox`       | `Sequence[float]` | `[x_min, y_min, x_max, y_max]` query region.    |
| _Returns_    | `list[int]`       | Sorted list of matching item indices.           |
| _Complexity_ |                   | O(cells + k) where k = number of matching items |

### `remove()`

```python
remove(index: int, bbox: Sequence[float]) -> None
```

Remove an item from the grid by its bounding box.

| Parameter    | Type              | Description                                                  |
| ------------ | ----------------- | ------------------------------------------------------------ |
| `index`      | `int`             | Unique identifier for the item.                              |
| `bbox`       | `Sequence[float]` | `[x_min, y_min, x_max, y_max]` bounding box.                 |
| _Returns_    | `None`            |                                                              |
| _Complexity_ |                   | O(cells) where cells = number of grid cells the bbox touches |
