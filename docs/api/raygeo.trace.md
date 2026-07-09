---
title: raygeo.trace
sidebar_label: raygeo.trace
---

Binary trace-file reader and shared move-type classification.

MoveKind — standard move-type classification shared by all operations. TraceFile — read a .bin trace
file with span/event access. Span — one span record from a trace file. Event — one event record from
a trace file. ToolSnapshot — tool position and heading snapshot. ProgressSnapshot — step progress
snapshot.

## Event

One trace event (init / move / resume / exit).

### `kind`

```python
kind: str
```

### `meta`

```python
meta: Any
```

### `move_kind`

```python
move_kind: Optional[str]
```

### `progress`

```python
progress: Optional[ProgressSnapshot]
```

### `seq`

```python
seq: int
```

### `source`

```python
source: str
```

### `span`

```python
span: int
```

### `tool`

```python
tool: Optional[ToolSnapshot]
```

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

## ProgressSnapshot

Snapshot of step progress during trace execution.

### `ops_len`

```python
ops_len: int
```

### `status`

```python
status: int
```

### `step_idx`

```python
step_idx: int
```

## Span

One span record from a trace file.

### `attrs`

```python
attrs: Any
```

### `children`

```python
children: list[Span]
```

### `events`

```python
events: list[Event]
```

### `id`

```python
id: int
```

### `label`

```python
label: str
```

### `parent`

```python
parent: int
```

### `source`

```python
source: str
```

## ToolSnapshot

Snapshot of tool position and heading at a trace event.

### `heading`

```python
heading: float
```

### `pos_x`

```python
pos_x: float
```

### `pos_y`

```python
pos_y: float
```

### `pos_z`

```python
pos_z: float
```

### `prev_x`

```python
prev_x: float
```

### `prev_y`

```python
prev_y: float
```

### `prev_z`

```python
prev_z: float
```

## TraceFile

Binary trace file with span/event access.

Usage::

```
>>> from raygeo.trace import TraceFile
>>> t = TraceFile("path/to/trace.bin")
>>> t.ver
3
>>> t.root
Span(id=1, parent=0, source='workplan', label='Workplan')
>>> len(t.events)
42
```

### `events`

```python
events: list[Event]
```

### `root`

```python
root: Optional[Span]
```

The root span (first span with parent == 0), or None.

### `sources`

```python
sources: Any
```

Distinct source strings across all spans and events.

### `spans`

```python
spans: list[Span]
```

### `ver`

```python
ver: int
```

### `toolpath()`

```python
toolpath(span: Optional[Any] = None) -> list[tuple[float, float, str]]
```

Toolpath points from Move events.

Returns a list of `(x, y, move_kind_name)` tuples. If *span* is given (an int span id or a Span
object), restrict to events belonging to that span.

| Parameter | Type                             | Description |
| --------- | -------------------------------- | ----------- |
| `span`    | `Optional[Any] = None`           |             |
| _Returns_ | `list[tuple[float, float, str]]` |             |

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
