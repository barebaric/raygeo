---
title: raygeo.ops.convert
sidebar_label: raygeo.ops.convert
---

## EncodeOutput

Non-Ops artifact produced by an Encode stage.

### `height_px`

```python
height_px: Optional[int]
```

Texture height in pixels. Returns `None` unless this is the `Texture` variant.

### `machine_code_to_op`

```python
machine_code_to_op: Optional[dict[int, int]]
```

Mapping `machine-code line index -> op_index`. Returns `None` unless this is the `MachineCode`
variant.

### `op_to_machine_code`

```python
op_to_machine_code: Optional[dict[int, list[int]]]
```

Mapping `op_index -> list of machine-code line indices`. Returns `None` unless this is the
`MachineCode` variant.

### `power_texture`

```python
power_texture: Optional[list[int]]
```

Raw texture bytes (row-major uint8 power map). Returns `None` unless this is the `Texture` variant.

### `repr`

```python
repr: Optional[str]
```

The vertex-array or scene debug repr. Returns `None` unless this is the `VertexArrays` or `Scene`
variant.

### `text`

```python
text: Optional[str]
```

The G-code text. Returns `None` unless this is the `MachineCode` variant.

### `variant`

```python
variant: str
```

The variant's name: `"MachineCode"`, `"VertexArrays"`, or `"Texture"`.

### `width_px`

```python
width_px: Optional[int]
```

Texture width in pixels. Returns `None` unless this is the `Texture` variant.

### `MachineCode()`

```python
@classmethod
MachineCode(
    text: str,
    op_to_machine_code: Mapping[int, Sequence[int]],
    machine_code_to_op: Mapping[int, int],
) -> EncodeOutput
```

| Parameter            | Type                          | Description |
| -------------------- | ----------------------------- | ----------- |
| `text`               | `str`                         |             |
| `op_to_machine_code` | `Mapping[int, Sequence[int]]` |             |
| `machine_code_to_op` | `Mapping[int, int]`           |             |
| _Returns_            | `EncodeOutput`                |             |

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

## LayerConfig

Per-layer rendering configuration for the scene encoder.

### `axis_position`

```python
axis_position: float
```

### `reverse`

```python
reverse: bool
```

### `rotary_diameter`

```python
rotary_diameter: float
```

### `rotary_enabled`

```python
rotary_enabled: bool
```

## PythonEncoder

Python-side constructor for \[`PythonEncoder`\].

Wraps a Python callable `(ops: Ops) -> EncodeOutput` so it can be driven through the Rust
`EncoderCompute` stage. The callable runs under the GIL on a rayon worker thread. Use this to route
encoders that remain in Python through the same pipeline as native Rust encoders.

### `callable`

```python
callable: Any
```

### `name`

```python
name: str
```

## SceneSpec

Parameters for the 3D scene encoder.

### `layer_configs`

```python
layer_configs: list[tuple[str, LayerConfig]]
```

Stored as a list of (uid, config) pairs for hash-free equality.

### `world_to_visual`

```python
world_to_visual: list[list[float]]
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

## ViewSpec

Parameters for the view encoder.

### `cut_color`

```python
cut_color: list[int]
```

### `cut_lut`

```python
cut_lut: list[list[int]]
```

### `engrave_lut`

```python
engrave_lut: list[list[int]]
```

### `max_dimension_px`

```python
max_dimension_px: int
```

### `max_total_pixels`

```python
max_total_pixels: int
```

### `pixels_per_mm`

```python
pixels_per_mm: tuple[float, float]
```

### `render_bbox`

```python
render_bbox: tuple[float, float, float, float]
```

### `show_travel_moves`

```python
show_travel_moves: bool
```

### `travel_color`

```python
travel_color: list[int]
```

### `zero_power_color`

```python
zero_power_color: list[int]
```
