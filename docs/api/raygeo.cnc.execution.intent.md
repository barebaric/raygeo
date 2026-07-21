---
title: raygeo.cnc.execution.intent
sidebar_label: raygeo.cnc.execution.intent
---

## Intent

An executable Intent produced by \[`create_intent`\].

Holds the raw \[`NodeRequest`\]s inside a shared container so that \[`run_intent`\] can move them
out at execution time.

### `step_count`

```python
step_count: int
```

Number of compute nodes in this intent (excluding the final aggregate).

## Functions

### `create_intent()`

```python
create_intent(plan: plan.Plan, part: part.Part, generation_id: int) -> Intent
```

Convert a Plan and Part into an executable Intent.

| Parameter       | Type        | Description |
| --------------- | ----------- | ----------- |
| `plan`          | `plan.Plan` |             |
| `part`          | `part.Part` |             |
| `generation_id` | `int`       |             |
| _Returns_       | `Intent`    |             |

### `run_intent()`

```python
run_intent(
    intent: Intent,
    on_completed: Optional[Any] = None,
    on_batch_progress: Optional[Any] = None,
    pipeline: Optional[execute.Pipeline] = None,
) -> ops.Ops
```

Run an Intent through the pipeline, consuming the node list.

Returns the final aggregated **~raygeo.ops.Ops** (all steps linked with safe-Z travel).
`on_completed` is invoked for each completed node (including the aggregate) for progress monitoring.

| Parameter           | Type                                | Description |
| ------------------- | ----------------------------------- | ----------- |
| `intent`            | `Intent`                            |             |
| `on_completed`      | `Optional[Any] = None`              |             |
| `on_batch_progress` | `Optional[Any] = None`              |             |
| `pipeline`          | `Optional[execute.Pipeline] = None` |             |
| _Returns_           | `ops.Ops`                           |             |
