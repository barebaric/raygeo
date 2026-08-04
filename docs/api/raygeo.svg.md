---
title: raygeo.svg
sidebar_label: raygeo.svg
---

![SVG path data parsed into geometries](images/svg-parsing.png)

*SVG path data parsed into geometries* SVG parsing and geometry extraction.

Extracts Geometry objects from SVG documents — either as a flat list or grouped by layer or by color
— and provides parsers for path data and transforms, length handling, metadata extraction and path
export.

## Functions

### `filter_svg_by_color()`

```python
filter_svg_by_color(
    svg_str: str,
    color_key: str,
    color_attr: svg.color.ColorAttr = raygeo.svg.color.ColorAttr.ANY,
) -> str
```

Return a copy of the SVG containing only shapes of one color.

Non-matching shapes are removed, preserving the rest of the document (groups, defs, namespaces)
verbatim. Useful for rendering a color layer's base image.

| Parameter    | Type                                                   | Description                                                |
| ------------ | ------------------------------------------------------ | ---------------------------------------------------------- |
| `svg_str`    | `str`                                                  | SVG document as a string.                                  |
| `color_key`  | `str`                                                  | Color bucket key to keep (e.g. '#ff0000' or '\_no_color'). |
| `color_attr` | `svg.color.ColorAttr = raygeo.svg.color.ColorAttr.ANY` | Color attribute to bucket by.                              |
| _Returns_    | `str`                                                  | The filtered SVG document as a string.                     |
| _Complexity_ |                                                        | O(n) where n = size of SVG document                        |

### `geometry_to_svg_path()`

```python
geometry_to_svg_path(geometry: geo.Geometry, width: int, height: int) -> str
```

Convert a normalized Geometry to an SVG path d attribute string.

The geometry coordinates should be in normalized [0, 1] space. Coordinates are scaled to pixel
dimensions via width and height, with the Y axis flipped (SVG Y increases downward).

| Parameter    | Type           | Description                                    |
| ------------ | -------------- | ---------------------------------------------- |
| `geometry`   | `geo.Geometry` | A Geometry object with normalized coordinates. |
| `width`      | `int`          | Target pixel width.                            |
| `height`     | `int`          | Target pixel height.                           |
| _Returns_    | `str`          | SVG path d attribute string.                   |
| _Complexity_ |                | O(n) where n = number of commands              |

### `parse_svg_path_data()`

```python
parse_svg_path_data(
    path_data: str,
    transform: numpy.NDArray[numpy.float64] | None = None,
    scale_x: float = 1,
    scale_y: float = 1,
) -> list[geo.Geometry]
```

Parse an SVG path d attribute into a list of Geometry objects.

Supports M/m, L/l, H/h, V/v, C/c, Z/z commands. Cubic Bezier curves are flattened to line segments
(20 steps).

| Parameter    | Type                                              | Description                                             |
| ------------ | ------------------------------------------------- | ------------------------------------------------------- |
| `path_data`  | `str`                                             | SVG path d attribute string.                            |
| `transform`  | `numpy.NDArray[numpy.float64] &#124; None = None` | 3x3 affine transformation matrix, or None for identity. |
| `scale_x`    | `float = 1`                                       | X-axis scale factor for coordinate transform.           |
| `scale_y`    | `float = 1`                                       | Y-axis scale factor for coordinate transform.           |
| _Returns_    | `list[geo.Geometry]`                              | List of Geometry objects, one per subpath.              |
| _Complexity_ |                                                   | O(n) where n = length of path data                      |

### `svg_string_to_geometries()`

```python
svg_string_to_geometries(
    svg_str: str,
    scale_x: float = 1,
    scale_y: float = 1,
) -> list[geo.Geometry]
```

Parse an SVG string and extract all path elements as Geometry objects.

Recursively traverses the SVG XML tree, extracting d attributes from path elements and converting
them to Geometry.

| Parameter    | Type                 | Description                                      |
| ------------ | -------------------- | ------------------------------------------------ |
| `svg_str`    | `str`                | SVG document as a string.                        |
| `scale_x`    | `float = 1`          | X-axis scale factor for coordinate transform.    |
| `scale_y`    | `float = 1`          | Y-axis scale factor for coordinate transform.    |
| _Returns_    | `list[geo.Geometry]` | List of Geometry objects from all path elements. |
| _Complexity_ |                      | O(n) where n = size of SVG document              |

### `svg_string_to_geometries_by_color()`

```python
svg_string_to_geometries_by_color(
    svg_str: str,
    scale_x: float = 1,
    scale_y: float = 1,
    color_attr: svg.color.ColorAttr = raygeo.svg.color.ColorAttr.FILL,
) -> list[tuple[str, list[geo.Geometry]]]
```

Extract geometries grouped by color.

