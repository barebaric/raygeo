---
title: raygeo.sketch.snap.producers
sidebar_label: raygeo.sketch.snap.producers
---

## CentersProducer

### `produce()`

```python
produce(
    registry: registry.EntityRegistry,
    _drag_position: tuple[float, float],
    drag_context: types.DragContext,
    _threshold: float,
) -> list[types.SnapLine]
```

| Parameter        | Type                      | Description |
| ---------------- | ------------------------- | ----------- |
| `registry`       | `registry.EntityRegistry` |             |
| `_drag_position` | `tuple[float, float]`     |             |
| `drag_context`   | `types.DragContext`       |             |
| `_threshold`     | `float`                   |             |
| _Returns_        | `list[types.SnapLine]`    |             |

### `produce_points()`

```python
produce_points(
    registry: registry.EntityRegistry,
    drag_position: tuple[float, float],
    drag_context: types.DragContext,
    threshold: float,
) -> list[types.SnapPoint]
```

| Parameter       | Type                      | Description |
| --------------- | ------------------------- | ----------- |
| `registry`      | `registry.EntityRegistry` |             |
| `drag_position` | `tuple[float, float]`     |             |
| `drag_context`  | `types.DragContext`       |             |
| `threshold`     | `float`                   |             |
| _Returns_       | `list[types.SnapPoint]`   |             |

## EntityPointsProducer

### `produce()`

```python
produce(
    registry: registry.EntityRegistry,
    _drag_position: tuple[float, float],
    drag_context: types.DragContext,
    _threshold: float,
) -> list[types.SnapLine]
```

| Parameter        | Type                      | Description |
| ---------------- | ------------------------- | ----------- |
| `registry`       | `registry.EntityRegistry` |             |
| `_drag_position` | `tuple[float, float]`     |             |
| `drag_context`   | `types.DragContext`       |             |
| `_threshold`     | `float`                   |             |
| _Returns_        | `list[types.SnapLine]`    |             |

### `produce_points()`

```python
produce_points(
    registry: registry.EntityRegistry,
    drag_position: tuple[float, float],
    drag_context: types.DragContext,
    threshold: float,
) -> list[types.SnapPoint]
```

| Parameter       | Type                      | Description |
| --------------- | ------------------------- | ----------- |
| `registry`      | `registry.EntityRegistry` |             |
| `drag_position` | `tuple[float, float]`     |             |
| `drag_context`  | `types.DragContext`       |             |
| `threshold`     | `float`                   |             |
| _Returns_       | `list[types.SnapPoint]`   |             |

## EquidistantLinesProducer

### `produce()`

```python
produce(
    _registry: registry.EntityRegistry,
    _drag_position: tuple[float, float],
    _drag_context: types.DragContext,
    _threshold: float,
) -> list[types.SnapLine]
```

| Parameter        | Type                      | Description |
| ---------------- | ------------------------- | ----------- |
| `_registry`      | `registry.EntityRegistry` |             |
| `_drag_position` | `tuple[float, float]`     |             |
| `_drag_context`  | `types.DragContext`       |             |
| `_threshold`     | `float`                   |             |
| _Returns_        | `list[types.SnapLine]`    |             |

### `produce_points()`

```python
produce_points(
    registry: registry.EntityRegistry,
    drag_position: tuple[float, float],
    drag_context: types.DragContext,
    threshold: float,
) -> list[types.SnapPoint]
```

| Parameter       | Type                      | Description |
| --------------- | ------------------------- | ----------- |
| `registry`      | `registry.EntityRegistry` |             |
| `drag_position` | `tuple[float, float]`     |             |
| `drag_context`  | `types.DragContext`       |             |
| `threshold`     | `float`                   |             |
| _Returns_       | `list[types.SnapPoint]`   |             |

## IntersectionsProducer

### `produce()`

```python
produce(
    registry: registry.EntityRegistry,
    _drag_position: tuple[float, float],
    drag_context: Any,
    _threshold: float,
) -> list[types.SnapLine]
```

| Parameter        | Type                      | Description |
| ---------------- | ------------------------- | ----------- |
| `registry`       | `registry.EntityRegistry` |             |
| `_drag_position` | `tuple[float, float]`     |             |
| `drag_context`   | `Any`                     |             |
| `_threshold`     | `float`                   |             |
| _Returns_        | `list[types.SnapLine]`    |             |

### `produce_points()`

```python
produce_points(
    registry: registry.EntityRegistry,
    drag_position: tuple[float, float],
    drag_context: Any,
    threshold: float,
) -> list[types.SnapPoint]
```

| Parameter       | Type                      | Description |
| --------------- | ------------------------- | ----------- |
| `registry`      | `registry.EntityRegistry` |             |
| `drag_position` | `tuple[float, float]`     |             |
| `drag_context`  | `Any`                     |             |
| `threshold`     | `float`                   |             |
| _Returns_       | `list[types.SnapPoint]`   |             |

## MidpointsProducer

### `produce()`

```python
produce(
    _registry: registry.EntityRegistry,
    _drag_position: tuple[float, float],
    _drag_context: types.DragContext,
    _threshold: float,
) -> list[types.SnapLine]
```

| Parameter        | Type                      | Description |
| ---------------- | ------------------------- | ----------- |
| `_registry`      | `registry.EntityRegistry` |             |
| `_drag_position` | `tuple[float, float]`     |             |
| `_drag_context`  | `types.DragContext`       |             |
| `_threshold`     | `float`                   |             |
| _Returns_        | `list[types.SnapLine]`    |             |

### `produce_points()`

```python
produce_points(
    registry: registry.EntityRegistry,
    drag_position: tuple[float, float],
    drag_context: types.DragContext,
    threshold: float,
) -> list[types.SnapPoint]
```

| Parameter       | Type                      | Description |
| --------------- | ------------------------- | ----------- |
| `registry`      | `registry.EntityRegistry` |             |
| `drag_position` | `tuple[float, float]`     |             |
| `drag_context`  | `types.DragContext`       |             |
| `threshold`     | `float`                   |             |
| _Returns_       | `list[types.SnapPoint]`   |             |

## OnEntityProducer

### `produce()`

```python
produce(
    _registry: registry.EntityRegistry,
    _drag_position: tuple[float, float],
    _drag_context: types.DragContext,
    _threshold: float,
) -> list[types.SnapLine]
```

| Parameter        | Type                      | Description |
| ---------------- | ------------------------- | ----------- |
| `_registry`      | `registry.EntityRegistry` |             |
| `_drag_position` | `tuple[float, float]`     |             |
| `_drag_context`  | `types.DragContext`       |             |
| `_threshold`     | `float`                   |             |
| _Returns_        | `list[types.SnapLine]`    |             |

### `produce_points()`

```python
produce_points(
    registry: registry.EntityRegistry,
    drag_position: tuple[float, float],
    drag_context: types.DragContext,
    threshold: float,
) -> list[types.SnapPoint]
```

| Parameter       | Type                      | Description |
| --------------- | ------------------------- | ----------- |
| `registry`      | `registry.EntityRegistry` |             |
| `drag_position` | `tuple[float, float]`     |             |
| `drag_context`  | `types.DragContext`       |             |
| `threshold`     | `float`                   |             |
| _Returns_       | `list[types.SnapPoint]`   |             |
