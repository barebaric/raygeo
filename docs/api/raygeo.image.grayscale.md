---
title: raygeo.image.grayscale
sidebar_label: raygeo.image.grayscale
---

## Functions

### `compute_auto_levels()`

```python
compute_auto_levels(
    gray_image: numpy.NDArray[numpy.uint8],
    clip_percent: float = 1,
) -> tuple[int, int]
```

Compute auto black/white levels from a grayscale image histogram.

| Parameter      | Type                         | Description                                 |
| -------------- | ---------------------------- | ------------------------------------------- |
| `gray_image`   | `numpy.NDArray[numpy.uint8]` | Grayscale image as uint8 array.             |
| `clip_percent` | `float = 1`                  | Percentage of pixels to clip from each end. |
| _Returns_      | `tuple[int, int]`            | Tuple of (black_point, white_point).        |
| _Complexity_   |                              | O(n) where n = number of pixels             |

### `normalize_grayscale()`

```python
normalize_grayscale(
    gray_image: numpy.NDArray[numpy.uint8],
    black_point: int = 0,
    white_point: int = 255,
) -> numpy.NDArray[numpy.uint8]
```

Normalize a grayscale image by stretching the dynamic range.

**Raises:** `ValueError` — If black_point >= white_point.

| Parameter     | Type                         | Description                                     |
| ------------- | ---------------------------- | ----------------------------------------------- |
| `gray_image`  | `numpy.NDArray[numpy.uint8]` | Input grayscale image as uint8 array.           |
| `black_point` | `int = 0`                    | Black point for normalization.                  |
| `white_point` | `int = 255`                  | White point for normalization.                  |
| _Returns_     | `numpy.NDArray[numpy.uint8]` | Normalized grayscale image with the same shape. |
| _Complexity_  |                              | O(n) where n = number of pixels                 |
