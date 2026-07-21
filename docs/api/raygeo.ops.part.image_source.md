---
title: raygeo.ops.part.image_source
sidebar_label: raygeo.ops.part.image_source
---

## VipsChunkSource

Lazy `ImageSource` wrapping a `pyvips.Image` for bounded peak RSS.

Unlike \[`WholeImageSource`\], which eagerly copies the entire numpy array into Rust memory,
`VipsChunkSource` holds only a reference to the pyvips image and materialises horizontal slabs on
demand via `image.crop(0, y, w, h).write_to_memory()`.

For images below *in_memory_threshold_mb* (default 256 MB),
\[`read_all`\][PyVipsChunkSource::read_all] materialises the full buffer so callers that need random
access (raster, shrinkwrap) work unchanged. Above the threshold, `read_all` returns `None`, forcing
the caller to fall back to slab-by-slab reads.

The pyvips image **must** be single-band uchar. Convert before constructing:

```python
img = image.colourspace("b-w").cast("uchar")
src = VipsChunkSource(img)
```

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

Cancellation probe. `VipsChunkSource` is never cancelled by itself — the assembler polls its own
callbacks.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `read_all()`

```python
read_all() -> Optional[list[int]]
```

Return the full image as flat row-major `bytes`, or `None` when the source cannot materialise the
full buffer (image above the configured threshold).

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| _Returns_ | `Optional[list[int]]` |             |

### `read_slab()`

```python
read_slab(y_start: int, y_end: int) -> list[int]
```

Pull a horizontal slab `[y_start, y_end)` and return it as a flat `bytes` object of length
`rows * width`.

| Parameter | Type        | Description                                            |
| --------- | ----------- | ------------------------------------------------------ |
| `y_start` | `int`       | First row to read (inclusive).                         |
| `y_end`   | `int`       | Last row to read (exclusive); clipped to image height. |
| _Returns_ | `list[int]` | `bytes` of length `(y_end_clamped - y_start) * width`. |

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
