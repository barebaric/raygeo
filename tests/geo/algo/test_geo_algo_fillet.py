"""Tests for raygeo.geo.algo.fillet module."""

import math

from raygeo.geo.algo.fillet import (
    append_end_fillets,
    create_fillet_polyline,
    trim_to_safe_fillet_span,
)


def _arc_length(polyline: list[tuple[float, float]]) -> float:
    """Sum of segment lengths along an open polyline."""
    total = 0.0
    for i in range(len(polyline) - 1):
        dx = polyline[i + 1][0] - polyline[i][0]
        dy = polyline[i + 1][1] - polyline[i][1]
        total += math.sqrt(dx * dx + dy * dy)
    return total


class TestCreateFilletPolyline:
    def test_quarter_circle(self):
        """Default 90° quarter-circle has expected arc length."""
        p = (0.0, 0.0)
        dir_ = (1.0, 0.0)
        radius = 10.0
        sweep_angle = math.pi / 2
        center, polyline = create_fillet_polyline(
            p, dir_, radius, sweep_angle, 1.0, False
        )
        # arc length ≈ πr/2 (polyline approximation with 5 segments)
        expected = math.pi * radius / 2
        assert abs(_arc_length(polyline) - expected) < 0.15
        # start point is p
        assert polyline[0] == p
        # centre distance = r
        cx, cy = center
        dx = polyline[0][0] - cx
        dy = polyline[0][1] - cy
        assert abs(math.sqrt(dx * dx + dy * dy) - radius) < 0.001

    def test_quarter_circle_reverse(self):
        """Reverse flag produces arc on the opposite side."""
        p = (0.0, 0.0)
        dir_ = (1.0, 0.0)
        radius = 10.0
        sweep_angle = math.pi / 2
        _, fwd = create_fillet_polyline(
            p, dir_, radius, sweep_angle, 1.0, False
        )
        _, rev = create_fillet_polyline(
            p, dir_, radius, sweep_angle, 1.0, True
        )
        # endpoints differ: forward ends at (r, r), reverse at (-r, r)
        assert abs(fwd[-1][0] - radius) < 0.001
        assert abs(fwd[-1][1] - radius) < 0.001
        assert abs(rev[-1][0] + radius) < 0.001
        assert abs(rev[-1][1] - radius) < 0.001

    def test_sign_negative(self):
        """Negative side places arc on opposite side of direction."""
        p = (0.0, 0.0)
        dir_ = (1.0, 0.0)
        radius = 10.0
        sweep_angle = math.pi / 2
        _, pos = create_fillet_polyline(
            p, dir_, radius, sweep_angle, 1.0, False
        )
        _, neg = create_fillet_polyline(
            p, dir_, radius, sweep_angle, -1.0, False
        )
        # positive side → arc goes up (positive y)
        # negative side → arc goes down (negative y)
        mid_idx = len(pos) // 2
        assert pos[mid_idx][1] > 0
        assert neg[mid_idx][1] < 0

    def test_half_circle(self):
        """180° sweep produces a half-circle."""
        p = (0.0, 0.0)
        dir_ = (1.0, 0.0)
        radius = 10.0
        sweep_angle = math.pi
        center, polyline = create_fillet_polyline(
            p, dir_, radius, sweep_angle, 1.0, False
        )
        # arc length ≈ πr (polyline approximation with 9 segments)
        expected = math.pi * radius
        assert abs(_arc_length(polyline) - expected) < 0.25
        # centre distance = r
        cx, cy = center
        for pt in polyline:
            dx = pt[0] - cx
            dy = pt[1] - cy
            assert abs(math.sqrt(dx * dx + dy * dy) - radius) < 0.001

    def test_45_degrees(self):
        """45° sweep has correct arc length."""
        p = (0.0, 0.0)
        dir_ = (1.0, 0.0)
        radius = 10.0
        sweep_angle = math.pi / 4
        _, polyline = create_fillet_polyline(
            p, dir_, radius, sweep_angle, 1.0, False
        )
        expected = math.pi * radius / 4
        assert abs(_arc_length(polyline) - expected) < 0.1

    def test_zero_radius(self):
        """Zero radius produces degenerate polyline (start == end)."""
        p = (5.0, 5.0)
        dir_ = (1.0, 0.0)
        center, polyline = create_fillet_polyline(
            p, dir_, 0.0, math.pi / 2, 1.0, False
        )
        assert center == (5.0, 5.0)
        assert all(pt == p for pt in polyline)

    def test_zero_angle(self):
        """Zero sweep angle produces collinear points along direction."""
        p = (5.0, 5.0)
        dir_ = (1.0, 0.0)
        _, polyline = create_fillet_polyline(p, dir_, 10.0, 0.0, 1.0, False)
        # n_arc is clamped to 4, so we get 5 points all at p
        assert len(polyline) == 5
        for pt in polyline:
            assert abs(pt[0] - p[0]) < 0.001
            assert abs(pt[1] - p[1]) < 0.001

    def test_degenerate_direction(self):
        """Zero-length direction defaults to +X."""
        p = (3.0, 4.0)
        dir_ = (0.0, 0.0)
        center, polyline = create_fillet_polyline(
            p, dir_, 5.0, math.pi / 2, 1.0, False
        )
        # should behave like dir=(1,0)
        expected_center = (3.0, 9.0)  # p + side * n * r where n=(0,1)
        assert abs(center[0] - expected_center[0]) < 0.001
        assert abs(center[1] - expected_center[1]) < 0.001
        assert len(polyline) > 1


