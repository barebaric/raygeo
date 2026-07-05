---
title: raygeo.ops.assembly.result
sidebar_label: raygeo.ops.assembly.result
---

## AssemblyResult

Universal return type for every assembly-level generator.

Returned by `generate_helix`, `adaptive_entry`, and all other assemblers. Contains the generated
`Ops` sequence, the set of polygons that this operation clears, and the tool pose at the start and
end of the path.

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
