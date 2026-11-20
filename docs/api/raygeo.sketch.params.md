---
title: raygeo.sketch.params
sidebar_label: raygeo.sketch.params
---

Named sketch parameters and expressions.

A parameter may be a number or a math expression. Expressions are resolved in multiple passes so
they can reference parameters defined later, and unresolved expressions are reported through
`unresolved_errors()`. The context serializes to the reference `{'expressions': {...}}` dictionary.

## ParameterContext

### `evaluate()`

```python
evaluate(expression: Any) -> float
```

Evaluates an expression using the current parameters.

| Parameter    | Type    | Description |
| ------------ | ------- | ----------- |
| `expression` | `Any`   |             |
| _Returns_    | `float` |             |

### `evaluate_all()`

```python
evaluate_all(initial_values: Optional[dict] = None) -> None
```

Resolves every expression, optionally seeded with initial values.

| Parameter        | Type                    | Description |
| ---------------- | ----------------------- | ----------- |
| `initial_values` | `Optional[dict] = None` |             |
| _Returns_        | `None`                  |             |

### `from_dict()`

```python
from_dict(dict: dict) -> ParameterContext
```

Deserializes a context from the reference dictionary shape.

| Parameter | Type               | Description |
| --------- | ------------------ | ----------- |
| `dict`    | `dict`             |             |
| _Returns_ | `ParameterContext` |             |

### `get()`

```python
get(name: str, default: float = 0.0) -> float
```

Returns the evaluated value of a parameter.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| `name`    | `str`         |             |
| `default` | `float = 0.0` |             |
| _Returns_ | `float`       |             |

### `get_all_values()`

```python
get_all_values() -> Any
```

Returns a copy of all evaluated parameter values.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `Any` |             |

### `invalidate()`

```python
invalidate() -> None
```

Clears cached values so the next access re-evaluates everything.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `set()`

```python
set(name: str, value: Any) -> None
```

Sets a parameter from a number or an expression string.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `name`    | `str`  |             |
| `value`   | `Any`  |             |
| _Returns_ | `None` |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the context to the reference dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `unresolved_errors()`

```python
unresolved_errors() -> dict[str, str]
```

Expressions that remained unresolved during the last evaluation.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| _Returns_ | `dict[str, str]` |             |
