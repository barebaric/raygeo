---
title: raygeo.ops.assembly.raster
sidebar_label: raygeo.ops.assembly.raster
---

## RasterSpec

Parameters for the `raster` assembler.

Construct with `RasterSpec(mode, line_interval_mm, ...)`. The optional `alpha` buffer is set via the
`alpha` keyword. Wrap in an **~raygeo.ops.assembly.Assembler** instance to drive the `Assembler`
trait.

### `angle`

```python
angle: float
```

### `angle_increment`

```python
angle_increment: float
```

### `cross_hatch`

```python
cross_hatch: bool
```

### `dot_width_correction_mm`

```python
dot_width_correction_mm: float
```

Compensates for the physical width of the laser spot by delaying laser-on and advancing laser-off by
this distance at each end of every continuous engraved run.

### `line_interval_mm`

```python
line_interval_mm: float
```

### `max_power`

```python
max_power: float
```

### `min_power`

```python
min_power: float
```

### `mode`

```python
mode: str
```

### `num_depth_levels`

```python
num_depth_levels: int
```

### `num_power_levels`

```python
num_power_levels: int
```

### `offset_x_mm`

```python
offset_x_mm: float
```

### `offset_y_mm`

```python
offset_y_mm: float
```

### `sample_interval_mm`

```python
sample_interval_mm: float
```

### `scan_mode`

```python
scan_mode: str
```

### `step_power`

```python
step_power: float
```

### `z_step_down`

```python
z_step_down: float
```

## Functions

### `raster()`

```python
raster(
    part: ops.part.Part,
    alpha: numpy.ndarray | None = None,
    mode: str = 'power_modulated',
    line_interval_mm: float = 0.1,
    sample_interval_mm: float = 0.05,
    min_power: float = 0,
    max_power: float = 1,
    step_power: float = 0.1,
    num_power_levels: int = 10,
    angle: float = 0,
    offset_x_mm: float = 0,
    offset_y_mm: float = 0,
    scan_mode: str = 'segmented',
    cross_hatch: bool = False,
    num_depth_levels: int = 5,
    z_step_down: float = 0,
    angle_increment: float = 0,
    dot_width_correction_mm: float = 0,
) -> ops.assembly.AssemblyResult
```

Rasterise a part image into scan paths.

Reads the pixel image from `part.image` (a 2-D uint8 numpy array) and converts it into scan-line
toolpath commands.

Three modes are supported:

- `"power_modulated"` *(default)* — uses grayscale + alpha channels to produce power-modulated scan
  lines.
- `"mask_scan"` — treats the image as a binary mask and produces scan-line segments with constant
  power. Also used for `"dither"` — the caller pre-ditheres the image and stores it on `part.image`
  as a binary mask.
- `"multi_pass"` — decomposes the grayscale image into *num_depth_levels* layers, rasterising each
  at a progressive Z offset.

When *cross_hatch* is True the scan is run twice — once at *angle* and once at *angle* + 90° — and
the results are concatenated.

**Raises:** `ValueError` — If the mode is unknown, required data is

```
missing, or `part.image` is None.
```

| Parameter                 | Type                               | Description                                                                                                                                                                                            |
| ------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `part`                    | `ops.part.Part`                    | Part providing pixel density, size metadata, and the image buffer (`part.image`).                                                                                                                      |
| `alpha`                   | `numpy.ndarray &#124; None = None` | Optional 2-D alpha mask (uint8). Required for `power_modulated` mode when the image is not pre-masked.                                                                                                 |
| `mode`                    | `str = 'power_modulated'`          | `"power_modulated"`, `"mask_scan"`, or `"multi_pass"`.                                                                                                                                                 |
| `line_interval_mm`        | `float = 0.1`                      | Spacing between scan lines in mm.                                                                                                                                                                      |
| `sample_interval_mm`      | `float = 0.05`                     | Power sampling interval along a scan line in mm (power_modulated only).                                                                                                                                |
| `min_power`               | `float = 0`                        | Minimum laser power (0–1).                                                                                                                                                                             |
| `max_power`               | `float = 1`                        | Maximum laser power (0–1).                                                                                                                                                                             |
| `step_power`              | `float = 0.1`                      | Power step per level.                                                                                                                                                                                  |
| `num_power_levels`        | `int = 10`                         | Number of discrete power levels.                                                                                                                                                                       |
| `angle`                   | `float = 0`                        | Scan angle in degrees.                                                                                                                                                                                 |
| `offset_x_mm`             | `float = 0`                        | Global X offset in mm.                                                                                                                                                                                 |
| `offset_y_mm`             | `float = 0`                        | Global Y offset in mm.                                                                                                                                                                                 |
| `scan_mode`               | `str = 'segmented'`                | `"segmented"` or `"full_sweep"`.                                                                                                                                                                       |
| `cross_hatch`             | `bool = False`                     | If True, add a second pass at angle + 90° (default False).                                                                                                                                             |
| `num_depth_levels`        | `int = 5`                          | Number of depth layers (multi_pass only, default 5).                                                                                                                                                   |
| `z_step_down`             | `float = 0`                        | Z decrement per depth layer in mm (multi_pass only, default 0.0).                                                                                                                                      |
| `angle_increment`         | `float = 0`                        | Angle added per depth layer in degrees (multi_pass only, default 0.0).                                                                                                                                 |
| `dot_width_correction_mm` | `float = 0`                        | Shortens laser firing by this distance at each end of every engraved run, to compensate for the physical width of the laser spot. Geometry is unaffected. `power_modulated`/`mask_scan`/`dither` only. |
| _Returns_                 | `ops.assembly.AssemblyResult`      | An **AssemblyResult** with the raster path.                                                                                                                                                            |
