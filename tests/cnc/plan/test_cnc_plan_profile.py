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


def test_side_selects_outer_profile():
    _, plan = _plan(_rect(0, 0, 40, 30), side="outer")
    assert [s.kind for s in plan.steps] == ["profile_outer"]


def test_cut_direction_reaches_the_step_params():
    _, plan = _plan(_rect(0, 0, 40, 30))
    params = plan.steps[0].spec_params()
    assert params["cut_direction"] == "Ccw"
    _, plan = _plan(_rect(0, 0, 40, 30), cut_direction="cw")
    assert plan.steps[0].spec_params()["cut_direction"] == "Cw"


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
