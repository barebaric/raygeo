---
title: raygeo.pipeline.execute
sidebar_label: raygeo.pipeline.execute
---

Pipeline execution entry point.

## Pipeline

### `cache_budget_bytes`

```python
cache_budget_bytes: int
```

Configured byte budget.

### `cache_used_bytes`

```python
cache_used_bytes: int
```

Current bytes in use by the cache.

### `clear_cache()`

```python
clear_cache() -> None
```

Clear the entire cache.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `clear_cache_prefix()`

```python
clear_cache_prefix(prefix: str) -> None
```

Clear all entries whose tag starts with `prefix`.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `prefix`  | `str`  |             |
| _Returns_ | `None` |             |

### `execute()`

```python
execute(
    nodes: Sequence[request.NodeRequest],
    on_completed: Any,
    on_batch_progress: Optional[Any],
) -> None
```

Run all nodes in a single `rayon::scope`.

| Parameter           | Type                            | Description                                                    |
| ------------------- | ------------------------------- | -------------------------------------------------------------- |
| `nodes`             | `Sequence[request.NodeRequest]` | List of **~raygeo.pipeline.request.NodeRequest** instances.    |
| `on_completed`      | `Any`                           | Callable `(node: CompletedNode) -> None` fired for every node. |
| `on_batch_progress` | `Optional[Any]`                 | Optional callable `(fraction: float, message: str) -> None`.   |
| _Returns_           | `None`                          |                                                                |

### `set_cache_budget_bytes()`

```python
set_cache_budget_bytes(budget: int) -> None
```

Override the cache byte budget at runtime.

If the new budget is smaller than current usage, entries are evicted (oldest first) until usage fits
within the new limit.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `budget`  | `int`  |             |
| _Returns_ | `None` |             |

## PipelineCancelled

Raised when pipeline execution was cancelled (normal during rapid rebuilds).

## Functions

### `clear_cache()`

```python
clear_cache() -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `execute_stages()`

```python
execute_stages(
    nodes: list[pipeline.request.NodeRequest],
    on_completed: Callable[[pipeline.completed.CompletedNode], None],
    on_batch_progress: Callable[[float, str], None] | None = None,
) -> None
```

Run all nodes in a single rayon::scope.

Fires `on_completed` for every node (success, failure, or cancellation) with a **CompletedNode**
carrying the node's `key`, `generation_id`, and either `output` or `error`. `on_batch_progress`
(optional) fires with the aggregate fraction and a status message on every per-node progress report
and every completion.

| Parameter           | Type                                                 | Description |
| ------------------- | ---------------------------------------------------- | ----------- |
| `nodes`             | `list[pipeline.request.NodeRequest]`                 |             |
| `on_completed`      | `Callable[[pipeline.completed.CompletedNode], None]` |             |
| `on_batch_progress` | `Callable[[float, str], None] &#124; None = None`    |             |
| _Returns_           | `None`                                               |             |
