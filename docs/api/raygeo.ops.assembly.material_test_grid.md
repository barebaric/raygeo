---
title: raygeo.ops.assembly.material_test_grid
sidebar_label: raygeo.ops.assembly.material_test_grid
---

## MaterialTestGridSpec

Parameters for the material-test-grid assembler.

### `cols`

```python
cols: int
```

### `fixed_power`

```python
fixed_power: float
```

### `fixed_speed`

```python
fixed_speed: float
```

### `grid_mode`

```python
grid_mode: str
```

`"Power vs Speed"`, `"Power vs Passes"`, `"Speed vs Passes"`, or `"Speed vs Offset"`.

### `include_labels`

```python
include_labels: bool
```

### `label_power_percent`

```python
label_power_percent: float
```

Power for label engraving in percent (0–100).

### `label_speed`

```python
label_speed: float
```

Feed rate for label engraving in mm/min.

### `line_interval_mm`

```python
line_interval_mm: float
```

### `max_offset`

```python
max_offset: float
```

### `max_passes`

```python
max_passes: int
```

### `max_power`

```python
max_power: float
```

### `max_speed`

```python
max_speed: float
```

### `min_offset`

```python
min_offset: float
```

### `min_passes`

```python
min_passes: int
```

### `min_power`

```python
min_power: float
```

### `min_speed`

```python
min_speed: float
```

### `mode`

```python
mode: str
```

`"engrave"` or `"cut"`.

### `rows`

```python
rows: int
```

### `shape_size`

```python
shape_size: float
```

### `size_mm`

```python
size_mm: tuple[float, float]
```

`(width_mm, height_mm)` of the workpiece area to fill.

### `spacing`

```python
spacing: float
```

## Functions

### `generate_material_test_grid()`

```python
generate_material_test_grid(
    size_mm: tuple[float, float],
    cols: int = 5,
    rows: int = 5,
    min_speed: float = 100,
    max_speed: float = 500,
    min_power: float = 10,
    max_power: float = 100,
    min_passes: int = 1,
    max_passes: int = 5,
    fixed_speed: float = 1000,
    fixed_power: float = 50,
    shape_size: float = 10,
    spacing: float = 2,
    line_interval_mm: float = 0.1,
    mode: str = 'engrave',
    grid_mode: str = 'Power vs Speed',
    include_labels: bool = True,
    label_power_percent: float = 10,
    label_speed: float = 1000,
    min_offset: float = -0.5,
    max_offset: float = 0.5,
) -> ops.assembly.AssemblyResult
```

Generate a material test grid with varying speed and power.

Creates grid cells in rows x cols arrangement, each with baked-in power, speed, and pass count. When
*include_labels* is True (default), column headers, row labels, and axis titles are generated using
raygeo's built-in text-to-geometry (swash/fontdb).

| Parameter             | Type                          | Description                                                                                              |
| --------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------- |
| `size_mm`             | `tuple[float, float]`         | The (width, height) of the workpiece in mm.                                                              |
| `cols`                | `int = 5`                     | Number of columns (default 5).                                                                           |
| `rows`                | `int = 5`                     | Number of rows (default 5).                                                                              |
| `min_speed`           | `float = 100`                 | Minimum speed in mm/min (default 100.0).                                                                 |
| `max_speed`           | `float = 500`                 | Maximum speed in mm/min (default 500.0).                                                                 |
| `min_power`           | `float = 10`                  | Minimum power in percent (default 10.0).                                                                 |
| `max_power`           | `float = 100`                 | Maximum power in percent (default 100.0).                                                                |
| `min_passes`          | `int = 1`                     | Minimum number of passes (default 1).                                                                    |
| `max_passes`          | `int = 5`                     | Maximum number of passes (default 5).                                                                    |
| `fixed_speed`         | `float = 1000`                | Fixed speed for Power vs Passes mode (default 1000.0).                                                   |
| `fixed_power`         | `float = 50`                  | Fixed power for Speed vs Passes mode (default 50.0).                                                     |
| `shape_size`          | `float = 10`                  | Size of each grid cell in mm (default 10.0).                                                             |
| `spacing`             | `float = 2`                   | Spacing between cells in mm (default 2.0).                                                               |
| `line_interval_mm`    | `float = 0.1`                 | Line spacing for engrave mode (default 0.1).                                                             |
| `mode`                | `str = 'engrave'`             | "engrave" or "cut" (default "engrave").                                                                  |
| `grid_mode`           | `str = 'Power vs Speed'`      | "Power vs Speed", "Power vs Passes", "Speed vs Passes", or "Speed vs Offset" (default "Power vs Speed"). |
| `include_labels`      | `bool = True`                 | Generate text labels (default True).                                                                     |
| `label_power_percent` | `float = 10`                  | Power for label engraving in percent (default 10.0).                                                     |
| `label_speed`         | `float = 1000`                | Feed rate for label engraving in mm/min (default 1000.0).                                                |
| `min_offset`          | `float = -0.5`                | Minimum bidirectional scan offset in mm for Speed vs Offset mode (default -0.5).                         |
| `max_offset`          | `float = 0.5`                 | Maximum bidirectional scan offset in mm for Speed vs Offset mode (default 0.5).                          |
| _Returns_             | `ops.assembly.AssemblyResult` | An **AssemblyResult** with grid cell paths and labels.                                                   |

