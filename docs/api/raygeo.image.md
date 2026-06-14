---
title: raygeo.image
sidebar_label: raygeo.image
sidebar_position: 22
---

Image processing functions for laser cutting applications.

Provides sRGB/linear color space conversions, RGBA-to-grayscale/binary conversions with alpha
unpremultiplication, grayscale normalization with auto-levels, and dithering algorithms
(Floyd-Steinberg, Bayer, minimum run length) for converting grayscale images to binary output.

## Functions

### `apply_bayer_dither()`

`apply_bayer_dither(grayscale: numpy.NDArray[numpy.uint8], bayer_matrix: numpy.NDArray[numpy.float32], invert: bool, cell_size: int = 1) -> numpy.NDArray[numpy.uint8]`

Apply ordered (Bayer) dithering using a threshold matrix.

**Returns:** 2D binary uint8 array (values 0 or 1).

| Parameter      | Type                           | Description                            |
| -------------- | ------------------------------ | -------------------------------------- |
| `grayscale`    | `numpy.NDArray[numpy.uint8]`   | 2D grayscale image as uint8 array.     |
| `bayer_matrix` | `numpy.NDArray[numpy.float32]` | 2D Bayer threshold matrix as float32.  |
| `invert`       | `bool`                         | If True, invert the output.            |
| `cell_size`    | `int = 1`                      | Pixel grouping size for the threshold. |
| _Returns_      | `numpy.NDArray[numpy.uint8]`   |                                        |

![Bayer 4x4 ordered dithering](images/image-processing-dither-bayer.png)

_Bayer 4x4 ordered dithering_

### `apply_floyd_steinberg_dither()`

`apply_floyd_steinberg_dither(grayscale: numpy.NDArray[numpy.uint8], invert: bool) -> numpy.NDArray[numpy.uint8]`

Apply Floyd-Steinberg error-diffusion dithering.

**Returns:** 2D binary uint8 array (values 0 or 1).

| Parameter   | Type                         | Description                                    |
| ----------- | ---------------------------- | ---------------------------------------------- |
| `grayscale` | `numpy.NDArray[numpy.uint8]` | 2D grayscale image as uint8 array.             |
| `invert`    | `bool`                       | If True, invert the output (swap black/white). |
| _Returns_   | `numpy.NDArray[numpy.uint8]` |                                                |

![Floyd-Steinberg dithering](images/image-processing-dither-floyd.png)

_Floyd-Steinberg dithering_

### `apply_minimum_run_length()`

`apply_minimum_run_length(binary: numpy.NDArray[numpy.uint8], min_run_length: int) -> numpy.NDArray[numpy.uint8]`

Remove binary runs shorter than the given minimum.

**Returns:** 2D binary uint8 array with short runs removed.

| Parameter        | Type                         | Description                            |
| ---------------- | ---------------------------- | -------------------------------------- |
| `binary`         | `numpy.NDArray[numpy.uint8]` | 2D binary uint8 array (values 0 or 1). |
| `min_run_length` | `int`                        | Minimum run length to keep.            |
| _Returns_        | `numpy.NDArray[numpy.uint8]` |                                        |

### `compute_auto_levels()`

`compute_auto_levels(gray_image: numpy.NDArray[numpy.uint8], clip_percent: float = 1) -> tuple[int, int]`

Compute auto black/white levels from a grayscale image histogram.

**Returns:** Tuple of (black_point, white_point).

| Parameter      | Type                         | Description                                 |
| -------------- | ---------------------------- | ------------------------------------------- |
| `gray_image`   | `numpy.NDArray[numpy.uint8]` | Grayscale image as uint8 array.             |
| `clip_percent` | `float = 1`                  | Percentage of pixels to clip from each end. |
| _Returns_      | `tuple[int, int]`            |                                             |

### `linear_to_srgb()`

`linear_to_srgb(array: numpy.NDArray[numpy.float32], dither: bool = False) -> numpy.NDArray[numpy.uint8]`

Convert linear light values to sRGB pixel values.

**Returns:** Array of sRGB uint8 values with the same shape.

| Parameter | Type                           | Description                                     |
| --------- | ------------------------------ | ----------------------------------------------- |
| `array`   | `numpy.NDArray[numpy.float32]` | Input array of linear float32 values in [0, 1]. |
| `dither`  | `bool = False`                 | Apply dithering to reduce banding artifacts.    |
| _Returns_ | `numpy.NDArray[numpy.uint8]`   |                                                 |

### `normalize_grayscale()`

