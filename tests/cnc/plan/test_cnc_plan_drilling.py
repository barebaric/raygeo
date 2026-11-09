"""Tests for the drilling plan builder."""

import math

import pytest

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.drilling import plan_drilling
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


def _part_with_holes():
    return Part.from_polygons(
        _rect(0, 0, 100, 60),
        [_circle(30, 30, 2.5), _circle(70, 30, 3.0), _rect(10, 45, 15, 10)],
        (100.0, 60.0),
    )


def _plan(part, **kwargs):
    kwargs.setdefault("cycle", "plain")
    kwargs.setdefault("diameter", 5.0)
    kwargs.setdefault("target_z", -10.0)
    return plan_drilling(part, **kwargs)


def test_detects_circular_islands_and_skips_the_rest():
    plan = _plan(_part_with_holes())
    assert [s.kind for s in plan.steps] == ["plain_drill", "plain_drill"]
    centers = [s.spec_params()["center"] for s in plan.steps]
    assert (30.0, 30.0) == pytest.approx(centers[0], abs=1e-2)
    assert (70.0, 30.0) == pytest.approx(centers[1], abs=1e-2)


def test_explicit_centers_skip_detection():
    plan = _plan(_part_with_holes(), holes=[(10.0, 10.0), (90.0, 10.0)])
    assert plan.step_count == 2
    centers = [s.spec_params()["center"] for s in plan.steps]
    assert centers == [(10.0, 10.0), (90.0, 10.0)]


def test_cycle_kind_names_the_steps():
    part = _part_with_holes()
    assert [s.kind for s in _plan(part, cycle="spot").steps] == [
        "spot_drill",
        "spot_drill",
    ]
    peck = _plan(part, cycle="peck", peck_increment=4.0)
    assert [s.kind for s in peck.steps] == ["peck_drill", "peck_drill"]
    params = peck.steps[0].spec_params()
    assert params["peck_mode"] == "FULL_RETRACT"
    assert params["peck_increment"] == 4.0
    chip = _plan(part, cycle="chip_break", peck_increment=4.0)
    assert [s.kind for s in chip.steps] == [
        "chip_break_drill",
        "chip_break_drill",
    ]
    assert chip.steps[0].spec_params()["peck_mode"] == "CHIP_BREAK"


def test_no_detection_hits_are_rejected():
    part = Part.from_polygons(
        _rect(0, 0, 40, 30), [_rect(5, 5, 10, 10)], (40.0, 30.0)
    )
    with pytest.raises(RuntimeError, match="no holes found"):
        _plan(part)


def test_empty_explicit_hole_list_is_rejected():
    with pytest.raises(RuntimeError, match="empty"):
        _plan(_part_with_holes(), holes=[])


def test_missing_face_is_rejected():
    with pytest.raises(RuntimeError, match="no face"):
        _plan(_part_with_holes(), face_id="does-not-exist")


def test_undersized_islands_are_skipped_in_detection_mode():
    part = Part.from_polygons(
        _rect(0, 0, 100, 60), [_circle(30, 30, 1.0)], (100.0, 60.0)
    )
    with pytest.raises(RuntimeError, match="no holes found"):
        _plan(part, diameter=5.0)


def test_nearest_neighbor_order_minimises_travel():
    plan = _plan(
        _part_with_holes(),
        holes=[(90.0, 10.0), (10.0, 10.0), (50.0, 10.0)],
        hole_order="nearest_neighbor",
    )
    centers = [s.spec_params()["center"] for s in plan.steps]
    assert centers[0] == (90.0, 10.0)  # first hole stays first
    assert centers[1] == (50.0, 10.0)  # then the nearest neighbour
    assert centers[2] == (10.0, 10.0)


def test_feeds_spindle_and_coolant_reach_the_cut_state():
    part = _part_with_holes()
    plan = _plan(
        part,
        feed_rate=1000.0,
        plunge_feed=300.0,
        spindle_rpm=4000,
        coolant="flood",
    )
    ops = run_intent(create_intent(plan, part, 0))
    cmds = ops.to_dict()["commands"]
    speeds = {c["speed"] for c in cmds if c["type"] == "SET_FEED_RATE"}
    assert {1000, 300} <= speeds
    assert any(c["type"] == "SET_SPINDLE_RPM" for c in cmds)
    assert any(c["type"] == "SET_COOLANT" for c in cmds)


def test_plan_runs_with_safe_travel_between_holes():
    part = _part_with_holes()
    plan = _plan(part)
    ops = run_intent(create_intent(plan, part, 0))
    cmds = ops.to_dict()["commands"]

    # Both cycles are wrapped in their own DRILL_CYCLE section.
    sections = [
        c["section_type"] for c in cmds if c["type"] == "OPS_SECTION_START"
    ]
    assert sections == ["DRILL_CYCLE", "DRILL_CYCLE"]

    moves = [
        (c["type"], c["end"])
        for c in cmds
        if c["type"] in ("MOVE_TO", "LINE_TO")
    ]
    # Both holes are drilled (feed moves below the surface at their
    # centers).
    for cx, cy in ((30.0, 30.0), (70.0, 30.0)):
        assert any(
            t == "LINE_TO"
            and end[2] < 0.0
            and math.hypot(end[0] - cx, end[1] - cy) < 1e-6
            for t, end in moves
        ), f"hole at ({cx}, {cy}) was not drilled"
    # Between the cycles the tool retracts to the safe height before
    # travelling (the cycle's final retract, then the link retract).
    first_done = next(
        i
        for i, (t, end) in enumerate(moves)
        if t == "LINE_TO" and end[2] < 0.0
    )
    retract = moves[first_done + 1]
    assert retract[0] == "MOVE_TO" and retract[1][2] == 5.0
    travel = [m for m in moves[first_done + 2 : first_done + 4]]
    assert all(m[0] == "MOVE_TO" for m in travel), moves[first_done + 1 :]


def test_peck_plan_produces_repeated_plunges():
    part = _part_with_holes()
    plan = _plan(
        part,
        holes=[(30.0, 30.0)],
        cycle="peck",
        peck_increment=4.0,
        target_z=-12.0,
        tip_angle_deg=90.0,
    )
    ops = run_intent(create_intent(plan, part, 0))
    zs = [
        c["end"][2]
        for c in ops.to_dict()["commands"]
        if c["type"] == "LINE_TO"
    ]
    # 90° point: tip to -14.5; four 4 mm pecks from the surface.
    assert zs == [
        pytest.approx(-4.0),
        pytest.approx(-8.0),
        pytest.approx(-12.0),
        pytest.approx(-14.5),
    ]
