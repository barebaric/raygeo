---
title: raygeo.sketch.constraints.angle
sidebar_label: raygeo.sketch.constraints.angle
---

## AngleConstraint

Enforces a clockwise angle in degrees between two lines. The angle is measured at the lines'
intersection between the rays selected by `e1_far`/`e2_far` (endpoint ids; `None` picks the endpoint
farther from the intersection).

### `e1_far`

```python
e1_far: Optional[int]
```

Far point of the anchor line.

### `e2_far`

```python
e2_far: Optional[int]
```

Far point of the other line.

### `p1`

```python
p1: int
```

First point of the anchor line.

### `p2`

```python
p2: int
```

Second point of the anchor line.

### `p3`

```python
p3: int
```

First point of the other line.

### `p4`

```python
p4: int
```

Second point of the other line.

### `value_deg`

```python
value_deg: float
```

Resolved angle value in degrees.

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

### `is_mirror_compatible()`

```python
is_mirror_compatible() -> bool
```

Whether the constraint survives mirroring of all referenced geometry. An expression-based angle
encodes user intent that does not auto-flip under reflection.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

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
