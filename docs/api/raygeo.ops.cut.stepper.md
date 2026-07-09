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

Solver steering angle (radians). Only non-zero for `step`.

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

## StepperOptions

Configuration options for **step**.

Holds the constant parameters for the adaptive stepper.

### `angle_max`

```python
angle_max: float
```

Maximum trial deflection angle in radians.

### `angle_min`

```python
angle_min: float
```

Minimum trial deflection angle in radians.

### `dir_sign`

```python
dir_sign: float
```

Directional bias sign: `+1.0` for CW, `-1.0` for CCW, `0.0` for no bias.

### `max_deflection`

```python
max_deflection: float
```

Maximum steering deflection in radians.

### `radius`

```python
radius: float
```

Disk radius in mm.

### `step_length`

```python
step_length: float
```

Forward step length in mm.

### `target_area_pd`

```python
target_area_pd: float
```

Target cut-area per unit distance.

### `valid_area`

```python
valid_area: list[list[tuple[float, float]]]
```

Valid tool-centre region polygons.

## Functions

### `step()`

```python
step(
    cleared: cleared_area.ClearedArea,
    pos: tuple[float, float],
    heading: float,
    predicted_angle: float,
    opts: StepperOptions,
) -> StepResult
```

Perform one forward step using the area-based adaptive solver.

| Parameter         | Type                       | Description                                              |
| ----------------- | -------------------------- | -------------------------------------------------------- |
| `cleared`         | `cleared_area.ClearedArea` | `ClearedArea` instance.                                  |
| `pos`             | `tuple[float, float]`      | Current centre position `(x, y)`.                        |
| `heading`         | `float`                    | Smoothed heading angle (radians).                        |
| `predicted_angle` | `float`                    | Predicted steering angle from history.                   |
| `opts`            | `StepperOptions`           | `StepperOptions` instance with fixed parameters.         |
| _Returns_         | `StepResult`               | `StepResult` with the next position and updated heading. |

![Wall following along four boundary shapes: curved, square wave, zig zag, and circle.](images/ops-cut-stepper-wall-following.png)

*Wall following along four boundary shapes: curved, square wave, zig zag, and circle.*

![90° corner: the solver deflects the heading to keep engagement constant around the turn.](images/ops-cut-stepper-pocket-corner.png)

*90° corner: the solver deflects the heading to keep engagement constant around the turn.*

![Engagement histogram on a straight wall. Tight peak near target shows stable behaviour.](images/ops-cut-stepper-engagement-histogram.png)

*Engagement histogram on a straight wall. Tight peak near target shows stable behaviour.*
