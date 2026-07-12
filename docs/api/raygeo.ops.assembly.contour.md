---
title: raygeo.ops.assembly.contour
sidebar_label: raygeo.ops.assembly.contour
---

## Functions

### `contour()`

```python
contour(
    part: ops.part.Part,
    kerf_mm: float = 0,
    path_offset_mm: float = 0,
    cut_side: str = 'centerline',
    overcut: float = 0,
    cut_order: str = 'inside_outside',
    remove_inner: bool = False,
    arc_tolerance: float = 0,
    allow_arcs: bool = True,
    supports_curves: bool = False,
) -> ops.assembly.result.AssemblyResult
```

Trace contours from the part geometry.

Extracts the vector geometry from *part*, computes the total offset from kerf / path-offset /
cut-side, applies it with winding-order normalisation and offset fallback, orders inner/outer
contours, applies overcut, optionally fits arcs and curves, and returns the result as an
**AssemblyResult**.

**Raises:** `ValueError` — If the part has no geometry.

| Parameter         | Type                                 | Description                                                                    |
| ----------------- | ------------------------------------ | ------------------------------------------------------------------------------ |
| `part`            | `ops.part.Part`                      | The part whose geometry defines the contours.                                  |
| `kerf_mm`         | `float = 0`                          | Tool kerf width in mm (default 0.0).                                           |
| `path_offset_mm`  | `float = 0`                          | Additional offset distance in mm (default 0.0).                                |
| `cut_side`        | `str = 'centerline'`                 | `"centerline"`, `"outside"`, or `"inside"` (default `"centerline"`).           |
| `overcut`         | `float = 0`                          | Distance to extend closed contours past their start point (mm, default 0.0).   |
| `cut_order`       | `str = 'inside_outside'`             | `"inside_outside"` or `"outside_inside"` (default `"inside_outside"`).         |
| `remove_inner`    | `bool = False`                       | Remove inner (hole) contours (default False).                                  |
| `arc_tolerance`   | `float = 0`                          | Curve fitting tolerance in mm; when > 0 arcs/beziers are fitted (default 0.0). |
| `allow_arcs`      | `bool = True`                        | Fit arcs when arc_tolerance > 0 (default True).                                |
| `supports_curves` | `bool = False`                       | Keep Bézier curves when arc_tolerance > 0 (default False).                     |
| _Returns_         | `ops.assembly.result.AssemblyResult` | An **AssemblyResult** with the contour path.                                   |
