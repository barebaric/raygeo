---
title: raygeo.sketch.constraints.point_on_line
sidebar_label: raygeo.sketch.constraints.point_on_line
---

## PointOnLineConstraint

Enforces a point lies on a line or circle/arc circumference.

### `point`

```python
point: int
```

The constrained point.

### `target`

```python
target: PointOnTarget
```

Resolved target geometry.

### `constrains_radius()`

```python
constrains_radius(registry: registry.EntityRegistry, entity_id: int) -> bool
```

Whether this constraint determines the target's radius: true when the target is a circle/arc
circumference and the constrained point is itself solver-constrained.

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

### `set_user_visible()`

```python
set_user_visible(value: bool) -> None
```

Sets the reference user-visibility flag.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `value`   | `bool` |             |
| _Returns_ | `None` |             |

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

## PointOnTarget

The resolved target geometry: an infinite line, or a circle/arc circumference.

### Line

The infinite line through two points.

### RadiusCurve

The circle/arc through center and radius-bearing point.
