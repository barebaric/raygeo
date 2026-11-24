---
title: raygeo.sketch.types
sidebar_label: raygeo.sketch.types
---

## Type Aliases

### EntityID

Type: `int`

## ConstraintStatus

The status of a sketch constraint.

The runtime object is an `IntEnum` built in \[`register_types`\]; this declaration exists so the
stub generator emits its members.

## WaypointType

The role a point plays where two sketch segments meet.

The constants are the only supported values. They compare equal to their serialized string form and
hash like it, so dictionaries keyed by strings keep working.

### `name`

```python
name: str
```

The constant name (`"SHARP"`, `"SMOOTH"`, `"SYMMETRIC"`).

### `value`

```python
value: str
```

The serialized form (`"sharp"`, `"smooth"`, `"symmetric"`).
