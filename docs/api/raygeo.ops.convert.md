---
title: raygeo.ops.convert
sidebar_label: raygeo.ops.convert
---

## Encoder

Python-visible wrapper around an encoder spec.

Construct as `Encoder(spec)` where `spec` is an instance of one of the encoder spec classes under
`raygeo.ops.convert` (e.g. **~raygeo.ops.convert.GcodeSpec**). Callers that drive the `Encoder`
trait hold an `Encoder` instance.

### `spec`

```python
spec: Any
```

The wrapped Python-side spec object. Type-erased here; dispatched to a concrete `Box<dyn Encoder>`
by \[`PyEncoder::into_core`\].

## GcodeDialectSpec

Typed Python wrapper around the Rust `GcodeDialectSpec`.

Constructed in Python with keyword arguments; the inner Rust struct is passed directly to the
encoder without a serde round-trip.

## GcodeSpec

Parameters for the G-code encoder.

### `context_json`

```python
context_json: str
```

### `dialect`

```python
dialect: GcodeDialectSpec
```

## TextureSpec

Parameters for the texture encoder.

### `height_px`

```python
height_px: int
```

### `origin_mm`

```python
origin_mm: tuple[float, float]
```

### `px_per_mm`

```python
px_per_mm: tuple[float, float]
```

### `width_px`

```python
width_px: int
```

## VertexSpec

Parameters for the vertex-array encoder.
