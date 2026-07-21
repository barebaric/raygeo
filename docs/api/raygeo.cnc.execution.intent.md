---
title: raygeo.cnc.execution.intent
sidebar_label: raygeo.cnc.execution.intent
---

## Intent

An executable Intent produced by \[`create_intent`\].

Holds the raw \[`NodeRequest`\]s inside a shared container so that \[`run_intent`\] can move them
out at execution time. The `last_tokens` map records each node's `version_token` from the last
\[`update <PyIntent::__pyo3_get__update>`\] call, enabling diff-based cache invalidation.

### `step_count`

```python
step_count: int
```

Number of compute nodes in this intent (excluding the final aggregate).

### `invalidate()`

```python
invalidate(
    keys: Sequence[str],
    pipeline: Optional[execute.Pipeline] = None,
) -> None
```

Manually invalidate specific node keys and their transitive dependents.

Cache entries for each key (and every node that depends on them, transitively) are evicted and their
epochs bumped. The next **run_intent** call recomputes them.

This is the escape hatch for cases where node content changed without a `version_token` change —
e.g. an in-place raster pixel edit.

| Parameter  | Type                                | Description |
| ---------- | ----------------------------------- | ----------- |
| `keys`     | `Sequence[str]`                     |             |
| `pipeline` | `Optional[execute.Pipeline] = None` |             |
| _Returns_  | `None`                              |             |

### `update()`

```python
update(new_intent: Intent, pipeline: Optional[execute.Pipeline] = None) -> None
```

Diff this intent against `new_intent` and update internal state, evicting stale cache entries.

For each node whose `version_token` changed (or that was removed), the corresponding cache entry is
evicted and its epoch bumped. Transitive dependents are invalidated too. Unchanged nodes keep their
cache entries.

After `update`, the old intent holds the new node list and can be executed with **run_intent**.

| Parameter    | Type                                | Description |
| ------------ | ----------------------------------- | ----------- |
| `new_intent` | `Intent`                            |             |
| `pipeline`   | `Optional[execute.Pipeline] = None` |             |
| _Returns_    | `None`                              |             |

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

### `create_intent_from_nodes()`

```python
create_intent_from_nodes(nodes: Sequence[request.NodeRequest]) -> Intent
```

Build an Intent from a list of raw **~raygeo.pipeline.request.NodeRequest** objects.

Useful for callers (e.g. rayforge's IntentBuilder) that construct their own node list without going
through the Plan API.

| Parameter | Type                            | Description |
| --------- | ------------------------------- | ----------- |
| `nodes`   | `Sequence[request.NodeRequest]` |             |
| _Returns_ | `Intent`                        |             |

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
