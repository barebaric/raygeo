---
title: raygeo.dxf.reader
sidebar_label: raygeo.dxf.reader
---

Reading DXF documents.

The reader never fails on a single unreadable entity: anything it could not represent is reported in
`ReadOutput.issues`, so a file that is partly unsupported still imports the parts that are.

## DxfHeader

Details of a DXF document that are not geometry.

### `acad_version`

```python
acad_version: str
```

The drawing format marker from `$ACADVER`.

### `extents`

```python
extents: Optional[tuple[float, float, float, float]]
```

The declared drawing extents as `(x_min, y_min, x_max, y_max)`, or `None` when the file does not
state them.

### `insunits`

```python
insunits: int
```

The `$INSUNITS` value, or 0 when the file states no unit.

## ReadOutput

The geometry and metadata a DXF reader extracted from a document.

### `bounds`

```python
bounds: tuple[float, float, float, float]
```

Bounding rectangle of all drawn content, in native units, as `(x_min, y_min, x_max, y_max)`.

### `is_y_down`

```python
is_y_down: bool
```

Whether the native Y axis points down. Always `False` for DXF.

### `issues`

```python
issues: list[str]
```

Notes about content that was skipped or approximated.

### `layers`

```python
layers: list[tuple[str, geo.Geometry]]
```

One geometry per layer, as `(layer_name, geometry)` pairs in document order.

### `max_z`

```python
max_z: float
```

Highest Z value the drawing spans, in native units.

### `min_z`

```python
min_z: float
```

Lowest Z value the drawing spans, in native units.

### `units_to_mm`

```python
units_to_mm: float
```

Multiplier converting native units to millimetres.

### `has_z_variation()`

```python
has_z_variation(tolerance: float = 0.0001) -> bool
```

True when the drawing spans more than one Z level.

| Parameter   | Type             | Description                                                      |
| ----------- | ---------------- | ---------------------------------------------------------------- |
| `tolerance` | `float = 0.0001` | Smallest Z difference that counts as variation, in native units. |
| _Returns_   | `bool`           |                                                                  |

### `is_empty()`

```python
is_empty() -> bool
```

True when nothing drawable was found.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

## Functions

### `read()`

```python
read(data: str) -> dxf.reader.ReadOutput
```

Read a DXF document into geometry.

Supports lines, arcs, circles, ellipses, polylines with bulges, splines (including NURBS) and block
insertions, which are flattened into the drawing. Content the reader cannot represent is listed in
`issues` rather than raising.

**Raises:** `ValueError` — When the input is not a group-code stream.

| Parameter    | Type                    | Description                                                                                                   |
| ------------ | ----------------------- | ------------------------------------------------------------------------------------------------------------- |
| `data`       | `str`                   | The DXF file contents as text.                                                                                |
| _Returns_    | `dxf.reader.ReadOutput` | Per-layer geometry in native units, Y up, with the drawing bounds, the unit conversion factor and any issues. |
| _Complexity_ |                         | O(n) where n = size of the document                                                                           |
