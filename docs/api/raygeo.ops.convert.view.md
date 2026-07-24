---
title: raygeo.ops.convert.view
sidebar_label: raygeo.ops.convert.view
---

View rendering: rasterise Ops into pre-multiplied ARGB32 bitmaps on the rayon pool.

Provides both a free-function API (render_ops / render_ops_batch) and an Encoder spec class
(raygeo.ops.convert.ViewSpec) that plugs into the generic encoder pipeline.

## RenderResult

Result of **render_ops**.

Exposes the pre-multiplied ARGB32 bitmap as a `(H, W, 4)` `numpy.uint8` array (byte order B, G, R, A
on little‑endian, matching Cairo's `FORMAT_ARGB32`) plus the geometry bbox and effective
pixels-per-mm actually applied by the renderer.

### `bbox_mm`

```python
bbox_mm: tuple[float, float, float, float]
```

Geometry bbox in mm as `(min_x, min_y, max_x, max_y)`.

### `bitmap`

```python
bitmap: Any
```

`(H, W, 4)` ARGB32 bitmap (BGRA on little-endian).

### `effective_ppm`

```python
effective_ppm: tuple[float, float]
```

Effective pixels-per-mm applied by the renderer after clamping to `max_dimension_px` /
`max_total_pixels`.

## Functions

### `render_ops()`

```python
render_ops(ops: ops.Ops, spec: convert.ViewSpec) -> Optional[RenderResult]
```

Rasterise a single **raygeo.ops.Ops** to an ARGB32 bitmap.

| Parameter | Type                     | Description                                                                                                                   |
| --------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| `ops`     | `ops.Ops`                | The **~raygeo.ops.Ops** object to render.                                                                                     |
| `spec`    | `convert.ViewSpec`       | A **~raygeo.ops.convert.ViewSpec** with the rendering parameters (resolution, margins, colours, LUT).                         |
| _Returns_ | `Optional[RenderResult]` | A **RenderResult**, or `None` if there is no geometry to draw (empty Ops, all travel moves with `show_travel_moves = False`). |

### `render_ops_batch()`

```python
render_ops_batch(items: list) -> list[Optional[RenderResult]]
```

Rasterise a batch of **Ops** objects in parallel on the rayon pool.

| Parameter | Type                           | Description                                                                                                                                           |
| --------- | ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `items`   | `list`                         | List of `(ops, spec)` tuples where *spec* is a **~raygeo.ops.convert.ViewSpec**. Each item is rendered independently. See **render_ops** for details. |
| _Returns_ | `list[Optional[RenderResult]]` | A list parallel to *items*; each slot is either a **RenderResult** or `None` if that workpiece had no geometry to draw.                               |

### `render_ops_into()`

```python
render_ops_into(
    ops: ops.Ops,
    spec: convert.ViewSpec,
    bitmap: Any,
    view_bbox: tuple[float, float, float, float],
) -> bool
```

Render an **Ops** chunk directly into a caller-provided bitmap.

Unlike **render_ops**, this does not allocate a buffer — it writes into *bitmap*, a `(H, W, 4)`
`numpy.uint8` array. The *view_bbox* `(min_x, min_y, max_x, max_y)` defines the mm-space area the
bitmap covers; the effective ppm is derived from the bitmap dimensions and the bbox.

Texture (scanline) data is rendered first, then vertex strokes on top.

| Parameter   | Type                                | Description                                                                                                                                                                           |
| ----------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ops`       | `ops.Ops`                           | The **~raygeo.ops.Ops** chunk to render.                                                                                                                                              |
| `spec`      | `convert.ViewSpec`                  | A **~raygeo.ops.convert.ViewSpec** (only the colour/LUT/show_travel fields are used; `pixels_per_mm` and `render_bbox` are ignored — ppm is derived from the bitmap and *view_bbox*). |
| `bitmap`    | `Any`                               | A `(H, W, 4)` `numpy.uint8` array to render into.                                                                                                                                     |
| `view_bbox` | `tuple[float, float, float, float]` | The mm-space bbox `(min_x, min_y, max_x, max_y)` the bitmap covers.                                                                                                                   |
| _Returns_   | `bool`                              | `True` if any content was drawn, `False` if the bbox is degenerate.                                                                                                                   |
