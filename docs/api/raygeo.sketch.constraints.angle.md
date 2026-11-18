---
title: raygeo.sketch.constraints.angle
sidebar_label: raygeo.sketch.constraints.angle
---

## Constants

### ANGLE_WEIGHT

`float`

### ARC_RADIUS

`float`

### LABEL_RADIUS

`float`

## AngleConstraint

Enforces a clockwise angle in degrees between two lines. The angle is measured at the lines'
intersection between the rays selected by `e1_far_idx`/`e2_far_idx` (endpoint ids; `None` picks the
endpoint farther from the intersection).

### `e1_far_idx`

```python
e1_far_idx: Optional[int]
```

Far point of the anchor line.

### `e1_id`

```python
e1_id: int
```

The anchor line entity.

### `e2_far_idx`

```python
e2_far_idx: Optional[int]
```

Far point of the other line.

### `e2_id`

```python
e2_id: int
```

The other line entity.

### `expression`

```python
expression: Optional[str]
```

Stored expression string (evaluated by `update_from_context`).

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

### `value`

```python
value: float
```

Angle value in degrees.

### `applies_to_entities()`

```python
@classmethod applies_to_entities(entities: Any) -> bool
```

Whether both operands are lines.

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `entities` | `Any`  |             |
| _Returns_  | `bool` |             |

### `can_apply_to()`

```python
@classmethod can_apply_to(selection: Any, sketch: Optional[Any] = None) -> bool
```

Whether this constraint can be applied to the selection.

| Parameter   | Type                   | Description |
| ----------- | ---------------------- | ----------- |
| `selection` | `Any`                  |             |
| `sketch`    | `Optional[Any] = None` |             |
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
error(reg: protocols.Registry, params: Optional[Any] = None) -> float
```

Returns the constraint error at the registry's point positions.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `reg`     | `protocols.Registry`   |             |
| `params`  | `Optional[Any] = None` |             |
| _Returns_ | `float`                |             |

### `from_dict()`

```python
@classmethod from_dict(data: dict) -> Any
```

Rebuilds the constraint from its reference dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `data`    | `dict` |             |
| _Returns_ | `Any`  |             |

### `get_edit_subtitle()`

```python
get_edit_subtitle() -> str
```

The user-facing hint for the constraint edit dialog.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

### `get_referenced_point_ids()`

```python
get_referenced_point_ids() -> set[int]
```

The set of point ids this constraint references: the far points selected on the two lines.

| Parameter | Type       | Description |
| --------- | ---------- | ----------- |
| _Returns_ | `set[int]` |             |

### `get_subtitle()`

```python
get_subtitle(registry: protocols.Registry) -> str
```

The subtitle describing the constrained entities.

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

### `get_type_key()`

```python
@classmethod get_type_key() -> str
```

The string key identifying this constraint type.

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

Whether the constraint's rendered symbol is hit at (sx, sy).

| Parameter   | Type                                                   | Description |
| ----------- | ------------------------------------------------------ | ----------- |
| `sx`        | `float`                                                |             |
| `sy`        | `float`                                                |             |
| `registry`  | `protocols.Registry`                                   |             |
| `to_screen` | `Callable[[tuple[float, float]], tuple[float, float]]` |             |
| `element`   | `Optional[Any]`                                        |             |
| `threshold` | `float`                                                |             |
| _Returns_   | `bool`                                                 |             |

### `is_mirror_compatible()`

```python
is_mirror_compatible() -> bool
```

Whether the constraint survives mirroring of all referenced geometry. An expression-based angle
encodes user intent that does not auto-flip under reflection.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `mirror()`

```python
mirror(_axis: Any) -> None
```

Mirrors the angle value: reflection reverses the signed (clockwise) angle, so the numeric value is
negated.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `_axis`   | `Any`  |             |
| _Returns_ | `None` |             |

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
