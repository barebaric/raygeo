---
title: raygeo.template
sidebar_label: raygeo.template
---

## CounterSettings

An immutable job counter.

The counter is the number the next job starts with, the value a reset returns to, the increment per
copy and per job, and an optional end value after which the range wraps back to `start`.

### `advance_on_export`

```python
advance_on_export: bool
```

Whether exporting G-code advances the counter.

### `auto_advance`

```python
auto_advance: bool
```

Whether a finished job advances the counter.

### `current`

```python
current: int
```

The value the next job starts with.

### `end`

```python
end: Optional[int]
```

The optional last value of the range.

### `start`

```python
start: int
```

The value a reset returns to and a wrapped range restarts at.

### `step`

```python
step: int
```

The increment per copy and per job.

### `advanced()`

```python
advanced(copies: int = 1) -> CounterSettings
```

A counter moved on (or back, if negative) by `copies`.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `copies`  | `int = 1`         |             |
| _Returns_ | `CounterSettings` |             |

### `from_dict()`

```python
from_dict(dict: dict) -> CounterSettings
```

Deserializes a counter from the project dictionary shape.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `dict`    | `dict`            |             |
| _Returns_ | `CounterSettings` |             |

### `reset()`

```python
reset() -> CounterSettings
```

A counter whose current value is back at `start`.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `CounterSettings` |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the counter to the project dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `value_at()`

```python
value_at(offset: int) -> int
```

The value shown by a copy that is `offset` copies further.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `offset`  | `int` |             |
| _Returns_ | `int` |             |

### `values_for()`

```python
values_for(copies: int) -> list[int]
```

The values of `copies` consecutive copies.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| `copies`  | `int`       |             |
| _Returns_ | `list[int]` |             |

## Functions

### `evaluate()`

```python
evaluate(
    expression: str,
    values: dict,
    functions: dict | None = None,
) -> object
```

Evaluate an expression with host variables and functions.

| Parameter    | Type                      | Description |
| ------------ | ------------------------- | ----------- |
| `expression` | `str`                     |             |
| `values`     | `dict`                    |             |
| `functions`  | `dict &#124; None = None` |             |
| _Returns_    | `object`                  |             |

### `evaluate_number()`

```python
evaluate_number(
    expression: str,
    values: dict,
    functions: dict | None = None,
) -> float
```

Evaluate an expression and return a float.

| Parameter    | Type                      | Description |
| ------------ | ------------------------- | ----------- |
| `expression` | `str`                     |             |
| `values`     | `dict`                    |             |
| `functions`  | `dict &#124; None = None` |             |
| _Returns_    | `float`                   |             |

### `format_value()`

```python
format_value(value: object, spec: str) -> str
```

Format a value with a Python-style format spec.

| Parameter | Type     | Description |
| --------- | -------- | ----------- |
| `value`   | `object` |             |
| `spec`    | `str`    |             |
| _Returns_ | `str`    |             |

### `names_in_expression()`

```python
names_in_expression(expression: str) -> set[str]
```

Return the variable names an expression reads.

String literals, keywords, numeric literals and attribute names are ignored.

| Parameter    | Type       | Description |
| ------------ | ---------- | ----------- |
| `expression` | `str`      |             |
| _Returns_    | `set[str]` |             |

### `placeholder_names()`

```python
placeholder_names(template: str) -> set[str]
```

Return the placeholder names of a template.

Dotted path names such as `workpiece.counter` are returned verbatim; a format spec after the first
colon is not part of the name.

| Parameter  | Type       | Description |
| ---------- | ---------- | ----------- |
| `template` | `str`      |             |
| _Returns_  | `set[str]` |             |

### `referenced_names()`

```python
referenced_names(template: str) -> set[str]
```

Return the variable names a template reads.

A format spec after the first colon is ignored. String literals, keywords and attribute names are
not reported.

| Parameter  | Type       | Description |
| ---------- | ---------- | ----------- |
| `template` | `str`      |             |
| _Returns_  | `set[str]` |             |

### `resolve_text()`

```python
resolve_text(
    template: str,
    values: dict,
    functions: dict,
    previous: tuple | None = None,
) -> tuple[str | None, tuple, bool]
```

Resolve a text template.

`values` are the variables, `functions` the host's zero-argument template functions, and `previous`
the entry returned by an earlier call. Returns `(resolved, entry, volatile)`.

| Parameter   | Type                                  | Description |
| ----------- | ------------------------------------- | ----------- |
| `template`  | `str`                                 |             |
| `values`    | `dict`                                |             |
| `functions` | `dict`                                |             |
| `previous`  | `tuple &#124; None = None`            |             |
| _Returns_   | `tuple[str &#124; None, tuple, bool]` |             |
