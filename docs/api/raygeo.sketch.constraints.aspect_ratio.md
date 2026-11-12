---
title: raygeo.sketch.constraints.aspect_ratio
sidebar_label: raygeo.sketch.constraints.aspect_ratio
---

## AspectRatioConstraint

Enforces dist(p1, p2) == ratio * dist(p3, p4).

### `p1`

```python
p1: int
```

First point of the reference segment.

### `p2`

```python
p2: int
```

Second point of the reference segment.

### `p3`

```python
p3: int
```

First point of the scaled segment.

### `p4`

```python
p4: int
```

Second point of the scaled segment.

### `ratio`

```python
ratio: float
```

Resolved ratio value.

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
