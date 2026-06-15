---
title: raygeo.svg
sidebar_label: raygeo.svg
sidebar_position: 36
---

![SVG path data parsed into geometries](images/svg-parsing.png)

_SVG path data parsed into geometries_

## SvgMetadata

### `height`

`height: Optional[float]`

### `height_unit`

`height_unit: str`

### `viewbox`

`viewbox: Optional[tuple[float, float, float, float]]`

### `width`

`width: Optional[float]`

### `width_unit`

`width_unit: str`

### `height_mm()`

`height_mm(dpi: float = 96.0) -> Optional[float]`

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `dpi`     | `float = 96.0`    |             |
| _Returns_ | `Optional[float]` |             |

### `height_px()`

`height_px(dpi: float = 96.0) -> Optional[float]`

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `dpi`     | `float = 96.0`    |             |
| _Returns_ | `Optional[float]` |             |

### `width_mm()`

`width_mm(dpi: float = 96.0) -> Optional[float]`

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `dpi`     | `float = 96.0`    |             |
| _Returns_ | `Optional[float]` |             |

### `width_px()`

`width_px(dpi: float = 96.0) -> Optional[float]`

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `dpi`     | `float = 96.0`    |             |
| _Returns_ | `Optional[float]` |             |

## Functions

### `extract_svg_metadata()`

`extract_svg_metadata(svg_str: str) -> SvgMetadata`

Extract width, height, units and viewBox from an SVG string.

**Returns:** SvgMetadata instance with width, height, width_unit, height_unit, and viewbox
attributes.

| Parameter    | Type          | Description                         |
| ------------ | ------------- | ----------------------------------- |
| `svg_str`    | `str`         | SVG document as a string.           |
| _Returns_    | `SvgMetadata` |                                     |
| _Complexity_ |               | O(n) where n = size of SVG document |

### `geometry_to_svg_path()`

`geometry_to_svg_path(geometry: Geometry, width: int, height: int) -> str`

Convert a normalized Geometry to an SVG path d attribute string.

The geometry coordinates should be in normalized [0, 1] space. Coordinates are scaled to pixel
dimensions via width and height, with the Y axis flipped (SVG Y increases downward).

**Returns:** SVG path d attribute string.

| Parameter    | Type       | Description                                    |
| ------------ | ---------- | ---------------------------------------------- |
| `geometry`   | `Geometry` | A Geometry object with normalized coordinates. |
| `width`      | `int`      | Target pixel width.                            |
| `height`     | `int`      | Target pixel height.                           |
| _Returns_    | `str`      |                                                |
| _Complexity_ |            | O(n) where n = number of commands              |

### `parse_svg_length()`

`parse_svg_length(length_str: str) -> tuple[float, str]`

Parse an SVG length string into a (value, unit) tuple.

Supports: mm, cm, in, pt, pc, px. Unitless values default to 'px'.

**Returns:** Tuple of (value, unit).

| Parameter    | Type                | Description                                      |
| ------------ | ------------------- | ------------------------------------------------ |
| `length_str` | `str`               | SVG length string (e.g. '10mm', '2.5in', '100'). |
| _Returns_    | `tuple[float, str]` |                                                  |
| _Complexity_ |                     | O(1)                                             |

### `parse_svg_path_data()`

`parse_svg_path_data(path_data: str, transform: numpy.NDArray[numpy.float64] | None = None, scale_x: float = 1, scale_y: float = 1) -> list[Geometry]`

Parse an SVG path d attribute into a list of Geometry objects.

Supports M/m, L/l, H/h, V/v, C/c, Z/z commands. Cubic Bezier curves are flattened to line segments
(20 steps).

**Returns:** List of Geometry objects, one per subpath.

| Parameter    | Type                                              | Description                                             |
| ------------ | ------------------------------------------------- | ------------------------------------------------------- |
| `path_data`  | `str`                                             | SVG path d attribute string.                            |
| `transform`  | `numpy.NDArray[numpy.float64] &#124; None = None` | 3x3 affine transformation matrix, or None for identity. |
| `scale_x`    | `float = 1`                                       | X-axis scale factor for coordinate transform.           |
| `scale_y`    | `float = 1`                                       | Y-axis scale factor for coordinate transform.           |
| _Returns_    | `list[Geometry]`                                  |                                                         |
| _Complexity_ |                                                   | O(n) where n = length of path data                      |

### `parse_svg_transform()`

`parse_svg_transform(transform_str: str) -> numpy.NDArray[numpy.float64]`

Parse an SVG transform attribute string (translate only).

Returns a 3x3 identity matrix with translation applied.

**Returns:** 3x3 affine transformation matrix as numpy array.

