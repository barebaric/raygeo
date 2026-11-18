---
title: raygeo.sketch.constraints.symmetry
sidebar_label: raygeo.sketch.constraints.symmetry
---

## SymmetryConstraint

Enforces two points are symmetric about a point or a line.

### `axis`

```python
axis: Optional[int]
```

Symmetry axis line entity, for line symmetry.

### `center`

```python
center: Optional[int]
```

Symmetry center point, for point symmetry.

### `p1`

```python
p1: int
```

First point.

### `p2`

```python
p2: int
```

Second point.

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

### `applies_to_entities()`

```python
@classmethod applies_to_entities(entities: Any) -> bool
```

Whether the operand is a line (the symmetry axis).

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
error(reg: protocols.Registry, params: Optional[Any] = None) -> list[float]
```

Returns the constraint error at the registry's point positions.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `reg`     | `protocols.Registry`   |             |
| `params`  | `Optional[Any] = None` |             |
| _Returns_ | `list[float]`          |             |

### `from_dict()`

```python
@classmethod from_dict(data: dict) -> Any
```

Rebuilds the constraint from its reference dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `data`    | `dict` |             |
| _Returns_ | `Any`  |             |

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

## Functions

### `draw_symmetry_arrows()`

```python
draw_symmetry_arrows(
    ctx: protocols.Canvas,
    s1: tuple[float, float],
    s2: tuple[float, float],
) -> None
```

Draws the two symmetry arrow heads along the segment between two screen-space points (the reference
module-level helper).

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `ctx`     | `protocols.Canvas`    |             |
| `s1`      | `tuple[float, float]` |             |
| `s2`      | `tuple[float, float]` |             |
| _Returns_ | `None`                |             |
