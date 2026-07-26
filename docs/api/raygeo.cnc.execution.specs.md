---
title: raygeo.cnc.execution.specs
sidebar_label: raygeo.cnc.execution.specs
---

CNC execution spec types.

## AggregateGroup

A group of inputs within an `AggregateSpec`.

Each group emits its own start/end markers, processes its inputs in order (optionally linking
consecutive inputs when `link_mode` is `Sequential`), and supports per-input placement and scaling.

### `end_markers`

```python
end_markers: list[Marker]
```

Markers emitted after this group's inputs.

### `inputs`

```python
inputs: list[AggregateInput]
```

Upstream inputs to concatenate (in order).

### `link_mode`

```python
link_mode: LinkMode
```

Inter-input linking control (default `LinkMode.none()`).

### `start_markers`

```python
start_markers: list[Marker]
```

Markers emitted before this group's inputs.

## AggregateInput

A single source input in an `AggregateGroup`.

Identifies an upstream node by key and optionally applies a placement matrix and uniform scaling.

### `placement_matrix`

```python
placement_matrix: list[list[float]]
```

4×4 placement matrix (row-major) applied to all points in the source Ops.

### `source_key`

```python
source_key: str
```

Key of the upstream node whose Ops to consume.

### `target_dimensions`

```python
target_dimensions: tuple[float, float]
```

Target dimensions for uniform scaling `(width, height)`. When `(0, 0)` (default), no scaling is
applied.

### `uid`

```python
uid: str
```

Optional UID carried through for marker correlation (not used by the pipeline core).

## AggregateOutput

Result of the `Aggregate` stage.

Produced when the pipeline executes an `AggregateSpec` and contains the concatenated/transformed Ops
together with an optional time estimate.

### `ops`

```python
ops: ops.Ops
```

The concatenated Ops from all inputs in the aggregate.

### `ops_len`

```python
ops_len: int
```

Number of commands in the aggregated Ops.

### `time_estimate`

```python
time_estimate: Optional[float]
```

Estimated machining time in seconds, or `None` when the `MachineParams` rates are all zero.

## AggregateSpec

Configuration for the `Aggregate` stage.

An aggregate concatenates Ops from multiple upstream nodes, wraps them in markers
(job/layer/workpiece start/end), applies per-input placement and scaling, optionally links
consecutive inputs with travel moves, and applies batch transformers.

### `groups`

```python
groups: list[AggregateGroup]
```

Groups of inputs, each processed in order.

### `machine`

```python
machine: MachineParams
```

Machine parameters for time estimation.

### `transformers`

```python
transformers: list[Any]
```

Batch transformers applied to the aggregated Ops.

### `wrap_end`

```python
wrap_end: list[Marker]
```

Markers emitted after all groups (e.g. `JobEnd`).

### `wrap_start`

```python
wrap_start: list[Marker]
```

Markers emitted before all groups (e.g. `JobStart`).

## ComputePayload

Opaque parameter bundle for a `Compute` stage node.

The pipeline's `StageSpec.Compute` stores this as an opaque `Any` — the CNC converter unpacks it
during traversal.

### `assembler`

```python
assembler: Any
```

The assembler spec (e.g. `ContourSpec`, `AdaptiveClearingSpec`).

### `cut_speed`

```python
cut_speed: int
```

Cut speed (mm/min) injected as `SetFeedRate`.

### `head_uid`

```python
head_uid: Optional[str]
```

Active head/laser UID injected as `SetHead`.

### `power`

```python
power: float
```

Laser power fraction (0.0 – 1.0) injected as `SetPower`.

### `state_source_keys`

```python
state_source_keys: list[str]
```

Source keys for cleared-area state threading (CNC only).

### `transformers`

```python
transformers: list[Any]
```

Optional list of transformer specs applied post-assembly.

## EncodeSpec

An encode-only compute stage.

The pipeline's generic `StageSpec` has no `Encode` variant — encoding is a CNC concern. Use
`EncodeSpec` when you need to run an encoder through the pipeline.

### `encoder`

```python
encoder: convert.Encoder
```

The encoder to use (e.g. `GcodeSpec`, `VertexSpec`).

### `source_key`

```python
source_key: str
```

Key of the upstream node whose Ops to encode.

## LinkMode

Controls inter-input linking in an AggregateGroup.

Create via class methods: LinkMode.none() LinkMode.sequential(safe_z=2.0)

### `safe_z`

```python
safe_z: float
```

Safe Z height for retract/lift moves (only meaningful when `tag == "sequential"`).

### `tag`

```python
tag: str
```

Discriminant: `"none"` or `"sequential"`.

### `none()`

```python
@classmethod none() -> LinkMode
```

Create a `LinkMode` with no inter-input linking.

| Parameter | Type       | Description |
| --------- | ---------- | ----------- |
| _Returns_ | `LinkMode` |             |

### `sequential()`

```python
@classmethod sequential(safe_z: float) -> LinkMode
```

Create a `LinkMode` that emits travel moves (retract → XY travel → plunge) between consecutive
inputs.

| Parameter | Type       | Description                          |
| --------- | ---------- | ------------------------------------ |
| `safe_z`  | `float`    | Z height for retract and final lift. |
| _Returns_ | `LinkMode` |                                      |

## MachineParams

Feed/speed rates used for time estimation.

All three fields default to `0.0`, which disables the time estimate (returning `None`).

### `acceleration`

```python
acceleration: float
```

Acceleration (mm/s²) for time estimation.

### `default_feed_rate`

```python
default_feed_rate: float
```

Default feed rate (mm/min) for cutting moves.

### `default_rapid_rate`

```python
default_rapid_rate: float
```

Default rapid rate (mm/min) for travel moves.

## Marker

Declarative markers for aggregate groups.

Each variant has a `_tag` field (always `True`) required by the stub generator; pass it as keyword
argument:

Marker.JobStart(\_tag=True) Marker.LayerStart(uid="my-layer", \_tag=True)
Marker.WorkpieceEnd(uid="my-wp", \_tag=True)

### JobEnd

Marks the end of a job.

### JobStart

Marks the beginning of a job.

### LayerEnd

Marks the end of a layer with the given UID.

### LayerStart

Marks the start of a layer with the given UID.

### WorkpieceEnd

Marks the end of a workpiece with the given UID.

### WorkpieceStart

Marks the start of a workpiece with the given UID.
