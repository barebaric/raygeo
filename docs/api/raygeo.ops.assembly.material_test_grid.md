---
title: raygeo.ops.assembly.material_test_grid
sidebar_label: raygeo.ops.assembly.material_test_grid
---

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
) -> ops.assembly.AssemblyResult
```

Generate a material test grid with varying speed and power.

Creates grid cells in rows x cols arrangement, each with baked-in power, speed, and pass count. When
*include_labels* is True (default), column headers, row labels, and axis titles are generated using
raygeo's built-in text-to-geometry (swash/fontdb).

| Parameter          | Type                          | Description                                                                           |
| ------------------ | ----------------------------- | ------------------------------------------------------------------------------------- |
| `size_mm`          | `tuple[float, float]`         | The (width, height) of the workpiece in mm.                                           |
| `cols`             | `int = 5`                     | Number of columns (default 5).                                                        |
| `rows`             | `int = 5`                     | Number of rows (default 5).                                                           |
| `min_speed`        | `float = 100`                 | Minimum speed in mm/min (default 100.0).                                              |
| `max_speed`        | `float = 500`                 | Maximum speed in mm/min (default 500.0).                                              |
| `min_power`        | `float = 10`                  | Minimum power in percent (default 10.0).                                              |
| `max_power`        | `float = 100`                 | Maximum power in percent (default 100.0).                                             |
| `min_passes`       | `int = 1`                     | Minimum number of passes (default 1).                                                 |
| `max_passes`       | `int = 5`                     | Maximum number of passes (default 5).                                                 |
| `fixed_speed`      | `float = 1000`                | Fixed speed for Power vs Passes mode (default 1000.0).                                |
| `fixed_power`      | `float = 50`                  | Fixed power for Speed vs Passes mode (default 50.0).                                  |
| `shape_size`       | `float = 10`                  | Size of each grid cell in mm (default 10.0).                                          |
| `spacing`          | `float = 2`                   | Spacing between cells in mm (default 2.0).                                            |
| `line_interval_mm` | `float = 0.1`                 | Line spacing for engrave mode (default 0.1).                                          |
| `mode`             | `str = 'engrave'`             | "engrave" or "cut" (default "engrave").                                               |
| `grid_mode`        | `str = 'Power vs Speed'`      | "Power vs Speed", "Power vs Passes", or "Speed vs Passes" (default "Power vs Speed"). |
| `include_labels`   | `bool = True`                 | Generate text labels (default True).                                                  |
| _Returns_          | `ops.assembly.AssemblyResult` | An **AssemblyResult** with grid cell paths and labels.                                |

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
) -> numpy.ndarray
```

Generate a raster preview of the material test grid.

Creates the same grid as **generate_material_test_grid** but renders it to an RGBA numpy array
instead of returning Ops.

| Parameter          | Type                     | Description                                                                           |
| ------------------ | ------------------------ | ------------------------------------------------------------------------------------- |
| `size_mm`          | `tuple[float, float]`    | The (width, height) of the workpiece in mm.                                           |
| `dpi`              | `float = 96`             | Output resolution in dots per inch (default 96.0).                                    |
| `cols`             | `int = 5`                | Number of columns (default 5).                                                        |
| `rows`             | `int = 5`                | Number of rows (default 5).                                                           |
| `min_speed`        | `float = 100`            | Minimum speed in mm/min (default 100.0).                                              |
| `max_speed`        | `float = 500`            | Maximum speed in mm/min (default 500.0).                                              |
| `min_power`        | `float = 10`             | Minimum power in percent (default 10.0).                                              |
| `max_power`        | `float = 100`            | Maximum power in percent (default 100.0).                                             |
| `min_passes`       | `int = 1`                | Minimum number of passes (default 1).                                                 |
| `max_passes`       | `int = 5`                | Maximum number of passes (default 5).                                                 |
| `fixed_speed`      | `float = 1000`           | Fixed speed for Power vs Passes mode (default 1000.0).                                |
| `fixed_power`      | `float = 50`             | Fixed power for Speed vs Passes mode (default 50.0).                                  |
| `shape_size`       | `float = 10`             | Size of each grid cell in mm (default 10.0).                                          |
| `spacing`          | `float = 2`              | Spacing between cells in mm (default 2.0).                                            |
| `line_interval_mm` | `float = 0.1`            | Line spacing for engrave mode (default 0.1).                                          |
| `mode`             | `str = 'engrave'`        | "engrave" or "cut" (default "engrave").                                               |
| `grid_mode`        | `str = 'Power vs Speed'` | "Power vs Speed", "Power vs Passes", or "Speed vs Passes" (default "Power vs Speed"). |
| `include_labels`   | `bool = True`            | Generate text labels (default True).                                                  |
| _Returns_          | `numpy.ndarray`          | A (H, W, 4) RGBA uint8 numpy array.                                                   |
