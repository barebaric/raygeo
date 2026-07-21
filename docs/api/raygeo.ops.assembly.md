---
title: raygeo.ops.assembly
sidebar_label: raygeo.ops.assembly
---

Motion-path assembly: turning raw geometry primitives into Ops.

Functions in this module compose geo-layer primitives (polylines, arcs, polygons) into complete
motion sequences represented as Ops objects. They decide traversal order, linking strategy,
lead-in/out, overscan, and tab insertion — concerns that belong to motion assembly rather than pure
geometry.

Each assembler exposes a spec class (e.g. **~raygeo.ops.assembly.contour.ContourSpec**) implementing
the Rust `Assembler` trait; **Assembler** wraps any spec so callers can drive it through the trait.

## Assembler

Python-visible wrapper around an assembler spec.

Construct as `Assembler(spec)` where `spec` is an instance of one of the assembler spec classes
under `raygeo.ops.assembly.*` (e.g. **~raygeo.ops.assembly.contour.ContourSpec**). Callers that
drive the `Assembler` trait hold an `Assembler` instance.

### `spec`

```python
spec: Any
```

The wrapped Python-side spec object. Type-erased here; dispatched to a concrete `Box<dyn Assembler>`
by \[`PyAssembler::into_core`\].

## AssemblyOutput

The output of an assembler, packaged for caching.

Produced by **Assembler.store_cache() \<raygeo.ops.assembly.Assembler.store_cache>** and consumed by
**Assembler.restore_cache() \<raygeo.ops.assembly.Assembler.restore_cache>**.

Carries the assembled `Ops`, metadata, and optional post-assembly cleared fragments for face-state
restoration on cache hit.

### `cleared_fragments`

```python
cleared_fragments: Optional[list[list[tuple[float, float]]]]
```

Post-assembly cleared fragments (`list[list[(x, y)]]`), or `None` for assemblers that don't touch
`FaceState.cleared`.

### `is_scalable`

```python
is_scalable: bool
```

Whether the Ops may be uniformly scaled during aggregation.

### `ops`

```python
ops: ops.Ops
```

The assembled Ops.

### `source_dimensions`

```python
source_dimensions: Optional[tuple[float, float]]
```

Source `(width_mm, height_mm)` of the part that produced the Ops.

## AssemblyResult

Universal return type for every assembly-level generator.

Returned by assemblers such as `generate_helix`, `generate_toroidal_clear`, `generate_slot`, and all
other assembly-level motion functions. Contains the generated `Ops` sequence, the set of polygons
that this operation clears, and the tool pose at the start and end of the path.

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

### `from_ops()`

```python
from_ops(
    ops: ops.Ops,
    start: tuple[float, float, float],
    end: tuple[float, float, float],
) -> AssemblyResult
```

Construct an AssemblyResult from ops, start, and end poses.

| Parameter | Type                         | Description |
| --------- | ---------------------------- | ----------- |
| `ops`     | `ops.Ops`                    |             |
| `start`   | `tuple[float, float, float]` |             |
| `end`     | `tuple[float, float, float]` |             |
| _Returns_ | `AssemblyResult`             |             |

### `write_trace()`

```python
write_trace(path: str, source: str, label: str) -> None
```

Write this result's trace events to a trace file.

Emits a root "workplan" span with one child assembler span containing either the self-traced events
or a minimal init/exit pair.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `path`    | `str`  |             |
| `source`  | `str`  |             |
| `label`   | `str`  |             |
| _Returns_ | `None` |             |
