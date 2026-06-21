"""Tests for raygeo.geo.shape.line functions."""

from raygeo.geo.shape.line import (
    does_line_cross_polygon,
    get_segment_segment_distance,
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
