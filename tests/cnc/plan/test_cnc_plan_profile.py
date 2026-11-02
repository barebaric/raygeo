"""Tests for the profile plan builder."""

import math

import pytest

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.profile import plan_profile
from raygeo.geo import Geometry
from raygeo.ops.part import Part


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _circle_geometry(r, cx=0.0, cy=0.0):
    g = Geometry()
    g.move_to(cx + r, cy, 0)
    g.arc_to(cx - r, cy, -r, 0, False, 0)
    g.arc_to(cx + r, cy, r, 0, False, 0)
    return g


def _plan(boundary, **kwargs):
    part = Part.from_polygons(boundary, [], (0.0, 0.0))
    return part, plan_profile(part, **kwargs)


def test_plan_has_one_profile_step():
    _, plan = _plan(_rect(0, 0, 40, 30))
    assert plan.step_count == 1
    kinds = [s.kind for s in plan.steps]
    assert kinds == ["profile_inner"]


def test_ramp_angle_prepends_an_entry_step():
    _, plan = _plan(_rect(0, 0, 40, 30), ramp_angle_deg=30.0)
    assert [s.kind for s in plan.steps] == ["profile_ramp", "profile_inner"]
    params = plan.steps[0].spec_params()
    assert params["ramp_angle"] == pytest.approx(math.radians(30.0))
    assert params["z_start"] == plan.safe_z


def test_zero_ramp_angle_keeps_the_single_step():
    _, plan = _plan(_rect(0, 0, 40, 30), ramp_angle_deg=0.0)
    assert [s.kind for s in plan.steps] == ["profile_inner"]


def test_side_selects_outer_profile():
    _, plan = _plan(_rect(0, 0, 40, 30), side="outer")
    assert [s.kind for s in plan.steps] == ["profile_outer"]


def test_cut_direction_reaches_the_step_params():
    _, plan = _plan(_rect(0, 0, 40, 30))
    params = plan.steps[0].spec_params()
    assert params["cut_direction"] == "Ccw"
    _, plan = _plan(_rect(0, 0, 40, 30), cut_direction="cw")
    assert plan.steps[0].spec_params()["cut_direction"] == "Cw"


def test_step_over_above_the_tool_diameter_is_rejected():
    part = Part.from_polygons(_rect(0, 0, 40, 30), [], (60.0, 50.0))
    with pytest.raises(RuntimeError, match="step_over"):
        plan_profile(part, tool_radius=3.0, step_over=7.0)
    # At exactly the diameter the passes still fully overlap.
    plan_profile(part, tool_radius=3.0, step_over=6.0)


def test_missing_face_is_rejected():
    part = Part.from_polygons(_rect(0, 0, 40, 30), [], (60.0, 50.0))
    with pytest.raises(RuntimeError, match="no face"):
        plan_profile(part, "does-not-exist")


def test_boundaryless_part_is_rejected():
    with pytest.raises(RuntimeError, match="boundary"):
        plan_profile(Part(geometry=Geometry(), size_mm=(0.0, 0.0)))


def test_plan_runs_to_arcs_at_the_exact_offset():
    """The built plan executes and, on curve-true geometry, cuts a circle
    at exactly R − tool_radius in arcs."""
    part = Part(geometry=_circle_geometry(30.0), size_mm=(80.0, 80.0))
    plan = plan_profile(part, tool_radius=3.0)
    ops = run_intent(create_intent(plan, part, 0))
    arcs = [c for c in ops.to_dict()["commands"] if c["type"] == "ARC_TO"]
    assert arcs, "no arcs emitted"
    for c in arcs:
        radius = math.hypot(c["end"][0], c["end"][1])
        assert radius == pytest.approx(27.0, abs=1e-6), (
            f"cut at radius {radius}"
        )


def test_feed_rate_flows_into_the_run():
    part = Part.from_polygons(_rect(0, 0, 40, 30), [], (60.0, 50.0))
    plan = plan_profile(part, feed_rate=1200.0)
    ops = run_intent(create_intent(plan, part, 0))
    feeds = [
        c for c in ops.to_dict()["commands"] if c["type"] == "SET_FEED_RATE"
    ]
    assert feeds, "no feed rate emitted"
    assert feeds[0]["speed"] == 1200


