"""Tests for ops/feature/slot_path — slot carrier finder.

The slot carrier is found by a disk-probe snake walk: at each step,
a disk of radius `tool_radius` is probed ahead of the current
position, intersected with the eroded region, and the centroid of
the intersection becomes the next carrier point.  The heading is
updated with exponential smoothing so the walk follows bends
naturally.  This handles curved, S-shaped, and zig-zag slots that
the previous AABB-slice approach could not.
"""

import math

from raygeo.geo.shape.polygon import is_point_inside_polygon
from raygeo.ops.feature.slot_path import find_slot_path


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_slot_path_rect_slot():
    """Slot 40×7 with tool_radius=3 returns a carrier inside eroded region.

    The snake walk returns multiple sample points; check the first and
    last lie inside the eroded rectangle.
    """
    slot = _rect(0, 0, 40, 7)
    result = find_slot_path(
        slot_polygon=slot,
        entry_edges=[0],
        entry_point=(20, 0),
        tool_radius=3.0,
    )
    assert result is not None, "expected a carrier for 40×7 slot"
    p_first, p_last = result[0], result[-1]

    eroded = _rect(0 + 3, 0 + 3, 40 - 6, 7 - 6)  # [3, 37] × [3, 4]
    assert is_point_inside_polygon(p_first, eroded), (
        f"point {p_first} outside eroded slot"
    )
    assert is_point_inside_polygon(p_last, eroded), (
        f"point {p_last} outside eroded slot"
    )

    length = math.dist(p_first, p_last)
    assert length >= 30.0, f"carrier length {length:.2f} < 30.0"

    assert 3.0 <= p_first[1] <= 4.0, f"p_first y {p_first[1]} outside [3, 4]"
    assert 3.0 <= p_last[1] <= 4.0, f"p_last y {p_last[1]} outside [3, 4]"


def test_slot_path_too_narrow_returns_none():
    """Slot 5 mm wide with tool_radius=3 returns None (eroded empty)."""
    slot = _rect(0, 0, 30, 5)
    result = find_slot_path(
        slot_polygon=slot,
        entry_edges=[0],
        entry_point=(15, 0),
        tool_radius=3.0,
    )
    assert result is None, "expected None for slot narrower than tool"


def test_slot_path_vertical_slot():
    """Slot 7×40 (tall, long axis = y) returns carrier along y."""
    slot = _rect(0, 0, 7, 40)
    result = find_slot_path(
        slot_polygon=slot,
        entry_edges=[0],
        entry_point=(3, 0),
        tool_radius=3.0,
    )
    assert result is not None, "expected a carrier for 7×40 slot"
    p_first, p_last = result[0], result[-1]

    dy = abs(p_last[1] - p_first[1])
    dx = abs(p_last[0] - p_first[0])
    assert dy > dx, f"expected y-axis carrier, dx={dx}, dy={dy}"

    assert 3.0 <= p_first[0] <= 4.0, f"p_first x {p_first[0]} outside [3, 4]"
    assert 3.0 <= p_last[0] <= 4.0, f"p_last x {p_last[0]} outside [3, 4]"

    length = math.dist(p_first, p_last)
    assert length >= 30.0, f"carrier length {length:.2f} < 30.0"


def test_slot_path_endpoint_at_entry_side():
    """First returned point is closer to entry_point than the last."""
    slot = _rect(0, 0, 40, 7)
    entry_point = (0, 0)  # corner entry, unambiguous side
    result = find_slot_path(
        slot_polygon=slot,
        entry_edges=[0],
        entry_point=entry_point,
        tool_radius=3.0,
    )
    assert result is not None, "expected a carrier"
    p_first = result[0]
    p_last = result[-1]

    d_first = math.dist(p_first, entry_point)
    d_last = math.dist(p_last, entry_point)
    assert d_first < d_last, (
        f"first {p_first} (dist={d_first:.3f}) should be closer to entry"
        f" than last {p_last} (dist={d_last:.3f})"
    )


