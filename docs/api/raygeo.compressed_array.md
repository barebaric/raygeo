---
title: raygeo.compressed_array
sidebar_label: raygeo.compressed_array
---

## CompressedArray

A numpy-compatible array stored in compressed (zstd) form.

Created by raygeo's scene compiler and texture rasterizer to keep large numpy buffers compressed in
memory until they are needed for GPU upload. Call **to_numpy** to decompress on demand.

Arrays smaller than 4 KB are stored uncompressed to avoid overhead.

### `compressed_size`

```python
compressed_size: int
```

Size of the compressed (or raw) payload in bytes.

### `ratio`

```python
ratio: float
```

Compression ratio (compressed / uncompressed).

### `uncompressed_size`

```python
uncompressed_size: int
```

Size of the original uncompressed data in bytes.

### `from_float32()`

```python
from_float32(data: numpy.NDArray[numpy.float32]) -> CompressedArray
```

Create a CompressedArray from a 1-D float32 numpy array.

| Parameter | Type                           | Description |
| --------- | ------------------------------ | ----------- |
| `data`    | `numpy.NDArray[numpy.float32]` |             |
| _Returns_ | `CompressedArray`              |             |

### `from_float32_2d()`

```python
from_float32_2d(data: numpy.NDArray[numpy.float32]) -> CompressedArray
```

Create a CompressedArray from a 2-D float32 numpy array.

| Parameter | Type                           | Description |
| --------- | ------------------------------ | ----------- |
| `data`    | `numpy.NDArray[numpy.float32]` |             |
| _Returns_ | `CompressedArray`              |             |

### `from_int32()`

```python
from_int32(data: numpy.NDArray[numpy.int32]) -> CompressedArray
```

Create a CompressedArray from a 1-D int32 numpy array.

| Parameter | Type                         | Description |
| --------- | ---------------------------- | ----------- |
| `data`    | `numpy.NDArray[numpy.int32]` |             |
| _Returns_ | `CompressedArray`            |             |

### `from_uint8_2d()`

```python
from_uint8_2d(data: numpy.NDArray[numpy.uint8]) -> CompressedArray
```

Create a CompressedArray from a 2-D uint8 numpy array.

| Parameter | Type                         | Description |
| --------- | ---------------------------- | ----------- |
| `data`    | `numpy.NDArray[numpy.uint8]` |             |
| _Returns_ | `CompressedArray`            |             |

### `to_numpy()`

```python
to_numpy() -> Any
```

Decompress and return a numpy array with the original dtype and shape.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `Any` |             |
