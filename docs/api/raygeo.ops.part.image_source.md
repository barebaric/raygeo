---
title: raygeo.ops.part.image_source
sidebar_label: raygeo.ops.part.image_source
---

## WholeImageSource

In-memory `ImageSource` wrapping a 2-D uint8 raster buffer.

Constructed from a numpy array and read lazily by assemblers via the Rust-side `ImageSource` trait.
May be attached to a `Part` via `part.image_source = WholeImageSource(array)`; the `part.image`
property is a convenience shim that constructs a `WholeImageSource` on assignment.

### `dimensions`

```python
dimensions: tuple[int, int]
```

Pixel dimensions as `(width, height)`.

### `height`

```python
height: int
```

Pixel height.

### `width`

```python
width: int
```

Pixel width.

### `is_cancelled()`

```python
is_cancelled() -> bool
```

Cancellation probe. `WholeImageSource` is never cancelled.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `read_all()`

```python
read_all() -> Optional[list[int]]
```

Return the full image as flat row-major `bytes`, or `None` when the source cannot materialise a full
buffer.

`WholeImageSource` always returns `Some`.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| _Returns_ | `Optional[list[int]]` |             |

### `read_slab()`

```python
read_slab(y_start: int, y_end: int) -> list[int]
```

Pull a horizontal slab `[y_start, y_end)` and return it as a flat `bytes` object of length
`rows * width`.

| Parameter | Type        | Description                                                                                                          |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------- |
| `y_start` | `int`       | First row to read (inclusive).                                                                                       |
| `y_end`   | `int`       | Last row to read (exclusive); clipped to image height.                                                               |
| _Returns_ | `list[int]` | `bytes` of length `(y_end_clamped - y_start) * width`. Returns `b""` when `y_start >= height` or `y_end <= y_start`. |
