"""Tests for raygeo.geo.shape.line functions."""

from raygeo.geo.shape.line import does_line_cross_polygon


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
