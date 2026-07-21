"""Tests for plan_entry in cnc/plan/entry."""

import math

from raygeo.cnc.plan.entry import plan_entry
from raygeo.geo.shape.polygon import is_point_inside_polygon
from raygeo.ops.feature.region import find_regions


def _build_entry(boundary, islands=None, **kwargs):
    tool_radius = kwargs.get("tool_radius", 3.0)
    regions = find_regions(
        boundary=boundary,
        islands=islands or [],
        tool_radius=tool_radius,
    )
    if not regions:
        return []
    poly, _area, entry_pt, r_max = regions[0]
    return plan_entry(
        poly,
        entry_pt,
        r_max,
        islands=islands or [],
        **kwargs,
    )


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _dumbbell():
    return [
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 12.5),
        (50.0, 12.5),
        (50.0, 0.0),
        (80.0, 0.0),
        (80.0, 30.0),
        (50.0, 30.0),
        (50.0, 17.5),
        (30.0, 17.5),
        (30.0, 30.0),
        (0.0, 30.0),
    ]


def test_entry_workplan_wide_rectangle():
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(
        boundary, tool_radius=3.0, step_over=2.0, safe_z=2.0, target_z=-5.0
    )
    kinds = [s.kind for s in steps]
    assert "helix" in kinds, f"expected helix, got {kinds}"
    assert "spiral" in kinds, f"expected spiral, got {kinds}"
    assert "ramp" not in kinds, f"unexpected ramp in wide rect: {kinds}"


def test_entry_workplan_tight_slot():
    boundary = _rect(0, 0, 40, 8)
    steps = _build_entry(
        boundary, tool_radius=3.0, step_over=2.0, safe_z=2.0, target_z=-5.0
    )
    kinds = [s.kind for s in steps]
    assert "helix" not in kinds, f"unexpected helix in tight slot: {kinds}"


def test_entry_workplan_dumbbell():
    boundary = _dumbbell()
    steps = _build_entry(
        boundary, tool_radius=3.0, step_over=2.0, safe_z=2.0, target_z=-5.0
    )
    plunge_count = sum(1 for s in steps if s.kind == "helix")
    spiral_count = sum(1 for s in steps if s.kind == "spiral")
    assert plunge_count == 1, f"expected 1 helix, got {plunge_count}"
    assert spiral_count == 1, f"expected 1 spiral, got {spiral_count}"


def test_entry_workplan_steps_have_kind():
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(boundary, tool_radius=3.0)
    for s in steps:
        assert s.kind, f"step missing kind: {s}"


def test_entry_workplan_islands_optional():
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(boundary, tool_radius=3.0)
    assert len(steps) >= 1


def test_entry_workplan_step_over_zero():
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(
        boundary, tool_radius=3.0, step_over=0.0, safe_z=2.0, target_z=-5.0
    )
    kinds = [s.kind for s in steps]
    assert "spiral" not in kinds, (
        f"unexpected spiral with step_over=0: {kinds}"
    )
    assert "helix" in kinds, f"expected helix, got {kinds}"


def test_entry_workplan_degenerate_boundary():
    steps = _build_entry([], tool_radius=3.0)
    assert isinstance(steps, list)
    assert len(steps) == 0


def test_entry_workplan_empty_islands_list():
    boundary = _rect(-20, -20, 40, 40)
    wp1 = _build_entry(boundary, tool_radius=3.0)
    wp2 = _build_entry(boundary, islands=[], tool_radius=3.0)
    assert len(wp1) == len(wp2)


def _cup():
    return [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 8.0),
        (24.0, 8.0),
        (24.0, 30.0),
        (16.0, 30.0),
        (16.0, 8.0),
        (0.0, 8.0),
    ]


def _assert_tool_disc_fits_boundary(start, end, boundary, tool_radius):
    sx, sy = start
    ex, ey = end
    for i in range(11):
        t = i / 10.0
        xt = sx + (ex - sx) * t
        yt = sy + (ey - sy) * t
        for j in range(8):
            ang = j * math.pi / 4
            dx = tool_radius * math.cos(ang)
            dy = tool_radius * math.sin(ang)
            assert is_point_inside_polygon((xt + dx, yt + dy), boundary), (
                f"tool disc at ({xt:.3f}, {yt:.3f}) +r"
                f" {tool_radius} pokes outside boundary"
            )


def test_entry_workplan_helix_kind():
    """Helix step has the right kind string."""
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
        angular_step=0.1,
    )
    helix_steps = [s for s in steps if s.kind == "helix"]
    assert len(helix_steps) == 1


def test_entry_workplan_no_toroid_variant():
    boundary = _rect(-20, -20, 40, 40)
    steps = _build_entry(
        boundary, tool_radius=3.0, step_over=2.0, safe_z=2.0, target_z=-5.0
    )
    for s in steps:
        assert s.kind not in ("toroid",), f"unexpected toroid step: {s.kind}"


def test_entry_workplan_island_avoids_entry():
    outer = _rect(-5, -5, 10, 10)
    island = _rect(-2.5, -2.5, 5, 5)
    steps = _build_entry(
        outer,
        islands=[island],
        tool_radius=4.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    kinds = [s.kind for s in steps]
    assert "helix" not in kinds, (
        f"unexpected helix in tight pocket with island: {kinds}"
    )
