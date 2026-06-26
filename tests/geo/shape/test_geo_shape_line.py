"""Tests for raygeo.geo.shape.line functions."""

import math

import pytest

from raygeo.geo.shape.line import (
    does_line_cross_polygon,
    does_line_segment_intersect_circle,
    does_line_segment_intersect_rect,
    get_interior_angle,
    get_line_closest_point,
    get_line_line_intersection,
    get_line_segment_closest_point,
    get_line_segment_intersection,
    get_line_segment_length,
    get_line_segment_polygon_intersections,
    get_point_line_distance,
    get_segment_segment_distance,
    interpolated_segment_3d,
    is_point_on_line_segment,
    longest_line_through_point,
)


class TestDoesLineCrossPolygon:
    def test_crosses_through_center(self):
        result = does_line_cross_polygon(
            (0.0, 5.0), (10.0, 5.0), [(2, 2), (8, 2), (8, 8), (2, 8)]
        )
        assert result is True

    def test_crosses_diagonal(self):
        result = does_line_cross_polygon(
            (0.0, 0.0), (10.0, 10.0), [(2, 2), (8, 2), (8, 8), (2, 8)]
        )
        assert result is True

    def test_touches_vertex_not_crossing(self):
        result = does_line_cross_polygon(
            (0.0, 0.0), (2.0, 2.0), [(2, 2), (8, 2), (8, 8), (2, 8)]
        )
        assert result is False

    def test_endpoint_on_edge_not_crossing(self):
        result = does_line_cross_polygon(
            (0.0, 2.0), (2.0, 2.0), [(2, 2), (8, 2), (8, 8), (2, 8)]
        )
        assert result is False

    def test_outside_no_cross(self):
        result = does_line_cross_polygon(
            (0.0, 0.0), (1.0, 1.0), [(5, 5), (10, 5), (10, 10), (5, 10)]
        )
        assert result is False

    def test_segment_inside_polygon_not_crossing(self):
        result = does_line_cross_polygon(
            (3.0, 3.0), (7.0, 3.0), [(2, 2), (8, 2), (8, 8), (2, 8)]
        )
        assert result is False

    def test_crosses_near_bottom_edge(self):
        result = does_line_cross_polygon(
            (0.0, 2.5), (10.0, 2.5), [(2, 2), (8, 2), (8, 8), (2, 8)]
        )
        assert result is True

    def test_empty_polygon_returns_false(self):
        result = does_line_cross_polygon((0.0, 0.0), (10.0, 10.0), [])
        assert result is False

    def test_crosses_triangle(self):
        result = does_line_cross_polygon(
            (0.0, 5.0), (10.0, 5.0), [(5, 0), (10, 10), (0, 10)]
        )
        assert result is True

    def test_along_edge_not_crossing(self):
        result = does_line_cross_polygon(
            (2.0, 2.0), (8.0, 2.0), [(2, 2), (8, 2), (8, 8), (2, 8)]
        )
        assert result is False


class TestLongestLineThroughPoint:
    def test_wider_than_tall_horizontal(self):
        """bbox wider than tall returns a horizontal line."""
        (x1, y1), (x2, y2) = longest_line_through_point(
            (5.0, 5.0), (0.0, 0.0, 10.0, 6.0)
        )
        assert abs(y1 - 5.0) < 1e-9
        assert abs(y2 - 5.0) < 1e-9
        assert abs(x1 - 0.0) < 1e-9
        assert abs(x2 - 10.0) < 1e-9

    def test_taller_than_wide_vertical(self):
        """bbox taller than wide returns a vertical line."""
        (x1, y1), (x2, y2) = longest_line_through_point(
            (5.0, 5.0), (0.0, 0.0, 6.0, 10.0)
        )
        assert abs(x1 - 5.0) < 1e-9
        assert abs(x2 - 5.0) < 1e-9
        assert abs(y1 - 0.0) < 1e-9
        assert abs(y2 - 10.0) < 1e-9

    def test_square_prefers_horizontal(self):
        """Square bbox (w == h) returns a horizontal line (w >= h)."""
        (x1, y1), (x2, y2) = longest_line_through_point(
            (3.0, 4.0), (0.0, 0.0, 10.0, 10.0)
        )
        assert abs(y1 - 4.0) < 1e-9
        assert abs(y2 - 4.0) < 1e-9

    def test_point_on_corner(self):
        """Point at a corner still produces a line spanning the bbox."""
        (x1, y1), (x2, y2) = longest_line_through_point(
            (0.0, 0.0), (0.0, 0.0, 10.0, 20.0)
        )
        # Taller than wide — vertical
        assert abs(x1 - 0.0) < 1e-9
        assert abs(x2 - 0.0) < 1e-9

    def test_line_span_full_extent(self):
        """Line spans full extent of the bbox along the chosen axis."""
        (x1, y1), (x2, y2) = longest_line_through_point(
            (7.0, 3.0), (2.0, 1.0, 12.0, 5.0)
        )
        # Wider than tall — horizontal
        assert abs(x1 - 2.0) < 1e-9
        assert abs(x2 - 12.0) < 1e-9


