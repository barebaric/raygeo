---
title: raygeo.ops.assembly.raster
sidebar_label: raygeo.ops.assembly.raster
---

## Functions

### `raster()`

```python
raster(
    part: ops.part.Part,
    image: numpy.ndarray,
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
) -> ops.assembly.AssemblyResult
```

Rasterise a part image into scan paths.

Converts the grayscale or binary *image* into a sequence of scan-line toolpath commands suitable for
laser engraving or similar raster operations.

Three modes are supported:

- `"power_modulated"` *(default)* — uses grayscale + alpha channels to produce power-modulated scan
  lines.
- `"mask_scan"` — treats *image* as a binary mask and produces scan-line segments with constant
  power. Also used for `"dither"` — the caller pre-ditheres the image and passes it as a binary
  mask.
- `"multi_pass"` — decomposes the grayscale image into *num_depth_levels* layers, rasterising each
  at a progressive Z offset.

When *cross_hatch* is True the scan is run twice — once at *angle* and once at *angle* + 90° — and
the results are concatenated.

**Raises:** `ValueError` — If the mode is unknown or required data is

```
missing.
```

| Parameter            | Type                               | Description                                                                                            |
| -------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `part`               | `ops.part.Part`                    | Part providing pixel density and size metadata.                                                        |
| `image`              | `numpy.ndarray`                    | 2-D grayscale (uint8) or binary numpy array.                                                           |
| `alpha`              | `numpy.ndarray &#124; None = None` | Optional 2-D alpha mask (uint8). Required for `power_modulated` mode when the image is not pre-masked. |
| `mode`               | `str = 'power_modulated'`          | `"power_modulated"`, `"mask_scan"`, or `"multi_pass"`.                                                 |
| `line_interval_mm`   | `float = 0.1`                      | Spacing between scan lines in mm.                                                                      |
| `sample_interval_mm` | `float = 0.05`                     | Power sampling interval along a scan line in mm (power_modulated only).                                |
| `min_power`          | `float = 0`                        | Minimum laser power (0–1).                                                                             |
| `max_power`          | `float = 1`                        | Maximum laser power (0–1).                                                                             |
| `step_power`         | `float = 0.1`                      | Power step per level.                                                                                  |
| `num_power_levels`   | `int = 10`                         | Number of discrete power levels.                                                                       |
| `angle`              | `float = 0`                        | Scan angle in degrees.                                                                                 |
| `offset_x_mm`        | `float = 0`                        | Global X offset in mm.                                                                                 |
| `offset_y_mm`        | `float = 0`                        | Global Y offset in mm.                                                                                 |
| `scan_mode`          | `str = 'segmented'`                | `"segmented"` or `"full_sweep"`.                                                                       |
| `cross_hatch`        | `bool = False`                     | If True, add a second pass at angle + 90° (default False).                                             |
| `num_depth_levels`   | `int = 5`                          | Number of depth layers (multi_pass only, default 5).                                                   |
| `z_step_down`        | `float = 0`                        | Z decrement per depth layer in mm (multi_pass only, default 0.0).                                      |
| `angle_increment`    | `float = 0`                        | Angle added per depth layer in degrees (multi_pass only, default 0.0).                                 |
| _Returns_            | `ops.assembly.AssemblyResult`      | An **AssemblyResult** with the raster path.                                                            |
