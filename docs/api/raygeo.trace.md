---
title: raygeo.trace
sidebar_label: raygeo.trace
---

Binary trace-file reader and shared move-type classification.

MoveKind — standard move-type classification shared by all operations. TraceFile — read a .bin trace
file with random access to records. TraceRecord — one per-step record with dot-accessible fields.

## MoveKind

Standard move-type classification shared by all operations.

Every toolpath point is tagged with one of these so that renderers can colour and categorise moves
generically.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## ResumeSource

Resume-strategy enum for trace records.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## RouteSource

Route-strategy enum for trace records.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## StepStatus

Step-status enum for trace records.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## TraceFile

Binary trace file with random access to records.

Usage:

```python
>>> from raygeo.trace import TraceFile
>>> t = TraceFile("path/to/trace.bin")
>>> len(t)          # number of records
>>> t[0]            # first record (TraceRecord with dot access)
>>> t.toolpath      # list of (x, y, move_kind) tuples
>>> t.geometry      # dict with tool_radius, boundary, islands, seeds
>>> t.mat_nodes     # MAT nodes or empty list
```

### `geometry`

```python
geometry: dict
```

Decoded geometry dict (tool_radius, boundary, islands, seeds) from the first record with
`kind == "geometry"`, or an empty dict.

### `mat_clearances`

```python
mat_clearances: list[float]
```

### `mat_edges`

```python
mat_edges: list[tuple[int, int]]
```

### `mat_nodes`

```python
mat_nodes: list[tuple[float, float]]
```

MAT nodes from the first record with `kind == "mat"`, or an empty list.

### `mat_root`

```python
mat_root: int
```

### `toolpath`

```python
toolpath: list[tuple[float, float, int]]
```

Toolpath points extracted from all motion records (those with `pos_x` / `pos_y`). Returns a list of
`(x, y, move_kind)` where `move_kind` is a `MoveKind` value.

### `ver`

```python
ver: int
```

## TraceKind

Record-kind enum for trace events.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## TraceRecord

One per-step trace record.

Wraps the decoded msgpack map and exposes fields as attributes. Supports both dot access
(`rec.kind`) and dict access (`rec["kind"]`).

### `keys()`

```python
keys() -> list[str]
```

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[str]` |             |

## Functions

### `get_route_detail_name()`

```python
get_route_detail_name(detail: int) -> str
```

Return a human-readable label for a route-strategy detail code.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `detail`  | `int` |             |
| _Returns_ | `str` |             |
