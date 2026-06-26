"""Tests for cut search module."""

import math

from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.search import (
    ToolPose,
    search_frontier_engagement,
    search_reengagement,
)


def _circle(cx, cy, r, n=32):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


class TestSearchFrontierEngagement:
    def test_returns_tool_pose(self):
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            0.1,
            float("inf"),
        )
        assert isinstance(result, ToolPose)

    def test_none_for_empty(self):
        ca = ClearedArea(boundary=[])
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            0.1,
            float("inf"),
        )
        assert result is None

    def test_none_when_min_too_high(self):
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1e6,
            float("inf"),
        )
        assert result is None

    def test_skips_closest_vertex(self):
        """Result differs from the closest frontier vertex (not start=end)."""
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            0.1,
            float("inf"),
        )
        assert result is not None
        # Result should be at least one vertex away.
        d = math.hypot(result.pos[0] - 50.0, result.pos[1] - 55.0)
        assert d > 0.1


class TestSearchReengagement:
    def test_returns_tool_pose(self):
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_reengagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            0.1,
        )
        assert isinstance(result, ToolPose)

    def test_none_for_empty(self):
        ca = ClearedArea(boundary=[])
        result = search_reengagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            0.1,
        )
        assert result is None

    def test_forward_and_backward_return_valid(self):
        """Both forward and backward return valid results."""
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        fwd = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            0.1,
            float("inf"),
        )
        bwd = search_reengagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            0.1,
        )
        assert fwd is not None and bwd is not None


class TestToolPose:
    def test_repr(self):
        rp = ToolPose(pos=(10.0, 20.0), heading=1.5)
        s = repr(rp)
        assert "ToolPose" in s
        assert "10" in s
