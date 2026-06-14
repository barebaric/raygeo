---
title: raygeo.svg
sidebar_label: raygeo.svg
sidebar_position: 36
---

![SVG path data parsed into geometries](images/svg-parsing.png)

_SVG path data parsed into geometries_

## Functions

### `geometry_to_svg_path()`

`geometry_to_svg_path(geometry: Geometry, width: int, height: int) -> str`

Convert a normalized Geometry to an SVG path d attribute string.

The geometry coordinates should be in normalized [0, 1] space. Coordinates are scaled to pixel
dimensions via width and height, with the Y axis flipped (SVG Y increases downward).

**Returns:** SVG path d attribute string.

| Parameter  | Type       | Description                                    |
| ---------- | ---------- | ---------------------------------------------- |
| `geometry` | `Geometry` | A Geometry object with normalized coordinates. |
| `width`    | `int`      | Target pixel width.                            |
| `height`   | `int`      | Target pixel height.                           |
| _Returns_  | `str`      |                                                |

### `parse_svg_path_data()`

`parse_svg_path_data(path_data: str, transform: numpy.NDArray[numpy.float64] | None = None, scale_x: float = 1, scale_y: float = 1) -> list[Geometry]`

Parse an SVG path d attribute into a list of Geometry objects.

Supports M/m, L/l, H/h, V/v, C/c, Z/z commands. Cubic Bezier curves are flattened to line segments
(20 steps).

**Returns:** List of Geometry objects, one per subpath.

| Parameter   | Type                                              | Description                                             |
| ----------- | ------------------------------------------------- | ------------------------------------------------------- |
| `path_data` | `str`                                             | SVG path d attribute string.                            |
| `transform` | `numpy.NDArray[numpy.float64] &#124; None = None` | 3x3 affine transformation matrix, or None for identity. |
| `scale_x`   | `float = 1`                                       | X-axis scale factor for coordinate transform.           |
| `scale_y`   | `float = 1`                                       | Y-axis scale factor for coordinate transform.           |
| _Returns_   | `list[Geometry]`                                  |                                                         |

### `parse_svg_transform()`

`parse_svg_transform(transform_str: str) -> numpy.NDArray[numpy.float64]`

Parse an SVG transform attribute string (translate only).

Returns a 3x3 identity matrix with translation applied.

**Returns:** 3x3 affine transformation matrix as numpy array.

| Parameter       | Type                           | Description                    |
| --------------- | ------------------------------ | ------------------------------ |
| `transform_str` | `str`                          | SVG transform attribute value. |
| _Returns_       | `numpy.NDArray[numpy.float64]` |                                |

### `svg_string_to_geometries()`

`svg_string_to_geometries(svg_str: str, scale_x: float = 1, scale_y: float = 1) -> list[Geometry]`

Parse an SVG string and extract all path elements as Geometry objects.

Recursively traverses the SVG XML tree, extracting d attributes from path elements and converting
them to Geometry.

**Returns:** List of Geometry objects from all path elements.

| Parameter | Type             | Description                                   |
| --------- | ---------------- | --------------------------------------------- |
| `svg_str` | `str`            | SVG document as a string.                     |
| `scale_x` | `float = 1`      | X-axis scale factor for coordinate transform. |
| `scale_y` | `float = 1`      | Y-axis scale factor for coordinate transform. |
| _Returns_ | `list[Geometry]` |                                               |
