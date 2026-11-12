---
title: raygeo.sketch.constraints.point_on_curve
sidebar_label: raygeo.sketch.constraints.point_on_curve
---

## PointOnCurveConstraint

Enforces a point lies on a cubic Bezier curve. The control points are stored as offsets from their
owning endpoints; curves without control points use the straight-segment residual (`is_line`).

### `cp1_offset`

```python
cp1_offset: tuple[float, float]
```

First control point as offset from the start point.

### `cp2_offset`

```python
cp2_offset: tuple[float, float]
```

Second control point as offset from the end point.

### `end`

```python
end: int
```

Curve end point.

### `is_line`

```python
is_line: bool
```

Whether the curve degenerates to a straight segment.

### `point`

```python
point: int
```

The constrained point.

### `start`

```python
start: int
```

Curve start point.

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
