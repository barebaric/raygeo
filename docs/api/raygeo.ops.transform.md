---
title: raygeo.ops.transform
sidebar_label: raygeo.ops.transform
---

## ExecutionPhase

Execution phase of a transformer.

Phases are applied in this order: `GEOMETRY_REFINEMENT` first, then `PATH_INTERRUPTION`, then
`POST_PROCESSING`.

## Functions

### `is_position_sensitive()`

```python
is_position_sensitive(ob: Any) -> bool
```

Check whether a transformer spec is position-sensitive.

Returns `True` if the transformer's output depends on absolute placement (e.g. `CropSpec`), `False`
otherwise. Unknown types return `False`.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `ob`      | `Any`  |             |
| _Returns_ | `bool` |             |