class TestAppendEndFillets:
    def test_basic_polyline(self):
        """Fillets added to a 3-point polyline produce a longer path."""
        polyline = [(0.0, 0.0), (50.0, 0.0), (100.0, 0.0)]
        radius = 5.0
        result = append_end_fillets(polyline, radius, math.pi / 2, 1.0)
        assert len(result) > len(polyline)
        # start point should be filleted backward
        assert result[0][0] < 0.0  # reversed fillet goes left of start
        # end point should be filleted forward
        assert result[-1][0] > 100.0  # forward fillet goes right of end

    def test_two_points(self):
        """Two-point polyline gets fillets at both ends (same direction)."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        result = append_end_fillets(polyline, 5.0, math.pi / 2, 1.0)
        assert len(result) > len(polyline)
        # reversed start fillet goes left of (0,0)
        assert result[0][0] < 0.0
        # forward end fillet goes right of (10,0)
        assert result[-1][0] > 10.0

    def test_single_point(self):
        """Single-point polyline returns as-is."""
        polyline = [(0.0, 0.0)]
        result = append_end_fillets(polyline, 5.0, math.pi / 2, 1.0)
        assert result == polyline

    def test_empty(self):
        """Empty polyline returns empty."""
        result = append_end_fillets([], 5.0, math.pi / 2, 1.0)
        assert result == []


class TestTrimToSafeFilletSpan:
    def test_no_obstacles(self):
        """Without obstacles the full span is safe."""
        polyline = [(0.0, 0.0), (50.0, 10.0), (100.0, 0.0)]
        outer = [(-10.0, -20.0), (110.0, -20.0), (110.0, 30.0), (-10.0, 30.0)]
        result = trim_to_safe_fillet_span(polyline, outer, [], 5.0, 0.0)
        assert result is not None
        enter, exit_ = result
        assert abs(enter[0] - polyline[0][0]) < 0.01
        assert abs(exit_[0] - polyline[-1][0]) < 0.01

    def test_collision_trims(self):
        """Obstacle near start forces trim inward."""
        polyline = [(0.0, 0.0), (50.0, 0.0), (100.0, 0.0)]
        outer = [(-10.0, -10.0), (110.0, -10.0), (110.0, 10.0), (-10.0, 10.0)]
        # obstacle blocking the start fillet
        obstacles = [[(0.0, -2.0), (10.0, -2.0), (10.0, 5.0), (0.0, 5.0)]]
        result = trim_to_safe_fillet_span(polyline, outer, obstacles, 5.0, 0.0)
        assert result is not None
        enter, _ = result
        # enter should be past the obstacle
        assert enter[0] > polyline[0][0] + 1.0

    def test_fully_blocked_returns_none(self):
        """When there's no safe span, return None."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)]
        # tiny outer boundary that blocks everything
        outer = [(5.0, -2.0), (15.0, -2.0), (15.0, 2.0), (5.0, 2.0)]
        result = trim_to_safe_fillet_span(polyline, outer, [], 10.0, 0.0)
        assert result is None

    def test_short_polyline(self):
        """Fewer than 3 points returns None."""
        result = trim_to_safe_fillet_span(
            [(0.0, 0.0), (10.0, 0.0)], [], [], 5.0, 0.0
        )
        assert result is None

    def test_zero_radius(self):
        """Zero radius returns None."""
        polyline = [(0.0, 0.0), (50.0, 0.0), (100.0, 0.0)]
        result = trim_to_safe_fillet_span(polyline, [], [], 0.0, 0.0)
        assert result is None
