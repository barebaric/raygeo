"""Tests for the cut-area stepper (StepperOptions, step, run_segment)
and the adaptive stepper (step_adaptive).
"""

import math

import pytest

from raygeo.ops.assembly.adaptive import target_area_per_distance
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.stepper import (
    StepperOptions,
    StepStatus,
    run_segment,
    step,
    step_adaptive,
    target_engagement_from_advance,
)

# ── Geometry helpers ────────────────────────────────────────────────


def _rect_wall():
    """Cleared area with a horizontal top edge at y=40."""
    ca = ClearedArea(boundary=[])
    ca.cut([[(-100, -100), (100, -100), (100, 40), (-100, 40)]])
    return ca


def _vertical_wall_cleared(wall_x: float = 0.0, span: float = 1000.0):
    """ClearedArea with everything to the LEFT of ``wall_x`` already cut.

    The wall runs vertically at ``x = wall_x``.  The tool should hug
    it from the left side.
    """
    ca = ClearedArea(boundary=[])
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


def test_stepper_options_default():
    opts = StepperOptions()
    assert opts.radius == 3.0
    assert opts.step_length == 0.6
    assert abs(opts.target_engagement - math.pi) < 1e-12
    assert opts.engagement_tol == 0.01
    assert abs(opts.max_deflection - math.pi / 6) < 1e-12
    assert opts.max_solver_iters == 6


def test_stepper_options_custom():
    opts = StepperOptions(
        radius=5.0,
        step_length=1.0,
        target_engagement=math.pi * 0.8,
        max_solver_iters=10,
    )
    assert opts.radius == 5.0
    assert opts.target_engagement == math.pi * 0.8
    assert opts.max_solver_iters == 10


def test_stepper_options_setters():
    opts = StepperOptions()
    opts.radius = 4.0
    opts.step_length = 0.5
    assert opts.radius == 4.0
    assert opts.step_length == 0.5


def test_step_parallel_to_wall():
    """Stepping along a flat wall should maintain Ok status."""
    ca = _rect_wall()
    opts = StepperOptions(
        radius=5.0,
        step_length=1.0,
        target_engagement=math.pi,
    )
    pos = (0.0, 40.0)
    heading = 0.0  # right

    for i in range(20):
        result = step(ca, pos, heading, opts)
        assert "Ok" in repr(result.status), (
            f"step {i} failed: {result.status}, eng={result.iters}"
        )
        pos = result.next
        heading = result.heading


def test_step_heading_toward_wall_deflects():
    """Steering toward the wall should deflect away."""
    ca = _rect_wall()
    opts = StepperOptions(
        radius=5.0,
        step_length=1.0,
        target_engagement=math.pi,
    )
    result = step(ca, (0.0, 40.0), math.pi / 2, opts)
    # Heading should change away from pi/2.
    assert abs(result.heading - math.pi / 2) > 0.01


def test_step_lost_engagement():
    """When fully inside cleared area, step returns LostEngagement."""
    ca = ClearedArea(boundary=[])
    ca.cut([[(-50, -50), (50, -50), (50, 50), (-50, 50)]])
    opts = StepperOptions(
        radius=5.0,
        step_length=1.0,
        target_engagement=math.pi,
    )
    result = step(ca, (0.0, 40.0), 0.0, opts)
    assert "LostEngagement" in repr(result.status)


def test_run_segment_returns_path():
    ca = _rect_wall()
    opts = StepperOptions(
        radius=5.0,
        step_length=1.0,
        target_engagement=math.pi,
    )
    path, status = run_segment(ca, (0.0, 40.0), 0.0, opts, 20)
    assert len(path) >= 2
    assert status == "Ok"


def test_run_segment_determinism():
    ca = _rect_wall()
    opts = StepperOptions()
    p1, _ = run_segment(ca, (0.0, 40.0), 0.0, opts, 10)
    p2, _ = run_segment(ca, (0.0, 40.0), 0.0, opts, 10)
    assert p1 == p2


def test_target_engagement():
    t = target_engagement_from_advance(1.25, 5.0)
    assert t > 0.0
    assert t <= 2.0 * math.pi


def test_target_engagement_saturates():
    t = target_engagement_from_advance(5.0, 5.0)
    assert abs(t - math.pi) < 1e-12


def test_target_engagement_zero():
    t = target_engagement_from_advance(0.0, 5.0)
    assert abs(t - math.pi) < 1e-12


def test_step_status_repr():
    s = StepStatus.ok()
    assert "Ok" in repr(s)


def test_step_result_attributes():
    ca = _rect_wall()
    opts = StepperOptions(
        radius=5.0,
        step_length=1.0,
        target_engagement=math.pi,
    )
    r = step(ca, (0.0, 40.0), 0.0, opts)
    assert len(r.next) == 2
    assert isinstance(r.heading, float)
    assert isinstance(r.iters, int)
    assert isinstance(r.status, StepStatus)


# ── Basic API (adaptive) ────────────────────────────────────────────


def test_step_adaptive_returns_step_result():
    """step_adaptive returns a StepResult with the expected fields."""
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()
    r = step_adaptive(
        cleared=ca,
        pos=(-advance, 0.0),
        heading=math.pi / 2,
        predicted_angle=0.0,
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
        angle_min=-math.pi / 4,
        angle_max=math.pi / 4,
    )
    assert len(r.next) == 2
    assert isinstance(r.heading, float)
    assert isinstance(r.iters, int)
    assert isinstance(r.iteration_angle, float)
    assert isinstance(r.status, StepStatus)


def test_step_adaptive_flat_wall_ok_status():
    """A single step along a flat wall should return Ok (not Lost)."""
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()
    r = step_adaptive(
        cleared=ca,
        pos=(-advance, 0.0),
        heading=math.pi / 2,
        predicted_angle=0.0,
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
        angle_min=-math.pi / 4,
        angle_max=math.pi / 4,
    )
    assert "Ok" in repr(r.status), f"Expected Ok, got {r.status}"


def test_step_adaptive_lost_engagement_in_open_space():
    """When the disk is fully inside the cleared area, the step is Lost."""
    R = 5.0
    step_length = 1.0
    ca = ClearedArea(boundary=[])
    # Huge cleared area, tool deep inside — no wall nearby.
    ca.cut([[(-1000, -1000), (1000, -1000), (1000, 1000), (-1000, 1000)]])
    valid = _huge_valid_area()
    r = step_adaptive(
        cleared=ca,
        pos=(0.0, 0.0),
        heading=0.0,
        predicted_angle=0.0,
        target_area_pd=2.0,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
        angle_min=-math.pi / 4,
        angle_max=math.pi / 4,
    )
    assert "Lost" in repr(r.status), f"Expected Lost, got {r.status}"


# ── Best-angle selection (Bug A regression) ─────────────────────────


def test_step_adaptive_returns_best_angle_not_last():
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
    r = step_adaptive(
        cleared=ca,
        pos=(-advance, 0.0),
        heading=math.pi / 2,
        predicted_angle=0.0,
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
        angle_min=-math.pi / 4,
        angle_max=math.pi / 4,
    )
    # If the solver used all 20 iterations, the best angle must not
    # equal zero (the trivial drift target).  If it converged early,
    # the angle is whatever was accepted — also non-zero here.
    assert abs(r.iteration_angle) > 1e-3, (
        f"Best angle is ~0; solver returned drift target. "
        f"angle={r.iteration_angle:.4f}, iters={r.iters}"
    )


# ── Convergence for non-zero angles (Bug B regression) ──────────────


def test_step_adaptive_converges_for_nonzero_optimal_angle():
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
    r = step_adaptive(
        cleared=ca,
        pos=(-advance, 0.0),
        heading=math.pi / 2,
        predicted_angle=0.0,
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
        angle_min=-math.pi / 4,
        angle_max=math.pi / 4,
    )
    # MAX_IT = 20 in the Rust source.  A healthy solver should converge
    # in 3-6 iterations on a flat wall.
    assert r.iters < 20, (
        f"Solver exhausted all 20 iterations on a flat wall — "
        f"the is_conv gate is blocking convergence. "
        f"angle={r.iteration_angle:.4f}"
    )


def test_step_adaptive_accepts_small_nonconventional_angle():
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
    r = step_adaptive(
        cleared=ca,
        pos=(-advance, 0.0),
        heading=math.pi / 2,
        predicted_angle=0.0,
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
        angle_min=-math.pi / 4,
        angle_max=math.pi / 4,
    )
    # The optimal angle for this geometry is ~0.14 rad (~8°), well
    # above the 0.03 conventional threshold.  The solver MUST accept
    # it and converge.
    assert r.iters < 20, (
        f"Solver exhausted iterations; is_conv blocked acceptance. "
        f"angle={r.iteration_angle:.4f}, iters={r.iters}"
    )


# ── Sequential stability ────────────────────────────────────────────


def test_step_adaptive_sequential_flat_wall_no_deflection_accumulation():
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

    pos = (-advance, 0.0)
    heading = math.pi / 2
    predicted = 0.0
    initial_heading = heading

    angles = []
    headings = []
    for i in range(30):
        r = step_adaptive(
            cleared=ca,
            pos=pos,
            heading=heading,
            predicted_angle=predicted,
            target_area_pd=target_apd,
            step_length=step_length,
            radius=R,
            max_deflection=max_def,
            valid_area=valid,
        )
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


def test_step_adaptive_determinism():
    """Same inputs produce identical output."""
    R = 5.0
    advance = 2.0
    step_length = 1.0
    target_apd = target_area_per_distance(R, advance, step_length)
    ca = _vertical_wall_cleared()
    valid = _huge_valid_area()

    args = (
        ca,
        (-advance, 0.0),
        math.pi / 2,
        0.0,
        target_apd,
        step_length,
        R,
        math.radians(30),
        valid,
        -math.pi / 4,
        math.pi / 4,
    )
    r1 = step_adaptive(*args)
    r2 = step_adaptive(*args)
    assert r1.next == r2.next
    assert r1.heading == r2.heading
    assert r1.iteration_angle == r2.iteration_angle
    assert r1.iters == r2.iters


def test_step_adaptive_converges_from_correct_depth():
    """Starting from a properly-offset position against a circular
    frontier, step_adaptive should converge quickly (not exhaust
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

    ca = ClearedArea(boundary=[])
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

    result = step_adaptive(
        cleared=ca,
        pos=start_pos,
        heading=0.0,
        predicted_angle=0.0,
        target_area_pd=target_apd,
        step_length=step_length,
        radius=R,
        max_deflection=math.radians(30),
        valid_area=valid,
        angle_min=-math.pi / 4,
        angle_max=math.pi / 4,
    )
    assert result.iters < 20, (
        f"Solver exhausted iterations at correct depth — "
        f"iters={result.iters}, angle={result.iteration_angle:.4f}"
    )
