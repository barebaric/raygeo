---
title: raygeo
sidebar_label: raygeo
sidebar_position: 1
---

![Various geometry shapes and operations](images/geometry-playground.png)

_Various geometry shapes and operations_ RayGeo — 2D/3D geometry engine for laser cutting and CAM
applications.

Core features:

- Geometry types: points, lines, arcs, circles, beziers, polygons, rectangles
- Path analysis: length, area, bounding box, containment, intersection
- Path manipulation: offset, clipping, fitting, simplification, smoothing
- Minkowski sums for toolpath generation
- Command sequence (Ops) for laser cutter motion control
- Serialization to/from industry formats

Submodules:

- raygeo.geo — Geometry and path/shape/algo operations
- raygeo.ops — Command sequence (Ops) manipulation

Examples: Creating and inspecting geometry:

    >>> from raygeo.geo import Geometry
    >>> geom = Geometry()
    >>> geom.add_rect(0, 0, 100, 50)
    >>> geom.add_circle(50, 25, 10)
    >>> geom.area()
    5000.0 - 314.159...
    >>> len(geom)
    2

    Manipulating command sequences:

    >>> from raygeo.ops import Ops, Command
    >>> ops = Ops()
    >>> ops.set_speed(100)
    >>> ops.move_to(0, 0)
    >>> ops.line_to(100, 0)
    >>> ops.travel_distance()
    100.0

## Re-exports

This module re-exports from `raygeo.geo`: Geometry.

This module re-exports from `raygeo.ops`: Ops.
