---
title: raygeo.dxf.writer
sidebar_label: raygeo.dxf.writer
---

Writing DXF documents.

Emits R12-compatible ASCII DXF with millimetres declared, the variant every CAD tool and laser
cutter controller accepts.

## Functions

### `write()`

```python
write(geometry: geo.Geometry) -> str
```

Write geometry as a DXF document.

Consecutive lines become one `LWPOLYLINE` per contour, arcs remain `ARC` entities, and beziers are
written as splines with control points and knots, so a geometry round-trips through `write()` and
`reader.read()` without loss.

| Parameter    | Type           | Description                            |
| ------------ | -------------- | -------------------------------------- |
| `geometry`   | `geo.Geometry` | The geometry to write, in millimetres. |
| _Returns_    | `str`          | The DXF document as text.              |
| _Complexity_ |                | O(n) where n = number of commands      |
