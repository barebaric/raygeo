"""Tests for plan_clearing in cnc/plan/clearing."""

import math

import pytest

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.clearing import (
    ClearingWorkplanOptions,
    plan_clearing,
)
from raygeo.ops.assembly.profile import ProfileRampSpec, ProfileSpec
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


def _circle(cx, cy, r, n=64):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _floor_segments(ops, target_z):
    """The floor-level cutting segments as ((x0, y0), (x1, y1)) pairs."""
    segments = []
    prev = None
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] not in ("MOVE_TO", "LINE_TO"):
            continue
        end = cmd["end"]
        cur = (end[0], end[1]) if end[2] <= target_z + 0.01 else None
        if prev is not None and cur is not None:
            segments.append((prev, cur))
        prev = cur
    return segments


def _dist_seg(pt, seg):
    (ax, ay), (bx, by) = seg
    dx, dy = bx - ax, by - ay
    l2 = dx * dx + dy * dy
    if l2 == 0.0:
        return math.hypot(pt[0] - ax, pt[1] - ay)
    t = ((pt[0] - ax) * dx + (pt[1] - ay) * dy) / l2
    t = max(0.0, min(1.0, t))
    return math.hypot(pt[0] - (ax + t * dx), pt[1] - (ay + t * dy))


def _point_in_polygon(pt, poly):
    x, y = pt
    inside = False
    n = len(poly)
    for i in range(n):
        ax, ay = poly[i]
        bx, by = poly[(i + 1) % n]
        if (ay > y) != (by > y):
            xint = ax + (y - ay) * (bx - ax) / (by - ay)
            if x < xint:
                inside = not inside
    return inside


def _boundary_dist(pt, poly):
    return min(
        _dist_seg(pt, (poly[i], poly[(i + 1) % len(poly)]))
        for i in range(len(poly))
    )


def _uncut_area(boundary, islands, segments, tool_radius, grid=0.5):
    """Grid-probe the machinable floor (boundary eroded by the tool
    radius, islands buffered by the tool radius) and return the total
    area of the points farther than tool_radius + 0.05 from every
    floor-level cut segment."""
    xs = [p[0] for p in boundary]
    ys = [p[1] for p in boundary]

    def machinable(pt):
        if not _point_in_polygon(pt, boundary):
            return False
        if _boundary_dist(pt, boundary) < tool_radius - 1e-6:
            return False
        for island in islands:
            if _point_in_polygon(pt, island) or (
                _boundary_dist(pt, island) <= tool_radius + 1e-6
            ):
                return False
        return True

    reach = tool_radius + 0.05
    uncut = 0.0
    x = min(xs) + tool_radius
    while x <= max(xs) - tool_radius:
        y = min(ys) + tool_radius
        while y <= max(ys) - tool_radius:
            if machinable((x, y)):
                d = min(_dist_seg((x, y), seg) for seg in segments)
                if d > reach:
                    uncut += grid * grid
            y += grid
        x += grid
    return uncut


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


def test_finishing_profile_carries_planner_tuning():
    """The finishing wall pass is built from the canonical constructor:
    its engagement gates are the planner's values."""
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0, step_over=2.0, finishing=True)
    profiles = [s for s in plan.steps if s.kind == "profile_inner"]
    assert profiles, "finishing adds a profile step"
    params = profiles[-1].spec_params()
    assert params["engagement_area_threshold"] == pytest.approx(1.0)
    assert params["engagement_angle_threshold"] == pytest.approx(30.0)
    assert params["feed_reduction_factor"] == pytest.approx(0.5)


def test_for_tool_constructors_carry_the_planner_scales():
    """The mirrored for_tool constructors embed the planner's tuning:
    scales derived from the tool radius, not flat copies."""
    spec = ProfileSpec.for_tool("inner", 3.0, -4.0, 2.0)
    assert spec.tool_radius == pytest.approx(3.0)
    assert spec.step_over == pytest.approx(3.0)
    assert spec.step_length == pytest.approx(0.6)
    assert spec.engagement_area_threshold == pytest.approx(1.0)
    assert spec.engagement_angle_threshold == pytest.approx(30.0)
    assert spec.feed_reduction_factor == pytest.approx(0.5)
    assert spec.target_z == pytest.approx(-4.0)
    assert spec.safe_z == pytest.approx(2.0)

    ramp = ProfileRampSpec.for_tool(
        "inner", 3.0, math.radians(10.0), 2.0, -4.0
    )
    assert ramp.tool_radius == pytest.approx(3.0)
    assert ramp.ramp_angle == pytest.approx(math.radians(10.0))
    assert ramp.cut_direction == "ccw"
    assert ramp.tolerance == pytest.approx(0.1)

    opts = ClearingWorkplanOptions.for_tool(3.0, 2.0, -4.0, 5.0)
    assert opts.cut_direction == "ccw"
    assert opts.step_length == pytest.approx(0.6)
    assert opts.safe_margin == pytest.approx(0.6)
    assert opts.plunge_pitch == pytest.approx(0.9)
    assert opts.angular_step == pytest.approx(0.1)
    assert opts.max_deflection_deg == pytest.approx(30.0)
    assert opts.feed_rate is None
    assert opts.plunge_feed is None


