---
title: raygeo.ops.assembly.shrinkwrap
sidebar_label: raygeo.ops.assembly.shrinkwrap
---

## Functions

### `shrinkwrap()`

```python
shrinkwrap(
    part: ops.part.Part,
    gravity: float = 0.1,
    kerf_mm: float = 0,
    path_offset_mm: float = 0,
    cut_side: str = 'centerline',
    arc_tolerance: float = 0,
    allow_arcs: bool = True,
    supports_curves: bool = False,
) -> ops.assembly.AssemblyResult
```

Generate a shrink-wrapped (concave hull) contour around image content.

Reads the pixel image from `part.image` (a 2-D uint8 numpy array), computes a concave hull using
Bézier gravity attraction, transforms pixel coordinates to millimetre space via the part's *size_mm*
and image dimensions, computes the total offset from kerf / path-offset / cut-side, applies it,
optionally fits arcs/curves when *arc_tolerance* > 0, and returns the result as an
**AssemblyResult**.

**Raises:** `ValueError` — If the image is empty, part has no size,

```
or `part.image` is None.
```

| Parameter         | Type                          | Description                                                                |
| ----------------- | ----------------------------- | -------------------------------------------------------------------------- |
| `part`            | `ops.part.Part`               | Part providing physical size metadata and the image buffer (`part.image`). |
| `gravity`         | `float = 0.1`                 | Shrink-wrap factor 0.0–1.0 (0 = convex hull, default 0.1).                 |
| `kerf_mm`         | `float = 0`                   | Tool kerf width in mm (default 0.0).                                       |
| `path_offset_mm`  | `float = 0`                   | Additional offset distance in mm (default 0.0).                            |
| `cut_side`        | `str = 'centerline'`          | `"centerline"`, `"outside"`, or `"inside"` (default `"centerline"`).       |
| `arc_tolerance`   | `float = 0`                   | Curve fitting tolerance in mm (default 0.0).                               |
| `allow_arcs`      | `bool = True`                 | Fit arcs when arc_tolerance > 0 (default True).                            |
| `supports_curves` | `bool = False`                | Keep Bézier curves when arc_tolerance > 0 (default False).                 |
| _Returns_         | `ops.assembly.AssemblyResult` | An **AssemblyResult** with the shrinkwrap path.                            |
