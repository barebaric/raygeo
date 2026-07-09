---
title: raygeo.ops.assembly.result
sidebar_label: raygeo.ops.assembly.result
---

## AssemblyResult

Universal return type for every assembly-level generator.

Returned by assemblers such as `generate_helix`, `generate_toroidal_clear`, `generate_slot`, and all
other assembly-level motion functions. Contains the generated `Ops` sequence, the set of polygons
that this operation clears, and the tool pose at the start and end of the path.

### `cleared_polygons`

```python
cleared_polygons: list[list[tuple[float, float]]]
```

### `end`

```python
end: search.ToolPose
```

### `ops`

```python
ops: ops.Ops
```

### `start`

```python
start: search.ToolPose
```

### `trace`

```python
trace: Optional[Any]
```

### `write_trace()`

```python
write_trace(path: str, source: str, label: str) -> None
```

Write this result's AssemblyTrace bundle to a trace file.

Emits a root "workplan" span with one child assembler span containing either the self-traced events
or a minimal init/exit pair.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `path`    | `str`  |             |
| `source`  | `str`  |             |
| `label`   | `str`  |             |
| _Returns_ | `None` |             |
