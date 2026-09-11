---
title: raygeo.ops.transform.tangential_knife
sidebar_label: raygeo.ops.transform.tangential_knife
---

## TangentialKnifeSpec

Parameters for the `TangentialKnife` transformer.

### `angle_tolerance_deg`

```python
angle_tolerance_deg: float
```

Maximum heading change (degrees) rotated with the knife down.

### `radius_tolerance_mm`

```python
radius_tolerance_mm: float
```

Arcs tighter than this radius (millimeters) force a lift.

### `safe_z`

```python
safe_z: float
```

Z height used for lifting the knife at sharp corners.