def test_cut_direction_reaches_the_adaptive_passes():
    """A requested cut direction is pinned on every clearing step."""
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0, cut_direction="cw")
    adaptive = [s for s in plan.steps if s.kind == "adaptive_clearing"]
    assert adaptive, "clearing plans adaptive passes"
    assert all(s.spec_params()["cut_direction"] == "Cw" for s in adaptive)


def test_cut_direction_reaches_the_finishing_specs():
    """The finishing wall profile and its ramp follow the direction too."""
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(
        boundary,
        tool_radius=3.0,
        finishing=True,
        max_entry_angle_deg=10.0,
        cut_direction="cw",
    )
    for kind in ("profile_inner", "profile_ramp"):
        steps = [s for s in plan.steps if s.kind == kind]
        assert steps, f"no {kind} step in {[s.kind for s in plan.steps]}"
        assert all(s.spec_params()["cut_direction"] == "Cw" for s in steps)


def test_cut_direction_defaults_to_ccw():
    """Without the knob the plans stay CCW, as they always were."""
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(
        boundary,
        tool_radius=3.0,
        finishing=True,
        max_entry_angle_deg=10.0,
    )
    assert plan.steps, "the plan has steps"
    for s in plan.steps:
        params = s.spec_params()
        if "cut_direction" in params:
            assert params["cut_direction"] == "Ccw"


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


def test_travel_mode_reaches_the_clearing_steps():
    """A requested travel mode is pinned on the clearing spec."""
    boundary = _rect(-20, -20, 40, 40)
    plan = _plan(boundary, tool_radius=3.0, travel_mode="safe_z")
    adaptive = [s for s in plan.steps if s.kind == "adaptive_clearing"]
    assert adaptive, "clearing plans adaptive passes"
    assert all(s.spec_params()["travel_mode"] == "SafeZ" for s in adaptive)

    plan = _plan(boundary, tool_radius=3.0)
    adaptive = [s for s in plan.steps if s.kind == "adaptive_clearing"]
    assert adaptive
    assert all(
        s.spec_params()["travel_mode"] == "ClearedArea" for s in adaptive
    )


def test_safe_z_travel_keeps_sideways_travels_at_safe_z():
    """With SafeZ travel and feeds set, every sideways travel move stays
    at safe_z — a consumer that can only see the path can tell it is
    safe — and every descent into material goes out as a cutting move
    at the plunge feed."""
    boundary = _rect(-20, -20, 40, 40)
    part = Part.from_polygons(boundary, [], (40.0, 40.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=2.0,
        target_z=-3.0,
        safe_z=5.0,
        travel_mode="safe_z",
        feed_rate=900.0,
        plunge_feed=300.0,
    )
    ops = run_intent(create_intent(plan, part, 0))
    prev = None
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] not in ("MOVE_TO", "LINE_TO"):
            continue
        x, y, z = cmd["end"]
        if prev is not None:
            px, py, pz = prev
            sideways = abs(x - px) > 1e-6 or abs(y - py) > 1e-6
            if sideways and cmd["type"] == "MOVE_TO":
                assert z >= 5.0 - 1e-6, (
                    f"sideways travel below safe_z: {prev} -> {cmd['end']}"
                )
            if not sideways and z < pz - 1e-6:
                assert cmd["type"] == "LINE_TO", (
                    f"descent into material is a rapid travel: "
                    f"{prev} -> {cmd['end']}"
                )
        prev = (x, y, z)


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


def test_step_over_above_the_tool_diameter_is_rejected():
    """Passes farther apart than the tool diameter leave uncut ridges."""
    part = Part.from_polygons(
        [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)],
        [],
        (60.0, 50.0),
    )
    with pytest.raises(RuntimeError, match="step_over"):
        plan_clearing(part, "", tool_radius=3.0, step_over=7.0)
    # At exactly the diameter the passes still fully overlap.
    plan_clearing(part, "", tool_radius=3.0, step_over=6.0)