class TestGetSegmentSegmentDistance:
    def test_intersecting_segments(self):
        """Crossing segments have distance 0."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (10.0, 10.0), (0.0, 10.0), (10.0, 0.0)
        )
        assert d == 0.0

    def test_parallel_separated(self):
        """Parallel segments separated by 3 units."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (10.0, 0.0), (0.0, 3.0), (10.0, 3.0)
        )
        assert abs(d - 3.0) < 1e-9

    def test_non_overlapping_parallel(self):
        """Parallel segments that don't overlap along the axis."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (5.0, 0.0), (10.0, 3.0), (15.0, 3.0)
        )
        expected = ((5.0 - 10.0) ** 2 + (0.0 - 3.0) ** 2) ** 0.5
        assert abs(d - expected) < 1e-9

    def test_skew_segments(self):
        """Skew (non-parallel, non-intersecting) segments."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (10.0, 0.0), (5.0, 5.0), (5.0, 15.0)
        )
        assert abs(d - 5.0) < 1e-9

    def test_touching_at_endpoint(self):
        """Segments meeting at an endpoint have distance 0."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (5.0, 5.0), (5.0, 5.0), (10.0, 0.0)
        )
        assert d == 0.0

    def test_degenerate_first_segment(self):
        """First segment is a single point."""
        d = get_segment_segment_distance(
            (3.0, 4.0), (3.0, 4.0), (0.0, 0.0), (10.0, 0.0)
        )
        assert abs(d - 4.0) < 1e-9

    def test_degenerate_second_segment(self):
        """Second segment is a single point."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (10.0, 0.0), (3.0, 4.0), (3.0, 4.0)
        )
        assert abs(d - 4.0) < 1e-9

    def test_collinear_overlapping(self):
        """Collinear overlapping segments have distance 0."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (10.0, 0.0), (3.0, 0.0), (7.0, 0.0)
        )
        assert d == 0.0

    def test_collinear_non_overlapping(self):
        """Collinear non-overlapping segments."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (5.0, 0.0), (10.0, 0.0), (15.0, 0.0)
        )
        assert abs(d - 5.0) < 1e-9

    def test_endpoint_to_interior(self):
        """Endpoint-to-interior (both analytic params out of bounds)."""
        d = get_segment_segment_distance(
            (0.0, 0.0), (10.0, 0.0), (1.0, 4.0), (9.0, 1.0)
        )
        assert abs(d - 1.0) < 1e-6, f"expected 1.0, got {d}"

    def test_endpoint_to_interior_reversed(self):
        """Endpoint-to-interior with swapped segment order."""
        d = get_segment_segment_distance(
            (1.0, 4.0), (9.0, 1.0), (0.0, 0.0), (10.0, 0.0)
        )
        assert abs(d - 1.0) < 1e-6, f"expected 1.0, got {d}"


class TestGetInteriorAngle:
    def test_right_angle(self):
        angle = get_interior_angle((0.0, 0.0), (0.0, 1.0), (1.0, 1.0))
        assert abs(angle - math.pi / 2) < 1e-12

    def test_acute_45(self):
        a = math.pi / 4
        c = (math.cos(a), math.sin(a))
        angle = get_interior_angle((1.0, 0.0), (0.0, 0.0), c)
        assert abs(angle - a) < 1e-12

    def test_obtuse_135(self):
        a = 3 * math.pi / 4
        c = (math.cos(a), math.sin(a))
        angle = get_interior_angle((1.0, 0.0), (0.0, 0.0), c)
        assert abs(angle - a) < 1e-12

    def test_straight_line(self):
        angle = get_interior_angle((0.0, 0.0), (1.0, 0.0), (2.0, 0.0))
        assert abs(angle - math.pi) < 1e-12

    def test_degenerate_coincident(self):
        """Returns 0.0 when p0 == p1 (zero-length edge)."""
        angle = get_interior_angle((0.0, 0.0), (0.0, 0.0), (1.0, 0.0))
        assert angle == 0.0


