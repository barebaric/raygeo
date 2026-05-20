import pytest
import math
import numpy as np
from raygeo import Geometry
from raygeo.geo.path import (
    get_point_and_tangent_at,
    remove_duplicates,
    is_closed,
)
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


def test_get_point_and_tangent_at():
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)  # row 1
    assert geo.data is not None

    # Test horizontal line
    result = get_point_and_tangent_at(geo.data, 1, 0.5)
    assert result is not None
    pt, tan = result
    assert pt == pytest.approx((5, 0))
    assert tan == pytest.approx((1, 0))

    geo.line_to(10, 10)  # row 2
    assert geo.data is not None
    # Test vertical line
    result = get_point_and_tangent_at(geo.data, 2, 0.25)
    assert result is not None
    pt, tan = result
    assert pt == pytest.approx((10, 2.5))
    assert tan == pytest.approx((0, 1))

    # Test arc (CCW 90 degree from (10,10) to (0,10))
    # Start: (10,10). Center offset: (-10,0). Center: (0,10). Radius: 10.
    geo.arc_to(0, 10, i=-10, j=0, clockwise=False)  # row 3
    assert geo.data is not None
    # Start of arc
    result = get_point_and_tangent_at(geo.data, 3, 0.0)
    assert result is not None
    pt, tan = result
    assert pt == pytest.approx((10, 10))
    assert tan == pytest.approx((0, 1))  # Tangent is vertical up

    # Midpoint of arc
    result = get_point_and_tangent_at(geo.data, 3, 0.5)
    assert result is not None
    pt, tan = result
    # This arc is a spiral from (10,10) to its center (0,10), because the
    # end radius is 0.
    # At t=0.5, the radius is half the starting radius (5).
    # Point is (center_x + r*cos(angle), center_y + r*sin(angle))
    assert pt == pytest.approx((5, 10))
    # Tangent for a spiral towards the center should be perpendicular to the
    # radius vector from the center. Radius vec is (5,0), so tangent is (0,5).
    assert tan == pytest.approx((0, 1))


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
    """Tests the is_closed utility function."""
    # A perfectly closed square
    geo_closed = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    assert geo_closed.data is not None
    assert is_closed(geo_closed.data) is True

    # A nearly closed square
    geo_nearly_closed = Geometry()
    geo_nearly_closed.move_to(0, 0)
    geo_nearly_closed.line_to(10, 0)
    geo_nearly_closed.line_to(10, 10)
    geo_nearly_closed.line_to(0, 10)
    geo_nearly_closed.line_to(1e-7, -1e-7)
    assert geo_nearly_closed.data is not None
    assert is_closed(geo_nearly_closed.data, tolerance=1e-6) is True
    assert is_closed(geo_nearly_closed.data, tolerance=1e-8) is False

    # An open path
    geo_open = Geometry.from_points([(0, 0), (10, 10)], close=False)
    assert geo_open.data is not None
    assert is_closed(geo_open.data) is False

    # An empty path
    assert is_closed(np.array([])) is False

    # A single point path (less than 2 commands)
    geo_point = Geometry()
    geo_point.move_to(5, 5)
    assert geo_point.data is not None
    assert is_closed(geo_point.data) is False

    # Path that doesn't start with MoveTo
    geo_bad_start = Geometry()
    geo_bad_start.line_to(10, 10)
    geo_bad_start.line_to(0, 10)
    geo_bad_start.line_to(0, 0)
    assert geo_bad_start.data is not None
    assert is_closed(geo_bad_start.data) is False


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
