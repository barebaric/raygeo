---
title: raygeo.dxf
sidebar_label: raygeo.dxf
---

![Lines, arcs, circles and bulges grouped by layer](images/dxf-shapes.png)

*Lines, arcs, circles and bulges grouped by layer*

![Geometry written as DXF and read back](images/dxf-write-roundtrip.png)

*Geometry written as DXF and read back*

![Arcs stay as ARC entities, so a round trip is lossless](images/dxf-arc-accuracy.png)

*Arcs stay as ARC entities, so a round trip is lossless* DXF (Drawing Exchange Format) reading and
writing.

Reads the ASCII DXF files CAD applications export for laser cutting: lines, arcs, circles, ellipses,
polylines with bulges, splines and the block insertions that 3D CAD tools wrap their export in.

Results are in native drawing units with Y pointing up. Multiply by `ReadOutput.units_to_mm` to
convert, which reads the `$INSUNITS` header value and falls back to millimetres for unitless files.

Sub-modules mirror the Rust layout: `raygeo.dxf.reader` reads, and `raygeo.dxf.writer` writes.
