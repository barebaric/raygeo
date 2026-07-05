---
title: raygeo.ops.assembly.adaptive.routing
sidebar_label: raygeo.ops.assembly.adaptive.routing
---

## Functions

### `smooth_route()`

```python
smooth_route(
    from_pt: tuple[float, float, float],
    raw: Sequence[tuple[float, float, float]],
    obstacles: Sequence[Sequence[tuple[float, float]]] = [],
    clearance: float = 1,
) -> list[tuple[float, float, float]]
```

Smooth and shorten a cleared-territory travel path.

| Parameter   | Type                                           | Description |
| ----------- | ---------------------------------------------- | ----------- |
| `from_pt`   | `tuple[float, float, float]`                   |             |
| `raw`       | `Sequence[tuple[float, float, float]]`         |             |
| `obstacles` | `Sequence[Sequence[tuple[float, float]]] = []` |             |
| `clearance` | `float = 1`                                    |             |
| _Returns_   | `list[tuple[float, float, float]]`             |             |
