---
title: raygeo.sketch.snap.spatial
sidebar_label: raygeo.sketch.snap.spatial
---

## IndexedLine

### `coordinate`

```python
coordinate: float
```

### `snap_line`

```python
snap_line: Optional[types.SnapLine]
```

## SnapLineIndex

### `add()`

```python
add(snap_line: types.SnapLine) -> None
```

| Parameter   | Type             | Description |
| ----------- | ---------------- | ----------- |
| `snap_line` | `types.SnapLine` |             |
| _Returns_   | `None`           |             |

### `add_all()`

```python
add_all(snap_lines: Any) -> None
```

| Parameter    | Type   | Description |
| ------------ | ------ | ----------- |
| `snap_lines` | `Any`  |             |
| _Returns_    | `None` |             |

### `clear()`

```python
clear() -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `query()`

```python
query(
    x: float,
    y: float,
    threshold: float,
) -> list[tuple[types.SnapLine, float]]
```

| Parameter   | Type                                 | Description |
| ----------- | ------------------------------------ | ----------- |
| `x`         | `float`                              |             |
| `y`         | `float`                              |             |
| `threshold` | `float`                              |             |
| _Returns_   | `list[tuple[types.SnapLine, float]]` |             |

### `query_horizontal()`

```python
query_horizontal(
    y: float,
    threshold: float,
) -> list[tuple[types.SnapLine, float]]
```

| Parameter   | Type                                 | Description |
| ----------- | ------------------------------------ | ----------- |
| `y`         | `float`                              |             |
| `threshold` | `float`                              |             |
| _Returns_   | `list[tuple[types.SnapLine, float]]` |             |

### `query_vertical()`

```python
query_vertical(
    x: float,
    threshold: float,
) -> list[tuple[types.SnapLine, float]]
```

| Parameter   | Type                                 | Description |
| ----------- | ------------------------------------ | ----------- |
| `x`         | `float`                              |             |
| `threshold` | `float`                              |             |
| _Returns_   | `list[tuple[types.SnapLine, float]]` |             |
