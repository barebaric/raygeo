"""Tests for build_entry_workplan in cnc/machining/entry."""

import math

from raygeo.cnc.machining.entry import build_entry_workplan
from raygeo.geo.shape.polygon import is_point_inside_polygon


def _rect(x0, y0, w, h):
    """CCW rectangle starting at (x0, y0)."""
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _dumbbell():
    """Two 30x30 lobes connected by a 20x5 corridor.

    Left lobe: x=0..30, y=0..30
    Right lobe: x=50..80, y=0..30
    Corridor: x=30..50, y=12.5..17.5
    """
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
    """40x40 rect -> HelixPlunge + FlatSpiral (no RampEntry)."""
    boundary = _rect(-20, -20, 40, 40)
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    kinds = [step["kind"] for step in workplan]
    assert "HelixPlunge" in kinds, f"expected HelixPlunge, got {kinds}"
    # 40x40 rect with r_max approx 20 >= 2*6=12 should produce helix
    assert "FlatSpiral" in kinds, f"expected FlatSpiral, got {kinds}"
    assert "RampEntry" not in kinds, (
        f"unexpected RampEntry in wide rect: {kinds}"
    )


def test_entry_workplan_tight_slot():
    """40x8 slot -> RampEntry or ToroidalClear, no HelixPlunge."""
    boundary = _rect(0, 0, 40, 8)
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    kinds = [step["kind"] for step in workplan]
    assert "HelixPlunge" not in kinds, (
        f"unexpected HelixPlunge in tight slot: {kinds}"
    )
    # Must have at least one step
    assert len(workplan) >= 1, "expected at least 1 step for tight slot"
    # Kind should be RampEntry or ToroidalClear
    has_ramp_or_toroid = ("RampEntry" in kinds) or ("ToroidalClear" in kinds)
    assert has_ramp_or_toroid, (
        f"expected RampEntry or ToroidalClear, got {kinds}"
    )


def test_entry_workplan_dumbbell():
    """Dumbbell -> 2x HelixPlunge + 2x FlatSpiral (one per lobe)."""
    boundary = _dumbbell()
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    plunge_count = sum(1 for s in workplan if s["kind"] == "HelixPlunge")
    spiral_count = sum(1 for s in workplan if s["kind"] == "FlatSpiral")
    assert plunge_count == 2, f"expected 2 HelixPlunge, got {plunge_count}"
    assert spiral_count == 2, f"expected 2 FlatSpiral, got {spiral_count}"


def test_entry_workplan_no_toroid_variant():
    """No step should have the name 'Toroid' (old EntryMethod)."""
    boundary = _rect(-20, -20, 40, 40)
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    for step in workplan:
        assert "Toroid" not in step["kind"], (
            f"unexpected Toroid step: {step['kind']}"
        )


def test_entry_workplan_island_avoids_entry():
    """Tight pocket + large island -> no HelixPlunge (r_max too small)."""
    # 10x10 outer with 5x5 island at center; tool_radius=4 -> r_max tiny
    outer = _rect(-5, -5, 10, 10)
    island = _rect(-2.5, -2.5, 5, 5)
    workplan = build_entry_workplan(
        pocket_boundary=outer,
        islands=[island],
        tool_radius=4.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    kinds = [step["kind"] for step in workplan]
    assert "HelixPlunge" not in kinds, (
        f"unexpected HelixPlunge in tight pocket with island: {kinds}"
    )
    assert len(workplan) >= 1, "expected at least 1 step"


def test_entry_workplan_degenerate_boundary():
    """Empty boundary -> empty workplan."""
    workplan = build_entry_workplan(
        pocket_boundary=[],
        tool_radius=3.0,
    )
    assert isinstance(workplan, list)
    assert len(workplan) == 0


def test_entry_workplan_steps_have_kind():
    """Every step dict has a 'kind' key."""
    boundary = _rect(-20, -20, 40, 40)
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
    )
    for step in workplan:
        assert "kind" in step, f"step missing 'kind': {step}"


def test_entry_workplan_islands_optional():
    """Omitting islands -> boundary with no holes."""
    boundary = _rect(-20, -20, 40, 40)
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
    )
    assert len(workplan) >= 1


