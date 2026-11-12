---
title: raygeo.sketch.constraints.radius
sidebar_label: raygeo.sketch.constraints.radius
---

## RadiusConstraint

Enforces the radius of a circle or arc.

### `center`

```python
center: int
```

Center point of the circle/arc.

### `radius_point`

```python
radius_point: int
```

Radius-bearing point (circle radius point or arc start).

### `value`

```python
value: float
```

Resolved radius value.

### `constrains_radius()`

```python
constrains_radius(registry: registry.EntityRegistry, entity_id: int) -> bool
```

Whether this constraint explicitly defines the radius of the given entity. The entity is matched
through the registry by its defining points, mirroring the reference's stored entity id.

| Parameter   | Type                      | Description |
| ----------- | ------------------------- | ----------- |
| `registry`  | `registry.EntityRegistry` |             |
| `entity_id` | `int`                     |             |
| _Returns_   | `bool`                    |             |

### `error()`

```python
error(registry: registry.EntityRegistry) -> list[float]
```

Returns the constraint error at the registry's point positions.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| _Returns_  | `list[float]`             |             |

### `expression()`

```python
expression() -> Optional[str]
```

The stored expression string, if any (evaluated caller-side).

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| _Returns_ | `Optional[str]` |             |

### `is_hit()`

```python
is_hit(
    mx: float,
    my: float,
    threshold: float,
    registry: registry.EntityRegistry,
    to_screen: Optional[Any] = None,
) -> bool
```

Whether the constraint's rendered symbol is hit at (mx, my).

| Parameter   | Type                      | Description |
| ----------- | ------------------------- | ----------- |
| `mx`        | `float`                   |             |
| `my`        | `float`                   |             |
| `threshold` | `float`                   |             |
| `registry`  | `registry.EntityRegistry` |             |
| `to_screen` | `Optional[Any] = None`    |             |
| _Returns_   | `bool`                    |             |

### `set_expression()`

```python
set_expression(expression: Optional[str]) -> None
```

Sets the expression string (evaluated caller-side).

| Parameter    | Type            | Description |
| ------------ | --------------- | ----------- |
| `expression` | `Optional[str]` |             |
| _Returns_    | `None`          |             |

### `set_user_visible()`

```python
set_user_visible(value: bool) -> None
```

Sets the reference user-visibility flag.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `value`   | `bool` |             |
| _Returns_ | `None` |             |

### `targets_segment()`

```python
targets_segment(
    p1: int,
    p2: int,
    entity_id: Optional[int],
    registry: registry.EntityRegistry,
) -> bool
```

Whether this constraint restricts the given entity. The entity is matched through the registry by
its defining points, mirroring the reference's stored entity id.

| Parameter   | Type                      | Description |
| ----------- | ------------------------- | ----------- |
| `p1`        | `int`                     |             |
| `p2`        | `int`                     |             |
| `entity_id` | `Optional[int]`           |             |
| `registry`  | `registry.EntityRegistry` |             |
| _Returns_   | `bool`                    |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes to the reference constraint dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `user_visible()`

```python
user_visible() -> bool
```

Reference user-visibility flag.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |
