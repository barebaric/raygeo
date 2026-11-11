---
title: raygeo.sketch.topology
sidebar_label: raygeo.sketch.topology
---

Sketch topology: chain assembly and closed-loop detection.

plan_chains partitions two-endpoint edges into continuous chains of (entity_id, forward) tuples by
collapsing coincident points and growing paths from each unvisited edge - the topology behind
sketch-to-geometry conversion. find_closed_loops traces the faces of the sketch graph via half-edge
traversal with tangent-sorted outgoing edges, keeping positively oriented loops and appending closed
single-entity loops. Geometry emission stays with the caller; the tangent angles the traversal needs
are precomputed inputs.

## Edge

### `angle_at_p1`

```python
angle_at_p1: float
```

atan2 of the entity tangent leaving from `p1` (entity-kind specific, precomputed by the caller).

### `angle_at_p2`

```python
angle_at_p2: float
```

atan2 of the entity tangent leaving from `p2`.

### `entity_id`

```python
entity_id: int
```

Unique entity id.

### `p1`

```python
p1: int
```

First endpoint point id.

### `p2`

```python
p2: int
```

Second endpoint point id.

## LoopShape

### Arc

An arc edge, contributing the circular-segment area correction.

### Closed

A closed single-entity loop (circle, ellipse) reported as its own face with the caller-computed
enclosed signed area.

## Functions

### `find_closed_loops()`

```python
find_closed_loops(
    edges: list[sketch.topology.Edge],
    points: list[tuple[int, float, float]],
    coincident_groups: list[list[int]],
    loop_shapes: list[sketch.topology.LoopShape],
) -> list[list[tuple[int, bool]]]
```

Find all closed loops (faces) of the sketch graph.

Traces faces via half-edge traversal with tangent-sorted outgoing edges, keeping loops with positive
signed area (> 1e-6, refined by circular-segment corrections for arcs) and appending closed
single-entity loops.

| Parameter           | Type                              | Description                                                                        |
| ------------------- | --------------------------------- | ---------------------------------------------------------------------------------- |
| `edges`             | `list[sketch.topology.Edge]`      | Edge descriptors.                                                                  |
| `points`            | `list[tuple[int, float, float]]`  | Point id and coordinates `(id, x, y)`.                                             |
| `coincident_groups` | `list[list[int]]`                 | Transitive coincident groups (each group listing its member point ids).            |
| `loop_shapes`       | `list[sketch.topology.LoopShape]` | Arc descriptors and closed single-entity loops contributing to the loop detection. |
| _Returns_           | `list[list[tuple[int, bool]]]`    | Loops of `(entity_id, forward)` tuples.                                            |
| _Complexity_        |                                   | O(e log e) per traversal step                                                      |

### `plan_chains()`

```python
plan_chains(
    edges: list[sketch.topology.Edge],
    coincident_pairs: list[tuple[int, int]],
) -> list[list[tuple[int, bool]]]
```

Partition edges into continuous chains.

Collapses coincident points into shared nodes, then grows a path from each unvisited edge in both
directions, always taking the first unvisited continuation.

| Parameter          | Type                           | Description                                                                      |
| ------------------ | ------------------------------ | -------------------------------------------------------------------------------- |
| `edges`            | `list[sketch.topology.Edge]`   | Edge descriptors with endpoint point ids and precomputed leaving tangent angles. |
| `coincident_pairs` | `list[tuple[int, int]]`        | Coincident point id pairs.                                                       |
| _Returns_          | `list[list[tuple[int, bool]]]` | Chains of `(entity_id, forward)` tuples.                                         |
| _Complexity_       |                                | O(e^2) worst case, typically near-linear                                         |
