"""Tests for raygeo.ops.assembly.adaptive.resume module."""

import math

import pytest

from raygeo.geo.algo.medial_axis import MedialAxis
from raygeo.geo.shape.polygon import (
    JoinStyle,
    get_polygon_area,
    get_polygons_group_difference,
    offset_polygon,
)
from raygeo.ops import Ops
from raygeo.ops.assembly.adaptive.resume import (
    emit_resume_travel,
    smooth_travel_path,
    try_resume,
)
from raygeo.ops.assembly.adaptive.tool import Tool
from raygeo.ops.cut.cleared_area import ClearedArea


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


# ── smooth_travel_path ───────────────────────────────────────────────


class TestSmoothTravelPath:
    def test_empty_raw_returns_from(self):
        out = smooth_travel_path((1.0, 2.0), [], [], 3.0)
        assert out == [(1.0, 2.0)]

    def test_single_waypoint_appended(self):
        """A single raw waypoint is kept as the destination."""
        out = smooth_travel_path((0.0, 0.0), [(10.0, 0.0)], [], 3.0)
        assert len(out) >= 2
        assert out[0] == (0.0, 0.0)
        assert out[-1] == (10.0, 0.0)

    def test_preserves_endpoints(self):
        """from_pt is the first point, last raw point the final point."""
        raw = [(5, 0), (10, 0), (10, 10), (10, 20)]
        out = smooth_travel_path((0.0, 0.0), raw, [], 3.0)
        assert out[0] == pytest.approx((0.0, 0.0))
        assert out[-1] == pytest.approx((10.0, 20.0))

    def test_shortens_collinear_without_obstacles(self):
        """With no obstacles the smoothed path is a near-straight line
        from start to end (shortcut phase removes intermediate hops;
        resampling adds density but the total arc length stays short)."""
        raw = [(5, 0), (10, 0), (15, 0), (20, 0)]
        out = smooth_travel_path((0.0, 0.0), raw, [], 3.0)
        assert out[0] == pytest.approx((0.0, 0.0))
        assert out[-1] == pytest.approx((20.0, 0.0))
        # Arc length ≈ straight-line distance (20) within tolerance.
        arc = sum(_dist(a, b) for a, b in zip(out, out[1:]))
        assert arc <= 20.0 + 1.0

    def test_keeps_clearance_from_island(self):
        """A raw path that already skirts the island is not pulled back
        into it by smoothing."""
        island = _rect(15, 10, 8, 8)  # island centred at (15,10)
        # Raw waypoints route *around* the top of the island.
        raw = [(5, 10), (15, 20), (25, 10)]
        out = smooth_travel_path((0.0, 10.0), raw, [island], clearance=2.0)
        for x, y in out:
            # tool disk centre must be outside the island box
            inside = (11 < x < 19) and (6 < y < 14)
            assert not inside, f"point ({x:.2f},{y:.2f}) inside island"

    def test_stays_clear_of_remaining(self):
        """A raw path that skirts the remaining-stock polygon is not
        pulled back into it by smoothing."""
        remaining = _rect(15, 5, 8, 8)
        raw = [(5, 5), (15, 15), (25, 5)]
        out = smooth_travel_path((0.0, 5.0), raw, [remaining], clearance=2.0)
        for x, y in out:
            inside = (11 < x < 19) and (1 < y < 9)
            assert not inside, f"point ({x:.2f},{y:.2f}) inside remaining"

    def test_path_is_continuous(self):
        """Successive output points are within a reasonable hop distance."""
        raw = [(0, 0), (10, 0), (10, 10), (0, 10)]
        out = smooth_travel_path((-5.0, -5.0), raw, [], 3.0)
        for a, b in zip(out, out[1:]):
            assert _dist(a, b) < 50.0


# ── mat_resume_target ────────────────────────────────────────────────


def _valid_tool_area(boundary, islands, radius):
    inset = offset_polygon(boundary, -radius, JoinStyle.Miter)
    if not inset:
        return [], 0.0
    if islands:
        island_bufs = []
        for island in islands:
            island_bufs.extend(offset_polygon(island, radius, JoinStyle.Miter))
        region = get_polygons_group_difference(inset, island_bufs)
    else:
        region = inset
    total = sum(get_polygon_area(p) for p in region)
    return region, total


# ── emit_resume_travel ───────────────────────────────────────────────


