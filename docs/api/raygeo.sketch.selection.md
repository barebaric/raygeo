---
title: raygeo.sketch.selection
sidebar_label: raygeo.sketch.selection
---

## Selection

The current editor selection: point ids, entity ids, a constraint index and an optional junction
point.

### `constraint_idx`

```python
constraint_idx: Optional[int]
```

Selected constraint index.

### `entity_ids`

```python
entity_ids: list[types.EntityID]
```

Selected entity ids.

### `junction_pid`

```python
junction_pid: Optional[types.EntityID]
```

Selected implicit junction point.

### `point_ids`

```python
point_ids: list[types.EntityID]
```

Selected point ids.

### `add_entity()`

```python
add_entity(entity_id: types.EntityID) -> None
```

Adds one entity without clearing other selection kinds.

| Parameter   | Type             | Description |
| ----------- | ---------------- | ----------- |
| `entity_id` | `types.EntityID` |             |
| _Returns_   | `None`           |             |

### `add_point()`

```python
add_point(pid: types.EntityID) -> None
```

Adds one point without clearing other selection kinds.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `pid`     | `types.EntityID` |             |
| _Returns_ | `None`           |             |

### `clear()`

```python
clear() -> None
```

Clears every selected item.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `clear_entities()`

```python
clear_entities() -> None
```

Clears only the selected entities.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `clear_points()`

```python
clear_points() -> None
```

Clears only the selected points.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `copy()`

```python
copy() -> Selection
```

Creates a shallow copy of the selection.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `Selection` |             |

### `is_empty()`

```python
is_empty() -> bool
```

Whether nothing is selected.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `resolve_entities()`

```python
resolve_entities(
    registry: registry.EntityRegistry,
) -> Optional[list[entities.Entity]]
```

Resolves selected entity ids to entity objects.

| Parameter  | Type                              | Description |
| ---------- | --------------------------------- | ----------- |
| `registry` | `registry.EntityRegistry`         |             |
| _Returns_  | `Optional[list[entities.Entity]]` |             |

### `select_connected_entities()`

```python
select_connected_entities(
    entity_id: types.EntityID,
    registry: registry.EntityRegistry,
) -> None
```

Adds all entities connected through shared points.

| Parameter   | Type                      | Description |
| ----------- | ------------------------- | ----------- |
| `entity_id` | `types.EntityID`          |             |
| `registry`  | `registry.EntityRegistry` |             |
| _Returns_   | `None`                    |             |

### `select_constraint()`

```python
select_constraint(idx: int, is_multi: bool) -> None
```

Selects a constraint by index.

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `idx`      | `int`  |             |
| `is_multi` | `bool` |             |
| _Returns_  | `None` |             |

### `select_entity()`

```python
select_entity(entity_id: types.EntityID, is_multi: bool) -> None
```

Selects an entity by id.

| Parameter   | Type             | Description |
| ----------- | ---------------- | ----------- |
| `entity_id` | `types.EntityID` |             |
| `is_multi`  | `bool`           |             |
| _Returns_   | `None`           |             |

### `select_junction()`

```python
select_junction(pid: types.EntityID, is_multi: bool) -> None
```

Selects an implicit junction point.

| Parameter  | Type             | Description |
| ---------- | ---------------- | ----------- |
| `pid`      | `types.EntityID` |             |
| `is_multi` | `bool`           |             |
| _Returns_  | `None`           |             |

### `select_point()`

```python
select_point(pid: types.EntityID, is_multi: bool) -> None
```

Selects a point by id.

| Parameter  | Type             | Description |
| ---------- | ---------------- | ----------- |
| `pid`      | `types.EntityID` |             |
| `is_multi` | `bool`           |             |
| _Returns_  | `None`           |             |
