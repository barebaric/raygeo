"""Tests for the thread-milling plan builder."""

import math

import pytest

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.thread_milling import plan_thread_milling
from raygeo.ops.part import Part


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


NOMINAL, PITCH, TOOL = 6.0, 1.0, 4.0
ORBIT_R = (NOMINAL - TOOL) / 2.0


def _part():
    # Two M6 pilot bores (4.2 mm) plus a non-circular island.
    return Part.from_polygons(
        _rect(0, 0, 100, 60),
        [_circle(30, 30, 2.1), _circle(70, 30, 2.1), _rect(10, 45, 15, 10)],
        (100.0, 60.0),
    )


def _plan(part, **kwargs):
    kwargs.setdefault("nominal_diameter", NOMINAL)
    kwargs.setdefault("pitch", PITCH)
    kwargs.setdefault("tool_diameter", TOOL)
    kwargs.setdefault("z_top", 0.0)
    kwargs.setdefault("z_bottom", -12.0)
    return plan_thread_milling(part, **kwargs)


def test_detects_pilot_bores():
    plan = _plan(_part())
    assert [s.kind for s in plan.steps] == ["thread_mill", "thread_mill"]
    centers = [s.spec_params()["center"] for s in plan.steps]
    assert (30.0, 30.0) == pytest.approx(centers[0], abs=1e-2)
    assert (70.0, 30.0) == pytest.approx(centers[1], abs=1e-2)


def test_oversized_bore_is_rejected():
    part = Part.from_polygons(
        _rect(0, 0, 100, 60), [_circle(30, 30, 2.6)], (100.0, 60.0)
    )
    # A 5.2 mm bore leaves nothing for an M6x1 thread (minor ≈ 4.5 mm).
    with pytest.raises(RuntimeError, match="oversized"):
        _plan(part)


def test_tool_larger_than_bore_is_rejected():
    part = Part.from_polygons(
        _rect(0, 0, 100, 60), [_circle(30, 30, 2.1)], (100.0, 60.0)
    )
    # A 5 mm mill does not enter the 4.2 mm pilot bore.
    with pytest.raises(RuntimeError, match="no holes found"):
        _plan(part, tool_diameter=5.0)


def test_explicit_centers_skip_validation():
    plan = _plan(_part(), holes=[(30.0, 30.0)])
    assert plan.step_count == 1


def test_non_integer_thread_length_is_rejected():
    with pytest.raises(RuntimeError, match="integer multiple"):
        _plan(_part(), holes=[(30.0, 30.0)], z_bottom=-12.5)


def test_handedness_reaches_the_step_params():
    plan = _plan(_part(), holes=[(30.0, 30.0)])
    assert plan.steps[0].spec_params()["handedness"] == "RH"
    lh = _plan(_part(), holes=[(30.0, 30.0)], handedness="lh")
    assert lh.steps[0].spec_params()["handedness"] == "LH"


def test_plan_runs_as_helical_arcs_at_the_orbit_radius():
    part = _part()
    plan = _plan(part, holes=[(30.0, 30.0)])
    ops = run_intent(create_intent(plan, part, 0))
    cmds = ops.to_dict()["commands"]

    arcs = [c for c in cmds if c["type"] == "ARC_TO"]
    # Lead-in + 24 half-turns (12 revolutions) + lead-out.
    assert len(arcs) == 26
    helix = arcs[1:-1]
    for arc in helix:
        x, y, z = arc["end"]
        assert math.hypot(x - 30.0, y - 30.0) == pytest.approx(
            ORBIT_R, abs=1e-6
        ), f"arc off the orbit radius at {(x, y, z)}"
    # The helix climbs one pitch per revolution from the bottom to the top.
    assert helix[0]["end"][2] == pytest.approx(-11.5)
    assert helix[-1]["end"][2] == pytest.approx(0.0)
