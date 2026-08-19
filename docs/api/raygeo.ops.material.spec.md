---
title: raygeo.ops.material.spec
sidebar_label: raygeo.ops.material.spec
---

Fold input types: the stock, the entries, and the grid budget.

## FoldEntry

One compute node's contribution to a fold.

### `effects`

```python
effects: list[Any]
```

The effects this entry contributes.

### `placement`

```python
placement: geo.Matrix
```

Workpiece-local to world-mm placement of the effects.

### `source_key`

```python
source_key: str
```

Node key of the source (for provenance).

## GridBudget

Resolution budget for stock-grid outputs.

### `max_px`

```python
max_px: int
```

Per-side pixel cap; `px_per_mm` is scaled down to fit.

### `px_per_mm`

```python
px_per_mm: float
```

Requested grid density in pixels per millimetre.

## GridSpec

Grid placement of raster outputs in world mm.

### `origin_mm`

```python
origin_mm: tuple[float, float]
```

World-mm origin of the grid's (0, 0) pixel corner.

### `px_per_mm`

```python
px_per_mm: tuple[float, float]
```

Grid density in pixels per millimetre `(x, y)`.

### `size_px`

```python
size_px: tuple[int, int]
```

Grid size in pixels `(width, height)`.

## MaterialFoldSpec

Full input to **fold_effects**.

### `entries`

```python
entries: list[FoldEntry]
```

Effect-bearing entries, in any order.

### `grid`

```python
grid: GridBudget
```

Grid budget for raster outputs.

### `stock`

```python
stock: PrismaticStock
```

The stock to fold against.

## PrismaticStock

A prismatic stock: 2D outline extruded over a thickness.

Z convention: top surface at `z = 0`, bottom at `z = -thickness`.

### `polygons`

```python
polygons: list[list[tuple[float, float]]]
```

Stock outline polygons in world mm.

### `thickness`

```python
thickness: float
```

Stock thickness in mm.
