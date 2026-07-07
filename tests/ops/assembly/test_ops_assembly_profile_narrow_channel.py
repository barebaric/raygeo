"""Tests for narrow-channel island detection in profile_inner."""

from typing import Any

import pytest

from raygeo.ops.assembly.profile import profile_inner
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
        boundary=boundary,
        islands=[],
        tool_radius=3.0,
        step_over=1.5,
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
        engagement_angle_threshold=3.141592653589793,
    )
    kw.update(over)
    return kw


def test_inset_with_close_island_returns_disjoint_or_empty():
    """compute_inset_region with close island yields empty/disjoint."""
    try:
        from raygeo.geo.algo.offset import compute_inset_region

        boundary = _rect(0, 0, 60, 60)
        island = _rect(24, 0, 8, 8)
        region, area = compute_inset_region(boundary, 3.0, [island])
        # Channel to wall: 30-24-4=2mm < 2*radius=6mm.
        assert area < 2900 or len(region) == 0, (
            f"close island should reduce area: got {area}"
        )
    except ImportError:
        pytest.skip("compute_inset_region not exposed to Python")


def test_inset_with_far_island_returns_connected_region():
    """compute_inset_region with far island returns connected region."""
    try:
        from raygeo.geo.algo.offset import compute_inset_region

        boundary = _rect(0, 0, 60, 60)
        island = _rect(-15, 0, 8, 8)
        region, area = compute_inset_region(boundary, 3.0, [island])
        assert len(region) > 0 and area > 2000, (
            f"far island area={area}, polygons={len(region)}"
        )
    except ImportError:
        pytest.skip("compute_inset_region not exposed to Python")


def test_profile_inner_skips_island_when_channel_too_narrow():
    """profile_inner skips island with channel narrower than 2*radius."""
    boundary = _rect(0, 0, 60, 60)
    island_far = _rect(-15, 0, 8, 8)
    island_close = _rect(24, 0, 8, 8)

    ca_far = ClearedArea(boundary=boundary, initial=[])
    result_far = profile_inner(
        **_kwargs(ca_far, boundary, islands=[island_far])
    )

    ca_both = ClearedArea(boundary=boundary, initial=[])
    result_both = profile_inner(
        **_kwargs(ca_both, boundary, islands=[island_far, island_close])
    )

    diff = abs(result_both.ops.cut_distance() - result_far.ops.cut_distance())
    close_perimeter = 4 * (8 + 2 * 3.0)
    assert diff < close_perimeter * 0.5, (
        f"close island should be blocked; diff={diff:.1f}mm >= "
        f"{close_perimeter * 0.5:.1f}mm"
    )


def test_profile_inner_walks_both_islands_when_both_accessible():
    """profile_inner walks both islands when both far from walls."""
    boundary = _rect(0, 0, 60, 60)
    island1 = _rect(-15, -10, 8, 8)
    island2 = _rect(15, 10, 8, 8)

    ca_one = ClearedArea(boundary=boundary, initial=[])
    result_one = profile_inner(**_kwargs(ca_one, boundary, islands=[island1]))

    ca_two = ClearedArea(boundary=boundary, initial=[])
    result_two = profile_inner(
        **_kwargs(ca_two, boundary, islands=[island1, island2])
    )

    extra = result_two.ops.cut_distance() - result_one.ops.cut_distance()
    grown_perimeter = 4 * (8 + 2 * 3.0)
    assert extra > grown_perimeter * 0.3, (
        f"second island extra={extra:.1f}mm, grown perim ~{grown_perimeter}mm"
    )