class TestGetLineSegmentIntersection:
    def test_crossing(self):
        result = get_line_segment_intersection(
            (0, 0), (10, 10), (0, 10), (10, 0)
        )
        assert result == pytest.approx((5, 5))

    def test_t_junction(self):
        result = get_line_segment_intersection(
            (0, 0), (10, 0), (5, -5), (5, 5)
        )
        assert result == pytest.approx((5, 0))

    def test_parallel_no_intersection(self):
        result = get_line_segment_intersection(
            (0, 0), (10, 0), (0, 5), (10, 5)
        )
        assert result is None

    def test_non_parallel_no_intersection(self):
        result = get_line_segment_intersection((0, 0), (1, 1), (0, 10), (1, 9))
        assert result is None

    def test_collinear_returns_none(self):
        result = get_line_segment_intersection((0, 0), (5, 0), (3, 0), (8, 0))
        assert result is None


class TestGetLineSegmentPolygonIntersections:
    def test_simple_crossing(self):
        region = [(40.0, 45.0), (60.0, 45.0), (60.0, 55.0), (40.0, 55.0)]
        result = get_line_segment_polygon_intersections(
            (0.0, 50.0), (100.0, 50.0), [region]
        )
        assert result == pytest.approx([0.0, 0.4, 0.6, 1.0])

    def test_fully_outside(self):
        region = [(40.0, 45.0), (60.0, 45.0), (60.0, 55.0), (40.0, 55.0)]
        result = get_line_segment_polygon_intersections(
            (-20.0, 0.0), (-10.0, 0.0), [region]
        )
        assert result == pytest.approx([0.0, 1.0])


class TestGetLineLineIntersection:
    def test_intersecting(self):
        result = get_line_line_intersection((0, 0), (10, 10), (0, 10), (10, 0))
        assert result == pytest.approx((5, 5))

    def test_parallel_returns_none(self):
        result = get_line_line_intersection((0, 0), (10, 0), (0, 1), (10, 1))
        assert result is None

    def test_intersection_outside_segment(self):
        result = get_line_line_intersection((0, 0), (1, 0), (0, 1), (0, 2))
        assert result == pytest.approx((0, 0))


class TestIsPointOnLineSegment:
    def test_midpoint(self):
        assert is_point_on_line_segment((5, 5), (0, 0), (10, 10)) is True

    def test_startpoint(self):
        assert is_point_on_line_segment((0, 0), (0, 0), (10, 10)) is True

    def test_endpoint(self):
        assert is_point_on_line_segment((10, 10), (0, 0), (10, 10)) is True

    def test_beyond_start(self):
        assert is_point_on_line_segment((-1, -1), (0, 0), (10, 10)) is False

    def test_beyond_end(self):
        assert is_point_on_line_segment((11, 11), (0, 0), (10, 10)) is False


class TestGetLineClosestPoint:
    def test_horizontal_line(self):
        result = get_line_closest_point((0, 0), (10, 0), 5, 5)
        assert result == pytest.approx((5, 0))

    def test_vertical_line(self):
        result = get_line_closest_point((0, 0), (0, 10), 5, 5)
        assert result == pytest.approx((0, 5))

    def test_diagonal_line(self):
        result = get_line_closest_point((0, 0), (10, 10), 0, 10)
        assert result == pytest.approx((5, 5))

    def test_point_on_line(self):
        result = get_line_closest_point((0, 0), (10, 10), 3, 3)
        assert result == pytest.approx((3, 3))

    def test_projection_beyond_segment(self):
        result = get_line_closest_point((0, 0), (10, 0), 20, 5)
        assert result == pytest.approx((20, 0))

    def test_degenerate_single_point(self):
        result = get_line_closest_point((5, 5), (5, 5), 10, 10)
        assert result == pytest.approx((5, 5))


class TestGetLineSegmentClosestPoint:
    def test_projection_on_segment(self):
        t, pt, d2 = get_line_segment_closest_point((0, 0), (10, 0), 5, 5)
        assert t == pytest.approx(0.5)
        assert pt == pytest.approx((5, 0))
        assert d2 == pytest.approx(25)

    def test_closest_is_p1(self):
        t, pt, d2 = get_line_segment_closest_point((0, 0), (10, 0), -5, 5)
        assert t == pytest.approx(0.0)
        assert pt == pytest.approx((0, 0))
        assert d2 == pytest.approx(50)

    def test_closest_is_p2(self):
        t, pt, d2 = get_line_segment_closest_point((0, 0), (10, 0), 15, 5)
        assert t == pytest.approx(1.0)
        assert pt == pytest.approx((10, 0))
        assert d2 == pytest.approx(50)

    def test_point_on_segment(self):
        t, pt, d2 = get_line_segment_closest_point((0, 0), (10, 0), 7, 0)
        assert t == pytest.approx(0.7)
        assert pt == pytest.approx((7, 0))
        assert d2 == pytest.approx(0)


