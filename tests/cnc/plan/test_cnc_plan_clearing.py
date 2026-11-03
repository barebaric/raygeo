"""Tests for plan_clearing in cnc/plan/clearing."""

import math

import pytest

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
            if _point_in_polygon(pt, island) and (
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
    # Whole-floor probe: the machinable floor is covered except for the
    # sub-millimetre slivers a ~92% step-over strands between passes
    # (previously ~372 mm^2 stood uncut when the pass silently skipped).
    uncut = _uncut_area(rooms, [island], segments, 3.0, grid=0.5)
    assert uncut <= 12.0, (
        f"{uncut:.1f} mm^2 of the machinable floor is farther than the "
        "tool radius from every cut"
    )