### `generate_material_test_grid_preview()`

```python
generate_material_test_grid_preview(
    size_mm: tuple[float, float],
    dpi: float = 96,
    cols: int = 5,
    rows: int = 5,
    min_speed: float = 100,
    max_speed: float = 500,
    min_power: float = 10,
    max_power: float = 100,
    min_passes: int = 1,
    max_passes: int = 5,
    fixed_speed: float = 1000,
    fixed_power: float = 50,
    shape_size: float = 10,
    spacing: float = 2,
    line_interval_mm: float = 0.1,
    mode: str = 'engrave',
    grid_mode: str = 'Power vs Speed',
    include_labels: bool = True,
    label_power_percent: float = 10,
    label_speed: float = 1000,
    min_offset: float = -0.5,
    max_offset: float = 0.5,
) -> numpy.ndarray
```

Generate a raster preview of the material test grid.

Creates the same grid as **generate_material_test_grid** but renders it to an RGBA numpy array
instead of returning Ops.

| Parameter             | Type                     | Description                                                                                              |
| --------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------- |
| `size_mm`             | `tuple[float, float]`    | The (width, height) of the workpiece in mm.                                                              |
| `dpi`                 | `float = 96`             | Output resolution in dots per inch (default 96.0).                                                       |
| `cols`                | `int = 5`                | Number of columns (default 5).                                                                           |
| `rows`                | `int = 5`                | Number of rows (default 5).                                                                              |
| `min_speed`           | `float = 100`            | Minimum speed in mm/min (default 100.0).                                                                 |
| `max_speed`           | `float = 500`            | Maximum speed in mm/min (default 500.0).                                                                 |
| `min_power`           | `float = 10`             | Minimum power in percent (default 10.0).                                                                 |
| `max_power`           | `float = 100`            | Maximum power in percent (default 100.0).                                                                |
| `min_passes`          | `int = 1`                | Minimum number of passes (default 1).                                                                    |
| `max_passes`          | `int = 5`                | Maximum number of passes (default 5).                                                                    |
| `fixed_speed`         | `float = 1000`           | Fixed speed for Power vs Passes mode (default 1000.0).                                                   |
| `fixed_power`         | `float = 50`             | Fixed power for Speed vs Passes mode (default 50.0).                                                     |
| `shape_size`          | `float = 10`             | Size of each grid cell in mm (default 10.0).                                                             |
| `spacing`             | `float = 2`              | Spacing between cells in mm (default 2.0).                                                               |
| `line_interval_mm`    | `float = 0.1`            | Line spacing for engrave mode (default 0.1).                                                             |
| `mode`                | `str = 'engrave'`        | "engrave" or "cut" (default "engrave").                                                                  |
| `grid_mode`           | `str = 'Power vs Speed'` | "Power vs Speed", "Power vs Passes", "Speed vs Passes", or "Speed vs Offset" (default "Power vs Speed"). |
| `include_labels`      | `bool = True`            | Generate text labels (default True).                                                                     |
| `label_power_percent` | `float = 10`             | Power for label engraving in percent (default 10.0).                                                     |
| `label_speed`         | `float = 1000`           | Feed rate for label engraving in mm/min (default 1000.0).                                                |
| `min_offset`          | `float = -0.5`           | Minimum bidirectional scan offset in mm for Speed vs Offset mode (default -0.5).                         |
| `max_offset`          | `float = 0.5`            | Maximum bidirectional scan offset in mm for Speed vs Offset mode (default 0.5).                          |
| _Returns_             | `numpy.ndarray`          | A (H, W, 4) RGBA uint8 numpy array.                                                                      |
