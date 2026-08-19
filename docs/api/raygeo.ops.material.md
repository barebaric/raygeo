---
title: raygeo.ops.material
sidebar_label: raygeo.ops.material
---

Material-effect folding: classifying what operations removed.

Assemblers emit MaterialEffects alongside their Ops — a unified description of the material they
remove, for CNC and laser alike. fold_effects() aggregates the effects of many operations against
one stock into an immutable MaterialState snapshot: through-cut voids, the burn surface map,
provenance, and escalation signals for geometry the current profiles cannot represent exactly.

## RasterEffect

A raster material effect: an R8 power map plus its grid placement.

### `cut_power_threshold`

```python
cut_power_threshold: Optional[int]
```

Raster power at or above which the material is cut through.

### `origin_mm`

```python
origin_mm: tuple[float, float]
```

World-mm origin of the grid's (0, 0) pixel corner.

### `power`

```python
power: compressed_array.CompressedArray
```

The power map as a compressed R8 array.

### `px_per_mm`

```python
px_per_mm: tuple[float, float]
```

Grid density in pixels per millimetre `(x, y)`.

## VectorEffect

A vector material effect: polygons removed over a Z interval.

Z values use the toolpath convention (stock surface at `z = 0`, bottom at `z = -thickness`):
`z_from=None` means open to the surface, `z_to=None` means through the bottom.

### `polygons`

```python
polygons: list[list[tuple[float, float]]]
```

Footprint polygons in workpiece-local mm.

### `z_from`

```python
z_from: Optional[float]
```

Top of the removed interval; `None` = open to the surface.

### `z_to`

```python
z_to: Optional[float]
```

Bottom of the removed interval; `None` = through the bottom.

## VolumeEffect

A volume material effect: closed solids to be removed.

No assembler emits these yet; the variant exists so future 3D assemblers join the same fold without
a wire-format change.

### `positions`

```python
positions: list[tuple[float, float, float]]
```

Vertex positions of the first solid (world mm).

### `triangles`

```python
triangles: list[tuple[int, int, int]]
```

Triangle indices of the first solid.
