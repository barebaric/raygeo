---
title: raygeo.ops.assembly.contour
sidebar_label: raygeo.ops.assembly.contour
---

## ContourSpec

Parameters for the `contour` assembler.

Construct with
`ContourSpec(offset_mm, cut_side, overcut, cut_order, remove_inner, arc_tolerance, allow_arcs, supports_curves)`.
Wrap in an **~raygeo.ops.assembly.Assembler** instance to drive the `Assembler` trait.

### `allow_arcs`

```python
allow_arcs: bool
```

Fit arcs when arc_tolerance > 0.

### `arc_tolerance`

```python
arc_tolerance: float
```

Curve fitting tolerance in mm; when > 0 arcs/beziers are fitted.

### `cut_order`

```python
cut_order: str
```

`"inside_outside"` or `"outside_inside"`.

### `cut_side`

```python
cut_side: str
```

`"centerline"`, `"outside"`, or `"inside"`.

### `offset_mm`

```python
offset_mm: float
```

Total path offset distance in mm.

### `overcut`

```python
overcut: float
```

Distance to extend closed contours past their start point (mm).

### `remove_inner`

```python
remove_inner: bool
```

Remove inner (hole) contours.

### `supports_curves`

```python
supports_curves: bool
```

Keep Bézier curves when arc_tolerance > 0.

## Functions

### `contour()`

```python
contour(
    part: ops.part.Part,
    offset_mm: float = 0,
    cut_side: str = 'centerline',
    overcut: float = 0,
    cut_order: str = 'inside_outside',
    remove_inner: bool = False,
    arc_tolerance: float = 0,
    allow_arcs: bool = True,
    supports_curves: bool = False,
) -> ops.assembly.AssemblyResult
```

Trace contours from the part geometry.

Extracts the vector geometry from *part*, computes the total offset from offset / cut-side, applies
it with winding-order normalisation and offset fallback, orders inner/outer contours, applies
overcut, optionally fits arcs and curves, and returns the result as an **AssemblyResult**.

**Raises:** `ValueError` — If the part has no geometry.

| Parameter         | Type                          | Description                                                                    |
| ----------------- | ----------------------------- | ------------------------------------------------------------------------------ |
| `part`            | `ops.part.Part`               | The part whose geometry defines the contours.                                  |
| `offset_mm`       | `float = 0`                   | Total path offset distance in mm (default 0.0).                                |
| `cut_side`        | `str = 'centerline'`          | `"centerline"`, `"outside"`, or `"inside"` (default `"centerline"`).           |
| `overcut`         | `float = 0`                   | Distance to extend closed contours past their start point (mm, default 0.0).   |
| `cut_order`       | `str = 'inside_outside'`      | `"inside_outside"` or `"outside_inside"` (default `"inside_outside"`).         |
| `remove_inner`    | `bool = False`                | Remove inner (hole) contours (default False).                                  |
| `arc_tolerance`   | `float = 0`                   | Curve fitting tolerance in mm; when > 0 arcs/beziers are fitted (default 0.0). |
| `allow_arcs`      | `bool = True`                 | Fit arcs when arc_tolerance > 0 (default True).                                |
| `supports_curves` | `bool = False`                | Keep Bézier curves when arc_tolerance > 0 (default False).                     |
| _Returns_         | `ops.assembly.AssemblyResult` | An **AssemblyResult** with the contour path.                                   |