class TestDoesLineSegmentIntersectRect:
    def test_fully_contained(self):
        r = (10.0, 10.0, 50.0, 50.0)
        assert does_line_segment_intersect_rect((20, 20), (40, 40), r)

    def test_one_point_in(self):
        r = (10.0, 10.0, 50.0, 50.0)
        assert does_line_segment_intersect_rect((25, 25), (60, 60), r)

    def test_crossing_through(self):
        r = (10.0, 10.0, 50.0, 50.0)
        assert does_line_segment_intersect_rect((0, 25), (60, 25), r)

    def test_touching_edge(self):
        r = (10.0, 10.0, 50.0, 50.0)
        assert does_line_segment_intersect_rect((0, 10), (20, 10), r)

    def test_fully_outside(self):
        r = (10.0, 10.0, 50.0, 50.0)
        assert not does_line_segment_intersect_rect((0, 0), (5, 5), r)

    def test_diagonal_crossing_bbox_intersects(self):
        r = (10.0, 10.0, 50.0, 50.0)
        assert does_line_segment_intersect_rect((0, 60), (60, 0), r)


class TestGetLineSegmentLength:
    def test_3_4_5_triangle(self):
        assert get_line_segment_length((0, 0), (3, 4)) == pytest.approx(5.0)

    def test_zero_length(self):
        assert get_line_segment_length((0, 0), (0, 0)) == pytest.approx(0.0)


class TestGetPointLineDistance:
    def test_point_above_horizontal(self):
        d = get_point_line_distance((0, 1), (0, 0), (1, 0))
        assert d == pytest.approx(1.0)

    def test_point_on_line(self):
        d = get_point_line_distance((0.5, 0), (0, 0), (1, 0))
        assert d == pytest.approx(0.0)

    def test_degenerate_line(self):
        d = get_point_line_distance((1, 1), (0, 0), (0, 0))
        assert d == pytest.approx(2**0.5)


class TestDoesLineSegmentIntersectCircle:
    def test_center_on_segment_intersects(self):
        assert does_line_segment_intersect_circle((0, 0), (10, 0), (5, 0), 2)

    def test_circle_above_segment_intersects(self):
        assert does_line_segment_intersect_circle((0, 0), (10, 0), (5, 2), 2)

    def test_circle_too_far_no_intersection(self):
        assert not does_line_segment_intersect_circle(
            (0, 5), (10, 5), (5, 0), 2
        )


class TestInterpolatedSegment3D:
    def test_n_equals_1(self):
        """n=1 returns just the end point."""
        pts = interpolated_segment_3d(0.0, 0.0, 10.0, 0.0, 5.0, 1)
        assert len(pts) == 1
        assert pts[0] == (10.0, 0.0, 5.0)

    def test_n_equals_5(self):
        """n=5 returns evenly spaced points, ending at `to`."""
        pts = interpolated_segment_3d(0.0, 0.0, 10.0, 0.0, 3.0, 5)
        assert len(pts) == 5
        assert pts[0] == (2.0, 0.0, 3.0)
        assert pts[1] == (4.0, 0.0, 3.0)
        assert pts[2] == (6.0, 0.0, 3.0)
        assert pts[3] == (8.0, 0.0, 3.0)
        assert pts[4] == (10.0, 0.0, 3.0)

    def test_n_equals_0(self):
        """n=0 returns empty list."""
        assert interpolated_segment_3d(0.0, 0.0, 10.0, 0.0, 5.0, 0) == []

    def test_diagonal_interpolation(self):
        """Diagonal segment produces correct XY and Z."""
        pts = interpolated_segment_3d(0.0, 0.0, 6.0, 8.0, 10.0, 2)
        assert len(pts) == 2
        assert pts[0] == (3.0, 4.0, 10.0)
        assert pts[1] == (6.0, 8.0, 10.0)

    def test_z_preserved(self):
        """All points share the same Z."""
        pts = interpolated_segment_3d(1.0, 2.0, 3.0, 4.0, 7.0, 10)
        for pt in pts:
            assert pt[2] == 7.0
