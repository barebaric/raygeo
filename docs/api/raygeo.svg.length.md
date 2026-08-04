---
title: raygeo.svg.length
sidebar_label: raygeo.svg.length
---

SVG length parsing and unit conversion.

Parses SVG length strings such as '10mm' or '2.5in' and converts between millimetres, pixels and
other CSS units.

## Functions

### `parse_svg_length()`

```python
parse_svg_length(length_str: str) -> tuple[float, str]
```

Parse an SVG length string into a (value, unit) tuple.

Supports: mm, cm, in, pt, pc, px. Unitless values default to 'px'.

| Parameter    | Type                | Description                                      |
| ------------ | ------------------- | ------------------------------------------------ |
| `length_str` | `str`               | SVG length string (e.g. '10mm', '2.5in', '100'). |
| _Returns_    | `tuple[float, str]` | Tuple of (value, unit).                          |
| _Complexity_ |                     | O(1)                                             |

### `svg_length_to_mm()`

```python
svg_length_to_mm(length_str: str, dpi: float = 96) -> float
```

Parse an SVG length string and convert to millimetres.

| Parameter    | Type         | Description                                                   |
| ------------ | ------------ | ------------------------------------------------------------- |
| `length_str` | `str`        | SVG length string (e.g. '10mm', '2.5in', '100').              |
| `dpi`        | `float = 96` | Pixels per inch used for px/unitless conversion (default 96). |
| _Returns_    | `float`      | Length in millimetres.                                        |
| _Complexity_ |              | O(1)                                                          |

### `svg_length_to_px()`

```python
svg_length_to_px(length_str: str, dpi: float = 96) -> float
```

Parse an SVG length string and convert to pixels.

| Parameter    | Type         | Description                                                   |
| ------------ | ------------ | ------------------------------------------------------------- |
| `length_str` | `str`        | SVG length string (e.g. '10mm', '2.5in', '100').              |
| `dpi`        | `float = 96` | Pixels per inch used for px/unitless conversion (default 96). |
| _Returns_    | `float`      | Length in pixels.                                             |
| _Complexity_ |              | O(1)                                                          |
