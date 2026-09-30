---
title: raygeo.ops.assembly.custom
sidebar_label: raygeo.ops.assembly.custom
---

## CustomSpec

Parameters for the `custom` assembler.

Construct with `CustomSpec(lines)`. Wrap in an **~raygeo.ops.assembly.Assembler** instance to drive
the `Assembler` trait.

### `lines`

```python
lines: list[str]
```

Raw (unexpanded) machine-code lines, one command each.
