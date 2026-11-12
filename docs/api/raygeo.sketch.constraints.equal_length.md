---
title: raygeo.sketch.constraints.equal_length
sidebar_label: raygeo.sketch.constraints.equal_length
---

## EqualLengthConstraint

Enforces equal characteristic lengths, one residual per `(base_a, base_b, other_a, other_b)` tuple.
The caller expands the entities' characteristic-length pairs (including the shorter-side fallback)
into flat tuples.

### `pairs`

```python
pairs: list[list[int]]
```

Resolved characteristic-length pairs.

### `constrains_radius()`

```python
constrains_radius(registry: registry.EntityRegistry, entity_id: int) -> bool
```

Whether this constraint explicitly defines the radius/length of the given entity, matched through
the registry by its defining points.

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

### `targets_segment()`

```python
targets_segment(
    p1: int,
    p2: int,
    entity_id: Optional[int],
    registry: registry.EntityRegistry,
) -> bool
```

Whether this constraint restricts the length of the given entity, matched through the registry by
its defining points (the point ids are ignored, mirroring the reference).

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
