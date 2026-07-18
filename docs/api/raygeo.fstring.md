---
title: raygeo.fstring
sidebar_label: raygeo.fstring
---

## Functions

### `parse_include_directive()`

```python
parse_include_directive(line: str) -> str | None
```

Parse an `@include(MacroName)` directive.

Returns the macro name (stripped of whitespace) or `None` if the line is not an include directive.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `line`    | `str`             |             |
| _Returns_ | `str &#124; None` |             |

### `render_named()`

```python
render_named(template: str, vars: dict[str, str]) -> str
```

Resolve named substitution variables in a template string.

Replaces `{name}` placeholders using the provided dict. Unknown placeholders (including path-style
`{machine.name}`) are left verbatim for a subsequent `resolve_path_vars` call.

| Parameter  | Type             | Description |
| ---------- | ---------------- | ----------- |
| `template` | `str`            |             |
| `vars`     | `dict[str, str]` |             |
| _Returns_  | `str`            |             |

### `resolve_path_vars()`

```python
resolve_path_vars(template: str, path_vars: dict[str, str]) -> str
```

Resolve path-style placeholders using a flat dict.

Replaces `{machine.name}`, `{job.extents[0]}` etc. using the provided dict. Unresolved placeholders
are left verbatim.

| Parameter   | Type             | Description |
| ----------- | ---------------- | ----------- |
| `template`  | `str`            |             |
| `path_vars` | `dict[str, str]` |             |
| _Returns_   | `str`            |             |