def test_entry_workplan_step_over_zero():
    """step_over=0 should not emit FlatSpiral (no radial progress)."""
    boundary = _rect(-20, -20, 40, 40)
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=0.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    kinds = [step["kind"] for step in workplan]
    assert "FlatSpiral" not in kinds, (
        f"unexpected FlatSpiral with step_over=0: {kinds}"
    )
    assert "HelixPlunge" in kinds, f"expected HelixPlunge, got {kinds}"


def test_entry_workplan_helix_plunge_params():
    """HelixPlunge step has expected fields."""
    boundary = _rect(-20, -20, 40, 40)
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
        angular_step=0.1,
    )
    helix_steps = [s for s in workplan if s["kind"] == "HelixPlunge"]
    assert len(helix_steps) == 1
    h = helix_steps[0]
    assert abs(h["z_start"] - 2.0) < 1e-6
    assert abs(h["z_end"] - (-5.0)) < 1e-6
    assert abs(h["pitch"] - 1.0) < 1e-6
    assert abs(h["angular_step"] - 0.1) < 1e-6
    assert h["direction"] == "CW"
    assert h["helix_r"] > 0.0
    cx, cy = h["center"]
    # Accept small offset from origin (polylabel precision)
    assert abs(cx) < 0.5
    assert abs(cy) < 0.5


def test_entry_workplan_flat_spiral_params():
    """FlatSpiral step has expected fields."""
    boundary = _rect(-20, -20, 40, 40)
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        angular_step=0.1,
    )
    spiral_steps = [s for s in workplan if s["kind"] == "FlatSpiral"]
    assert len(spiral_steps) == 1
    s = spiral_steps[0]
    assert abs(s["z"] - (-5.0)) < 1e-6
    assert s["revolutions"] > 0.0
    assert s["start_radius"] < s["end_radius"]
    cx, cy = s["center"]
    assert abs(cx) < 0.5
    assert abs(cy) < 0.5


def test_entry_workplan_empty_islands_list():
    """Empty islands list (not None) should work same as None."""
    boundary = _rect(-20, -20, 40, 40)
    wp1 = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
    )
    wp2 = build_entry_workplan(
        pocket_boundary=boundary,
        islands=[],
        tool_radius=3.0,
    )
    assert len(wp1) == len(wp2)


def _cup():
    """Inverted-U: 40x8 bar with 8-wide vertical post (x=16..24).

    The bar and chimney are both 8 mm wide (between D=6 and 2*D=12), so
    ``analyze_pocket`` classifies them all as Narrow.  ``find_regions``
    filters out sliver leftovers that are too narrow for the tool disc
    (``r_max < tool_radius``), so no region is returned.  The workplan
    falls through to the fallback ramp carrier, which is constrained to
    the eroded boundary so the tool disc fits the whole length.
    """
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
    """Sample 11 points along the segment; verify the tool disc
    centred at each fits inside ``boundary``."""
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


def test_entry_workplan_ramp_stays_inside_boundary():
    """RampEntry carrier must not poke outside the pocket boundary."""
    boundary = _cup()
    workplan = build_entry_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    ramp_steps = [s for s in workplan if s["kind"] == "RampEntry"]
    assert len(ramp_steps) == 1, f"expected 1 RampEntry, got {len(ramp_steps)}"
    step = ramp_steps[0]
    _assert_tool_disc_fits_boundary(
        step["start"], step["end"], boundary, tool_radius=3.0
    )


def test_entry_workplan_ramp_uses_eroded_region():
    """RampEntry carrier must keep the tool disc inside the boundary.

    Previously the carrier was generated by spanning the AABB of
    the (non-eroded) region polygon, so a triangular leftover
    region produced a carrier whose tool disc stuck out past the
    sliver's edges.  The fix clips the candidate axes against the
    *eroded* region (boundary eroded by tool_radius) so the tool
    disc always fits.
    """
    boundary = _cup()
    workplan = build_entry_workplan(pocket_boundary=boundary, tool_radius=3.0)
    for step in workplan:
        if step["kind"] != "RampEntry":
            continue
        _assert_tool_disc_fits_boundary(
            step["start"], step["end"], boundary, tool_radius=3.0
        )
