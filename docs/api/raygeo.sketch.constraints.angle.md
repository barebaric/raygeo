---
title: raygeo.sketch.constraints.angle
sidebar_label: raygeo.sketch.constraints.angle
---

## AngleConstraint

Enforces a clockwise angle in degrees between two lines. The angle is measured at the lines'
intersection between the rays selected by `e1_far`/`e2_far` (endpoint ids; `None` picks the endpoint
farther from the intersection).

### `e1_far`

```python
e1_far: Optional[int]
```

Far point of the anchor line.

### `e2_far`

```python
e2_far: Optional[int]
```

Far point of the other line.

### `p1`

```python
p1: int
```

First point of the anchor line.

### `p2`

```python
p2: int
```

Second point of the anchor line.

### `p3`

```python
p3: int
```

First point of the other line.

### `p4`

```python
p4: int
```

Second point of the other line.

### `value_deg`

```python
value_deg: float
```

Resolved angle value in degrees.
