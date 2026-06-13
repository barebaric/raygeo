import math

import pytest

from raygeo.geo import Geometry
from raygeo.geo.algo.analysis import remove_duplicates
from raygeo.geo.shape.arc import is_arc_clockwise
from raygeo.geo.shape.line import get_angle_at_vertex
from raygeo.geo.shape.polygon import is_polygon_clockwise as is_clockwise


@pytest.fixture
def ccw_square_geometry():
    geo = Geometry()
    geo.move_to(0, 0)  # cmd 0
    geo.line_to(10, 0)  # cmd 1: bottom
    geo.line_to(10, 10)  # cmd 2: right
    geo.line_to(0, 10)  # cmd 3: top
    geo.close_path()  # cmd 4: left (back to 0,0)
    return geo


@pytest.fixture
def cw_square_geometry():
    geo = Geometry()
    geo.move_to(0, 0)  # cmd 0
    geo.line_to(0, 10)  # cmd 1: left
    geo.line_to(10, 10)  # cmd 2: top
    geo.line_to(10, 0)  # cmd 3: right
    geo.close_path()  # cmd 4: bottom (back to 0,0)
    return geo


class TestGetPointAt:
    def test_line(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        assert geo.data is not None

        result = geo.get_point_at(1, 0.5)
        assert result == pytest.approx((5.0, 0.0, 0.0))

        result = geo.get_point_at(1, 0.0)
        assert result == pytest.approx((0.0, 0.0, 0.0))

        result = geo.get_point_at(1, 1.0)
        assert result == pytest.approx((10.0, 0.0, 0.0))

    def test_line_vertical(self):
        geo = Geometry()
        geo.move_to(5, 0)
        geo.line_to(5, 10)

        result = geo.get_point_at(1, 0.25)
        assert result == pytest.approx((5.0, 2.5, 0.0))

    def test_line_diagonal(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 10)

        result = geo.get_point_at(1, 0.3)
        assert result == pytest.approx((3.0, 3.0, 0.0))

    def test_line_with_z(self):
        geo = Geometry()
        geo.move_to(0, 0, 0)
        geo.line_to(10, 0, 5)

        result = geo.get_point_at(1, 0.5)
        assert result == pytest.approx((5.0, 0.0, 2.5))

    def test_arc(self):
        geo = Geometry()
        geo.move_to(10, 0)
        geo.arc_to(0, 10, i=0, j=0, clockwise=False)
        assert geo.data is not None

        result = geo.get_point_at(1, 0.0)
        assert result is not None
        assert result[0] == pytest.approx(10.0)
        assert result[1] == pytest.approx(0.0)

        result = geo.get_point_at(1, 0.5)
        assert result is not None
        assert result[0] == pytest.approx(12.706, abs=1e-2)
        assert result[1] == pytest.approx(6.533, abs=1e-2)

    def test_bezier(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 0, c1x=0, c1y=10, c2x=10, c2y=10)
        assert geo.data is not None

        result = geo.get_point_at(1, 0.0)
        assert result == pytest.approx((0.0, 0.0, 0.0))

        result = geo.get_point_at(1, 1.0)
        assert result == pytest.approx((10.0, 0.0, 0.0))

        result = geo.get_point_at(1, 0.5)
        assert result is not None
        assert result[0] == pytest.approx(5.0, abs=1e-3)
        assert result[1] == pytest.approx(7.5, abs=1e-3)

    def test_move_returns_none(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)

        result = geo.get_point_at(0, 0.5)
        assert result is None

    def test_out_of_range_index(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)

        result = geo.get_point_at(99, 0.5)
        assert result is None

    def test_empty_geometry(self):
        geo = Geometry()
        result = geo.get_point_at(0, 0.5)
        assert result is None


class TestGetTangentAt:
    def test_line_horizontal(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)

        result = geo.get_tangent_at(1, 0.5)
        assert result == pytest.approx((1.0, 0.0))

    def test_line_vertical(self):
        geo = Geometry()
        geo.move_to(5, 0)
        geo.line_to(5, 10)

        result = geo.get_tangent_at(1, 0.5)
        assert result == pytest.approx((0.0, 1.0))

    def test_line_normalized(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(3, 4)

        result = geo.get_tangent_at(1, 0.5)
        assert result is not None
        norm = (result[0] ** 2 + result[1] ** 2) ** 0.5
        assert norm == pytest.approx(1.0)
        assert result[0] == pytest.approx(0.6)
        assert result[1] == pytest.approx(0.8)

    def test_bezier(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 0, c1x=0, c1y=10, c2x=10, c2y=10)

        result = geo.get_tangent_at(1, 0.0)
        assert result is not None
        norm = (result[0] ** 2 + result[1] ** 2) ** 0.5
        assert norm == pytest.approx(1.0)

    def test_arc(self):
        geo = Geometry()
        geo.move_to(10, 0)
        geo.arc_to(0, 10, i=0, j=0, clockwise=False)

        result = geo.get_tangent_at(1, 0.0)
        assert result is not None
        norm = (result[0] ** 2 + result[1] ** 2) ** 0.5
        assert norm == pytest.approx(1.0)

    def test_move_returns_none(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)

        result = geo.get_tangent_at(0, 0.5)
        assert result is None

    def test_out_of_range_index(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)

        result = geo.get_tangent_at(99, 0.5)
        assert result is None

    def test_empty_geometry(self):
        geo = Geometry()
        result = geo.get_tangent_at(0, 0.5)
        assert result is None


def test_get_angle_at_vertex():
    # 90 degree corner
    p0, p1, p2 = (0.0, 10.0), (0.0, 0.0), (10.0, 0.0)
    assert get_angle_at_vertex(p0, p1, p2) == pytest.approx(math.pi / 2)

    # Straight line (180 degrees)
    p0, p1, p2 = (-10.0, 0.0), (0.0, 0.0), (10.0, 0.0)
    assert get_angle_at_vertex(p0, p1, p2) == pytest.approx(math.pi)

    # 45 degree corner
    p0, p1, p2 = (0.0, 10.0), (0.0, 0.0), (10.0, 10.0)
    assert get_angle_at_vertex(p0, p1, p2) == pytest.approx(math.pi / 4)

    # Coincident points
    p0, p1, p2 = (10.0, 10.0), (0.0, 0.0), (0.0, 0.0)
    assert get_angle_at_vertex(p0, p1, p2) == pytest.approx(math.pi)


def test_remove_duplicates():
    points = [(1.0, 1.0), (1.0, 1.0), (2.0, 2.0), (2.0, 2.0)]
    assert remove_duplicates(points) == [(1.0, 1.0), (2.0, 2.0)]


def test_is_clockwise():
    # Clockwise points (right half-circle)
    points = [(0.0, 0.0, 0.0), (1.0, 1.0, 0.0), (2.0, 0.0, 0.0)]
    assert is_clockwise(points) is True

    # Counter-clockwise points (left half-circle)
    points = [(0.0, 0.0, 0.0), (-1.0, 1.0, 0.0), (-2.0, 0.0, 0.0)]
    assert is_clockwise(points) is False


def test_is_arc_clockwise_half_circle():
    """Test a semicircle moving clockwise."""
    center = (0.0, 0.0)
    points = [
        (1.0, 0.0, 0.0),
        (0.0, -1.0, 0.0),
        (-1.0, 0.0, 0.0),
    ]
    assert is_arc_clockwise(points, center) is True


def test_arc_direction_is_counter_clockwise_half_circle():
    """Test a semicircle moving counter-clockwise."""
    center = (0.0, 0.0)
    points = [
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (-1.0, 0.0, 0.0),
    ]
    assert is_arc_clockwise(points, center) is False


def test_is_arc_clockwise_full_circle():
    """Test a full clockwise circle."""
    center = (0.0, 0.0)
    points = [
        (1.0, 0.0, 0.0),
        (0.0, -1.0, 0.0),
        (-1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (1.0, 0.0, 0.0),
    ]
    assert is_arc_clockwise(points, center) is True


def test_arc_direction_is_counter_clockwise_full_circle():
    """Test a full counter-clockwise circle."""
    center = (0.0, 0.0)
    points = [
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (-1.0, 0.0, 0.0),
        (0.0, -1.0, 0.0),
        (1.0, 0.0, 0.0),
    ]
    assert is_arc_clockwise(points, center) is False


def test_arc_direction_is_minimal_clockwise_arc():
    """Test a minimal 3-point clockwise arc."""
    center = (0.0, 0.0)
    points = [
        (2.0, 0.0, 0.0),
        (1.0, -1.0, 0.0),
        (0.0, 0.0, 0.0),
    ]
    assert is_arc_clockwise(points, center) is True


def test_arc_direction_is_minimal_counter_clockwise_arc():
    """Test a minimal 3-point counter-clockwise arc."""
    center = (0.0, 0.0)
    points = [
        (2.0, 0.0, 0.0),
        (1.0, 1.0, 0.0),
        (0.0, 0.0, 0.0),
    ]
    assert is_arc_clockwise(points, center) is False


def test_arc_direction_is_crossing_angle_discontinuity_counter_clockwise():
    """Test an arc crossing the π/-π discontinuity (counter-clockwise)."""
    center = (0.0, 0.0)
    points = [
        (1.0, 0.1, 0.0),
        (0.0, 1.0, 0.0),
        (-1.0, 0.1, 0.0),
    ]
    assert is_arc_clockwise(points, center) is False


def test_arc_direction_is_small_radius_arc():
    """Test a small-radius clockwise arc."""
    center = (1.0, 1.0)
    points = [
        (1.1, 1.0, 0.0),
        (1.0, 0.9, 0.0),
        (0.9, 1.0, 0.0),
    ]
    assert is_arc_clockwise(points, center) is True


def test_is_closed():
    """Tests the Geometry.is_closed() method."""
    # A perfectly closed square
    geo_closed = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    assert geo_closed.data is not None
    assert geo_closed.is_closed(1e-6) is True

    # A nearly closed square
    geo_nearly_closed = Geometry()
    geo_nearly_closed.move_to(0, 0)
    geo_nearly_closed.line_to(10, 0)
    geo_nearly_closed.line_to(10, 10)
    geo_nearly_closed.line_to(0, 10)
    geo_nearly_closed.line_to(1e-7, -1e-7)
    assert geo_nearly_closed.data is not None
    assert geo_nearly_closed.is_closed(1e-6) is True
    assert geo_nearly_closed.is_closed(1e-8) is False

    # An open path
    geo_open = Geometry.from_points([(0, 0), (10, 10)], close=False)
    assert geo_open.data is not None
    assert geo_open.is_closed(1e-6) is False

    # An empty path
    geo_empty = Geometry()
    assert geo_empty.is_closed(1e-6) is False

    # A single point path (less than 2 commands)
    geo_point = Geometry()
    geo_point.move_to(5, 5)
    assert geo_point.data is not None
    assert geo_point.is_closed(1e-6) is False

    # Path that doesn't start with MoveTo — unreachable via Geometry API
    # since Geometry always starts with MoveTo. Previously tested the
    # raw array function is_closed() which accepted arbitrary arrays.


def test_encloses_simple():
    """Test a simple case of one square enclosing another."""
    outer = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    inner = Geometry.from_points([(2, 2), (8, 2), (8, 8), (2, 8)])
    assert outer.encloses(inner) is True
    assert inner.encloses(outer) is False


def test_encloses_separate():
    """Test non-enclosing, separate shapes."""
    geo1 = Geometry.from_points([(0, 0), (5, 0), (5, 5), (0, 5)])
    geo2 = Geometry.from_points([(10, 10), (15, 10), (15, 15), (10, 15)])
    assert geo1.encloses(geo2) is False
    assert geo2.encloses(geo1) is False


def test_encloses_intersecting():
    """Test intersecting shapes do not enclose."""
    geo1 = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    geo2 = Geometry.from_points([(5, 5), (15, 5), (15, 15), (5, 15)])
    assert geo1.encloses(geo2) is False
    assert geo2.encloses(geo1) is False


def test_encloses_touching():
    """Test touching shapes do not enclose."""
    geo1 = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    geo2 = Geometry.from_points([(10, 0), (20, 0), (20, 10), (10, 10)])
    assert geo1.encloses(geo2) is False
    assert geo2.encloses(geo1) is False


def test_encloses_with_hole():
    """Test enclosure in a shape with a hole."""
    # Outer CCW rect
    outer = Geometry.from_points([(0, 0), (20, 0), (20, 20), (0, 20)])
    # Inner CW rect (the hole)
    hole = Geometry.from_points([(5, 5), (5, 15), (15, 15), (15, 5)])

    donut = outer.copy()
    donut.extend(hole)

    # Shape fully inside the donut's material
    content_inside = Geometry.from_points([(1, 1), (4, 1), (4, 4), (1, 4)])
    assert donut.encloses(content_inside) is True

    # Shape fully inside the donut's hole
    content_in_hole = Geometry.from_points(
        [(7, 7), (13, 7), (13, 13), (7, 13)]
    )
    assert donut.encloses(content_in_hole) is False


def test_encloses_bbox_contained_but_path_outside():
    """
    Test a C-shape where the bbox contains the other shape, but path does not.
    """
    c_shape = Geometry.from_points(
        [
            (0, 0),
            (10, 0),
            (10, 1),
            (1, 1),
            (1, 9),
            (10, 9),
            (10, 10),
            (0, 10),
        ],
        close=True,
    )
    other = Geometry.from_points([(2, 4), (5, 4), (5, 6), (2, 6)])
    assert c_shape.encloses(other) is False
