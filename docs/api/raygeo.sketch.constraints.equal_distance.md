---
title: raygeo.sketch.constraints.equal_distance
sidebar_label: raygeo.sketch.constraints.equal_distance
---

## EqualDistanceConstraint

Enforces dist(p1, p2) == dist(p3, p4).

### `p1`

```python
p1: int
```

First point of the first pair.

### `p2`

```python
p2: int
```

Second point of the first pair.

### `p3`

```python
p3: int
```

First point of the second pair.

### `p4`

```python
p4: int
```

Second point of the second pair.

### `error()`

```python
error(registry: registry.EntityRegistry) -> list[float]
```

Returns the constraint error at the registry's point positions.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| _Returns_  | `list[float]`             |             |

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

Whether this constraint defines the length/distance of the segment (p1, p2), for either pair (order
independent).

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
