---
title: raygeo.sketch.constraints.parallelogram
sidebar_label: raygeo.sketch.constraints.parallelogram
---

## ParallelogramConstraint

Enforces (p_width - p_origin) == (p4 - p_height).

### `p4`

```python
p4: int
```

Fourth corner.

### `p_height`

```python
p_height: int
```

Height point.

### `p_origin`

```python
p_origin: int
```

Origin point.

### `p_width`

```python
p_width: int
```

Width point.

### `status`

```python
status: int
```

Reference validation status ordinal.

### `user_visible`

```python
user_visible: bool
```

Reference user-visibility flag.

### `can_apply_to()`

```python
@classmethod
can_apply_to(
    selection: Any,
    _sketch: Optional[Any] = None,
) -> bool
```

Whether this constraint can be applied to the selection.

| Parameter   | Type                   | Description |
| ----------- | ---------------------- | ----------- |
| `selection` | `Any`                  |             |
| `_sketch`   | `Optional[Any] = None` |             |
| _Returns_   | `bool`                 |             |

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

### `from_dict()`

```python
@classmethod from_dict(data: dict) -> Any
```

Rebuilds the constraint from its reference dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `data`    | `dict` |             |
| _Returns_ | `Any`  |             |

### `get_referenced_point_ids()`

```python
get_referenced_point_ids() -> set[int]
```

The set of point ids this constraint references.

| Parameter | Type       | Description |
| --------- | ---------- | ----------- |
| _Returns_ | `set[int]` |             |

### `get_subtitle()`

```python
get_subtitle(registry: protocols.Registry) -> str
```

The subtitle describing the constrained points.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `str`                |             |

### `get_title()`

```python
get_title() -> str
```

The human-readable title for this constraint.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

### `get_type_name()`

```python
get_type_name() -> str
```

The human-readable type name (translated).

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

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

### `to_dict()`

```python
to_dict() -> dict
```

Serializes to the reference constraint dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |
