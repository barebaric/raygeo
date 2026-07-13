---
title: raygeo.image.convert
sidebar_label: raygeo.image.convert
---

## Functions

### `rgba_to_binary()`

```python
rgba_to_binary(
    rgba: numpy.NDArray[numpy.uint8],
    width: int,
    height: int,
    stride: int,
    threshold: int = 128,
    invert: bool = False,
) -> numpy.NDArray[numpy.uint8]
```

Convert raw BGRA pixel buffer to binary image using thresholding.

Transparent pixels (alpha == 0) are always treated as white (0).

| Parameter    | Type                         | Description                                                       |
| ------------ | ---------------------------- | ----------------------------------------------------------------- |
| `rgba`       | `numpy.NDArray[numpy.uint8]` | Flattened uint8 buffer of shape (stride * height * 4,).           |
| `width`      | `int`                        | Image width in pixels.                                            |
| `height`     | `int`                        | Image height in pixels.                                           |
| `stride`     | `int`                        | Row stride in pixels.                                             |
| `threshold`  | `int = 128`                  | Brightness value (0-255) for binarization.                        |
| `invert`     | `bool = False`               | If True, pixels above threshold become black (1).                 |
| _Returns_    | `numpy.NDArray[numpy.uint8]` | 2D binary uint8 array (values 0 or 1) with shape (height, width). |
| _Complexity_ |                              | O(w\*h)                                                           |

### `rgba_to_grayscale()`

```python
rgba_to_grayscale(
    rgba: numpy.NDArray[numpy.uint8],
    width: int,
    height: int,
    stride: int,
) -> tuple[numpy.NDArray[numpy.uint8], numpy.NDArray[numpy.float32]]
```

Convert raw BGRA pixel buffer to grayscale with alpha unpremultiplication.

Performs proper unpremultiplication of alpha and blends to white background for grayscale
calculation using BT.601 luminance weights.

| Parameter    | Type                                                              | Description                                                             |
| ------------ | ----------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `rgba`       | `numpy.NDArray[numpy.uint8]`                                      | Flattened uint8 buffer of shape (stride * height * 4,).                 |
| `width`      | `int`                                                             | Image width in pixels.                                                  |
| `height`     | `int`                                                             | Image height in pixels.                                                 |
| `stride`     | `int`                                                             | Row stride in pixels (may be larger than width).                        |
| _Returns_    | `tuple[numpy.NDArray[numpy.uint8], numpy.NDArray[numpy.float32]]` | Tuple of (grayscale_uint8, alpha_float32) arrays, each (height, width). |
| _Complexity_ |                                                                   | O(w\*h)                                                                 |

### `rgba_to_grayscale_inplace()`

```python
rgba_to_grayscale_inplace(
    rgba: numpy.NDArray[numpy.uint8],
    width: int,
    height: int,
    stride: int,
) -> None
```

Convert raw BGRA pixel buffer to grayscale in place.

Modifies the buffer directly, converting BGR channels to grayscale while preserving the alpha
channel.

| Parameter    | Type                         | Description                                             |
| ------------ | ---------------------------- | ------------------------------------------------------- |
| `rgba`       | `numpy.NDArray[numpy.uint8]` | Flattened uint8 buffer of shape (stride * height * 4,). |
| `width`      | `int`                        | Image width in pixels.                                  |
| `height`     | `int`                        | Image height in pixels.                                 |
| `stride`     | `int`                        | Row stride in pixels.                                   |
| _Returns_    | `None`                       |                                                         |
| _Complexity_ |                              | O(w\*h)                                                 |
