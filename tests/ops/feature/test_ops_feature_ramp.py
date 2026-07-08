"""Tests for ops/feature/ramp — ramp carrier finder."""

import math

from raygeo.geo.shape.polygon import is_point_inside_polygon
from raygeo.ops.feature import ramp as _ramp

find_ramp_carrier = _ramp.find_ramp_carrier


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_ramp_rect_returns_carrier():
    """Rectangle 40×20 with tool_radius=3 returns a carrier."""
    boundary = _rect(-20, -10, 40, 20)
    result = find_ramp_carrier(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        max_ramp_angle_deg=45.0,
    )
    assert result is not None, "expected a carrier for 40×20 rect"
    start, end = result
    # Both endpoints inside the boundary.
    assert is_point_inside_polygon(start, boundary), (
        f"start {start} outside boundary"
    )
    assert is_point_inside_polygon(end, boundary), (
        f"end {end} outside boundary"
    )
    # Length at least L_min = max(3.0, 3.0/tan(45°)) = 3.0.
    length = math.dist(start, end)
    assert length >= 3.0, f"carrier length {length:.2f} < L_min = 3.0"
    # Y coordinate is within the eroded slab [-10+3, 10-3] = [-7, 7].
    assert -7.0 <= start[1] <= 7.0, (
        f"start y {start[1]} outside eroded slab [-7, 7]"
    )
    assert -7.0 <= end[1] <= 7.0, f"end y {end[1]} outside eroded slab [-7, 7]"


def test_ramp_tight_slot_returns_carrier():
    """Slot 30×8 with tool_radius=3 returns carrier along long axis (x)."""
    boundary = _rect(0, 0, 30, 8)
    result = find_ramp_carrier(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        max_ramp_angle_deg=45.0,
    )
    assert result is not None, "expected a carrier for 30×8 slot"
    start, end = result
    length = math.dist(start, end)
    assert length >= 3.0, f"carrier length {length:.2f} < L_min = 3.0"
    # Axis is the long axis (x-direction).
    x_extent = abs(end[0] - start[0])
    y_extent = abs(end[1] - start[1])
    assert x_extent > y_extent, (
        f"expected x-axis carrier, dx={x_extent}, dy={y_extent}"
    )
    assert is_point_inside_polygon(start, boundary), (
        f"start {start} outside boundary"
    )
    assert is_point_inside_polygon(end, boundary), (
        f"end {end} outside boundary"
    )


def test_ramp_no_space_returns_none():
    """Pocket smaller than 2×tool_radius returns None."""
    boundary = _rect(0, 0, 5, 5)
    result = find_ramp_carrier(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        max_ramp_angle_deg=45.0,
    )
    assert result is None, "expected None for pocket < tool diameter"


def test_ramp_island_does_not_block_carrier():
    """Island blocks centroid sweep — carrier avoids dilated no-go band.

    Boundary 50×40, island at x:20..30, y:17..23, tool_radius=3.
    The dilated no-go band is x:17..33, y:14..26.  The returned segment
    must NOT pass through this band at any point.
    """
    boundary = _rect(0, 0, 50, 40)
    island = _rect(20, 17, 10, 6)
    result = find_ramp_carrier(
        boundary=boundary,
        islands=[island],
        tool_radius=3.0,
        max_ramp_angle_deg=45.0,
    )
    assert result is not None, "expected a carrier despite blocking island"
    start, end = result

    no_go_x0, no_go_x1 = 17.0, 33.0
    no_go_y0, no_go_y1 = 14.0, 26.0

    # Check 11 evenly-spaced sample points along the segment.
    for i in range(11):
        t = i / 10.0
        mx = start[0] + t * (end[0] - start[0])
        my = start[1] + t * (end[1] - start[1])
        assert not (
            no_go_x0 <= mx <= no_go_x1 and no_go_y0 <= my <= no_go_y1
        ), (
            f"point ({mx:.1f}, {my:.1f}) at t={t:.1f} lies inside dilated "
            f"island band [{no_go_x0}, {no_go_x1}] × [{no_go_y0}, {no_go_y1}]"
        )

    # Length still >= L_min.
    length = math.dist(start, end)
    assert length >= 3.0, f"carrier length {length:.2f} < L_min = 3.0"