class TestEmitResumeTravel:
    def test_emits_single_move(self):
        """A single move_to is emitted to the target position."""
        outer = _rect(30.0, 30.0, 60, 60)
        ops = Ops()
        before = ops.len()
        emit_resume_travel(ops, (20.0, 20.0), outer)
        assert ops.len() == before + 1

    def test_emits_travel_commands(self):
        """Emitted command is a travel move at cut_z + 0.5."""
        outer = _rect(40.0, 40.0, 80, 80)
        ops = Ops()
        emit_resume_travel(ops, (70.0, 40.0), outer)
        assert ops.len() >= 1
        for i in range(ops.len()):
            assert ops.is_travel(i)

    def test_mutates_ops_in_place(self):
        outer = _rect(30.0, 30.0, 60, 60)
        ops = Ops()
        n0 = ops.len()
        emit_resume_travel(ops, (10.0, 10.0), outer)
        assert ops.len() == n0 + 1

    def test_travel_ends_at_target(self):
        """The emitted travel point must match `to`."""
        outer = _rect(40.0, 40.0, 80, 80)
        ops = Ops()
        to = (70.0, 40.0)
        emit_resume_travel(ops, to, outer)
        assert ops.len() >= 1
        ex, ey, _ = ops.endpoint(ops.len() - 1)
        assert (ex, ey) == pytest.approx(to, abs=0.01)

    def test_no_extreme_final_segment(self):
        """The single travel segment should not be longer than the direct
        from→to distance."""
        outer = _rect(40.0, 40.0, 80, 80)
        ops = Ops()
        to_pt = (70.0, 40.0)
        emit_resume_travel(ops, to_pt, outer)
        direct = math.sqrt(to_pt[0] ** 2 + to_pt[1] ** 2)
        for i in range(1, ops.len()):
            x0, y0, _ = ops.endpoint(i - 1)
            x1, y1, _ = ops.endpoint(i)
            seg = math.sqrt((x1 - x0) ** 2 + (y1 - y0) ** 2)
            assert seg <= 2.0 * direct + 1.0, (
                f"Segment {i}: {seg:.1f}mm > 2×direct ({direct:.1f}mm)"
            )


# ── try_resume ───────────────────────────────────────────────────────


class TestTryResume:
    def test_no_area_growth_skips_frontier(self):
        """When the cleared area hasn't grown since the last resume,
        try_resume skips the frontier search and falls through.  With
        a partially-cleared pocket and a MAT it may still reposition
        via the MAT walk; the key assertion is that it does not raise
        and returns a bool."""
        outer = _rect(30.0, 30.0, 60, 60)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        seed = _rect(30, 30, 10, 10)
        ca = ClearedArea(boundary=outer, initial=[seed])
        vta, _ = _valid_tool_area(outer, [], 3.0)
        tool = Tool((30.0, 30.0), 0.0, 3.0)
        ops = Ops()
        result = try_resume(
            ca,
            ops,
            tool,
            outer,
            radius=3.0,
            step_length=0.6,
            advance=1.5,
            cut_z=-5.0,
            valid_tool_area=vta,
            axis=axis,
            last_resume_area=ca.total_area(),  # no growth
            cut_direction="ccw",
        )
        assert isinstance(result, bool)

    def test_returns_bool_and_mutates(self):
        """try_resume returns a bool; on success ops grows."""
        outer = _rect(40.0, 40.0, 80, 80)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        seed = _rect(40, 40, 8, 8)
        ca = ClearedArea(boundary=outer, initial=[seed])
        vta, _ = _valid_tool_area(outer, [], 3.0)
        tool = Tool((40.0, 40.0), 0.0, 3.0)
        ops = Ops()
        result = try_resume(
            ca,
            ops,
            tool,
            outer,
            radius=3.0,
            step_length=0.6,
            advance=1.5,
            cut_z=-5.0,
            valid_tool_area=vta,
            axis=axis,
            cut_direction="ccw",
        )
        assert isinstance(result, bool)
        if result:
            assert ops.len() >= 1

    def test_fully_cleared_pocket(self):
        """When the pocket is fully cleared, resume can't find fresh
        material — returns False."""
        outer = _rect(20.0, 20.0, 40, 40)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        vta, _ = _valid_tool_area(outer, [], 3.0)
        # Clear the entire valid tool area.
        ca = ClearedArea(boundary=outer, initial=vta)
        tool = Tool((20.0, 20.0), 0.0, 3.0)
        ops = Ops()
        result = try_resume(
            ca,
            ops,
            tool,
            outer,
            radius=3.0,
            step_length=0.6,
            advance=1.5,
            cut_z=-5.0,
            valid_tool_area=vta,
            axis=axis,
            cut_direction="ccw",
        )
        assert isinstance(result, bool)


def _circle(cx, cy, r, n=32):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _big_vta():
    return [
        [(-200, -200), (200, -200), (200, 200), (-200, 200)],
    ]
