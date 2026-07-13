---
title: raygeo.image.transparency
sidebar_label: raygeo.image.transparency
---

## Functions

### `make_transparent_by_brightness()`

```python
make_transparent_by_brightness(
    rgba: numpy.NDArray[numpy.uint8],
    width: int,
    height: int,
    stride: int,
    threshold: int = 250,
) -> None
```

Clear alpha for bright pixels in an ARGB32 buffer (in-place).

Pixels with BT.601-weighted brightness >= threshold have their alpha channel set to 0.

| Parameter    | Type                         | Description                                             |
| ------------ | ---------------------------- | ------------------------------------------------------- |
| `rgba`       | `numpy.NDArray[numpy.uint8]` | Flattened uint8 buffer of shape (stride * height * 4,). |
| `width`      | `int`                        | Image width in pixels.                                  |
| `height`     | `int`                        | Image height in pixels.                                 |
| `stride`     | `int`                        | Row stride in pixels (may be larger than width).        |
| `threshold`  | `int = 250`                  | Brightness threshold (0-255).                           |
| _Returns_    | `None`                       |                                                         |
| _Complexity_ |                              | O(w\*h)                                                 |

### `make_transparent_except_color()`

```python
make_transparent_except_color(
    rgba: numpy.NDArray[numpy.uint8],
    width: int,
    height: int,
    stride: int,
    target_r: int,
    target_g: int,
    target_b: int,
) -> None
```

Clear alpha for non-matching pixels in an ARGB32 buffer (in-place).

Pixels that do not match the target RGB color have their alpha channel set to 0.

| Parameter    | Type                         | Description                                             |
| ------------ | ---------------------------- | ------------------------------------------------------- |
| `rgba`       | `numpy.NDArray[numpy.uint8]` | Flattened uint8 buffer of shape (stride * height * 4,). |
| `width`      | `int`                        | Image width in pixels.                                  |
| `height`     | `int`                        | Image height in pixels.                                 |
| `stride`     | `int`                        | Row stride in pixels.                                   |
| `target_r`   | `int`                        | Target red channel value (0-255).                       |
| `target_g`   | `int`                        | Target green channel value (0-255).                     |
| `target_b`   | `int`                        | Target blue channel value (0-255).                      |
| _Returns_    | `None`                       |                                                         |
| _Complexity_ |                              | O(w\*h)                                                 |
