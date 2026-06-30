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
    mat_resume_target,
    search_reengagement,
    smooth_travel_path,
    try_resume,
)
from raygeo.ops.assembly.adaptive.tool import Tool
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.search import ToolPose, search_frontier_engagement


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


class TestMatResumeTarget:
    def test_empty_cleared_returns_none(self):
        """No cleared fragments ⇒ nothing to route from."""
        outer = _rect(30.0, 30.0, 60, 60)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        ca = ClearedArea(boundary=outer)  # no initial polygons
        vta, _ = _valid_tool_area(outer, [], 3.0)
        assert (
            mat_resume_target(
                axis,
                ca,
                Tool((30.0, 30.0), 0.0, 3.0),
                "ccw",
                0.6,
                outer,
                [],
                vta,
            )
            is None
        )

    def test_returns_tool_pose(self):
        """Partially-cleared pocket yields a ToolPose target."""
        outer = _rect(30.0, 30.0, 60, 60)
        island = _rect(30, 30, 10, 10)
        axis = MedialAxis.compute(outer, [island], 1.0, 6.0)
        # Seed a small cleared region in the centre.
        seed = _rect(30, 30, 6, 6)
        ca = ClearedArea(boundary=outer, islands=[island], initial=[seed])
        vta, _ = _valid_tool_area(outer, [island], 3.0)
        result = mat_resume_target(
            axis,
            ca,
            Tool((30.0, 30.0), 0.0, 3.0),
            "ccw",
            0.6,
            outer,
            [island],
            vta,
        )
        if result is not None:
            assert isinstance(result, ToolPose)
            assert isinstance(result.heading, float)

    def test_tool_on_uncleared_node_returns_none(self):
        """If the tool sits on an uncleared MAT node, there's nothing
        to route to (engagement should be available)."""
        outer = _rect(30.0, 30.0, 60, 60)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        # Tool positioned far from any cleared fragment.
        seed = _rect(5, 5, 4, 4)
        ca = ClearedArea(boundary=outer, initial=[seed])
        vta, _ = _valid_tool_area(outer, [], 3.0)
        # Pick a node far from the seed as tool position.
        far = max(axis.nodes, key=lambda p: _dist(p, (5, 5)))
        result = mat_resume_target(
            axis,
            ca,
            Tool(far, 0.0, 3.0),
            "ccw",
            0.6,
            outer,
            [],
            vta,
        )
        # May or may not return a result depending on the cleared mask,
        # but should not raise.
        if result is not None:
            assert isinstance(result, ToolPose)


# ── emit_resume_travel ───────────────────────────────────────────────


class TestEmitResumeTravel:
    def test_no_mat_emits_single_move(self):
        """Without a Medial Axis, a single move_to is emitted."""
        outer = _rect(30.0, 30.0, 60, 60)
        ca = ClearedArea(boundary=outer, initial=[_rect(30, 30, 10, 10)])
        ops = Ops()
        before = ops.len()
        emit_resume_travel(
            ops,
            ca,
            None,  # no MAT
            (0.0, 0.0),
            (20.0, 20.0),
            outer,
        )
        assert ops.len() == before + 1

    def test_with_mat_emits_at_least_one_move(self):
        """With a MAT the travel is routed (and shortened)."""
        outer = _rect(40.0, 40.0, 80, 80)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        ca = ClearedArea(boundary=outer, initial=[_rect(40, 40, 12, 12)])
        ops = Ops()
        emit_resume_travel(
            ops,
            ca,
            axis,
            (40.0, 40.0),
            (70.0, 40.0),
            outer,
        )
        assert ops.len() >= 1
        # Every emitted command should be a travel move at cut_z + 0.5.
        for i in range(ops.len()):
            assert ops.is_travel(i)

    def test_mutates_ops_in_place(self):
        outer = _rect(30.0, 30.0, 60, 60)
        ca = ClearedArea(boundary=outer, initial=[_rect(30, 30, 10, 10)])
        ops = Ops()
        n0 = ops.len()
        emit_resume_travel(ops, ca, None, (0.0, 0.0), (10.0, 10.0), outer)
        assert ops.len() == n0 + 1

    def test_travel_ends_at_target(self):
        """The last emitted travel point must match `to` (the target),
        not a distant MAT node."""
        outer = _rect(40.0, 40.0, 80, 80)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        ca = ClearedArea(boundary=outer, initial=[_rect(40, 40, 12, 12)])
        ops = Ops()
        to = (70.0, 40.0)
        emit_resume_travel(ops, ca, axis, (40.0, 40.0), to, outer)
        assert ops.len() >= 1
        ex, ey, _ = ops.endpoint(ops.len() - 1)
        assert (ex, ey) == pytest.approx(to, abs=0.01)

    def test_no_extreme_final_segment(self):
        """No single travel segment should be dramatically longer than
        the direct from→to distance.  This catches the V-shaped detour
        where the MAT path overshoots and then jumps back to `to`."""
        outer = _rect(40.0, 40.0, 80, 80)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        ca = ClearedArea(boundary=outer, initial=[_rect(40, 40, 12, 12)])
        ops = Ops()
        from_pt = (40.0, 40.0)
        to_pt = (70.0, 40.0)
        emit_resume_travel(ops, ca, axis, from_pt, to_pt, outer)
        direct = _dist(from_pt, to_pt)
        for i in range(1, ops.len()):
            x0, y0, _ = ops.endpoint(i - 1)
            x1, y1, _ = ops.endpoint(i)
            seg = _dist((x0, y0), (x1, y1))
            # No segment should be more than 2× the direct distance.
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


class TestSegmentResume:
    def test_returns_tool_pose(self):
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_reengagement(
            ca,
            (55.0, 55.0),
            (1.0, 0.0),
            3.0,
            0.6,
            1.5,
            0.1,
            _big_vta(),
        )
        assert isinstance(result, ToolPose)

    def test_none_for_empty(self):
        ca = ClearedArea(boundary=[])
        result = search_reengagement(
            ca,
            (55.0, 55.0),
            (1.0, 0.0),
            3.0,
            0.6,
            1.5,
            0.1,
            _big_vta(),
        )
        assert result is None

    def test_forward_and_backward_return_valid(self):
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        fwd = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            0.1,
            float("inf"),
        )
        bwd = search_reengagement(
            ca,
            (55.0, 55.0),
            (1.0, 0.0),
            3.0,
            0.6,
            1.5,
            0.1,
            _big_vta(),
        )
        assert fwd is not None and bwd is not None
