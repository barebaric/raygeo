---
title: raygeo.ops.cut.stepper
sidebar_label: raygeo.ops.cut.stepper
---

## StepResult

Result of a single forward step.

Contains the next centre position, updated heading, solver iteration count, and the final status.

### `cut_area`

```python
cut_area: float
```

The incremental cut area (crescent) for this step.

### `heading`

```python
heading: float
```

Updated heading angle in radians.

### `iteration_angle`

```python
iteration_angle: float
```

Solver steering angle (radians). Only non-zero for `step_adaptive`.

### `iters`

```python
iters: int
```

Number of solver iterations used.

### `next`

```python
next: tuple[float, float]
```

Next centre position `(x, y)`.

### `status`

```python
status: StepStatus
```

Step completion status.

## StepStatus

Status of a single step or cut segment.

One of `Ok` (normal), `BoundaryHit` (hit pocket boundary), `LostEngagement` (no uncut material), or
`NoConvergence` (solver failed to converge).

### `boundary_hit()`

```python
@classmethod boundary_hit() -> StepStatus
```

Hit pocket boundary.

| Parameter | Type         | Description               |
| --------- | ------------ | ------------------------- |
| _Returns_ | `StepStatus` | `StepStatus.boundary_hit` |

### `lost_engagement()`

```python
@classmethod lost_engagement() -> StepStatus
```

No uncut material found.

| Parameter | Type         | Description                  |
| --------- | ------------ | ---------------------------- |
| _Returns_ | `StepStatus` | `StepStatus.lost_engagement` |

### `no_convergence()`

```python
@classmethod no_convergence() -> StepStatus
```

Solver failed to converge.

| Parameter | Type         | Description                 |
| --------- | ------------ | --------------------------- |
| _Returns_ | `StepStatus` | `StepStatus.no_convergence` |

### `ok()`

```python
@classmethod ok() -> StepStatus
```

Normal step completion.

| Parameter | Type         | Description     |
| --------- | ------------ | --------------- |
| _Returns_ | `StepStatus` | `StepStatus.ok` |

## Functions

### `step_adaptive()`

```python
step_adaptive(
    cleared: cleared_area.ClearedArea,
    pos: tuple[float, float],
    heading: float,
    predicted_angle: float,
    target_area_pd: float,
    step_length: float,
    radius: float,
    max_deflection: float,
    valid_area: Sequence[Sequence[tuple[float, float]]],
    angle_min: float = -0.7853981633974483,
    angle_max: float = 0.7853981633974483,
    dir_sign: float = 0.0,
) -> StepResult
```

Perform one forward step using the area-based adaptive solver.

Like **step**, but targets **cut-area per unit distance** rather than an engagement angle. Used
internally by `adaptive_clearing`.

| Parameter         | Type                                      | Description                                                                                                                                                                                                                                                                              |
| ----------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cleared`         | `cleared_area.ClearedArea`                | `ClearedArea` instance.                                                                                                                                                                                                                                                                  |
| `pos`             | `tuple[float, float]`                     | Current centre position `(x, y)`.                                                                                                                                                                                                                                                        |
| `heading`         | `float`                                   | Smoothed heading angle (radians).                                                                                                                                                                                                                                                        |
| `predicted_angle` | `float`                                   | Predicted steering angle from history.                                                                                                                                                                                                                                                   |
| `target_area_pd`  | `float`                                   | Target cut-area per unit distance.                                                                                                                                                                                                                                                       |
| `step_length`     | `float`                                   | Forward step length in mm.                                                                                                                                                                                                                                                               |
| `radius`          | `float`                                   | Disk radius in mm.                                                                                                                                                                                                                                                                       |
| `max_deflection`  | `float`                                   | Max steering deflection in radians.                                                                                                                                                                                                                                                      |
| `valid_area`      | `Sequence[Sequence[tuple[float, float]]]` | Valid tool-centre region polygons.                                                                                                                                                                                                                                                       |
| `angle_min`       | `float = -0.7853981633974483`             | Minimum trial deflection angle in radians (default -π/4).                                                                                                                                                                                                                                |
| `angle_max`       | `float = 0.7853981633974483`              | Maximum trial deflection angle in radians (default +π/4).                                                                                                                                                                                                                                |
| `dir_sign`        | `float = 0.0`                             | Directional bias sign (default `0.0`). `+1.0` to prefer positive angles (CW), `−1.0` to prefer negative angles (CCW). The bias penalises fresh material on the wrong side when the tool breaks through a web between two cleared regions. Has no effect during normal one-sided cutting. |
| _Returns_         | `StepResult`                              | `StepResult` with the next position and updated heading.                                                                                                                                                                                                                                 |

![Wall following along four boundary shapes: curved, square wave, zig zag, and circle.](images/ops-cut-stepper-wall-following.png)

*Wall following along four boundary shapes: curved, square wave, zig zag, and circle.*

![90° corner: the solver deflects the heading to keep engagement constant around the turn.](images/ops-cut-stepper-pocket-corner.png)

*90° corner: the solver deflects the heading to keep engagement constant around the turn.*

![Engagement histogram for 200 steps along a straight wall. Tight peak near target indicates stable behaviour.](images/ops-cut-stepper-engagement-histogram.png)

*Engagement histogram for 200 steps along a straight wall. Tight peak near target indicates stable
behaviour.*
