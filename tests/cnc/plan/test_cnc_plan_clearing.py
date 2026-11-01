"""Tests for plan_clearing in cnc/plan/clearing."""

from raygeo.cnc.execution.intent import create_intent, run_intent
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


def _island_pocket():
    boundary = _rect(0, 0, 40, 30)
    island = [(15.0, 10.0), (25.0, 10.0), (25.0, 20.0), (15.0, 20.0)]
    part = Part.from_polygons(boundary, [island], (40.0, 30.0))
    return part, plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=2.0,
        target_z=-4.0,
        safe_z=10.0,
        finishing=True,
    )


def _clearance(p):
    wall = min(p[0], 40.0 - p[0], p[1], 30.0 - p[1])
    dx = max(15.0 - p[0], 0.0, p[0] - 25.0)
    dy = max(10.0 - p[1], 0.0, p[1] - 20.0)
    return wall, (dx * dx + dy * dy) ** 0.5


def test_region_scoped_steps_do_not_machine_the_island():
    """Region-scoped steps (entries, adaptive clearing, finishing) used to
    swap the face's stock region for a hole-free boundary, so a pocket with
    an island was machined straight through it. The scopes carry the
    islands now: nothing cuts inside the island."""
    part, plan = _island_pocket()
    ops = run_intent(create_intent(plan, part, 0))
    inside = 0
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] not in ("MOVE_TO", "LINE_TO"):
            continue
        end = cmd["end"]
        if end[2] >= 0.0:
            continue
        # Strictly inside the island polygon (a margin in from its edge,
        # so points merely near a wall are not counted).
        if 15.1 < end[0] < 24.9 and 10.1 < end[1] < 19.9:
            inside += 1
    assert inside == 0, f"{inside} cutting positions inside the island"