`normalize_grayscale(gray_image: numpy.NDArray[numpy.uint8], black_point: int = 0, white_point: int = 255) -> numpy.NDArray[numpy.uint8]`

Normalize a grayscale image by stretching the dynamic range.

**Returns:** Normalized grayscale image with the same shape.

**Raises:** `ValueError` — If black_point >= white_point.

| Parameter     | Type                         | Description                           |
| ------------- | ---------------------------- | ------------------------------------- |
| `gray_image`  | `numpy.NDArray[numpy.uint8]` | Input grayscale image as uint8 array. |
| `black_point` | `int = 0`                    | Black point for normalization.        |
| `white_point` | `int = 255`                  | White point for normalization.        |
| _Returns_     | `numpy.NDArray[numpy.uint8]` |                                       |

### `rgba_to_binary()`

`rgba_to_binary(rgba: numpy.NDArray[numpy.uint8], width: int, height: int, stride: int, threshold: int = 128, invert: bool = False) -> numpy.NDArray[numpy.uint8]`

Convert raw BGRA pixel buffer to binary image using thresholding.

Transparent pixels (alpha == 0) are always treated as white (0).

**Returns:** 2D binary uint8 array (values 0 or 1) with shape (height, width).

| Parameter   | Type                         | Description                                             |
| ----------- | ---------------------------- | ------------------------------------------------------- |
| `rgba`      | `numpy.NDArray[numpy.uint8]` | Flattened uint8 buffer of shape (stride _ height _ 4,). |
| `width`     | `int`                        | Image width in pixels.                                  |
| `height`    | `int`                        | Image height in pixels.                                 |
| `stride`    | `int`                        | Row stride in pixels.                                   |
| `threshold` | `int = 128`                  | Brightness value (0-255) for binarization.              |
| `invert`    | `bool = False`               | If True, pixels above threshold become black (1).       |
| _Returns_   | `numpy.NDArray[numpy.uint8]` |                                                         |

### `rgba_to_grayscale()`

`rgba_to_grayscale(rgba: numpy.NDArray[numpy.uint8], width: int, height: int, stride: int) -> tuple[numpy.NDArray[numpy.uint8], numpy.NDArray[numpy.float32]]`

Convert raw BGRA pixel buffer to grayscale with alpha unpremultiplication.

Performs proper unpremultiplication of alpha and blends to white background for grayscale
calculation using BT.601 luminance weights.

**Returns:** Tuple of (grayscale_uint8, alpha_float32) arrays, each (height, width).

| Parameter | Type                                                              | Description                                             |
| --------- | ----------------------------------------------------------------- | ------------------------------------------------------- |
| `rgba`    | `numpy.NDArray[numpy.uint8]`                                      | Flattened uint8 buffer of shape (stride _ height _ 4,). |
| `width`   | `int`                                                             | Image width in pixels.                                  |
| `height`  | `int`                                                             | Image height in pixels.                                 |
| `stride`  | `int`                                                             | Row stride in pixels (may be larger than width).        |
| _Returns_ | `tuple[numpy.NDArray[numpy.uint8], numpy.NDArray[numpy.float32]]` |                                                         |

### `rgba_to_grayscale_inplace()`

`rgba_to_grayscale_inplace(rgba: numpy.NDArray[numpy.uint8], width: int, height: int, stride: int) -> None`

Convert raw BGRA pixel buffer to grayscale in place.

Modifies the buffer directly, converting BGR channels to grayscale while preserving the alpha
channel.

| Parameter | Type                         | Description                                             |
| --------- | ---------------------------- | ------------------------------------------------------- |
| `rgba`    | `numpy.NDArray[numpy.uint8]` | Flattened uint8 buffer of shape (stride _ height _ 4,). |
| `width`   | `int`                        | Image width in pixels.                                  |
| `height`  | `int`                        | Image height in pixels.                                 |
| `stride`  | `int`                        | Row stride in pixels.                                   |
| _Returns_ | `None`                       |                                                         |

### `srgb_to_linear()`

`srgb_to_linear(array: numpy.NDArray[numpy.uint8]) -> numpy.NDArray[numpy.float32]`

Convert sRGB pixel values to linear light values.

**Returns:** Array of linear float32 values with the same shape.

| Parameter | Type                           | Description                       |
| --------- | ------------------------------ | --------------------------------- |
| `array`   | `numpy.NDArray[numpy.uint8]`   | Input array of sRGB uint8 values. |
| _Returns_ | `numpy.NDArray[numpy.float32]` |                                   |

![sRGB to linear round-trip](images/image-processing-srgb.png)

_sRGB to linear round-trip_
