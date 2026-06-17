---
title: raygeo.geo.algo.nest2d.genetic
sidebar_label: raygeo.geo.algo.nest2d.genetic
sidebar_position: 14
---

Genetic algorithm for nesting optimization.

Provides a GeneticAlgorithm class that manages a population of placement configurations (rotations,
flips) and evolves them via mutation, crossover, and selection.

## GeneticAlgorithm

### `generation()`

```python
generation() -> None
```

Evolve one generation.

| Parameter    | Type   | Description                                        |
| ------------ | ------ | -------------------------------------------------- |
| _Returns_    | `None` |                                                    |
| _Complexity_ |        | O(p \* n) where p = population size, n = num_parts |

### `get_fitness()`

```python
get_fitness(idx: int) -> float
```

Returns the fitness of individual at idx.

| Parameter    | Type    | Description |
| ------------ | ------- | ----------- |
| `idx`        | `int`   |             |
| _Returns_    | `float` |             |
| _Complexity_ |         | O(1)        |

### `get_individual()`

```python
get_individual(idx: int) -> tuple[list[float], list[bool], list[bool], float]
```

Returns (rotations, flips_h, flips_v, fitness) for individual at idx.

| Parameter    | Type                                                | Description |
| ------------ | --------------------------------------------------- | ----------- |
| `idx`        | `int`                                               |             |
| _Returns_    | `tuple[list[float], list[bool], list[bool], float]` |             |
| _Complexity_ |                                                     | O(1)        |

### `mate()`

```python
mate(
    male_idx: int,
    female_idx: int,
) -> list[tuple[list[float], list[bool], list[bool]]]
```

Mate two individuals and return the two children.

| Parameter    | Type                                               | Description              |
| ------------ | -------------------------------------------------- | ------------------------ |
| `male_idx`   | `int`                                              |                          |
| `female_idx` | `int`                                              |                          |
| _Returns_    | `list[tuple[list[float], list[bool], list[bool]]]` |                          |
| _Complexity_ |                                                    | O(n) where n = num_parts |

### `mutate()`

```python
mutate(idx: int) -> tuple[list[float], list[bool], list[bool]]
```

Mutate and return a copy of individual at idx.

| Parameter    | Type                                         | Description              |
| ------------ | -------------------------------------------- | ------------------------ |
| `idx`        | `int`                                        |                          |
| _Returns_    | `tuple[list[float], list[bool], list[bool]]` |                          |
| _Complexity_ |                                              | O(n) where n = num_parts |

### `set_fitness()`

```python
set_fitness(idx: int, fitness: float) -> None
```

Set the fitness for individual at idx.

| Parameter    | Type    | Description |
| ------------ | ------- | ----------- |
| `idx`        | `int`   |             |
| `fitness`    | `float` |             |
| _Returns_    | `None`  |             |
| _Complexity_ |         | O(1)        |
