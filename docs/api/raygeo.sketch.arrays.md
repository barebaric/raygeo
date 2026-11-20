---
title: raygeo.sketch.arrays
sidebar_label: raygeo.sketch.arrays
---

## ArrayDefinition

Persistent definition of a sketch array.

### `align_to_tangent`

```python
align_to_tangent: bool
```

Whether curve-along copies follow the tangent.

### `count`

```python
count: int
```

Number of members.

### `guide_circle_id`

```python
guide_circle_id: types.EntityID
```

Guide circle or guide path entity id.

### `members`

```python
members: list[tuple[int, list[types.EntityID]]]
```

Array members as `(slot, [entity_id, ...])` pairs.

### `mode`

```python
mode: str
```

Array mode: `circular` or `curve_along`.

### `offset_to_start`

```python
offset_to_start: float
```

Curve-along start offset.

### `path_entity_id`

```python
path_entity_id: int
```

Curve-along guide path entity id.

### `rotate_copies`

```python
rotate_copies: bool
```

Whether copies are rotated with the guide.

### `spacing`

```python
spacing: float
```

Curve-along spacing.

### `standalone_pids`

```python
standalone_pids: list[tuple[int, list[types.EntityID]]]
```

Standalone member points as `(slot, [point_id, ...])` pairs.

### `template_anchor`

```python
template_anchor: Optional[tuple[tuple[float, float], float]]
```

Curve-along template anchor.

### `total_angle_deg`

```python
total_angle_deg: float
```

Circular-array sweep angle.

### `uid`

```python
uid: str
```

Unique array id.

### `circular()`

```python
circular(
    uid: str,
    guide_circle_id: types.EntityID,
    members: Optional[Sequence[tuple[int, Sequence[types.EntityID]]]] = None,
    count: int = 6,
    total_angle_deg: float = 360.0,
    rotate_copies: bool = True,
) -> ArrayDefinition
```

| Parameter         | Type                                                              | Description |
| ----------------- | ----------------------------------------------------------------- | ----------- |
| `uid`             | `str`                                                             |             |
| `guide_circle_id` | `types.EntityID`                                                  |             |
| `members`         | `Optional[Sequence[tuple[int, Sequence[types.EntityID]]]] = None` |             |
| `count`           | `int = 6`                                                         |             |
| `total_angle_deg` | `float = 360.0`                                                   |             |
| `rotate_copies`   | `bool = True`                                                     |             |
| _Returns_         | `ArrayDefinition`                                                 |             |

### `copy()`

```python
copy() -> ArrayDefinition
```

Creates a copy of the definition.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `ArrayDefinition` |             |

### `curve_along()`

```python
curve_along(
    uid: str,
    guide_circle_id: types.EntityID,
    members: Optional[Sequence[tuple[int, Sequence[types.EntityID]]]] = None,
    count: int = 6,
    rotate_copies: bool = True,
    path_entity_id: int = -1,
    align_to_tangent: bool = True,
    offset_to_start: float = 0.0,
    spacing: float = 0.0,
    template_anchor: Optional[tuple[tuple[float, float], float]] = None,
) -> ArrayDefinition
```

| Parameter          | Type                                                              | Description |
| ------------------ | ----------------------------------------------------------------- | ----------- |
| `uid`              | `str`                                                             |             |
| `guide_circle_id`  | `types.EntityID`                                                  |             |
| `members`          | `Optional[Sequence[tuple[int, Sequence[types.EntityID]]]] = None` |             |
| `count`            | `int = 6`                                                         |             |
| `rotate_copies`    | `bool = True`                                                     |             |
| `path_entity_id`   | `int = -1`                                                        |             |
| `align_to_tangent` | `bool = True`                                                     |             |
| `offset_to_start`  | `float = 0.0`                                                     |             |
| `spacing`          | `float = 0.0`                                                     |             |
| `template_anchor`  | `Optional[tuple[tuple[float, float], float]] = None`              |             |
| _Returns_          | `ArrayDefinition`                                                 |             |

### `from_dict()`

```python
from_dict(dict: dict) -> ArrayDefinition
```

Deserializes a definition.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `dict`    | `dict`            |             |
| _Returns_ | `ArrayDefinition` |             |

### `restore()`

```python
restore(state: Any) -> None
```

Restores editable state from a snapshot.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `state`   | `Any`  |             |
| _Returns_ | `None` |             |

### `snapshot()`

```python
snapshot() -> Any
```

Captures editable state for undo.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `Any` |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the definition.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |
