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


def _two_room_dumbbell(corridor_w):
    """Two rooms connected by a corridor of the given width."""
    cy = 20 - corridor_w / 2
    return [
        (0, 0),
        (30, 0),
        (30, cy),
        (50, cy),
        (50, 0),
        (80, 0),
        (80, 40),
        (50, 40),
        (50, cy + corridor_w),
        (30, cy + corridor_w),
        (30, 40),
        (0, 40),
    ]


def test_dead_end_slot_passage_is_cut_to_width():
    """A dead-end corridor the slot pass is assigned (here: 6.2 mm wide for
    a 6 mm tool), planned without a wall margin as this scenario is
    designed, is cut to width: the slot carrier erodes by the plain tool
    radius."""
    cy = 20 - 6.2 / 2
    boundary = [
        (0, 0),
        (30, 0),
        (30, cy),
        (50, cy),
        (50, cy + 6.2),
        (30, cy + 6.2),
        (30, 40),
        (0, 40),
    ]
    part = Part.from_polygons(boundary, [], (50.0, 40.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=2.0,
        target_z=-4.0,
        safe_z=10.0,
        finishing=True,
    )
    kinds = [s.kind for s in plan.steps]
    assert "slot" in kinds, f"no slot step in {kinds}"
    ops = run_intent(create_intent(plan, part, 0))
    in_corridor = [
        cmd["end"]
        for cmd in ops.to_dict()["commands"]
        if cmd["type"] in ("MOVE_TO", "LINE_TO")
        and cmd["end"][2] < 0.0
        and cmd["end"][0] > 30.5
        and cy + 1.0 < cmd["end"][1] < cy + 6.2 - 1.0
    ]
    assert in_corridor, "the corridor is never entered"
    for end in in_corridor:
        wall = min(end[1] - cy, cy + 6.2 - end[1], 50.0 - end[0])
        assert wall >= 3.0 - 0.02, (
            f"corridor cut at {end} is {wall:.3f} from the wall"
        )


def test_passage_too_narrow_for_the_margin_is_left_uncut():
    """A 6.2 mm corridor cannot hold a 6 mm tool plus a 0.5 mm margin on
    both sides (tool + 2 x margin = 7 mm): the passage pass honours the
    margin and cuts nothing in the corridor — the material stays for a
    later pass or a smaller tool — while both rooms are still machined
    inside the margin. The profile inset pinches the corridor closed, so
    the finishing profiles are scoped per room; both rooms keep theirs."""
    bnd = _two_room_dumbbell(6.2)
    part = Part.from_polygons(bnd, [], (80.0, 40.0))
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
    assert "slot" not in [s.kind for s in plan.steps]
    profiles = [s for s in plan.steps if s.kind == "profile_inner"]
    assert len(profiles) == 2, (
        f"one finishing profile per room, got {len(profiles)}"
    )
    ops = run_intent(create_intent(plan, part, 0))
    in_left_room = in_far_room = 0
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] not in ("MOVE_TO", "LINE_TO"):
            continue
        end = cmd["end"]
        if end[2] >= 0.0:
            continue
        assert not (30.0 < end[0] < 50.0), (
            f"the margin-blocked corridor was cut at {end}"
        )
        if end[0] < 30.0:
            in_left_room += 1
        elif end[0] > 50.0:
            in_far_room += 1
    assert in_left_room > 0, "the entry room was never machined"
    assert in_far_room > 0, "the far room was never machined"
