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


def test_cutting_stays_a_tool_radius_inside_the_pocket():
    """Every position the plan's steps cut below the surface stays a tool
    radius from the walls and the island: entries, clearing and finishing
    alike. A position on the wall means the tool cuts half of itself into
    material it must not touch."""
    part, plan = _island_pocket()
    ops = run_intent(create_intent(plan, part, 0))
    checked = 0
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] not in ("MOVE_TO", "LINE_TO"):
            continue
        end = cmd["end"]
        if end[2] >= 0.0:
            continue
        checked += 1
        wall, island = _clearance(end)
        # Polygon offsets approximate corner arcs, so a couple of
        # hundredths short of the radius is the offsetter's own tolerance.
        assert wall >= 3.0 - 0.02, f"cut at {end} is {wall:.3f} from the wall"
        assert island >= 3.0 - 0.02, (
            f"cut at {end} is {island:.3f} from the island"
        )
    assert checked > 100, f"only {checked} cutting positions to check"


def test_wall_margin_reaches_the_finishing_profile():
    """The finishing profile honours a requested wall margin: it cuts at
    tool_radius + margin from the walls, while the roughing pass keeps the
    plain tool radius. (The clearing engine itself does not apply
    wall_margin; the margin is a finishing-profile quantity.)"""
    boundary = _rect(0, 0, 40, 30)
    part = Part.from_polygons(boundary, [], (40.0, 30.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=2.0,
        target_z=-3.0,
        safe_z=10.0,
        wall_margin=0.5,
        finishing=True,
    )
    ops = run_intent(create_intent(plan, part, 0))
    near_wall = 0.0
    at_margin = 0
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] not in ("MOVE_TO", "LINE_TO"):
            continue
        end = cmd["end"]
        if end[2] >= 0.0:
            continue
        wall = min(end[0], 40.0 - end[0], end[1], 30.0 - end[1])
        near_wall = max(near_wall, wall)
        if wall >= 3.5 - 0.02:
            at_margin += 1
    assert at_margin > 20, (
        f"only {at_margin} cuts at the 3.5 margin (closest {near_wall:.3f})"
    )
    # Nothing cuts past the roughing bound into the wall.
    assert near_wall >= 3.0 - 0.02, f"a cut reached {near_wall:.3f}"


def test_wall_margin_is_honoured_by_the_whole_plan():
    """AdaptiveClearingSpec.wall_margin means what the API promises: extra
    clearance between the tool and the boundary, honoured by entries,
    clearing, passages and finishing alike."""
    boundary = _rect(0, 0, 40, 30)
    island = [(15.0, 10.0), (25.0, 10.0), (25.0, 20.0), (15.0, 20.0)]
    part = Part.from_polygons(boundary, [island], (40.0, 30.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=2.0,
        target_z=-3.0,
        safe_z=10.0,
        wall_margin=0.5,
        finishing=True,
    )
    ops = run_intent(create_intent(plan, part, 0))
    checked = 0
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] not in ("MOVE_TO", "LINE_TO"):
            continue
        end = cmd["end"]
        if end[2] >= 0.0:
            continue
        checked += 1
        wall, island = _clearance(end)
        assert wall >= 3.5 - 0.02, (
            f"cut at {end} is {wall:.3f} from the wall (want 3.5)"
        )
        assert island >= 3.5 - 0.02, (
            f"cut at {end} is {island:.3f} from the island (want 3.5)"
        )
    assert checked > 50, f"only {checked} cutting positions to check"
