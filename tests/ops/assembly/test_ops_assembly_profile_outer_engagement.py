"""Tests for profile_outer engagement check + adaptive step/feed."""

from typing import Any

from raygeo import Part
from raygeo.ops.assembly.profile import profile_outer
from raygeo.ops.cut.cleared_area import ClearedArea


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _kwargs(ca, boundary, **over: Any) -> dict[str, Any]:
    kw = dict(
        cleared=ca,
        part=Part.from_polygons(boundary),
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
        stock_to_leave=0.0,
        engagement_area_threshold=0.0,
        engagement_angle_threshold=3.14159,
    )
    kw.update(over)
    return kw


def test_profile_outer_finish_pass_runs():
    """Finish pass (empty pocket, stock_to_leave=0) runs without error."""
    boundary = _rect(0, 0, 60, 60)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_outer(**_kwargs(ca, boundary, stock_to_leave=0.0))
    assert result.ops.len() > 0


def test_profile_outer_rough_pass_runs():
    """Rough pass (stock_to_leave=0.5) runs without error."""
    boundary = _rect(0, 0, 60, 60)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_outer(**_kwargs(ca, boundary, stock_to_leave=0.5))
    assert result.ops.len() > 0


def test_profile_outer_high_threshold_runs():
    """Huge engagement thresholds do not prevent operation from completing."""
    boundary = _rect(0, 0, 60, 60)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_outer(
        **_kwargs(
            ca,
            boundary,
            engagement_area_threshold=1e9,
            engagement_angle_threshold=1e9,
        )
    )
    assert result.ops.len() > 0


def test_profile_outer_travel_skip_on_heavy_stock():
    """Low threshold may trigger travel-skips; op must still complete."""
    boundary = _rect(0, 0, 60, 60)
    seed = _rect(0, 0, 6, 6)
    ca = ClearedArea(boundary=boundary, initial=[seed])
    result = profile_outer(
        **_kwargs(
            ca,
            boundary,
            engagement_area_threshold=1.0,
            engagement_angle_threshold=0.5,
        )
    )
    assert result.ops.len() > 0