def test_plunge_feed_precedes_the_descent():
    """The descent into material runs at the plunge feed; the ring cuts at
    the cut feed afterwards."""
    part = Part.from_polygons(_rect(0, 0, 40, 30), [], (60.0, 50.0))
    plan = plan_profile(part, feed_rate=1200.0, plunge_feed=400.0)
    ops = run_intent(create_intent(plan, part, 0))
    seq = [
        (c["type"], c.get("speed"))
        for c in ops.to_dict()["commands"]
        if c["type"] in ("SET_FEED_RATE", "LINE_TO")
    ]
    descents = [
        i
        for i, (t, speed) in enumerate(seq)
        if t == "LINE_TO" and i > 0 and seq[i - 1] == ("SET_FEED_RATE", 400.0)
    ]
    assert descents, f"no plunge-fed descent in {seq}"
    assert seq[descents[0] + 1] == ("SET_FEED_RATE", 1200.0), (
        "cut feed not restored after the plunge"
    )


def test_ramp_entry_descends_along_the_loop():
    """With a ramp angle, the tool enters by descending along the profile
    loop — helical arc moves at the loop radius with gradually sinking z —
    at the plunge feed, and the wall pass still cuts the full ring on the
    exact offset."""
    part = Part(geometry=_circle_geometry(30.0), size_mm=(80.0, 80.0))
    plan = plan_profile(
        part,
        tool_radius=3.0,
        ramp_angle_deg=30.0,
        feed_rate=1200.0,
        plunge_feed=400.0,
    )
    assert [s.kind for s in plan.steps] == ["profile_ramp", "profile_inner"]
    ops = run_intent(create_intent(plan, part, 0))
    cmds = ops.to_dict()["commands"]

    # Ramp moves: cutting moves at heights strictly between the safe and
    # target heights. They ride the loop radius, and at least one is a
    # helical arc (the loop is a circle).
    ramp_moves = [
        c
        for c in cmds
        if c["type"] in ("LINE_TO", "ARC_TO") and -5.0 < c["end"][2] < 2.0
    ]
    assert ramp_moves, "no gradual-descent moves found"
    for c in ramp_moves:
        x, y, _ = c["end"]
        assert math.hypot(x, y) == pytest.approx(27.0, abs=1e-6), (
            f"ramp cut off the loop radius at {c['end']}"
        )
    assert any(c["type"] == "ARC_TO" for c in ramp_moves), (
        "the ramp over the circular loop should emit helical arcs"
    )

    # The ramp runs at the plunge feed: a plunge-feed rate is set before
    # the first ramp move and the cut feed comes back after the last one.
    idx_by_id = {id(c): i for i, c in enumerate(cmds)}
    first, last = idx_by_id[id(ramp_moves[0])], idx_by_id[id(ramp_moves[-1])]
    rates_before = [
        c["speed"] for c in cmds[:first] if c["type"] == "SET_FEED_RATE"
    ]
    rates_between = [
        c["speed"]
        for c in cmds[first : last + 1]
        if c["type"] == "SET_FEED_RATE"
    ]
    assert 400 in rates_before + rates_between, (
        "the ramp did not run at the plunge feed"
    )
    rates_after = [
        c["speed"] for c in cmds[last + 1 :] if c["type"] == "SET_FEED_RATE"
    ]
    assert 1200 in rates_after, "cut feed not restored after the ramp"

    # The wall pass still cuts the full ring on the exact offset.
    ring = [
        c
        for c in cmds
        if c["type"] in ("LINE_TO", "ARC_TO") and c["end"][2] == -5.0
    ]
    assert len(ring) > 10, "the wall pass did not cut the ring"
    for c in ring:
        radius = math.hypot(c["end"][0], c["end"][1])
        assert radius == pytest.approx(27.0, abs=1e-6), (
            f"ring cut at radius {radius}"
        )
