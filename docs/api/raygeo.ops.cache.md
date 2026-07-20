---
title: raygeo.ops.cache
sidebar_label: raygeo.ops.cache
---

Cache key type used by the Cacheable trait.

______________________________________________________________________

**CacheKey** is a caller-provided `tag` plus an assembler-computed hash of the spec and face-state
fields that the component actually reads. `AssemblyOutput` (the assembler's cached output) lives at
:py**raygeo.ops.assembly**.

## CacheKey

An assembler-computed cache key.

Pair of a caller-provided `tag` (used for prefix-based pruning) and an assembler-computed hash of
the spec plus face-state fields that the assembler actually reads.

Returned by **Assembler.cache_key() \<raygeo.ops.assembly.Assembler.cache_key>**. The consumer does
not interpret the `payload_hash` — it only compares it for equality.

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
