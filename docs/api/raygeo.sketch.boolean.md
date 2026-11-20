---
title: raygeo.sketch.boolean
sidebar_label: raygeo.sketch.boolean
---

## BooleanOp

Boolean operation kind.

### DIFFERENCE

### EXCLUDE

### INTERSECTION

### UNION

## BooleanRegion

One closed candidate region.

### `entity_ids`

```python
entity_ids: list[int]
```

### `rings`

```python
rings: list[list[tuple[float, float]]]
```

### `z`

```python
z: int
```

## BooleanSolid

One boolean result: outer ring plus holes.

### `holes`

```python
holes: list[list[tuple[float, float]]]
```

### `outer`

```python
outer: list[tuple[float, float]]
```

## Functions

### `apply_boolean()`

```python
apply_boolean(
    op: BooleanOp,
    regions: Sequence[BooleanRegion],
    tolerance: float = 0.02,
) -> Optional[list[BooleanSolid]]
```

| Parameter   | Type                           | Description |
| ----------- | ------------------------------ | ----------- |
| `op`        | `BooleanOp`                    |             |
| `regions`   | `Sequence[BooleanRegion]`      |             |
| `tolerance` | `float = 0.02`                 |             |
| _Returns_   | `Optional[list[BooleanSolid]]` |             |

### `build_boolean_regions()`

```python
build_boolean_regions(
    registry: registry.EntityRegistry,
    entity_ids: Sequence[int],
    coincident_pairs: Sequence[tuple[int, int]] = [],
    tolerance: float = 0.02,
) -> Optional[list[BooleanRegion]]
```

| Parameter          | Type                             | Description |
| ------------------ | -------------------------------- | ----------- |
| `registry`         | `registry.EntityRegistry`        |             |
| `entity_ids`       | `Sequence[int]`                  |             |
| `coincident_pairs` | `Sequence[tuple[int, int]] = []` |             |
| `tolerance`        | `float = 0.02`                   |             |
| _Returns_          | `Optional[list[BooleanRegion]]`  |             |
