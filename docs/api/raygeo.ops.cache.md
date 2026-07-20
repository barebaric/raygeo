---
title: raygeo.ops.cache
sidebar_label: raygeo.ops.cache
---

Assembler-output caching types (CacheKey, AssemblyOutput).

Types used by the Cacheable trait: cache keys for identifying entries and the packaged assembler
output (AssemblyOutput).

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

## CacheKey

An assembler-computed cache key.

Pair of a caller-provided `tag` (used for prefix-based pruning) and an assembler-computed hash of
the spec plus face-state fields that the assembler actually reads.

Returned by **Assembler.cache_key() \<raygeo.ops.assembly.Assembler.cache_key>**. The consumer does
not interpret the `payload_hash` — it only `payload_hash` — it only compares it for equality.

### `payload_hash`

```python
payload_hash: int
```

Assembler-computed hash of its read-set fields.

### `tag`

```python
tag: str
```

Caller-provided identifier used for prefix-based pruning.
