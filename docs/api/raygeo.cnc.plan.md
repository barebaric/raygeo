---
title: raygeo.cnc.plan
sidebar_label: raygeo.cnc.plan
---

Plan-time description of machining operations.

Plans are produced by planners and consumed by a downstream runtime to derive its own executable
steps. They are never executed directly.

## Plan

A descriptive Plan produced by planners.

### `safe_z`

```python
safe_z: float
```

The safe Z height for retract moves.

### `step_count`

```python
step_count: int
```

Number of PlanSteps in this plan.

### `steps`

```python
steps: list[PlanStep]
```

The list of PlanSteps in this plan.

### `extend()`

```python
extend(steps: Sequence[PlanStep]) -> None
```

Append PlanSteps to this plan.

| Parameter | Type                 | Description |
| --------- | -------------------- | ----------- |
| `steps`   | `Sequence[PlanStep]` |             |
| _Returns_ | `None`               |             |

## PlanStep

One step in a Plan: a face_id and an assembler spec.

### `face_id`

```python
face_id: str
```

The face this step targets.

### `kind`

```python
kind: str
```

The assembler kind (e.g. `"helix"`, `"adaptive_clearing"`).

### `spec_params()`

```python
spec_params() -> dict
```

All spec parameters as a Python dict.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |
