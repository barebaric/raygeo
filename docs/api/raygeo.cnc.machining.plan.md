---
title: raygeo.cnc.machining.plan
sidebar_label: raygeo.cnc.machining.plan
---

## Functions

### `execute_workplan()`

```python
execute_workplan(
    steps: list[dict],
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] | None = None,
    cut_feed_rate: int = 1200,
    cut_power: float = 1,
    rapid_feed_rate: int | None = None,
) -> ops.assembly.result.AssemblyResult
```

Execute a workplan: dispatch each step to its assembler.

Each entry in *steps* is a `WorkplanStep` dict as produced by a builder such as
**raygeo.cnc.machining.wavefront.build_wavefront_workplan** or
**raygeo.cnc.machining.entry.build_entry_workplan**. The executor owns a shared cleared area, asks
each step to invoke its own assembler, and chains the results into a single **AssemblyResult**.

| Parameter         | Type                                                         | Description                                                                            |
| ----------------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| `steps`           | `list[dict]`                                                 | List of WorkplanStep dicts (with a `"kind"` key).                                      |
| `pocket_boundary` | `Sequence[tuple[float, float]]`                              | Outer boundary of the pocket.                                                          |
| `islands`         | `Sequence[Sequence[tuple[float, float]]] &#124; None = None` | List of island polygons (default None).                                                |
| `cut_feed_rate`   | `int = 1200`                                                 | Feed rate for cutting moves (default 1200).                                            |
| `cut_power`       | `float = 1`                                                  | Laser power for cutting moves (default 1.0).                                           |
| `rapid_feed_rate` | `int &#124; None = None`                                     | Feed rate for travel/retract moves, or `None` to leave them unmodified (default None). |
| _Returns_         | `ops.assembly.result.AssemblyResult`                         | The combined **AssemblyResult**.                                                       |