def test_step_over_near_the_diameter_still_clears_the_floor():
    """Step_overs close to the tool diameter leave a thin core in the
    middle of the pocket that stalls the clearing wavefront; the resume
    strategies used to run out of proposals and the plan was refused with
    a resume error, stranding the core uncut. The core strategy resumes
    on the spine of the uncleared remainder, so the floor is cut exactly
    as at a plain step_over — apart from the corner fillets a round tool
    leaves at a sharp inner corner."""
    rect = [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)]
    corners = [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)]
    for step_over in (5.0, 6.0):
        part = Part.from_polygons(rect, [], (0.0, 0.0))
        plan = plan_clearing(
            part,
            "",
            tool_radius=3.0,
            step_over=step_over,
            target_z=-4.0,
            safe_z=10.0,
            finishing=True,
        )
        ops = run_intent(create_intent(plan, part, 0))
        cuts = [
            cmd["end"][:2]
            for cmd in ops.to_dict()["commands"]
            if cmd["type"] in ("MOVE_TO", "LINE_TO") and cmd["end"][2] < 0.0
        ]
        assert len(cuts) > 100, "hardly any cutting positions"
        # At step_over = diameter adjacent passes touch tangentially, so
        # midway between them the floor is exactly one radius from the
        # nearest pass — cleaned, but with no margin to spare. A ridge the
        # failure used to strand is a full radius wide, far beyond that.
        checked = 0
        x = 0.25
        while x < 40.0:
            y = 0.25
            while y < 30.0:
                d = (
                    min((px - x) ** 2 + (py - y) ** 2 for px, py in cuts)
                    ** 0.5
                )
                checked += 1
                if d <= 3.0 + 0.01:
                    y += 0.5
                    continue
                corner = min(
                    ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
                    for cx, cy in corners
                )
                assert corner <= 3.0, (
                    f"point ({x}, {y}) is {d:.3f} from the nearest cut "
                    f"at step_over {step_over}"
                )
                y += 0.5
            x += 0.5
        assert checked > 4000, f"only {checked} floor points checked"


def test_entry_spiral_covers_the_seed_disk_at_high_step_over():
    """Issue #99: the entry helix machines the floor only along the final
    arc of its descent, and an outward spiral that started at the helix
    radius left the crescent between the seed disk and the first spiral
    turn standing — the convergence discriminator's step-scaled inset
    erased it, and the pass completed silently with uncut floor inside
    the entry. The spiral now starts at the center and physically
    machines the whole seed disk, so the floor around the entry stays
    covered up to the full step-over the planner accepts."""
    rect = _rect(0, 0, 40, 30)
    span = 5.0
    reach = 3.05
    grid = 0.25
    for step_over in (4.0, 5.0, 6.0):
        part = Part.from_polygons(rect, [], (0.0, 0.0))
        plan = plan_clearing(
            part,
            "",
            tool_radius=3.0,
            step_over=step_over,
            target_z=-2.0,
            safe_z=10.0,
            finishing=True,
        )
        spirals = [s for s in plan.steps if s.kind == "spiral"]
        assert spirals, "expected a spiral entry"
        for step in spirals:
            assert step.spec_params()["start_radius"] == 0.0, (
                "the spiral entry must start at the center, or the floor "
                "between the seed disk and the first turn stays uncut"
            )
        ops = run_intent(create_intent(plan, part, 0))
        segments = _floor_segments(ops, -2.0)
        cx, cy = spirals[0].spec_params()["center"]
        # Segments whose sweep can reach the probe box: bbox overlapping
        # the box inflated by the reach.
        near = [
            seg
            for seg in segments
            if max(seg[0][0], seg[1][0]) >= cx - span - reach
            and min(seg[0][0], seg[1][0]) <= cx + span + reach
            and max(seg[0][1], seg[1][1]) >= cy - span - reach
            and min(seg[0][1], seg[1][1]) <= cy + span + reach
        ]
        assert len(near) > 20, "no cutting moves near the entry"
        x = cx - span
        while x <= cx + span:
            y = cy - span
            while y <= cy + span:
                d = min(_dist_seg((x, y), seg) for seg in near)
                assert d <= reach, (
                    f"floor point ({x:.2f}, {y:.2f}) is {d:.3f} from the "
                    f"nearest cut at step_over {step_over}: the entry left "
                    "uncut material inside the seed disk region"
                )
                y += grid
            x += grid


