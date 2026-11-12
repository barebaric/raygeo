---
title: raygeo.sketch.constraints.rotational
sidebar_label: raygeo.sketch.constraints.rotational
---

## RotationalConstraint

Enforces p2 is p1 rotated by angle_rad about center.

### `angle_rad`

```python
angle_rad: float
```

Resolved rotation angle in radians.

### `center`

```python
center: int
```

Rotation center.

### `p1`

```python
p1: int
```

Source point.

### `p2`

```python
p2: int
```

Rotated point.

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
