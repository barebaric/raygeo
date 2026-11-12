---
title: raygeo.sketch.constraints.symmetry
sidebar_label: raygeo.sketch.constraints.symmetry
---

## SymmetryConstraint

Enforces two points are symmetric about a point or a line.

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

### `target`

```python
target: SymmetryTarget
```

Symmetry reference.

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

## SymmetryTarget

The symmetry reference: point symmetry about a center, or line symmetry about an axis line.

### Line

p1 and p2 are mirrored across the line (l1, l2).

### Point

The center is the midpoint of p1 and p2.