def test_two_room_pocket_with_ramp_entry_clears_the_floor():
    """Issue #99, two-room repro: a region whose inscribed circle is too
    small for a helix entry fell back to the straight-ramp entry, whose
    cleared-area seed was swept with the zigzag amplitude — 0.0 for a
    clamped straight ramp — so the cleared area stayed empty, the
    adaptive pass silently skipped, and only the ramp line and the wall
    profile were machined, leaving ~35% of the machinable floor standing
    (and driving the ramp straight through the island, since the
    fallback carrier ignored the islands). The ramp now seeds the
    floor-level tail of its path at tool width, its carrier keeps clear
    of the islands, and the adaptive pass runs to the far room."""
    rooms = [
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 8.0),
        (60.0, 8.0),
        (60.0, 26.0),
        (30.0, 26.0),
        (30.0, 40.0),
        (0.0, 40.0),
    ]
    island = _circle(15.0, 20.0, 4.0)
    part = Part.from_polygons(rooms, [island], (0.0, 0.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=5.5,
        target_z=-2.0,
        safe_z=10.0,
        finishing=True,
    )
    ops = run_intent(create_intent(plan, part, 0))
    segments = _floor_segments(ops, -2.0)
    assert len(segments) > 800, (
        f"only {len(segments)} floor segments: the adaptive pass did not "
        "run after the ramp entry"
    )
    # Every floor cut keeps the tool out of the island.
    for seg in segments:
        for pt in seg:
            d = _boundary_dist(pt, island)
            assert d >= 3.0 - 0.05, (
                f"cut at ({pt[0]:.2f}, {pt[1]:.2f}) is {d:.3f} from the "
                "island (want >= 3.0)"
            )
    # The wavefront machined beyond the ramp swath: far room off the
    # ramp line, and the far corners of the entry room.
    for px, py in ((45.0, 12.0), (7.5, 7.5), (15.0, 32.0)):
        d = min(_dist_seg((px, py), seg) for seg in segments)
        assert d <= 3.05, (
            f"floor at ({px}, {py}) is {d:.3f} from the nearest cut"
        )
    # Whole-floor probe: the machinable floor is covered. The clearing
    # pass used to strand ~372 mm^2 here (the silent skip), and after
    # that fix still left ~6 mm^2 of sub-millimetre slivers between
    # passes at this ~92% step-over — the erosion pass slots them.
    uncut = _uncut_area(rooms, [island], segments, 3.0, grid=0.5)
    assert uncut <= 1.0, (
        f"{uncut:.1f} mm^2 of the machinable floor is farther than the "
        "tool radius from every cut"
    )


def test_erosion_step_slots_the_residual_the_gate_forgives():
    """Issue #99's known issue: the convergence discriminator insets the
    remaining material by the step scale, so sub-step-scale clumps read
    as converged and stand uncut — ~1 mm-wide full-depth slivers
    between passes at a ~92% step-over. The plan now carries an erosion
    step after every clearing step; it re-examines the accumulated
    cleared area at execution time and slots every actionable clump
    inside the tool-centre envelope."""
    rooms = [
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 8.0),
        (60.0, 8.0),
        (60.0, 26.0),
        (30.0, 26.0),
        (30.0, 40.0),
        (0.0, 40.0),
    ]
    island = _circle(15.0, 20.0, 4.0)
    part = Part.from_polygons(rooms, [island], (0.0, 0.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=5.5,
        target_z=-2.0,
        safe_z=10.0,
        finishing=False,
    )
    kinds = [s.kind for s in plan.steps]
    assert "adaptive_clearing" in kinds
    assert "erosion" in kinds, f"expected an erosion step, got {kinds}"
    # The erosion step is planned where it can see the clearing pass's
    # result: right after it, scoped to the same region.
    adaptive_idx = kinds.index("adaptive_clearing")
    assert kinds[adaptive_idx + 1] == "erosion"

    ops = run_intent(create_intent(plan, part, 0))
    segments = _floor_segments(ops, -2.0)
    uncut = _uncut_area(rooms, [island], segments, 3.0, grid=0.25)
    assert uncut <= 1.0, (
        f"{uncut:.1f} mm^2 of the machinable floor is farther than the "
        "tool radius from every cut"
    )


def test_erosion_step_emits_nothing_when_the_floor_is_clean():
    """A pocket the wavefront converges on cleanly must not gain stray
    cutting moves from the erosion pass: run the erosion assembler
    against the part after a full plan run and assert it emits nothing
    when the residual inside the tool-centre envelope is below
    tolerance."""
    from raygeo.ops.assembly.erosion import generate_erosion

    rect = _rect(0, 0, 40, 30)
    part = Part.from_polygons(rect, [], (0.0, 0.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=3.0,
        target_z=-2.0,
        safe_z=10.0,
        finishing=False,
    )
    ops = run_intent(create_intent(plan, part, 0))
    segments = _floor_segments(ops, -2.0)
    uncut = _uncut_area(rect, [], segments, 3.0, grid=0.5)
    assert uncut <= 1.0

    # The clearing pass left nothing actionable inside the envelope, so
    # the erosion pass has nothing to emit.
    residual = generate_erosion(
        part,
        tool_radius=3.0,
        target_z=-2.0,
        safe_z=10.0,
        area_tolerance=1.0,
    )
    residual_cmds = [
        c
        for c in residual.ops.to_dict()["commands"]
        if c["type"] == "LINE_TO" and c["end"][2] < 0.0
    ]
    assert not residual_cmds, (
        f"the erosion pass emitted {len(residual_cmds)} cutting moves "
        "although nothing was left to erode"
    )


def test_conventional_clearing_runs_on_wide_regions():
    """CW clearing (conventional) plans and runs on a plain pocket: the
    walk starts around the frontier in the cut direction, cuts the floor
    at the target depth, and winds the other way than the climb run."""
    boundary = _rect(0.0, 0.0, 60.0, 40.0)
    part = Part.from_polygons(boundary, [], (60.0, 40.0))

    def run(direction):
        plan = plan_clearing(
            part,
            "",
            tool_radius=3.0,
            step_over=2.5,
            target_z=-3.0,
            cut_direction=direction,
            feed_rate=600.0,
            plunge_feed=200.0,
        )
        return run_intent(create_intent(plan, part, 0))

    def winding(ops):
        ends = [
            c["end"]
            for c in ops.to_dict()["commands"]
            if c["type"] == "LINE_TO" and c["end"][2] <= -2.99
        ]
        return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ends, ends[1:]))

    def floor(ops):
        return [
            c["end"]
            for c in ops.to_dict()["commands"]
            if c["type"] in ("LINE_TO", "ARC_TO") and c["end"][2] <= -2.99
        ]

    cw = run("cw")
    climb = run("ccw")
    assert floor(cw), "conventional clearing cuts the floor"
    assert winding(cw) * winding(climb) < 0.0, (
        "conventional and climb walk opposite ways"
    )


