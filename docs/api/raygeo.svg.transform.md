---
title: raygeo.svg.transform
sidebar_label: raygeo.svg.transform
---

SVG transform attribute parsing.

Parses a transform attribute value (translate, scale, rotate, skewX, skewY, matrix, or a
space-separated combination) into a 3x3 matrix.

## Functions

### `parse_svg_transform()`

```python
parse_svg_transform(transform_str: str) -> numpy.NDArray[numpy.float64]
```

Parse an SVG transform attribute string (translate only).

Returns a 3x3 identity matrix with translation applied.

| Parameter       | Type                           | Description                                      |
| --------------- | ------------------------------ | ------------------------------------------------ |
| `transform_str` | `str`                          | SVG transform attribute value.                   |
| _Returns_       | `numpy.NDArray[numpy.float64]` | 3x3 affine transformation matrix as numpy array. |
| _Complexity_    |                                | O(1)                                             |
