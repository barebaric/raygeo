---
title: raygeo.ops.transform.merge_scanlines
sidebar_label: raygeo.ops.transform.merge_scanlines
---

## MergeScanlinesSpec

Parameters for the `MergeScanlines` transformer.

### `acceleration`

```python
acceleration: float
```

Machine acceleration in mm/s².

### `cut_speed`

```python
cut_speed: float
```

Fallback cut speed in mm/min.

### `max_gap_mm`

```python
max_gap_mm: float
```

Manual ceiling on bridged gap length in mm (0 = unlimited).

### `rapid_speed`

```python
rapid_speed: float
```

Fallback rapid speed in mm/min.

### `tolerance`

```python
tolerance: float
```

Maximum perpendicular distance for two parallel lines to be considered part of the same row, in mm.
