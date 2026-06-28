"""
Tests for raygeo.polyline module.
"""

import math

import pytest

from raygeo.geo.shape.polyline import (
    get_polyline_bounds,
    get_polyline_closest_point,
    split_polyline_at_v_junctions,
    trim_polyline_angular_ends,
    trim_polyline_at,
)
from raygeo.geo.shape.polyline import (
    resample_polyline as resample_polyline_2d,
)

# --- get_polyline_closest_point ---


class TestPolylineClosestPoint:
    def test_midpoint_of_single_edge(self):
        """Closest point to the midpoint of a single-edge polyline."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        res = get_polyline_closest_point(polyline, (5.0, 5.0))
        assert res is not None
        edge_idx, t = res
        assert edge_idx == 0
        assert abs(t - 0.5) < 1e-9

    def test_at_vertex(self):
        """Closest point exactly at a vertex."""
        polyline = [(0.0, 0.0), (5.0, 5.0), (10.0, 0.0)]
        res = get_polyline_closest_point(polyline, (5.0, 5.0))
        assert res is not None
        edge_idx, t = res
        assert edge_idx == 0
        assert abs(t - 1.0) < 1e-9

    def test_closer_to_second_edge(self):
        """Point closer to the second edge returns that edge index."""
        polyline = [(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)]
        res = get_polyline_closest_point(polyline, (7.5, 5.0))
        assert res is not None
        edge_idx, t = res
        assert edge_idx == 1
        assert abs(t - 0.5) < 1e-9

    def test_off_the_end_beyond_first_vertex(self):
        """Point beyond the polyline projects to the nearest endpoint."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        res = get_polyline_closest_point(polyline, (20.0, 5.0))
        assert res is not None
        edge_idx, t = res
        assert edge_idx == 0
        assert abs(t - 1.0) < 1e-9

    def test_degenerate_single_point(self):
        """Single-point polyline returns None."""
        res = get_polyline_closest_point([(5.0, 5.0)], (0.0, 0.0))
        assert res is None

    def test_degenerate_empty(self):
        """Empty polyline returns None."""
        res = get_polyline_closest_point([], (0.0, 0.0))
        assert res is None

    def test_three_edge_polyline(self):
        """Closest point on a polyline with three edges."""
        polyline = [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (10.0, 5.0)]
        res = get_polyline_closest_point(polyline, (7.0, 2.5))
        assert res is not None
        edge_idx, t = res
        assert edge_idx == 1
        assert abs(t - 0.5) < 1e-9


# --- trim_polyline_at ---


