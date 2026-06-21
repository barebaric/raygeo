"""Tests for raygeo.geo.shape.line functions."""

from raygeo.geo.shape.line import (
    does_line_cross_polygon,
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
