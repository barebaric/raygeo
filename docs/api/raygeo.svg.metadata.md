---
title: raygeo.svg.metadata
sidebar_label: raygeo.svg.metadata
---

SVG metadata extraction.

Extracts width, height, units and viewBox values from the root <svg> element of an SVG document.

## SvgMetadata

SVG document metadata extracted from an SVG string.

Provides width, height, units and viewBox values parsed from the root `<svg>` element.

### `height`

```python
height: Optional[float]
```

Document height as a numeric value (may be `None` if not set).

### `height_unit`

```python
height_unit: str
```

Unit string for the height attribute.

### `viewbox`

```python
viewbox: Optional[tuple[float, float, float, float]]
```

ViewBox as `(min_x, min_y, width, height)`, or `None`.

### `width`

```python
width: Optional[float]
```

Document width as a numeric value (may be `None` if not set).

### `width_unit`

```python
width_unit: str
```

Unit string for the width attribute (e.g. `"mm"`, `"in"`, `"px"`).

### `height_mm()`

```python
height_mm(dpi: float = 96.0) -> Optional[float]
```

Convert the document height to millimetres.

| Parameter    | Type              | Description                                              |
| ------------ | ----------------- | -------------------------------------------------------- |
| `dpi`        | `float = 96.0`    | Pixels-per-inch for px/unitless conversion (default 96). |
| _Returns_    | `Optional[float]` | Height in millimetres, or `None` if not set.             |
| _Complexity_ |                   | O(1)                                                     |

### `height_px()`

```python
height_px(dpi: float = 96.0) -> Optional[float]
```

Convert the document height to pixels.

| Parameter    | Type              | Description                                  |
| ------------ | ----------------- | -------------------------------------------- |
| `dpi`        | `float = 96.0`    | Pixels-per-inch for conversion (default 96). |
| _Returns_    | `Optional[float]` | Height in pixels, or `None` if not set.      |
| _Complexity_ |                   | O(1)                                         |

### `width_mm()`

```python
width_mm(dpi: float = 96.0) -> Optional[float]
```

Convert the document width to millimetres.

| Parameter    | Type              | Description                                              |
| ------------ | ----------------- | -------------------------------------------------------- |
| `dpi`        | `float = 96.0`    | Pixels-per-inch for px/unitless conversion (default 96). |
| _Returns_    | `Optional[float]` | Width in millimetres, or `None` if not set.              |
| _Complexity_ |                   | O(1)                                                     |

### `width_px()`

```python
width_px(dpi: float = 96.0) -> Optional[float]
```

Convert the document width to pixels.

| Parameter    | Type              | Description                                  |
| ------------ | ----------------- | -------------------------------------------- |
| `dpi`        | `float = 96.0`    | Pixels-per-inch for conversion (default 96). |
| _Returns_    | `Optional[float]` | Width in pixels, or `None` if not set.       |
| _Complexity_ |                   | O(1)                                         |

## Functions

### `extract_svg_metadata()`

```python
extract_svg_metadata(svg_str: str) -> SvgMetadata
```

Extract width, height, units and viewBox from an SVG string.

| Parameter    | Type          | Description                                                                               |
| ------------ | ------------- | ----------------------------------------------------------------------------------------- |
| `svg_str`    | `str`         | SVG document as a string.                                                                 |
| _Returns_    | `SvgMetadata` | SvgMetadata instance with width, height, width_unit, height_unit, and viewbox attributes. |
| _Complexity_ |               | O(n) where n = size of SVG document                                                       |
