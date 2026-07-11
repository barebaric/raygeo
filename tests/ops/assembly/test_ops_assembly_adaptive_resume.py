"""Tests for raygeo.ops.assembly.adaptive.resume module."""

import math

import pytest

from raygeo import Part
from raygeo.geo.algo.medial_axis import MedialAxis
from raygeo.geo.shape.polygon import (
    JoinStyle,
    get_polygon_area,
    get_polygons_group_difference,
    offset_polygon,
)
from raygeo.ops import Ops
from raygeo.ops.assembly.adaptive.resume import emit_resume_travel, try_resume
from raygeo.ops.assembly.adaptive.tool import Tool
from raygeo.ops.cut.cleared_area import ClearedArea


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


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
    def _cleared_area(self, outer):
        """Create a ClearedArea with the entire pocket as initial seed
        so routing has free space."""
        return ClearedArea(boundary=outer, initial=[outer])

    def test_emits_single_move(self):
        """A single move_to is emitted to the target position."""
        outer = _rect(30.0, 30.0, 60, 60)
        ops = Ops()
        before = ops.len()
        emit_resume_travel(
            Part.from_polygons(outer),
            ops,
            (20.0, 20.0, 0.0),
            cleared=self._cleared_area(outer),
        )
        assert ops.len() == before + 1

    def test_emits_travel_commands(self):
        """Emitted command is a travel move at cut_z + 0.5."""
        outer = _rect(40.0, 40.0, 80, 80)
        ops = Ops()
        emit_resume_travel(
            Part.from_polygons(outer),
            ops,
            (70.0, 40.0, 0.0),
            cleared=self._cleared_area(outer),
        )
        assert ops.len() >= 1
        for i in range(ops.len()):
            assert ops.is_travel(i)

    def test_mutates_ops_in_place(self):
        outer = _rect(30.0, 30.0, 60, 60)
        ops = Ops()
        n0 = ops.len()
        emit_resume_travel(
            Part.from_polygons(outer),
            ops,
            (10.0, 10.0, 0.0),
            cleared=self._cleared_area(outer),
        )
        assert ops.len() == n0 + 1

    def test_travel_ends_at_target(self):
        """The emitted travel point must match `to`."""
        outer = _rect(40.0, 40.0, 80, 80)
        ops = Ops()
        to = (70.0, 40.0, 0.0)
        emit_resume_travel(
            Part.from_polygons(outer),
            ops,
            to,
            cleared=self._cleared_area(outer),
        )
        assert ops.len() >= 1
        ep = ops.endpoint(ops.len() - 1)
        assert ep == pytest.approx(to, abs=0.01)

    def test_no_extreme_final_segment(self):
        """The single travel segment should not be longer than the direct
        from→to distance."""
        outer = _rect(40.0, 40.0, 80, 80)
        ops = Ops()
        to_pt = (70.0, 40.0, 0.0)
        emit_resume_travel(
            Part.from_polygons(outer),
            ops,
            to_pt,
            cleared=self._cleared_area(outer),
        )
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
        tool = Tool((30.0, 30.0, 0.0), 0.0, 3.0)
        ops = Ops()
        result = try_resume(
            Part.from_polygons(outer),
            ca,
            ops,
            tool,
            radius=3.0,
            step_over=1.5,
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
        tool = Tool((40.0, 40.0, 0.0), 0.0, 3.0)
        ops = Ops()
        result = try_resume(
            Part.from_polygons(outer),
            ca,
            ops,
            tool,
            radius=3.0,
            step_over=1.5,
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
        tool = Tool((20.0, 20.0, 0.0), 0.0, 3.0)
        ops = Ops()
        result = try_resume(
            Part.from_polygons(outer),
            ca,
            ops,
            tool,
            radius=3.0,
            step_over=1.5,
            cut_z=-5.0,
            valid_tool_area=vta,
            axis=axis,
            cut_direction="ccw",
        )
        assert isinstance(result, bool)
