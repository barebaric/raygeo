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

### `best_start_point`

```python
best_start_point: bool
```

When set, closed paths are rotated so they are entered at the vertex nearest the current head
position.

### `merge_scanlines`

```python
merge_scanlines: Optional[merge_scanlines.MergeScanlinesSpec]
```

When set, acceleration-aware scanline merging runs before the travel optimization.

### `prefer_corners`

```python
prefer_corners: bool
```

With *best_start_point*, restrict the candidate start vertices of a closed path to its corners.

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