def _build_smooth_s(centerline, half_width):
    """Build a corridor polygon around a planar centerline.

    ``centerline`` is a list of (x, y) points in order.  ``half_width``
    is the perpendicular offset to each side.
    The outline is traced CCW: left side forward, right side reversed.
    """
    left = []
    right = []
    n = len(centerline)
    for i, (cx, cy) in enumerate(centerline):
        if i < n - 1:
            dx = centerline[i + 1][0] - cx
            dy = centerline[i + 1][1] - cy
        else:
            dx = cx - centerline[i - 1][0]
            dy = cy - centerline[i - 1][1]
        L = math.hypot(dx, dy) or 1.0
        nx, ny = dx / L, dy / L
        px, py = -ny, nx
        left.append((cx + half_width * px, cy + half_width * py))
        right.append((cx - half_width * px, cy - half_width * py))
    return left + list(reversed(right))


def _point_segment_distance(p, a, b):
    """Shortest distance from point p to line segment [a, b]."""
    ax, ay = a
    bx, by = b
    px, py = p
    abx = bx - ax
    aby = by - ay
    t = ((px - ax) * abx + (py - ay) * aby) / (abx * abx + aby * aby)
    t = max(0.0, min(1.0, t))
    cx_ = ax + t * abx
    cy_ = ay + t * aby
    return math.dist(p, (cx_, cy_))


def test_slot_path_s_curve_follows_curve():
    """Sinusoidal S-slot: carrier follows the curve, not a straight line.

    Design: 6 mm-wide corridor snaking along y from 0 to 44 in a
    sine wave x = 10 + 7·sin(2π·y/30).  Tool radius 2, so the eroded
    region is a thin connected S (~2 mm wide), no disconnected
    components anywhere along the curve.
    The snake walk probes a disk at each step and follows the
    centroid, so the carrier's x oscillates like the centerline.
    """
    cl = [(10 + 7 * math.sin(2 * math.pi * y / 30), y) for y in range(0, 45)]
    slot = _build_smooth_s(cl, half_width=3.0)
    entry_point = cl[0]
    result = find_slot_path(
        slot_polygon=slot,
        entry_edges=[0],
        entry_point=entry_point,
        tool_radius=2.0,
    )
    assert result is not None, "S-shaped slot should produce a carrier"
    assert len(result) >= 10, (
        f"carrier should have at least 10 sample points (got {len(result)})"
    )

    for pt in result:
        d = min(
            _point_segment_distance(pt, cl[i], cl[i + 1])
            for i in range(len(cl) - 1)
        )
        assert d <= 3.5, (
            f"carrier point {pt} is {d:.2f} mm from centerline, exceeds 3.5 mm"
        )

    xs = [round(p[0], 1) for p in result]
    assert len(set(xs)) > 1, (
        "carrier x is constant; the snake walk did not follow the S curve"
    )

    y_range = max(p[1] for p in result) - min(p[1] for p in result)
    assert y_range >= 30.0, (
        f"carrier y-span {y_range:.2f} < 30.0; "
        "snake walk did not traverse the full S"
    )


def test_slot_path_zigzag_slot():
    """Zig-zag (staircase) slot: carrier follows the staircase.

    A 10 mm wide corridor that goes up, turns right, goes right,
    turns up, goes up.  A pure AABB-line carrier would not follow
    the horizontal step; the snake walk should navigate the corner.
    """
    w = 10
    slot = [
        (0, 0),
        (w, 0),
        (w, 18),
        (w + 12, 18),
        (w + 12, 40),
        (0, 40),
    ]
    entry_point = (w / 2, 0)
    result = find_slot_path(
        slot_polygon=slot,
        entry_edges=[0],
        entry_point=entry_point,
        tool_radius=3.0,
    )
    assert result is not None, "zig-zag slot should produce a carrier"
    assert len(result) >= 10, (
        f"carrier should have at least 10 points (got {len(result)})"
    )

    p_first = result[0]
    p_last = result[-1]
    assert p_last[1] > p_first[1], (
        f"carrier should end higher than it started; "
        f"first y={p_first[1]:.2f}, last y={p_last[1]:.2f}"
    )

    xs = [round(p[0], 1) for p in result]
    assert len(set(xs)) > 1, "carrier x is constant; did not zig-zag"
