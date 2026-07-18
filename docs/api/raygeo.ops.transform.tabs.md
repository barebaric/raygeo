---
title: raygeo.ops.transform.tabs
sidebar_label: raygeo.ops.transform.tabs
---

## TabsSpec

Parameters for the `Tabs` transformer.

The `clips` are pre-scaled clip points in ops space, computed by Python from the workpiece's tab
definitions.

Dispatch rule: if `tab_power > 0.0` the power mode is used (with effective power
`tab_power * original_power`); otherwise the gap mode is used.

### `clips`

```python
clips: list[tuple[float, float, float]]
```

`(x, y, width)` tuples defining tab positions.

### `original_power`

```python
original_power: float
```

Normal cutting power restored after each tab.

### `tab_power`

```python
tab_power: float
```

Tab power level (0.0-1.0). 0.0 selects gap mode.