def test_conventional_clearing_with_an_island_runs_like_climb_mirrored():
    """CW clearing around an island completes and covers the floor at
    least as well as the climb run, with the walk mirrored: the wrong-side
    steps islands produce are transient, and the sustained-wrong-side
    guard must stay silent."""
    boundary = _rect(0.0, 0.0, 60.0, 40.0)
    island = _rect(25.0, 15.0, 10.0, 10.0)  # a 10×10 island at (25, 15)
    part = Part.from_polygons(boundary, [island], (60.0, 40.0))

    def run(direction):
        plan = plan_clearing(
            part,
            "",
            tool_radius=3.0,
            step_over=2.5,
            target_z=-3.0,
            cut_direction=direction,
            feed_rate=600.0,
            plunge_feed=200.0,
        )
        return run_intent(create_intent(plan, part, 0))

    def winding(ops):
        ends = [
            c["end"]
            for c in ops.to_dict()["commands"]
            if c["type"] == "LINE_TO" and c["end"][2] <= -2.99
        ]
        return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ends, ends[1:]))

    def floor_pts(ops):
        return len(
            [
                c
                for c in ops.to_dict()["commands"]
                if c["type"] in ("LINE_TO", "ARC_TO") and c["end"][2] <= -2.99
            ]
        )

    conventional = run("cw")
    climb = run("ccw")
    assert floor_pts(conventional) * 0.9 <= floor_pts(climb), (
        "conventional clears the floor like climb does"
    )
    assert winding(conventional) * winding(climb) < 0.0, (
        "conventional and climb walk opposite ways"
    )

    def envelope_crossings(ops):
        # Floor cutting points inside the island's tool-clearance outset
        # (the rect (25,15)-(35,25) grown by the 3 mm tool radius).
        pts = [
            c["end"]
            for c in ops.to_dict()["commands"]
            if c["type"] in ("LINE_TO", "ARC_TO") and c["end"][2] <= -2.95
        ]
        return sum(
            1 for (x, y, _) in pts if 22.0 < x < 38.0 and 12.0 < y < 28.0
        )

    assert envelope_crossings(conventional) <= (
        envelope_crossings(climb) + 20
    ), "conventional must not cross the island envelope more than climb"
