---
title: raygeo.pipeline.completed
sidebar_label: raygeo.pipeline.completed
---

Completion record types.

## CompletedNode

### `error`

```python
error: Optional[str]
```

### `error_kind`

```python
error_kind: Optional[ErrorKind]
```

### `generation_id`

```python
generation_id: int
```

### `key`

```python
key: str
```

### `output`

```python
output: Optional[Any]
```

## ErrorKind

Machine-readable error category for a failed pipeline node.

**Values:**

- `CACHE_BUDGET_EXCEEDED` — The cache budget does not allow storing this node's output.
- `CACHE_LOCK_POISONED` — The pipeline cache mutex is poisoned.
- `CANCELLED` — Node was cancelled (normal during rapid rebuilds).
- `OTHER` — Any other execution failure.
- `UPSTREAM_FAILED` — A dependency of this node failed.
