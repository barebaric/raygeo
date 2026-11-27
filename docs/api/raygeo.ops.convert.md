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
machine_code_to_op: Optional[bytearray]
```

Mapping `machine-code line index -> op_index` (`-1` = no op). Returns `None` unless this is the
`MachineCode` variant.

Returned as a **bytearray** of `i32` values (4 bytes per line) to avoid the per-element Python int
overhead of a list. Decode with `np.frombuffer(ba, dtype=np.int32)`.

### `op_to_machine_code`

```python
op_to_machine_code: Optional[bytearray]
```

Mapping `op_index -> (start_line, line_count)` span. Returns `None` unless this is the `MachineCode`
variant.

Returned as a **bytearray** of interleaved `i32` pairs `(start, count)` (8 bytes per op) to avoid
the per-element Python tuple/int overhead of a list-of-tuples. Decode with
`np.frombuffer(ba, dtype=np.int32).reshape(-1, 2)`.

### `payload`

```python
payload: Optional[bytearray]
```

The device byte stream for binary machine languages (empty for text-only languages such as G-code).
Returns `None` unless this is the `MachineCode` variant.

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
    op_to_machine_code: Any,
    machine_code_to_op: Any,
    payload: Optional[Any] = None,
) -> EncodeOutput
```

| Parameter            | Type                   | Description |
| -------------------- | ---------------------- | ----------- |
| `text`               | `str`                  |             |
| `op_to_machine_code` | `Any`                  |             |
| `machine_code_to_op` | `Any`                  |             |
| `payload`            | `Optional[Any] = None` |             |
| _Returns_            | `EncodeOutput`         |             |

## Encoder

Python-visible wrapper around an encoder spec.

Construct as `Encoder(spec)` where `spec` is an encoder spec: one of the classes under
`raygeo.ops.convert` (e.g. **~raygeo.ops.convert.VertexSpec**), a machine-code spec (`GcodeSpec`,
`EpilogSpec`, `RuidaSpec`). Callers that drive the `Encoder` trait hold an `Encoder` instance.

### `spec`

```python
spec: Any
```

The wrapped Python-side spec object. Type-erased here; dispatched to a concrete `Box<dyn Encoder>`
by \[`PyEncoder::into_core`\].

## EpilogSpec

Parameters for the Epilog print-job encoder.

`context_json` is a JSON-serialised `EpilogContext`: model family, job title, resolution, frequency,
autofocus, bed size, speed reference, origin layout and the rotary flag.

### `context_json`

```python
context_json: str
```

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

## GlueScript

The Ruida job authoring and compilation model.

Call the authoring methods to build a job, then `stage_gluescript()` compiles it into rpascript.
`stage_gluescript(lines)` also replays a persisted transcript.

### `gluescript`

```python
gluescript: list[str]
```

The canonical transcript lines.

### `job_complete`

```python
job_complete: bool
```

Whether `end_job()` finalized the job.

### `rpascript`

```python
rpascript: list[str]
```

The compiled rpascript lines.

### `air_assist_off()`

```python
air_assist_off() -> None
```

Disable air assist.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `air_assist_on()`

```python
air_assist_on() -> None
```

Enable air assist.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `comment()`

```python
comment(comments: Sequence[str]) -> None
```

Append comment lines to the job header.

| Parameter  | Type            | Description |
| ---------- | --------------- | ----------- |
| `comments` | `Sequence[str]` |             |
| _Returns_  | `None`          |             |

### `cut_speed()`

```python
cut_speed(speed: float) -> None
```

Save the cut speed for the next cut.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `speed`   | `float` |             |
| _Returns_ | `None`  |             |

### `cut_x_to()`

```python
cut_x_to(x: float) -> None
```

Cut to an absolute X position.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `x`       | `float` |             |
| _Returns_ | `None`  |             |

### `cut_xy_to()`

```python
cut_xy_to(x: float, y: float) -> None
```

Cut to an absolute XY position.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `x`       | `float` |             |
| `y`       | `float` |             |
| _Returns_ | `None`  |             |

### `cut_y_to()`

```python
cut_y_to(y: float) -> None
```

Cut to an absolute Y position.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `y`       | `float` |             |
| _Returns_ | `None`  |             |

### `declare_job()`

```python
declare_job(
    label: str,
    ref_point: str = 'MACHINE',
    abs_xy: Optional[Sequence[float]] = None,
    columns: int = 1,
    rows: int = 1,
    xstep: float = 0.0,
    ystep: float = 0.0,
) -> None
```

Declare a job.

| Parameter   | Type                               | Description |
| ----------- | ---------------------------------- | ----------- |
| `label`     | `str`                              |             |
| `ref_point` | `str = 'MACHINE'`                  |             |
| `abs_xy`    | `Optional[Sequence[float]] = None` |             |
| `columns`   | `int = 1`                          |             |
| `rows`      | `int = 1`                          |             |
| `xstep`     | `float = 0.0`                      |             |
| `ystep`     | `float = 0.0`                      |             |
| _Returns_   | `None`                             |             |

