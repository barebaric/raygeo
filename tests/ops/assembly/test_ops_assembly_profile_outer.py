"""Tests for raygeo.ops.assembly.profile module (outer profiling)."""

from raygeo.geo.shape.polygon import (
    JoinStyle,
    get_polygons_closest_point,
    offset_polygon,
)
from raygeo.ops import Ops
from raygeo.ops.assembly.profile import profile_outer
from raygeo.ops.part import Part


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def test_profile_outer_smoke_and_returns_ops():
    """Binding returns an AssemblyResult with an Ops object."""
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])
    result = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        start_pos=None,
        cut_direction="ccw",
    )
    assert isinstance(result.ops, Ops)
    assert result.ops.len() > 0


def test_profile_outer_walks_and_returns_to_start():
    """profile_outer walks the offset boundary and returns to start."""
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])
    result = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        start_pos=None,
        cut_direction="ccw",
    )
    ops = result.ops
    assert ops.len() >= 4, f"expected >= 4 commands, got {ops.len()}"
    # Walk returns to start: first cut vertex ≈ last cut vertex
    first_cut = None
    last_cut = None
    for i in range(ops.len()):
        if ops.is_cutting(i):
            ep = ops.endpoint(i)
            if first_cut is None:
                first_cut = ep
            last_cut = ep
    assert first_cut is not None and last_cut is not None
    dx = last_cut[0] - first_cut[0]
    dy = last_cut[1] - first_cut[1]
    dist = (dx * dx + dy * dy) ** 0.5
    assert dist <= 1.0, (
        f"walk did not return to start: {first_cut} vs {last_cut}, dist={dist}"
    )
    # All commands are motion commands (MoveTo, LineTo) or state commands
    # (SetFeedRate, SetPower) that apply_state emits.

    allowed_names = {"MOVE_TO", "LINE_TO", "SET_FEED_RATE", "SET_POWER"}
    for i in range(ops.len()):
        ct = ops.command_type(i)
        assert ct.name in allowed_names, f"unexpected cmd {ct} at {i}"


def test_profile_outer_stays_close_to_offset():
    """profile_outer path stays within tolerance of the offset polygon."""
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])
    result = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        start_pos=None,
        cut_direction="ccw",
    )
    ops = result.ops
    offset_polys = offset_polygon(boundary, 3.0, JoinStyle.Round)
    max_drift = 0.0
    for i in range(ops.len()):
        if not ops.is_cutting(i):
            continue
        x, y, _ = ops.endpoint(i)
        result_cp = get_polygons_closest_point(offset_polys, x, y)
        if result_cp is not None:
            _, _, _, dist_sq = result_cp
            max_drift = max(max_drift, dist_sq**0.5)
    assert max_drift <= 5.0, f"max drift {max_drift} > 5.0mm"