Walks the entire SVG tree and buckets shapes by their resolved fill/stroke color, applying SVG
inheritance for presentation attributes. Bucket keys are lowercase #rrggbb hex strings; shapes with
no usable color go into a '\_no_color' bucket.

| Parameter    | Type                                                    | Description                                   |
| ------------ | ------------------------------------------------------- | --------------------------------------------- |
| `svg_str`    | `str`                                                   | SVG document as a string.                     |
| `scale_x`    | `float = 1`                                             | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1`                                             | Y-axis scale factor for coordinate transform. |
| `color_attr` | `svg.color.ColorAttr = raygeo.svg.color.ColorAttr.FILL` | Color attribute to bucket by.                 |
| _Returns_    | `list[tuple[str, list[geo.Geometry]]]`                  | List of (color_key, geometry_list) tuples.    |
| _Complexity_ |                                                         | O(n) where n = size of SVG document           |

### `svg_string_to_geometries_by_layer()`

```python
svg_string_to_geometries_by_layer(
    svg_str: str,
    scale_x: float = 1,
    scale_y: float = 1,
) -> list[tuple[str, list[geo.Geometry]]]
```

Extract geometries grouped by top-level <g> layer.

Returns a list of (layer_id, geometries) tuples. Only top-level <g> elements with an id attribute
are treated as layers.

| Parameter    | Type                                   | Description                                   |
| ------------ | -------------------------------------- | --------------------------------------------- |
| `svg_str`    | `str`                                  | SVG document as a string.                     |
| `scale_x`    | `float = 1`                            | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1`                            | Y-axis scale factor for coordinate transform. |
| _Returns_    | `list[tuple[str, list[geo.Geometry]]]` | List of (layer_id, geometry_list) tuples.     |
| _Complexity_ |                                        | O(n) where n = size of SVG document           |

### `svg_string_to_geometry()`

```python
svg_string_to_geometry(
    svg_str: str,
    scale_x: float = 1,
    scale_y: float = 1,
) -> geo.Geometry
```

Parse an SVG string and merge all subpaths into a single Geometry.

Like svg_string_to_geometries but returns one combined Geometry instead of a list, avoiding a
Python-side merge loop.

| Parameter    | Type           | Description                                   |
| ------------ | -------------- | --------------------------------------------- |
| `svg_str`    | `str`          | SVG document as a string.                     |
| `scale_x`    | `float = 1`    | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1`    | Y-axis scale factor for coordinate transform. |
| _Returns_    | `geo.Geometry` | A single Geometry containing all paths.       |
| _Complexity_ |                | O(n) where n = size of SVG document           |

### `svg_string_to_geometry_by_color()`

```python
svg_string_to_geometry_by_color(
    svg_str: str,
    scale_x: float = 1,
    scale_y: float = 1,
    color_attr: svg.color.ColorAttr = raygeo.svg.color.ColorAttr.FILL,
) -> list[tuple[str, geo.Geometry]]
```

Extract geometries grouped by color, merged into one Geometry each.

Like svg_string_to_geometries_by_color but merges each color bucket's subpaths into a single
Geometry, avoiding a Python merge loop.

| Parameter    | Type                                                    | Description                                   |
| ------------ | ------------------------------------------------------- | --------------------------------------------- |
| `svg_str`    | `str`                                                   | SVG document as a string.                     |
| `scale_x`    | `float = 1`                                             | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1`                                             | Y-axis scale factor for coordinate transform. |
| `color_attr` | `svg.color.ColorAttr = raygeo.svg.color.ColorAttr.FILL` | Color attribute to bucket by.                 |
| _Returns_    | `list[tuple[str, geo.Geometry]]`                        | List of (color_key, merged_geometry) tuples.  |
| _Complexity_ |                                                         | O(n) where n = size of SVG document           |

### `svg_string_to_geometry_by_layer()`

```python
svg_string_to_geometry_by_layer(
    svg_str: str,
    scale_x: float = 1,
    scale_y: float = 1,
) -> list[tuple[str, geo.Geometry]]
```

Extract geometries grouped by layer, merged into one Geometry each.

Like svg_string_to_geometries_by_layer but merges each layer's subpaths into a single Geometry,
avoiding a Python merge loop.

| Parameter    | Type                             | Description                                   |
| ------------ | -------------------------------- | --------------------------------------------- |
| `svg_str`    | `str`                            | SVG document as a string.                     |
| `scale_x`    | `float = 1`                      | X-axis scale factor for coordinate transform. |
| `scale_y`    | `float = 1`                      | Y-axis scale factor for coordinate transform. |
| _Returns_    | `list[tuple[str, geo.Geometry]]` | List of (layer_id, merged_geometry) tuples.   |
| _Complexity_ |                                  | O(n) where n = size of SVG document           |
