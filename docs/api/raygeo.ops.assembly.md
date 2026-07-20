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

### `cache_key()`

```python
cache_key(part: part.Part, tag: str) -> Optional[cache.CacheKey]
```

Compute a cache key for this assembler against the given part.

Returns `None` for assemblers that opt out of caching (e.g.
**~raygeo.ops.assembly.contour.ContourSpec**), or a **CacheKey** for assemblers that opt in (e.g.
**~raygeo.ops.assembly.adaptive.AdaptiveClearingSpec**).

| Parameter | Type                       | Description                                                                  |
| --------- | -------------------------- | ---------------------------------------------------------------------------- |
| `part`    | `part.Part`                | The part whose primary face state is hashed.                                 |
| `tag`     | `str`                      | Caller-provided identifier (used for prefix-based pruning of cache entries). |
| _Returns_ | `Optional[cache.CacheKey]` | A **CacheKey** or `None`.                                                    |

### `restore_cache()`

```python
restore_cache(cached: cache.AssemblyOutput) -> Optional[cache.AssemblyOutput]
```

Reconstruct a cached result from a **AssemblyOutput**.

Assemblers that opt out return `None` unconditionally. Assemblers that opt in (e.g. adaptive
clearing) return `Some` with the reconstructed value.

| Parameter | Type                             | Description                     |
| --------- | -------------------------------- | ------------------------------- |
| `cached`  | `cache.AssemblyOutput`           | The cached value to restore.    |
| _Returns_ | `Optional[cache.AssemblyOutput]` | A **AssemblyOutput** or `None`. |

### `store_cache()`

```python
store_cache(
    ops: ops.Ops,
    is_scalable: bool,
    source_dimensions: Optional[tuple[float, float]],
    part: part.Part,
) -> Optional[cache.AssemblyOutput]
```

Build a **AssemblyOutput** from the assembler's output.

Assemblers that opt out return `None` unconditionally. Assemblers that opt in return `Some` with the
output packaged for the cache.

| Parameter           | Type                             | Description                                          |
| ------------------- | -------------------------------- | ---------------------------------------------------- |
| `ops`               | `ops.Ops`                        | The assembled Ops.                                   |
| `is_scalable`       | `bool`                           | Whether the Ops may be uniformly scaled.             |
| `source_dimensions` | `Optional[tuple[float, float]]`  | Source `(width_mm, height_mm)`.                      |
| `part`              | `part.Part`                      | The part (face state is read for cleared fragments). |
| _Returns_           | `Optional[cache.AssemblyOutput]` | A **AssemblyOutput** or `None`.                      |

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
