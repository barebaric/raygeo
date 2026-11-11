---
title: raygeo.sketch.constraints.symmetry
sidebar_label: raygeo.sketch.constraints.symmetry
---

## SymmetryConstraint

Enforces two points are symmetric about a point or a line.

### `p1`

```python
p1: int
```

First point.

### `p2`

```python
p2: int
```

Second point.

### `target`

```python
target: SymmetryTarget
```

Symmetry reference.

## SymmetryTarget

The symmetry reference: point symmetry about a center, or line symmetry about an axis line.

### Line

p1 and p2 are mirrored across the line (l1, l2).

### Point

The center is the midpoint of p1 and p2.
