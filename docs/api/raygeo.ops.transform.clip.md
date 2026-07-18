---
title: raygeo.ops.transform.clip
sidebar_label: raygeo.ops.transform.clip
---

## CropSpec

Parameters for the `Crop` transformer.

The `regions` are pre-resolved clip regions in ops-local space, computed by Python from stock
geometries + workpiece transform.

### `offset`

```python
offset: float
```

Offset (mm) that was applied when growing the stock geometry; kept for traceability but not used by
the dispatch.

### `regions`

```python
regions: list[list[tuple[float, float]]]
```

Pre-resolved clip regions, each a polygon of `(x, y)` vertices.

### `tolerance`

```python
tolerance: float
```

Approximation tolerance for primitive refitting.
