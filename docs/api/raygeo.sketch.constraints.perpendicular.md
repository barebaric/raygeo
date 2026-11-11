---
title: raygeo.sketch.constraints.perpendicular
sidebar_label: raygeo.sketch.constraints.perpendicular
---

## PerpendicularConstraint

Enforces perpendicularity between two entities.

### `target`

```python
target: PerpendicularTarget
```

Resolved operand geometry.

## PerpendicularTarget

The resolved geometry of the two perpendicular operands: two lines, a line through a circle/arc
center, or two circles meeting at a right angle.

### Circles

Circle/arc pair: r1² + r2² == dist(c1, c2)².

### LineCenter

Line and circle/arc: the line passes through the center.

### LineLine

Line-line: direction dot product.
