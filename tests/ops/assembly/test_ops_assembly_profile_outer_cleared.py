"""Tests for profile_outer ClearedArea mutation."""

from typing import Any

from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.cut import Part


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _kwargs(initial, boundary, **over: Any) -> dict[str, Any]:
    part = Part.from_polygons(boundary, initial=initial)
    kw = dict(
        part=part,
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
    kw.update(over)
    return kw


def test_profile_outer_mutates_cleared_area():
    """profile_outer adds swept area to ClearedArea.

    The tool walks the GROWN boundary (outside the pocket), so the
    swept fragments lie outside the pocket interior.  The pocket's
    remaining area may not decrease (the tool clears stock beyond the
    wall, not inside).  Instead, verify that fragments are added.
    """
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])
    assert part.cleared.is_empty()

    profile_outer(**dict(_kwargs([], boundary), part=part))

    ca = part.cleared
    assert not ca.is_empty(), "no fragments added"
    assert ca.total_area() > 0, f"total_area={ca.total_area()}"


def test_profile_outer_idempotent_under_repeated_calls():
    """Running profile_outer twice on the same ClearedArea works."""
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])

    result1 = profile_outer(**dict(_kwargs([], boundary), part=part))
    assert result1.ops.len() > 0
    assert result1.ops.cut_distance() > 0

    result2 = profile_outer(**dict(_kwargs([], boundary), part=part))
    assert result2.ops.len() > 0
    cut_count = sum(
        1 for i in range(result2.ops.len()) if result2.ops.is_cutting(i)
    )
    assert cut_count > 0, "second call produced no cutting moves"


def test_profile_inner_then_outer_with_shared_cleared():
    """profile_inner then profile_outer on the same ClearedArea succeeds."""
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])
    result_inner = profile_inner(
        part,
        tool_radius=3.0,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        stock_to_leave=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        start_pos=None,
        cut_direction="ccw",
        engagement_area_threshold=0.0,
        engagement_angle_threshold=3.14159,
    )
    assert result_inner.ops.len() > 0
    result_outer = profile_outer(
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
        stock_to_leave=0.0,
        engagement_area_threshold=0.0,
        engagement_angle_threshold=3.14159,
    )
    assert result_outer.ops.len() > 0
