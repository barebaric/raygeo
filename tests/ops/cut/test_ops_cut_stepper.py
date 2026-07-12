"""Tests for the adaptive stepper (step)."""

import math

import pytest

from raygeo.ops.assembly.adaptive import target_area_per_distance
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.stepper import (
    StepperOptions,
    StepStatus,
    step,
)

# ── Geometry helpers ────────────────────────────────────────────────


def _vertical_wall_cleared(wall_x: float = 0.0, span: float = 1000.0):
    """ClearedArea with everything to the LEFT of ``wall_x`` already cut.

    The wall runs vertically at ``x = wall_x``.  The tool should hug
    it from the left side.
    """
    ca = ClearedArea()
    ca.cut(
        [
            [
                (wall_x - span, -span),
                (wall_x, -span),
                (wall_x, span),
                (wall_x - span, span),
            ]
        ]
    )
    return ca


def _huge_valid_area(span: float = 1000.0):
    """A single huge rectangle that admits any candidate position."""
    return [[(-span, -span), (span, -span), (span, span), (-span, span)]]


def test_step_status_repr():
    s = StepStatus.ok()
    assert "Ok" in repr(s)


# ── Basic API (adaptive) ────────────────────────────────────────────


def test_step_returns_step_result():
    """step returns a StepResult with the expected fields."""
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()
    opts = StepperOptions(
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
    )
    r = step(
        ca,
        (-advance, 0.0),
        math.pi / 2,
        0.0,
        opts,
    )
    assert len(r.next) == 2
    assert isinstance(r.heading, float)
    assert isinstance(r.iters, int)
    assert isinstance(r.iteration_angle, float)
    assert isinstance(r.status, StepStatus)


def test_step_flat_wall_ok_status():
    """A single step along a flat wall should return Ok (not Lost)."""
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()
    opts = StepperOptions(
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
    )
    r = step(ca, (-advance, 0.0), math.pi / 2, 0.0, opts)
    assert "Ok" in repr(r.status), f"Expected Ok, got {r.status}"


def test_step_lost_engagement_in_open_space():
    """When the disk is fully inside the cleared area, the step is Lost."""
    R = 5.0
    step_length = 1.0
    ca = ClearedArea()
    # Huge cleared area, tool deep inside — no wall nearby.
    ca.cut([[(-1000, -1000), (1000, -1000), (1000, 1000), (-1000, 1000)]])
    valid = _huge_valid_area()
    opts = StepperOptions(
        target_area_pd=2.0,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
    )
    r = step(ca, (0.0, 0.0), 0.0, 0.0, opts)
    assert "Lost" in repr(r.status), f"Expected Lost, got {r.status}"


# ── Best-angle selection (Bug A regression) ─────────────────────────


def test_step_returns_best_angle_not_last():
    """The returned ``iteration_angle`` must be the one with the smallest
    ``|error|`` across all iterations, not the last one tried.

    On a flat wall, the optimal angle is non-zero (the tool must steer
    slightly away from the wall to maintain the target engagement).
    The interpolation solver tends to drift toward angle=0 in later
    iterations, so the last iteration is usually worse than the best.
    """
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()
    opts = StepperOptions(
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
    )
    r = step(ca, (-advance, 0.0), math.pi / 2, 0.0, opts)
    # If the solver used all 20 iterations, the best angle must not
    # equal zero (the trivial drift target).  If it converged early,
    # the angle is whatever was accepted — also non-zero here.
    assert abs(r.iteration_angle) > 1e-3, (
        f"Best angle is ~0; solver returned drift target. "
        f"angle={r.iteration_angle:.4f}, iters={r.iters}"
    )


# ── Convergence for non-zero angles (Bug B regression) ──────────────


def test_step_converges_for_nonzero_optimal_angle():
    """The solver must converge (use < MAX_IT iterations) when the
    optimal steering angle is non-zero but small.

    On a flat wall with advance=2, R=5, the optimal angle is ~0.19 rad
    (~11°).  The ``is_conv`` gate (angle > 0.03) must NOT prevent
    convergence when the error is within tolerance.
    """
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()
    opts = StepperOptions(
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
    )
    r = step(ca, (-advance, 0.0), math.pi / 2, 0.0, opts)
    # MAX_IT = 20 in the Rust source.  A healthy solver should converge
    # in 3-6 iterations on a flat wall.
    assert r.iters < 20, (
        f"Solver exhausted all 20 iterations on a flat wall — "
        f"the is_conv gate is blocking convergence. "
        f"angle={r.iteration_angle:.4f}"
    )


