---
title: raygeo.ops.assembly.frame
sidebar_label: raygeo.ops.assembly.frame
---

## Functions

### `frame()`

```python
frame(
    part: Part,
    kerf_mm: float = 0,
    path_offset_mm: float = 0,
    cut_side: str = 'centerline',
) -> ops.assembly.result.AssemblyResult
```

Generate a rectangular frame around the part boundary.

Creates a rectangle matching `part.size_mm`, computes the total offset from kerf / path-offset /
cut-side, applies it, and returns the frame as an **AssemblyResult**.

**Raises:** `ValueError` — If the part has no size information.

| Parameter        | Type                                 | Description                                                          |
| ---------------- | ------------------------------------ | -------------------------------------------------------------------- |
| `part`           | `Part`                               | The part whose size defines the frame.                               |
| `kerf_mm`        | `float = 0`                          | Tool kerf width in mm (default 0.0).                                 |
| `path_offset_mm` | `float = 0`                          | Additional offset distance in mm (default 0.0).                      |
| `cut_side`       | `str = 'centerline'`                 | `"centerline"`, `"outside"`, or `"inside"` (default `"centerline"`). |
| _Returns_        | `ops.assembly.result.AssemblyResult` | An **AssemblyResult** with the frame path.                           |
