---
title: raygeo.sketch.constraints.drag
sidebar_label: raygeo.sketch.constraints.drag
---

## DragConstraint

Softly pulls a point towards a target coordinate.

### `point_id`

```python
point_id: int
```

The dragged point.

### `status`

```python
status: int
```

Reference validation status ordinal.

### `target_x`

```python
target_x: float
```

Target X coordinate.

### `target_y`

```python
target_y: float
```

Target Y coordinate.

### `user_visible`

```python
user_visible: bool
```

Reference user-visibility flag.

### `weight`

```python
weight: float
```

Pull strength.

### `draw()`

```python
draw(
    ctx: protocols.Canvas,
    registry: Any,
    to_screen: Callable[[tuple[float, float]], tuple[float, float]],
    is_selected: bool = False,
    is_hovered: bool = False,
    point_radius: float = 5.0,
) -> None
```

Draws the constraint's symbol onto the cairo context.

| Parameter      | Type                                                   | Description |
| -------------- | ------------------------------------------------------ | ----------- |
| `ctx`          | `protocols.Canvas`                                     |             |
| `registry`     | `Any`                                                  |             |
| `to_screen`    | `Callable[[tuple[float, float]], tuple[float, float]]` |             |
| `is_selected`  | `bool = False`                                         |             |
| `is_hovered`   | `bool = False`                                         |             |
| `point_radius` | `float = 5.0`                                          |             |
| _Returns_      | `None`                                                 |             |

### `error()`

```python
error(
    reg: protocols.Registry,
    params: Optional[Any] = None,
) -> tuple[float, float]
```

Returns the constraint error at the registry's point positions.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `reg`     | `protocols.Registry`   |             |
| `params`  | `Optional[Any] = None` |             |
| _Returns_ | `tuple[float, float]`  |             |

### `gradient()`

```python
gradient(
    reg: protocols.Registry,
    params: Optional[Any] = None,
) -> dict[int, list[tuple[float, float]]]
```

Partial derivatives of the error by point id.

| Parameter | Type                                   | Description |
| --------- | -------------------------------------- | ----------- |
| `reg`     | `protocols.Registry`                   |             |
| `params`  | `Optional[Any] = None`                 |             |
| _Returns_ | `dict[int, list[tuple[float, float]]]` |             |

### `is_hit()`

```python
is_hit(
    sx: float,
    sy: float,
    registry: protocols.Registry,
    to_screen: Callable[[tuple[float, float]], tuple[float, float]],
    element: Optional[Any],
    threshold: float,
) -> bool
```

Whether the constraint's rendered symbol is hit at (mx, my).

| Parameter   | Type                                                   | Description |
| ----------- | ------------------------------------------------------ | ----------- |
| `sx`        | `float`                                                |             |
| `sy`        | `float`                                                |             |
| `registry`  | `protocols.Registry`                                   |             |
| `to_screen` | `Callable[[tuple[float, float]], tuple[float, float]]` |             |
| `element`   | `Optional[Any]`                                        |             |
| `threshold` | `float`                                                |             |
| _Returns_   | `bool`                                                 |             |

### `referenced_points()`

```python
referenced_points(registry: protocols.Registry) -> list[int]
```

Point ids the constraint references.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `list[int]`          |             |

### `remap_references()`

```python
remap_references(id_map: Mapping[int, int]) -> None
```

Rewrites every id reference through *id_map*.

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `id_map`  | `Mapping[int, int]` |             |
| _Returns_ | `None`              |             |

### `set_target()`

```python
set_target(target_x: float, target_y: float, weight: float) -> None
```

Sets the target coordinates and pull strength.

| Parameter  | Type    | Description |
| ---------- | ------- | ----------- |
| `target_x` | `float` |             |
| `target_y` | `float` |             |
| `weight`   | `float` |             |
| _Returns_  | `None`  |             |
