---
title: raygeo.image.ramp
sidebar_label: raygeo.image.ramp
---

## Functions

### `edge_ramp()`

```python
edge_ramp(
    binary: numpy.NDArray[numpy.uint8],
    ramp_length_mm: float,
    pixels_per_mm: tuple[float, float] = (1, 1),
) -> numpy.NDArray[numpy.float32]
```

Distance of each burned pixel to the edge of the burned area.

For every nonzero pixel of `binary`, returns its distance to the edge of the burned area divided by
`ramp_length_mm` and clamped to 1, so 0 is the edge and 1 is at or beyond the ramp length; zero
pixels get 0. Pixels outside the image count as not burned. A stroke narrower than twice the ramp
length never reaches 1.

The distance is Euclidean and in millimetres, so non-square pixels are handled. It runs from the
pixel centre to the centre of the nearest unburned pixel, less the part of that line inside the
unburned pixel: exact along the axes, and within half a pixel otherwise.

Used for sloped edges in relief and stamp engraving, where the power ramps up from the edge over the
ramp length.

**Raises:** `ValueError` — If the ramp length or resolution is invalid.

| Parameter        | Type                           | Description                                   |
| ---------------- | ------------------------------ | --------------------------------------------- |
| `binary`         | `numpy.NDArray[numpy.uint8]`   | 2D uint8 array, nonzero marks a burned pixel. |
| `ramp_length_mm` | `float`                        | Ramp length in mm; 0 returns 0/1 floats.      |
| `pixels_per_mm`  | `tuple[float, float] = (1, 1)` | (x, y) image resolution in pixels per mm.     |
| _Returns_        | `numpy.NDArray[numpy.float32]` | 2D float32 array of ramp fractions in [0, 1]. |
| _Complexity_     |                                | O(w\*h)                                       |