class TestTrimPolyline:
    def test_trim_on_different_edges(self):
        """Trim between points on two different edges."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        result = trim_polyline_at(polyline, (2.0, 1.0), (12.0, 5.0))
        assert len(result) == 3
        assert result[0] == pytest.approx((2.0, 0.0))
        assert result[1] == (10.0, 0.0)
        assert result[2] == pytest.approx((10.0, 5.0))

    def test_trim_on_same_edge(self):
        """Both points on the same edge — no intermediate vertices."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        result = trim_polyline_at(polyline, (2.0, 1.0), (8.0, -1.0))
        assert len(result) == 2
        assert result[0] == pytest.approx((2.0, 0.0))
        assert result[1] == pytest.approx((8.0, 0.0))

    def test_trim_a_after_b_reversed(self):
        """a is further along than b — result goes a→b (reversed)."""
        polyline = [
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (20.0, 10.0),
        ]
        result = trim_polyline_at(polyline, (15.0, 12.0), (5.0, -2.0))
        assert len(result) == 4
        assert result[0] == pytest.approx((5.0, 0.0))
        assert result[1] == (10.0, 0.0)
        assert result[2] == (10.0, 10.0)
        assert result[3] == pytest.approx((15.0, 10.0))

    def test_trim_at_vertices(self):
        """Points exactly at existing vertices."""
        polyline = [(0.0, 0.0), (5.0, 5.0), (10.0, 0.0)]
        result = trim_polyline_at(polyline, (0.0, 0.0), (10.0, 0.0))
        assert result == [(0.0, 0.0), (5.0, 5.0), (10.0, 0.0)]

    def test_trim_preserves_intermediate_vertices(self):
        """Intermediate vertices between a and b are preserved."""
        polyline = [(0.0, 0.0), (3.0, 3.0), (7.0, 3.0), (10.0, 0.0)]
        result = trim_polyline_at(polyline, (1.0, 1.0), (9.0, 1.0))
        assert len(result) == 4
        assert result[0] == pytest.approx((1.0, 1.0))
        assert result[1] == (3.0, 3.0)
        assert result[2] == (7.0, 3.0)
        assert result[3] == pytest.approx((9.0, 1.0))

    def test_trim_same_point(self):
        """a and b at the same location — result is a single point."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        result = trim_polyline_at(polyline, (5.0, 1.0), (5.0, 1.0))
        assert len(result) == 1
        assert result[0] == pytest.approx((5.0, 0.0))

    def test_trim_degenerate_single_point(self):
        """Degenerate polyline with a single point returns itself."""
        polyline = [(5.0, 5.0)]
        result = trim_polyline_at(polyline, (0.0, 0.0), (10.0, 10.0))
        assert result == [(5.0, 5.0)]

    def test_trim_empty(self):
        """Empty polyline returns empty list."""
        polyline: list[tuple[float, float]] = []
        result = trim_polyline_at(polyline, (0.0, 0.0), (1.0, 1.0))
        assert result == []

    def test_trim_removes_adjacent_duplicates(self):
        """Adjacent near-duplicate points are removed."""
        polyline = [(0.0, 0.0), (10.0, 0.0)]
        result = trim_polyline_at(polyline, (0.0, 0.0), (10.0, 0.0))
        assert result == [(0.0, 0.0), (10.0, 0.0)]

    def test_trim_three_edge_polyline(self):
        """Trim across three edges preserves both intermediate vertices."""
        polyline = [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (10.0, 5.0)]
        result = trim_polyline_at(polyline, (2.5, -1.0), (7.5, 6.0))
        assert len(result) == 4
        assert result[0] == pytest.approx((2.5, 0.0))
        assert result[1] == (5.0, 0.0)
        assert result[2] == (5.0, 5.0)
        assert result[3] == pytest.approx((7.5, 5.0))

    def test_trim_a_at_vertex_b_on_edge(self):
        """Point a at a vertex, point b mid-edge."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        result = trim_polyline_at(polyline, (10.0, 0.0), (12.0, 5.0))
        assert len(result) == 2
        assert result[0] == (10.0, 0.0)
        assert result[1] == pytest.approx((10.0, 5.0))

    def test_trim_a_on_edge_b_at_vertex(self):
        """Point a mid-edge, point b at a vertex."""
        polyline = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        result = trim_polyline_at(polyline, (2.0, 1.0), (10.0, 10.0))
        assert len(result) == 3
        assert result[0] == pytest.approx((2.0, 0.0))
        assert result[1] == (10.0, 0.0)
        assert result[2] == (10.0, 10.0)


class TestGetPolylineBounds:
    def test_simple_rectangle(self):
        pts = [(1.0, 2.0), (5.0, 3.0), (3.0, 7.0), (0.0, 5.0)]
        min_x, min_y, max_x, max_y = get_polyline_bounds(pts)
        assert min_x == 0.0
        assert min_y == 2.0
        assert max_x == 5.0
        assert max_y == 7.0

    def test_single_point(self):
        min_x, min_y, max_x, max_y = get_polyline_bounds([(5.0, 10.0)])
        assert min_x == max_x == 5.0
        assert min_y == max_y == 10.0

    def test_two_points(self):
        min_x, min_y, max_x, max_y = get_polyline_bounds(
            [(0.0, 0.0), (10.0, 20.0)]
        )
        assert min_x == 0.0
        assert min_y == 0.0
        assert max_x == 10.0
        assert max_y == 20.0

    def test_empty(self):
        assert get_polyline_bounds([]) == (0.0, 0.0, 0.0, 0.0)


# --- trim_polyline_angular_ends ---


