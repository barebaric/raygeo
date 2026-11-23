---
title: raygeo.gcode
sidebar_label: raygeo.gcode
---

G-code (RS274/NGC) geometry extraction.

Parses the common subset of RS274/NGC used by laser cutters, CAM tools and LightBurn into `Geometry`
objects. Only material-removing moves are emitted: rapid moves, and feed moves at spindle speed
zero, split the result into separate contours.

## GcodeParseResult

The cutting geometry and skipped instructions of one G-code program.

### `geometry`

```python
geometry: geo.Geometry
```

Cutting moves in millimetres, Y-up, one subpath per contour.

### `unsupported`

```python
unsupported: list[str]
```

Instructions that were recognized but not applied, sorted.

## Functions

### `parse_gcode()`

```python
parse_gcode(text: str, s0_is_travel: bool = True) -> gcode.GcodeParseResult
```

Parse a G-code program into cutting geometry in millimetres.

Supports G0 (rapid), G1 (linear cut) and G2/G3 (arcs in I/J or R form), with G20/G21 units, G90/G91
distance mode and G90.1/G91.1 arc-centre mode. Comments, block delete, `%` delimiters and CRLF line
endings are handled. Feed, spindle speed, coolant and work offsets do not affect the geometry.

| Parameter      | Type                     | Description                                                                            |
| -------------- | ------------------------ | -------------------------------------------------------------------------------------- |
| `text`         | `str`                    | The G-code program as text.                                                            |
| `s0_is_travel` | `bool = True`            | Treat feed moves at spindle speed zero as travel (LightBurn fill passes and overscan). |
| _Returns_      | `gcode.GcodeParseResult` | Cutting geometry in millimetres plus the sorted set of instructions that were skipped. |
| _Complexity_   |                          | O(n) where n = size of the program                                                     |