### `declare_layer()`

```python
declare_layer(
    label: str,
    color: str,
    mode: str = 'VECTOR',
    overscan: str = 'NONE',
    speed: float = 100.0,
    frequency: float = 20.0,
    min_power_1: float = 8.0,
    max_power_1: float = 70.0,
) -> None
```

Declare a layer.

| Parameter     | Type             | Description |
| ------------- | ---------------- | ----------- |
| `label`       | `str`            |             |
| `color`       | `str`            |             |
| `mode`        | `str = 'VECTOR'` |             |
| `overscan`    | `str = 'NONE'`   |             |
| `speed`       | `float = 100.0`  |             |
| `frequency`   | `float = 20.0`   |             |
| `min_power_1` | `float = 8.0`    |             |
| `max_power_1` | `float = 70.0`   |             |
| _Returns_     | `None`           |             |

### `end_job()`

```python
end_job() -> None
```

Finalize the job.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `frequency()`

```python
frequency(frequency: float) -> None
```

Save the layer frequency for the next flush.

| Parameter   | Type    | Description |
| ----------- | ------- | ----------- |
| `frequency` | `float` |             |
| _Returns_   | `None`  |             |

### `move_speed()`

```python
move_speed(speed: float) -> None
```

Record a move speed (comment-only).

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `speed`   | `float` |             |
| _Returns_ | `None`  |             |

### `move_x_to()`

```python
move_x_to(x: float) -> None
```

Move to an absolute X position.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `x`       | `float` |             |
| _Returns_ | `None`  |             |

### `move_xy_to()`

```python
move_xy_to(x: float, y: float) -> None
```

Move to an absolute XY position.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `x`       | `float` |             |
| `y`       | `float` |             |
| _Returns_ | `None`  |             |

### `move_y_to()`

```python
move_y_to(y: float) -> None
```

Move to an absolute Y position.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `y`       | `float` |             |
| _Returns_ | `None`  |             |

### `new_gluescript()`

```python
new_gluescript() -> None
```

Reset all per-job state.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `power()`

```python
power(percent: Optional[float]) -> None
```

Set immediate power for an IMAGE/DEPTHMAP layer.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `percent` | `Optional[float]` |             |
| _Returns_ | `None`            |             |

### `power_range()`

```python
power_range(min_power: Optional[float], max_power: Optional[float]) -> None
```

Save a power ramp range for the next cut.

| Parameter   | Type              | Description |
| ----------- | ----------------- | ----------- |
| `min_power` | `Optional[float]` |             |
| `max_power` | `Optional[float]` |             |
| _Returns_   | `None`            |             |

### `pwm()`

```python
pwm(duration: float) -> None
```

Record the pulse width (comment-only).

| Parameter  | Type    | Description |
| ---------- | ------- | ----------- |
| `duration` | `float` |             |
| _Returns_  | `None`  |             |

### `select_laser()`

```python
select_laser(laser: int) -> None
```

Select a laser head.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `laser`   | `int`  |             |
| _Returns_ | `None` |             |

### `set_max_cut_speed()`

```python
set_max_cut_speed(speed: float) -> None
```

Set the maximum cut speed used by power scaling.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `speed`   | `float` |             |
| _Returns_ | `None`  |             |

### `set_mode()`

```python
set_mode(mode: str) -> None
```

Switch the layer mode mid-stream.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `mode`    | `str`  |             |
| _Returns_ | `None` |             |

### `set_overscan()`

```python
set_overscan(overscan: str) -> None
```

Set the overscan for the current layer.

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `overscan` | `str`  |             |
| _Returns_  | `None` |             |

### `set_power_floor()`

```python
set_power_floor(floor: float) -> None
```

Set the laser's minimum firing power.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `floor`   | `float` |             |
| _Returns_ | `None`  |             |

### `set_power_scaling_enabled()`

```python
set_power_scaling_enabled(enabled: bool) -> None
```

Enable or disable effective-min power scaling.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `enabled` | `bool` |             |
| _Returns_ | `None` |             |

### `stage_gluescript()`

```python
stage_gluescript(
    gluescript: Optional[Sequence[str]] = None,
    require_complete: bool = True,
) -> str
```

Compile the job, or replay a transcript and compile it.

| Parameter          | Type                             | Description |
| ------------------ | -------------------------------- | ----------- |
| `gluescript`       | `Optional[Sequence[str]] = None` |             |
| `require_complete` | `bool = True`                    |             |
| _Returns_          | `str`                            |             |

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

## RuidaSpec

Typed Python wrapper around the Rust `RuidaSpec`.

### `context_json`

```python
context_json: str
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