def test_step_accepts_small_nonconventional_angle():
    """When the error is within ``max_err`` but the angle is > 0.03 rad,
    the solver should accept it rather than exhausting iterations.

    This is the core regression test for Bug B: the ``!is_conv`` term
    in the break condition.
    """
    R = 5.0
    advance = 1.5
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()
    opts = StepperOptions(
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
    )
    r = step(ca, (-advance, 0.0), math.pi / 2, 0.0, opts)
    # The optimal angle for this geometry is ~0.14 rad (~8°), well
    # above the 0.03 conventional threshold.  The solver MUST accept
    # it and converge.
    assert r.iters < 20, (
        f"Solver exhausted iterations; is_conv blocked acceptance. "
        f"angle={r.iteration_angle:.4f}, iters={r.iters}"
    )


# ── Sequential stability ────────────────────────────────────────────


def test_step_sequential_flat_wall_no_deflection_accumulation():
    """Run 30 steps along a flat wall.  The heading should stay close
    to the initial heading (π/2) and the path should not curl.

    If the solver returns bad angles, the gyro/predicted-angle feedback
    loops compound the error and the path diverges.
    """
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    max_def = math.radians(30)

    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()

    opts = StepperOptions(
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=max_def,
        valid_area=valid,
    )

    pos = (-advance, 0.0)
    heading = math.pi / 2
    predicted = 0.0
    initial_heading = heading

    angles = []
    headings = []
    for i in range(30):
        r = step(ca, pos, heading, predicted, opts)
        if "Lost" in repr(r.status):
            pytest.fail(f"Step {i} lost engagement at pos={pos}")
        angles.append(r.iteration_angle)
        headings.append(r.heading)
        pos = r.next
        heading = r.heading
        predicted = r.iteration_angle

    # The heading should not drift more than ~20° from the initial.
    max_drift = max(abs(h - initial_heading) for h in headings)
    assert max_drift < math.radians(20), (
        f"Heading drifted {math.degrees(max_drift):.1f}° from initial "
        f"over 30 steps.  Angles: {[f'{a:.3f}' for a in angles[:10]]}..."
    )

    # The path should progress roughly in +y (the wall direction).
    # Total y displacement should be positive and substantial.
    y_progress = pos[1]  # final y
    assert y_progress > 10.0, (
        f"Path did not progress along the wall; final y={y_progress:.2f}"
    )


# ── Determinism ─────────────────────────────────────────────────────


def test_step_determinism():
    """Same inputs produce identical output."""
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()

    opts = StepperOptions(
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
    )
    args = (ca, (-advance, 0.0), math.pi / 2, 0.0, opts)
    r1 = step(*args)
    r2 = step(*args)
    assert r1.next == r2.next
    assert r1.heading == r2.heading
    assert r1.iteration_angle == r2.iteration_angle
    assert r1.iters == r2.iters


def test_step_converges_from_correct_depth():
    """Starting from a properly-offset position against a circular
    frontier, step should converge quickly (not exhaust
    iterations) because the tool is already at the correct depth.
    """
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)

    # Circle of radius 15 centred at (50, 40).  The tool should be
    # offset inward from the frontier by R - advance = 3.
    cx, cy, cr = 50.0, 40.0, 15.0
    offset_dist = cr - (R - advance)
    start_pos = (cx, cy + offset_dist)  # directly above centre

    ca = ClearedArea()
    n = 32
    ca.cut(
        [
            [
                (
                    cx + cr * math.cos(2 * math.pi * i / n),
                    cy + cr * math.sin(2 * math.pi * i / n),
                )
                for i in range(n)
            ]
        ]
    )

    valid = _huge_valid_area()

    opts = StepperOptions(
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
    )
    result = step(ca, start_pos, 0.0, 0.0, opts)
    assert result.iters < 20, (
        f"Solver exhausted iterations at correct depth — "
        f"iters={result.iters}, angle={result.iteration_angle:.4f}"
    )
