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

## Part

Unified workpiece description for motion assembly.

Carries geometry and/or metadata needed by assemblers. No machine parameters or step configuration —
just the workpiece data.

Every assembler accepts a `Part` and internally extracts what it needs (boundary polygons, islands,
size, …).

### `geometry`

```python
geometry: Optional[geo.Geometry]
```

Vector geometry (the outline(s) of the part), if any.

Returns `None` if no geometry was provided at construction time.

### `pixels_per_mm`

```python
pixels_per_mm: Optional[tuple[float, float]]
```

Pixel density `(x, y)` in px/mm, if set.

### `size_mm`

```python
size_mm: tuple[float, float]
```

Physical size `(width, height)` in millimetres.

### `from_polygons()`

```python
from_polygons(
    boundary: Sequence[tuple[float, float]],
    islands: Optional[Sequence[Sequence[tuple[float, float]]]] = None,
    size_mm: tuple[float, float] = (0.0, 0.0),
) -> Part
```

Build a Part from a boundary polygon and optional islands.

| Parameter  | Type                                                       | Description                                                         |
| ---------- | ---------------------------------------------------------- | ------------------------------------------------------------------- |
| `boundary` | `Sequence[tuple[float, float]]`                            | Outer boundary as `[(x, y), ...]`.                                  |
| `islands`  | `Optional[Sequence[Sequence[tuple[float, float]]]] = None` | List of island polygons, each `[(x, y), ...]` (default `[]`).       |
| `size_mm`  | `tuple[float, float] = (0.0, 0.0)`                         | Physical size `(width, height)` in mm (default `(0, 0)`).           |
| _Returns_  | `Part`                                                     | A new `Part` with the geometry constructed from the given polygons. |

### `has_geometry()`

```python
has_geometry() -> bool
```

True if this Part has geometry.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |
