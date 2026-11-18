---
title: raygeo.sketch.entities.point
sidebar_label: raygeo.sketch.entities.point
---

## SketchPoint

A sketch point: coordinates and solver flags.

When obtained from a registry it is a live view: coordinate writes are written back to the registry.
It also behaves like an `(x, y)` pair for indexing and iteration.

### `constrained`

```python
constrained: bool
```

Whether the point is fully constrained, as computed by the solver.

### `fixed`

```python
fixed: bool
```

Whether the point is pinned and never moves.

### `id`

```python
id: types.EntityID
```

Unique point id, stable across solves.

### `waypoint_type`

```python
waypoint_type: str
```

Waypoint type string ("sharp"/"smooth"/"symmetric").

### `x`

```python
x: float
```

X coordinate.

### `y`

```python
y: float
```

Y coordinate.

### `apply_constraint()`

```python
apply_constraint(
    registry: protocols.Registry,
    bezier: Any,
    cp_index: int,
    sketch: Optional[Any] = None,
) -> None
```

Applies the bezier-waypoint constraint after a control point edit on one of the point's connected
beziers.

| Parameter  | Type                   | Description |
| ---------- | ---------------------- | ----------- |
| `registry` | `protocols.Registry`   |             |
| `bezier`   | `Any`                  |             |
| `cp_index` | `int`                  |             |
| `sketch`   | `Optional[Any] = None` |             |
| _Returns_  | `None`                 |             |

### `enforce_constraint()`

```python
enforce_constraint(
    registry: protocols.Registry,
    sketch: Optional[Any] = None,
) -> None
```

Re-enforces the bezier-waypoint constraint across every connected bezier at this point.

| Parameter  | Type                   | Description |
| ---------- | ---------------------- | ----------- |
| `registry` | `protocols.Registry`   |             |
| `sketch`   | `Optional[Any] = None` |             |
| _Returns_  | `None`                 |             |

### `from_dict()`

```python
from_dict(dict: dict) -> SketchPoint
```

Builds a point from the reference point dictionary shape.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| `dict`    | `dict`        |             |
| _Returns_ | `SketchPoint` |             |

### `get_connected_beziers()`

```python
get_connected_beziers(
    registry: protocols.Registry,
    sketch: Optional[Any] = None,
) -> list[Any]
```

The bezier entities connected to this point, as the registry's own Python objects.

| Parameter  | Type                   | Description |
| ---------- | ---------------------- | ----------- |
| `registry` | `protocols.Registry`   |             |
| `sketch`   | `Optional[Any] = None` |             |
| _Returns_  | `list[Any]`            |             |

### `get_paired_beziers()`

```python
get_paired_beziers(
    registry: protocols.Registry,
    sketch: Optional[Any] = None,
) -> tuple[Any, Any]
```

The two bezier entities joined at this point, if any.

| Parameter  | Type                   | Description |
| ---------- | ---------------------- | ----------- |
| `registry` | `protocols.Registry`   |             |
| `sketch`   | `Optional[Any] = None` |             |
| _Returns_  | `tuple[Any, Any]`      |             |

### `is_in_rect()`

```python
is_in_rect(rect: tuple[float, float, float, float]) -> bool
```

Whether the point lies inside the given rect.

| Parameter | Type                                | Description |
| --------- | ----------------------------------- | ----------- |
| `rect`    | `tuple[float, float, float, float]` |             |
| _Returns_ | `bool`                              |             |

### `is_sharp()`

```python
is_sharp() -> bool
```

Whether the waypoint role is sharp.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `is_smooth()`

```python
is_smooth() -> bool
```

Whether the waypoint role is smooth.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `is_symmetric()`

```python
is_symmetric() -> bool
```

Whether the waypoint role is symmetric.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `pos()`

```python
pos() -> tuple[float, float]
```

The point position as a `(x, y)` tuple.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| _Returns_ | `tuple[float, float]` |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes to the reference point dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |
