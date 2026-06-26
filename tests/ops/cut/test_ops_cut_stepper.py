"""Tests for the cut-area stepper (StepperOptions, step, run_segment)."""

import math

from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.stepper import (
    StepperOptions,
    StepStatus,
    run_segment,
    step,
    target_engagement_from_advance,
)


def _rect_wall():
    """Cleared area with a horizontal top edge at y=40."""
    ca = ClearedArea(boundary=[])
    ca.cut([[(-100, -100), (100, -100), (100, 40), (-100, 40)]])
    return ca


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
