---
title: raygeo.sketch.constraints.equal_length
sidebar_label: raygeo.sketch.constraints.equal_length
---

## EqualLengthConstraint

Enforces equal characteristic lengths, one residual per `(base_a, base_b, other_a, other_b)` tuple.
The caller expands the entities' characteristic-length pairs (including the shorter-side fallback)
into flat tuples.

### `pairs`

```python
pairs: list[list[int]]
```

Resolved characteristic-length pairs.
