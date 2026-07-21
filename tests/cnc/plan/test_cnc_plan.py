"""Tests for Plan / PlanStep in cnc/plan/plan."""

from raygeo.cnc.plan import Plan
from raygeo.cnc.plan.entry import plan_entry
from raygeo.ops.feature.region import find_regions


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _build_entry(boundary, **kwargs):
    tool_radius = kwargs.get("tool_radius", 3.0)
    regions = find_regions(
        boundary=boundary, islands=[], tool_radius=tool_radius
    )
    if not regions:
        return []
    poly, _area, entry_pt, r_max = regions[0]
    return plan_entry(poly, entry_pt, r_max, **kwargs)


def test_plan_empty_steps():
    p = Plan(_rect(-20, -20, 40, 40), safe_z=2.0)
    assert p.step_count == 0


def test_plan_has_safe_z():
    p = Plan(_rect(-20, -20, 40, 40), safe_z=3.0)
    assert p.safe_z == 3.0


def test_plan_rectangle_has_steps():
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(
        boundary, tool_radius=3.0, step_over=2.0, safe_z=2.0, target_z=-5.0
    )
    assert len(steps) > 0
    for s in steps:
        assert isinstance(s.face_id, str)
        assert s.kind


def test_plan_step_basic():
    """Single step created directly."""

    # We don't expose a public PlanStep constructor in Python yet,
    # so we test through plan_entry which returns them.
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(boundary, tool_radius=3.0)
    assert len(steps) > 0
    s = steps[0]
    assert s.face_id == ""
    assert s.kind in ("helix", "spiral", "ramp", "toroidal_clear")
    assert isinstance(s.spec_params(), dict)
