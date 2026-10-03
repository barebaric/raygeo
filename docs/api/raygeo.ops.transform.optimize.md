---
title: raygeo.ops.transform.optimize
sidebar_label: raygeo.ops.transform.optimize
---

## OptimizeSpec

Parameters for the `Optimize` (travel optimization) transformer.

### `allow_flip`

```python
allow_flip: bool
```

Whether flipping subpaths is allowed.

### `merge_scanlines`

```python
merge_scanlines: Optional[merge_scanlines.MergeScanlinesSpec]
```

When set, acceleration-aware scanline merging runs before the travel optimization.

### `preserve_first`

```python
preserve_first: bool
```

Keep the first workpiece in place.

### `preserve_order`

```python
preserve_order: list[str]
```

Workpiece UIDs whose order to preserve.