class TestTrimPolylineAngularEnds:
    """Tests for trim_polyline_angular_ends."""

    def test_no_trimming_equal_angles(self):
        """Square — all interior angles 90°, no trim with 25° threshold."""
        poly = [(0, 0), (10, 0), (10, 10), (0, 10)]
        result = trim_polyline_angular_ends(poly, 0, 4, math.radians(25))
        assert result == (0, 4)

    def test_trim_start_only(self):
        """Only the start vertex of the subsequence gets trimmed."""
        poly = [
            (0.0, 4.0),
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 1.0),
            (5.0, 4.0),
            (0.0, 4.0),
        ]
        result = trim_polyline_angular_ends(poly, 1, 4, math.radians(25))
        assert result == (2, 3)

    def test_trim_end_only(self):
        """Only the end vertex of the subsequence gets trimmed."""
        poly = [
            (0.0, 0.0),
            (0.0, 1.0),
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 4.0),
            (0.0, 4.0),
        ]
        result = trim_polyline_angular_ends(poly, 1, 4, math.radians(25))
        assert result == (1, 3)

    def test_trim_both_ends(self):
        """Both ends of the subsequence get trimmed in one iteration."""
        poly = [
            (0.0, -5.0),
            (2.0, 1.0),
            (1.0, 0.0),
            (10.0, 0.0),
            (19.0, 0.0),
            (18.0, 1.0),
        ]
        result = trim_polyline_angular_ends(poly, 1, 5, math.radians(25))
        assert result == (2, 3)

    def test_minimum_length_preserved(self):
        """Length-3 subsequence is never trimmed."""
        poly = [(0, 0), (10, 0), (10, 10), (0, 10)]
        result = trim_polyline_angular_ends(poly, 0, 3, math.radians(25))
        assert result == (0, 3)

    def test_short_subsequence(self):
        """Length-2 subsequence is never trimmed."""
        poly = [(0, 0), (10, 0), (10, 10), (0, 10)]
        result = trim_polyline_angular_ends(poly, 0, 2, math.radians(25))
        assert result == (0, 2)

    def test_zero_threshold_trims_everything(self):
        """Threshold of 0 trims down to minimum length (3)."""
        poly = [(0, 0), (10, 0), (10, 10), (0, 10)]
        result = trim_polyline_angular_ends(poly, 0, 4, 0.0)
        assert result == (0, 4)

    def test_large_threshold_no_trimming(self):
        """Very large threshold prevents trimming."""
        poly = [
            (0.0, 4.0),
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 1.0),
            (5.0, 4.0),
            (0.0, 4.0),
        ]
        result = trim_polyline_angular_ends(poly, 1, 4, math.radians(180))
        assert result == (1, 4)

    def test_regular_pentagon_no_trim(self):
        """Regular pentagon — all interior angles equal, no trimming."""
        n = 5
        poly = [
            (math.cos(2 * math.pi * i / n), math.sin(2 * math.pi * i / n))
            for i in range(n)
        ]
        result = trim_polyline_angular_ends(poly, 0, n, math.radians(25))
        assert result == (0, n)


class TestResamplePolyline:
    def test_empty(self):
        assert resample_polyline_2d([], 1.0) == []

    def test_single_point(self):
        assert resample_polyline_2d([(5.0, 5.0)], 1.0) == [(5.0, 5.0)]

    def test_two_points_no_resample(self):
        points = [(0.0, 0.0), (10.0, 0.0)]
        result = resample_polyline_2d(points, 10.0)
        assert result == points

    def test_fine_spacing_adds_points(self):
        points = [(0.0, 0.0), (10.0, 0.0)]
        result = resample_polyline_2d(points, 2.0)
        assert len(result) == 6
        assert result[0] == (0.0, 0.0)
        assert result[-1] == (10.0, 0.0)
        assert (2.0, 0.0) in result
        assert (4.0, 0.0) in result
        assert (6.0, 0.0) in result
        assert (8.0, 0.0) in result

    def test_negative_spacing_returns_original(self):
        points = [(0.0, 0.0), (10.0, 0.0)]
        result = resample_polyline_2d(points, -1.0)
        assert result == points

    def test_multi_segment(self):
        points = [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)]
        result = resample_polyline_2d(points, 2.0)
        assert len(result) > 3


# --- split_polyline_at_v_junctions ---


class TestSplitPolylineAtVJunctions:
    def test_no_split_on_smooth_arc(self):
        pts = [
            (
                50.0 + 30.0 * math.cos(math.pi / 2 * i / 19),
                50.0 + 30.0 * math.sin(math.pi / 2 * i / 19),
            )
            for i in range(20)
        ]
        result = split_polyline_at_v_junctions(pts, 0.436)
        assert len(result) == 1
        assert len(result[0]) == len(pts)

    def test_no_split_on_line(self):
        pts = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0)]
        result = split_polyline_at_v_junctions(pts, 0.436)
        assert len(result) == 1

    def test_split_at_sharp_v(self):
        pts = [
            (0.0, 0.0),
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 5.0),
            (10.0, 10.0),
        ]
        result = split_polyline_at_v_junctions(pts, 0.1)
        assert len(result) >= 2

    def test_small_input(self):
        pts = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]
        result = split_polyline_at_v_junctions(pts, 0.1)
        assert len(result) == 1
        assert len(result[0]) == 3

    def test_empty_input(self):
        result = split_polyline_at_v_junctions([], 0.1)
        assert len(result) == 1
        assert len(result[0]) == 0

    def test_high_threshold_no_split(self):
        pts = [
            (0.0, 0.0),
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 5.0),
            (10.0, 10.0),
        ]
        result = split_polyline_at_v_junctions(pts, 100.0)
        assert len(result) == 1

    def test_multiple_splits(self):
        pts = [
            (0.0, 0.0),
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 5.0),
            (10.0, 10.0),
            (5.0, 10.0),
            (0.0, 10.0),
            (0.0, 5.0),
            (0.0, 0.0),
        ]
        result = split_polyline_at_v_junctions(pts, 0.1)
        assert len(result) >= 3