| Parameter       | Type                           | Description                    |
| --------------- | ------------------------------ | ------------------------------ |
| `transform_str` | `str`                          | SVG transform attribute value. |
| _Returns_       | `numpy.NDArray[numpy.float64]` |                                |
| _Complexity_    |                                | O(1)                           |

### `svg_length_to_mm()`

`svg_length_to_mm(length_str: str, dpi: float = 96) -> float`

Parse an SVG length string and convert to millimetres.

**Returns:** Length in millimetres.

| Parameter    | Type         | Description                                                   |
| ------------ | ------------ | ------------------------------------------------------------- |
| `length_str` | `str`        | SVG length string (e.g. '10mm', '2.5in', '100').              |
| `dpi`        | `float = 96` | Pixels per inch used for px/unitless conversion (default 96). |
| _Returns_    | `float`      |                                                               |
| _Complexity_ |              | O(1)                                                          |

### `svg_length_to_px()`

`svg_length_to_px(length_str: str, dpi: float = 96) -> float`

Parse an SVG length string and convert to pixels.

**Returns:** Length in pixels.

| Parameter    | Type         | Description                                                   |
| ------------ | ------------ | ------------------------------------------------------------- |
| `length_str` | `str`        | SVG length string (e.g. '10mm', '2.5in', '100').              |
| `dpi`        | `float = 96` | Pixels per inch used for px/unitless conversion (default 96). |
| _Returns_    | `float`      |                                                               |
| _Complexity_ |              | O(1)                                                          |

### `svg_string_to_geometries()`

`svg_string_to_geometries(svg_str: str, scale_x: float = 1, scale_y: float = 1) -> list[Geometry]`

Parse an SVG string and extract all path elements as Geometry objects.

Recursively traverses the SVG XML tree, extracting d attributes from path elements and converting
them to Geometry.

**Returns:** List of Geometry objects from all path elements.

| Parameter    | Type             | Description                                   |
| ------------ | ---------------- | --------------------------------------------- |
| `svg_str`    | `str`            | SVG document as a string.                     |
| `scale_x`    | `float = 1`      | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1`      | Y-axis scale factor for coordinate transform. |
| _Returns_    | `list[Geometry]` |                                               |
| _Complexity_ |                  | O(n) where n = size of SVG document           |

### `svg_string_to_geometries_by_layer()`

`svg_string_to_geometries_by_layer(svg_str: str, scale_x: float = 1, scale_y: float = 1) -> list[tuple[str, list[Geometry]]]`

Extract geometries grouped by top-level <g> layer.

Returns a list of (layer_id, geometries) tuples. Only top-level <g> elements with an id attribute
are treated as layers.

**Returns:** List of (layer_id, geometry_list) tuples.

| Parameter    | Type                               | Description                                   |
| ------------ | ---------------------------------- | --------------------------------------------- |
| `svg_str`    | `str`                              | SVG document as a string.                     |
| `scale_x`    | `float = 1`                        | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1`                        | Y-axis scale factor for coordinate transform. |
| _Returns_    | `list[tuple[str, list[Geometry]]]` |                                               |
| _Complexity_ |                                    | O(n) where n = size of SVG document           |

### `svg_string_to_geometry()`

`svg_string_to_geometry(svg_str: str, scale_x: float = 1, scale_y: float = 1) -> Geometry`

Parse an SVG string and merge all subpaths into a single Geometry.

Like svg_string_to_geometries but returns one combined Geometry instead of a list, avoiding a
Python-side merge loop.

**Returns:** A single Geometry containing all paths.

| Parameter    | Type        | Description                                   |
| ------------ | ----------- | --------------------------------------------- |
| `svg_str`    | `str`       | SVG document as a string.                     |
| `scale_x`    | `float = 1` | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1` | Y-axis scale factor for coordinate transform. |
| _Returns_    | `Geometry`  |                                               |
| _Complexity_ |             | O(n) where n = size of SVG document           |

### `svg_string_to_geometry_by_layer()`

`svg_string_to_geometry_by_layer(svg_str: str, scale_x: float = 1, scale_y: float = 1) -> list[tuple[str, Geometry]]`

Extract geometries grouped by layer, merged into one Geometry each.

Like svg_string_to_geometries_by_layer but merges each layer's subpaths into a single Geometry,
avoiding a Python merge loop.

**Returns:** List of (layer_id, merged_geometry) tuples.

| Parameter    | Type                         | Description                                   |
| ------------ | ---------------------------- | --------------------------------------------- |
| `svg_str`    | `str`                        | SVG document as a string.                     |
| `scale_x`    | `float = 1`                  | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1`                  | Y-axis scale factor for coordinate transform. |
| _Returns_    | `list[tuple[str, Geometry]]` |                                               |
| _Complexity_ |                              | O(n) where n = size of SVG document           |
