---
title: raygeo.sketch.snap.engine
sidebar_label: raygeo.sketch.snap.engine
---

## SnapEngine

### `enabled`

```python
enabled: bool
```

### `threshold`

```python
threshold: float
```

### `clear_producers()`

```python
clear_producers() -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `get_visible_snap_lines()`

```python
get_visible_snap_lines(
    registry: Any,
    position: tuple[float, float],
    drag_context: Optional[Any] = None,
) -> list[types.SnapLine]
```

| Parameter      | Type                   | Description |
| -------------- | ---------------------- | ----------- |
| `registry`     | `Any`                  |             |
| `position`     | `tuple[float, float]`  |             |
| `drag_context` | `Optional[Any] = None` |             |
| _Returns_      | `list[types.SnapLine]` |             |

### `invalidate_index()`

```python
invalidate_index() -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `is_snap_result_current()`

```python
is_snap_result_current(registry: Any) -> bool
```

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `registry` | `Any`  |             |
| _Returns_  | `bool` |             |

### `query()`

```python
query(
    registry: Any,
    position: tuple[float, float],
    drag_context: Optional[Any] = None,
) -> types.SnapResult
```

| Parameter      | Type                   | Description |
| -------------- | ---------------------- | ----------- |
| `registry`     | `Any`                  |             |
| `position`     | `tuple[float, float]`  |             |
| `drag_context` | `Optional[Any] = None` |             |
| _Returns_      | `types.SnapResult`     |             |

### `rebuild_index()`

```python
rebuild_index(
    registry: Any,
    drag_position: tuple[float, float],
    drag_context: Any,
) -> None
```

| Parameter       | Type                  | Description |
| --------------- | --------------------- | ----------- |
| `registry`      | `Any`                 |             |
| `drag_position` | `tuple[float, float]` |             |
| `drag_context`  | `Any`                 |             |
| _Returns_       | `None`                |             |

### `register_producer()`

```python
register_producer(producer: Any) -> None
```

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `producer` | `Any`  |             |
| _Returns_  | `None` |             |

### `unregister_producer()`

```python
unregister_producer(producer: Any) -> None
```

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `producer` | `Any`  |             |
| _Returns_  | `None` |             |

## SnapLineProducer

### `produce()`

```python
produce(
    _registry: Any,
    _drag_position: tuple[float, float],
    _drag_context: Any,
    _threshold: float,
) -> list[types.SnapLine]
```

| Parameter        | Type                   | Description |
| ---------------- | ---------------------- | ----------- |
| `_registry`      | `Any`                  |             |
| `_drag_position` | `tuple[float, float]`  |             |
| `_drag_context`  | `Any`                  |             |
| `_threshold`     | `float`                |             |
| _Returns_        | `list[types.SnapLine]` |             |

### `produce_points()`

```python
produce_points(
    _registry: Any,
    _drag_position: tuple[float, float],
    _drag_context: Any,
    _threshold: float,
) -> list[types.SnapPoint]
```

| Parameter        | Type                    | Description |
| ---------------- | ----------------------- | ----------- |
| `_registry`      | `Any`                   |             |
| `_drag_position` | `tuple[float, float]`   |             |
| `_drag_context`  | `Any`                   |             |
| `_threshold`     | `float`                 |             |
| _Returns_        | `list[types.SnapPoint]` |             |
