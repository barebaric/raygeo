---
title: raygeo.nest.genetic
sidebar_label: raygeo.nest.genetic
sidebar_position: 25
---

Genetic algorithm for nesting optimization.

Provides a GeneticAlgorithm class that manages a population of placement configurations (rotations,
flips) and evolves them via mutation, crossover, and selection.

## GeneticAlgorithm

### `generation()`

`generation() -> None`

Evolve one generation.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `get_fitness()`

`get_fitness(idx: int) -> float`

Returns the fitness of individual at idx.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `idx`     | `int`   |             |
| _Returns_ | `float` |             |

### `get_individual()`

`get_individual(idx: int) -> tuple[list[float], list[bool], list[bool], float]`

Returns (rotations, flips_h, flips_v, fitness) for individual at idx.

| Parameter | Type                                                | Description |
| --------- | --------------------------------------------------- | ----------- |
| `idx`     | `int`                                               |             |
| _Returns_ | `tuple[list[float], list[bool], list[bool], float]` |             |

### `mate()`

`mate(male_idx: int, female_idx: int) -> list[tuple[list[float], list[bool], list[bool]]]`

Mate two individuals and return the two children.

| Parameter    | Type                                               | Description |
| ------------ | -------------------------------------------------- | ----------- |
| `male_idx`   | `int`                                              |             |
| `female_idx` | `int`                                              |             |
| _Returns_    | `list[tuple[list[float], list[bool], list[bool]]]` |             |

### `mutate()`

`mutate(idx: int) -> tuple[list[float], list[bool], list[bool]]`

Mutate and return a copy of individual at idx.

| Parameter | Type                                         | Description |
| --------- | -------------------------------------------- | ----------- |
| `idx`     | `int`                                        |             |
| _Returns_ | `tuple[list[float], list[bool], list[bool]]` |             |

### `set_fitness()`

`set_fitness(idx: int, fitness: float) -> None`

Set the fitness for individual at idx.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `idx`     | `int`   |             |
| `fitness` | `float` |             |
| _Returns_ | `None`  |             |
