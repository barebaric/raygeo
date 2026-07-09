---
title: raygeo.cnc.machining.plan
sidebar_label: raygeo.cnc.machining.plan
---

## Workplan

Plan-time context and executor for a sequence of WorkplanSteps.

Captures the pocket boundary, islands, and safe-Z at construction time. After extending with steps
from a builder (e.g. **raygeo.cnc.machining.wavefront.build_wavefront_workplan** or
**raygeo.cnc.machining.entry.build_entry_workplan**), call **execute** to produce a combined
**AssemblyResult**.

### `execute()`

```python
execute(
    cut_feed_rate: int = 1200,
    cut_power: float = 1.0,
    rapid_feed_rate: Optional[int] = None,
    trace: Optional[str] = None,
) -> result.AssemblyResult
```

Execute all steps and return the combined **AssemblyResult**.

| Parameter         | Type                    | Description                                                                            |
| ----------------- | ----------------------- | -------------------------------------------------------------------------------------- |
| `cut_feed_rate`   | `int = 1200`            | Feed rate for cutting moves (default 1200).                                            |
| `cut_power`       | `float = 1.0`           | Laser power for cutting moves (default 1.0).                                           |
| `rapid_feed_rate` | `Optional[int] = None`  | Feed rate for travel/retract moves, or `None` to leave them unmodified (default None). |
| `trace`           | `Optional[str] = None`  | Optional file path for a trace file (`.bin`).                                          |
| _Returns_         | `result.AssemblyResult` | The combined **AssemblyResult**.                                                       |

### `extend()`

```python
extend(steps: Any) -> None
```

Append builder output steps (list of WorkplanStep dicts).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `steps`   | `Any`  |             |
| _Returns_ | `None` |             |
