---
title: raygeo.ops.transform.smooth
sidebar_label: raygeo.ops.transform.smooth
---

## SmoothSpec

Parameters for the `Smooth` transformer.

Construct with `SmoothSpec(amount, corner_angle_threshold)`.

### `amount`

```python
amount: int
```

Smoothing strength (0-100); 0 is a no-op.

### `corner_angle_threshold`

```python
corner_angle_threshold: float
```

Corners with an internal angle (degrees) smaller than this are preserved.
