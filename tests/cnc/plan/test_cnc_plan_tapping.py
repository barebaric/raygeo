"""Tests for the rigid-tapping plan builder."""

import math

import pytest

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.tapping import plan_tapping
from raygeo.ops.part import Part

NOMINAL, PITCH = 6.0, 1.0


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _circle(cx, cy, r, n=96):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _part():
    # Two M6 tap-drill bores (5.0 mm) plus a non-circular island.
    return Part.from_polygons(
        _rect(0, 0, 100, 60),
        [_circle(30, 30, 2.5), _circle(70, 30, 2.5), _rect(10, 45, 15, 10)],
        (100.0, 60.0),
    )


def _plan(part, **kwargs):
    kwargs.setdefault("nominal_diameter", NOMINAL)
    kwargs.setdefault("pitch", PITCH)
    kwargs.setdefault("target_z", -12.0)
    kwargs.setdefault("spindle_rpm", 500)
    return plan_tapping(part, **kwargs)


def test_detects_pilot_bores():
    plan = _plan(_part())
    assert [s.kind for s in plan.steps] == ["tap", "tap"]
    centers = [s.spec_params()["center"] for s in plan.steps]
    assert (30.0, 30.0) == pytest.approx(centers[0], abs=1e-2)
    assert (70.0, 30.0) == pytest.approx(centers[1], abs=1e-2)


def test_missing_spindle_rpm_is_rejected():
    # The synchronized feed is pitch * rpm, so tapping without a
    # spindle speed is meaningless.
    with pytest.raises(RuntimeError, match="spindle"):
        _plan(_part(), spindle_rpm=None)
    with pytest.raises(RuntimeError, match="spindle"):
        _plan(_part(), spindle_rpm=0)


def test_oversized_bore_is_rejected():
    part = Part.from_polygons(
        _rect(0, 0, 100, 60), [_circle(30, 30, 3.01)], (100.0, 60.0)
    )
    # A bore of 6.02 mm is at/above the 6 mm nominal diameter: the
    # thread is already there, nothing left to tap.
    with pytest.raises(RuntimeError, match="nothing left to tap"):
        _plan(part)


def test_explicit_centers_skip_detection():
    plan = _plan(_part(), holes=[(30.0, 30.0)])
    assert plan.step_count == 1


def test_handedness_reaches_the_step_params():
    plan = _plan(_part(), holes=[(30.0, 30.0)])
    assert plan.steps[0].spec_params()["handedness"] == "RH"
    lh = _plan(_part(), holes=[(30.0, 30.0)], handedness="lh")
    assert lh.steps[0].spec_params()["handedness"] == "LH"


def test_peck_reaches_the_step_params():
    plan = _plan(_part(), holes=[(30.0, 30.0)], peck=4.0, dwell_ms=20.0)
    params = plan.steps[0].spec_params()
    assert params["peck_increment"] == pytest.approx(4.0)
    assert params["dwell_ms"] == pytest.approx(20.0)


def test_nearest_neighbor_order():
    plan = _plan(
        _part(),
        holes=[(70.0, 30.0), (10.0, 10.0), (30.0, 30.0)],
        hole_order="nearest_neighbor",
    )
    centers = [s.spec_params()["center"] for s in plan.steps]
    assert centers[0] == pytest.approx((70.0, 30.0))  # first hole stays first
    assert centers[1] == pytest.approx((30.0, 30.0))  # nearest neighbour
    assert centers[2] == pytest.approx((10.0, 10.0))


def test_plan_runs_as_synchronized_cycles():
    part = _part()
    plan = _plan(part, holes=[(30.0, 30.0), (70.0, 30.0)], coolant="flood")
    ops = run_intent(create_intent(plan, part, 0))
    cmds = ops.to_dict()["commands"]

    sections = [
        c
        for c in cmds
        if c["type"] == "OPS_SECTION_START"
        and c["section_type"] == "TAPPING_CYCLE"
    ]
    assert len(sections) == 2, "one tapping cycle per bore"
    # Per hole: clockwise down, counter-clockwise up, clockwise restored.
    directions = [
        c["direction"] for c in cmds if c["type"] == "SET_SPINDLE_DIRECTION"
    ]
    assert directions == ["Cw", "Ccw", "Cw"] * 2
    # The spindle and coolant reach the stream via the plan cut state.
    assert any(
        c["type"] == "SET_SPINDLE_RPM" and c["spindle_speed"] == 500
        for c in cmds
    )
    assert any(c["type"] == "SET_COOLANT" for c in cmds)
    # Between holes the retract lifts to the safe plane.
    safe_lifts = [
        c
        for c in cmds
        if c["type"] == "MOVE_TO" and c["end"][2] == pytest.approx(5.0)
    ]
    assert len(safe_lifts) >= 4, "retract to safe_z per hole missing"
