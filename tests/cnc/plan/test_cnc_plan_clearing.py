"""Tests for plan_clearing in cnc/plan/clearing."""

from raygeo.cnc.plan.clearing import plan_clearing
from raygeo.ops.part import Part


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _dumbbell():
    return [
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 12.5),
        (50.0, 12.5),
        (50.0, 17.5),
        (30.0, 17.5),
        (30.0, 30.0),
        (0.0, 30.0),
    ]


def _plan(boundary, **kwargs):
    part = Part.from_polygons(boundary, [], (0.0, 0.0))
    return plan_clearing(part, "", **kwargs)


def test_plan_clearing_returns_plan():
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0)
    assert plan.step_count > 0


def test_plan_clearing_has_adaptive():
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0)
    kinds = [s.kind for s in plan.steps]
    assert "adaptive_clearing" in kinds


def test_plan_clearing_has_entry():
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0)
    kinds = [s.kind for s in plan.steps]
    entry_kinds = {"helix", "spiral", "ramp", "toroidal_clear"}
    assert len(entry_kinds & set(kinds)) > 0, f"no entry step in {kinds}"


def test_plan_clearing_with_finishing():
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0, finishing=True)
    kinds = [s.kind for s in plan.steps]
    assert "profile_inner" in kinds


def test_plan_clearing_without_finishing():
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0, finishing=False)
    kinds = [s.kind for s in plan.steps]
    assert "profile_inner" not in kinds


def test_plan_clearing_no_retract():
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0)
    kinds = [s.kind for s in plan.steps]
    assert "retract" not in kinds


def test_plan_clearing_dumbbell():
    boundary = _dumbbell()
    plan = _plan(boundary, tool_radius=3.0)
    kinds = [s.kind for s in plan.steps]
    assert "adaptive_clearing" in kinds


def test_plan_clearing_step_details():
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0)
    for s in plan.steps:
        assert s.face_id == ""
        assert isinstance(s.kind, str)
        assert isinstance(s.spec_params(), dict)
