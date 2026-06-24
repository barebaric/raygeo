---
title: raygeo.geo.algo.rootfind
sidebar_label: raygeo.geo.algo.rootfind
sidebar_position: 30
---

![Bisection, secant, and Illinois on $x^3 - 2x - 5$.](images/geo-algo-rootfind-rootfind.png)

_Bisection, secant, and Illinois on $x^3 - 2x - 5$._ 1D root-finding methods: bisection, secant,
Illinois.

## Functions

### `bisect()`

```python
bisect(
    f: Callable[[float], float],
    lo: float,
    hi: float,
    tol: float = 1e-10,
    max_iter: int = 100,
) -> tuple[float, str, int]
```

Bisection root-finding.

| Parameter  | Type                       | Description                                                |
| ---------- | -------------------------- | ---------------------------------------------------------- |
| `f`        | `Callable[[float], float]` | Function to find the root of (takes float, returns float). |
| `lo`       | `float`                    | Lower bound of the search interval.                        |
| `hi`       | `float`                    | Upper bound of the search interval.                        |
| `tol`      | `float = 1e-10`            | Convergence tolerance (default 1e-10).                     |
| `max_iter` | `int = 100`                | Maximum iterations (default 100).                          |
| _Returns_  | `tuple[float, str, int]`   | `(root, status_string, iteration_count)`.                  |

![Error vs iteration count: secant fastest, bisection slowest.](images/geo-algo-rootfind-convergence.png)

_Error vs iteration count: secant fastest, bisection slowest._

![Iterations to reach a given tolerance for sqrt(2): secant needs far fewer than bisection.](images/geo-algo-rootfind-precision.png)

_Iterations to reach a given tolerance for sqrt(2): secant needs far fewer than bisection._

### `bisect_tracked()`

```python
bisect_tracked(
    f: Any,
    lo: float,
    hi: float,
    tol: float = 1e-10,
    max_iter: int = 100,
) -> tuple[float, str, int, list[float]]
```

Bisection with iteration history.

| Parameter  | Type                                  | Description                               |
| ---------- | ------------------------------------- | ----------------------------------------- |
| `f`        | `Any`                                 |                                           |
| `lo`       | `float`                               |                                           |
| `hi`       | `float`                               |                                           |
| `tol`      | `float = 1e-10`                       |                                           |
| `max_iter` | `int = 100`                           |                                           |
| _Returns_  | `tuple[float, str, int, list[float]]` | `(root, status, iterations, [estimates])` |

### `illinois()`

```python
illinois(
    f: Callable[[float], float],
    lo: float,
    hi: float,
    tol: float = 1e-10,
    max_iter: int = 100,
) -> tuple[float, str, int]
```

Illinois (safeguarded false-position) root-finding.

| Parameter  | Type                       | Description                                                |
| ---------- | -------------------------- | ---------------------------------------------------------- |
| `f`        | `Callable[[float], float]` | Function to find the root of (takes float, returns float). |
| `lo`       | `float`                    | Lower bound of the search interval.                        |
| `hi`       | `float`                    | Upper bound of the search interval.                        |
| `tol`      | `float = 1e-10`            | Convergence tolerance (default 1e-10).                     |
| `max_iter` | `int = 100`                | Maximum iterations (default 100).                          |
| _Returns_  | `tuple[float, str, int]`   | `(root, status_string, iteration_count)`.                  |

### `illinois_tracked()`

```python
illinois_tracked(
    f: Any,
    lo: float,
    hi: float,
    tol: float = 1e-10,
    max_iter: int = 100,
) -> tuple[float, str, int, list[float]]
```

Illinois with iteration history.

| Parameter  | Type                                  | Description                               |
| ---------- | ------------------------------------- | ----------------------------------------- |
| `f`        | `Any`                                 |                                           |
| `lo`       | `float`                               |                                           |
| `hi`       | `float`                               |                                           |
| `tol`      | `float = 1e-10`                       |                                           |
| `max_iter` | `int = 100`                           |                                           |
| _Returns_  | `tuple[float, str, int, list[float]]` | `(root, status, iterations, [estimates])` |

### `secant()`

```python
secant(
    f: Callable[[float], float],
    x0: float,
    x1: float,
    tol: float = 1e-10,
    max_iter: int = 100,
) -> tuple[float, str, int]
```

Secant root-finding.

| Parameter  | Type                       | Description                                                |
| ---------- | -------------------------- | ---------------------------------------------------------- |
| `f`        | `Callable[[float], float]` | Function to find the root of (takes float, returns float). |
| `x0`       | `float`                    | First initial guess.                                       |
| `x1`       | `float`                    | Second initial guess.                                      |
| `tol`      | `float = 1e-10`            | Convergence tolerance (default 1e-10).                     |
| `max_iter` | `int = 100`                | Maximum iterations (default 100).                          |
| _Returns_  | `tuple[float, str, int]`   | `(root, status_string, iteration_count)`.                  |

### `secant_tracked()`

```python
secant_tracked(
    f: Any,
    x0: float,
    x1: float,
    tol: float = 1e-10,
    max_iter: int = 100,
) -> tuple[float, str, int, list[float]]
```

Secant with iteration history.

| Parameter  | Type                                  | Description                               |
| ---------- | ------------------------------------- | ----------------------------------------- |
| `f`        | `Any`                                 |                                           |
| `x0`       | `float`                               |                                           |
| `x1`       | `float`                               |                                           |
| `tol`      | `float = 1e-10`                       |                                           |
| `max_iter` | `int = 100`                           |                                           |
| _Returns_  | `tuple[float, str, int, list[float]]` | `(root, status, iterations, [estimates])` |
