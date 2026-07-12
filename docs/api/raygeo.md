---
title: raygeo
sidebar_label: raygeo
---

RayGeo — 2D/3D geometry engine for CAD/CAM applications.

## Layered architecture

The crate is split into two layers that depend only downward::

```
geo  →  ops      (never import upward)
```

`geo` — Pure geometry. Primitives & geometric algorithms: points, paths, offsets, medial axes,
clearing-state tracking, adaptive entry/wavefront generation. Knows nothing about machining, motion
commands, tools, or feed rates.

`ops` — Motion assembly. Turns geometric primitives into `Ops` command sequences. Linking,
classification (cut vs travel), lead-in/out, overscan, raster fill, peeling strategy. Holds the
generic `State` representation (feed_rate, rapid_rate, …) but does NOT decide what values to use —
those are passed in by the caller.

Key constraint: ops-layer assemblers always produce/consume `Ops` objects, never raw polygon or
polyline lists. Motion classification is encoded as `MoveTo` (travel) vs `LineTo` (cut) at the
command level.

## Core features

- Geometry types: points, lines, arcs, circles, beziers, polygons
- Path analysis: length, area, bounding box, containment, intersection
- Path manipulation: offset, clipping, fitting, simplification, smoothing
- Minkowski sums for toolpath generation
- Command sequence (Ops) for CNC motion control
- Serialization to/from industry formats

## Submodules

- raygeo.geo — Geometry and path/shape/algo operations
- raygeo.ops — Command sequence (Ops) manipulation and motion assembly

## Examples

```
Creating and inspecting geometry:

>>> from raygeo.geo import Geometry
>>> geom = Geometry()
>>> geom.add_rect(0, 0, 100, 50)
>>> geom.add_circle(50, 25, 10)
>>> geom.area()
5000.0 - 314.159...
>>> len(geom)
2

Manipulating command sequences:

>>> from raygeo.ops import Ops
>>> ops = Ops()
>>> ops.set_power(1.0)
>>> ops.move_to(0, 0, 0)
>>> ops.line_to(100, 0, 0)
>>> ops.distance()
100.0
```
