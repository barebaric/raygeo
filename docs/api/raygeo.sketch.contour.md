---
title: raygeo.sketch.contour
sidebar_label: raygeo.sketch.contour
---

Contour preparation: connected components, chain orientation, sampled chain vertices, ring grouping
and outline offsetting for offset and boolean tools.

## Functions

### `chain_vertices()`

```python
chain_vertices(
    registry: registry.EntityRegistry,
    edges: Sequence[tuple[int, bool]],
    closed: bool,
    tolerance: float = 0.02,
) -> list[tuple[float, float]]
```

| Parameter   | Type                         | Description |
| ----------- | ---------------------------- | ----------- |
| `registry`  | `registry.EntityRegistry`    |             |
| `edges`     | `Sequence[tuple[int, bool]]` |             |
| `closed`    | `bool`                       |             |
| `tolerance` | `float = 0.02`               |             |
| _Returns_   | `list[tuple[float, float]]`  |             |

### `group_rings_into_solids()`

```python
group_rings_into_solids(
    rings: Sequence[Sequence[tuple[float, float]]],
) -> list[tuple[list[tuple[float, float]], list[list[tuple[float, float]]]]]
```

| Parameter | Type                                                                      | Description |
| --------- | ------------------------------------------------------------------------- | ----------- |
| `rings`   | `Sequence[Sequence[tuple[float, float]]]`                                 |             |
| _Returns_ | `list[tuple[list[tuple[float, float]], list[list[tuple[float, float]]]]]` |             |

### `offset_outline()`

```python
offset_outline(
    vertices: Sequence[tuple[float, float]],
    closed: bool,
    distance: float,
    tolerance: float = 0.02,
) -> list[tuple[list[tuple[float, float]], bool]]
```

| Parameter   | Type                                           | Description |
| ----------- | ---------------------------------------------- | ----------- |
| `vertices`  | `Sequence[tuple[float, float]]`                |             |
| `closed`    | `bool`                                         |             |
| `distance`  | `float`                                        |             |
| `tolerance` | `float = 0.02`                                 |             |
| _Returns_   | `list[tuple[list[tuple[float, float]], bool]]` |             |

### `outline_frame()`

```python
outline_frame(
    vertices: Sequence[tuple[float, float]],
    closed: bool,
) -> tuple[tuple[float, float], tuple[float, float]]
```

| Parameter  | Type                                              | Description |
| ---------- | ------------------------------------------------- | ----------- |
| `vertices` | `Sequence[tuple[float, float]]`                   |             |
| `closed`   | `bool`                                            |             |
| _Returns_  | `tuple[tuple[float, float], tuple[float, float]]` |             |

### `partition_components()`

```python
partition_components(
    registry: registry.EntityRegistry,
    entity_ids: Sequence[int],
    coincident_pairs: Sequence[tuple[int, int]],
) -> Optional[list[list[int]]]
```

| Parameter          | Type                        | Description |
| ------------------ | --------------------------- | ----------- |
| `registry`         | `registry.EntityRegistry`   |             |
| `entity_ids`       | `Sequence[int]`             |             |
| `coincident_pairs` | `Sequence[tuple[int, int]]` |             |
| _Returns_          | `Optional[list[list[int]]]` |             |

### `rings_to_geometry()`

```python
rings_to_geometry(
    rings: Sequence[Sequence[tuple[float, float]]],
) -> geo.Geometry
```

| Parameter | Type                                      | Description |
| --------- | ----------------------------------------- | ----------- |
| `rings`   | `Sequence[Sequence[tuple[float, float]]]` |             |
| _Returns_ | `geo.Geometry`                            |             |

### `walk_component()`

```python
walk_component(
    registry: registry.EntityRegistry,
    entity_ids: Sequence[int],
    coincident_pairs: Sequence[tuple[int, int]] = [],
) -> tuple[list[tuple[int, bool]], bool]
```

| Parameter          | Type                                  | Description |
| ------------------ | ------------------------------------- | ----------- |
| `registry`         | `registry.EntityRegistry`             |             |
| `entity_ids`       | `Sequence[int]`                       |             |
| `coincident_pairs` | `Sequence[tuple[int, int]] = []`      |             |
| _Returns_          | `tuple[list[tuple[int, bool]], bool]` |             |
