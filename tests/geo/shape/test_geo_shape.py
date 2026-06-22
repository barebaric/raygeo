import pytest

from raygeo.geo.shape.line import (
    does_line_segment_intersect_circle,
    does_line_segment_intersect_rect,
    get_line_closest_point,
    get_line_line_intersection,
    get_line_segment_closest_point,
    get_line_segment_intersection,
    get_line_segment_length,
    get_line_segment_polygon_intersections,
    get_point_line_distance,
    interpolated_segment_3d,
    is_point_on_line_segment,
)
from raygeo.geo.shape.point import circumcenter_2d, midpoint, transform_point
from raygeo.geo.shape.polygon import is_point_inside_polygon
from raygeo.geo.shape.rect import (
    does_rect_contain_rect,
    is_point_inside_rect,
)


@pytest.fixture
def square_polygon():
    return [(0, 0), (10, 0), (10, 10), (0, 10)]


def test_is_point_inside_polygon(square_polygon):
    # Points inside
    assert is_point_inside_polygon((5, 5), square_polygon) is True
    assert is_point_inside_polygon((0.1, 0.1), square_polygon) is True

    # Points outside
    assert is_point_inside_polygon((15, 5), square_polygon) is False
    assert is_point_inside_polygon((-5, 5), square_polygon) is False
    assert is_point_inside_polygon((5, 15), square_polygon) is False
    assert is_point_inside_polygon((5, -5), square_polygon) is False

    # Points on edge should be considered inside
    assert is_point_inside_polygon((5, 0), square_polygon) is True
    assert is_point_inside_polygon((10, 5), square_polygon) is True
    assert is_point_inside_polygon((5, 10), square_polygon) is True
    assert is_point_inside_polygon((0, 5), square_polygon) is True
    assert is_point_inside_polygon((0, 0), square_polygon) is True
    assert is_point_inside_polygon((10, 10), square_polygon) is True


def test_get_line_segment_intersection():
    # Crossing lines
    p1, p2 = (0, 0), (10, 10)
    p3, p4 = (0, 10), (10, 0)
    assert get_line_segment_intersection(p1, p2, p3, p4) == pytest.approx(
        (5, 5)
    )

    # T-junction (endpoint on segment)
    p1, p2 = (0, 0), (10, 0)
    p3, p4 = (5, -5), (5, 5)
    assert get_line_segment_intersection(p1, p2, p3, p4) == pytest.approx(
        (5, 0)
    )

    # No intersection (parallel)
    p1, p2 = (0, 0), (10, 0)
    p3, p4 = (0, 5), (10, 5)
    assert get_line_segment_intersection(p1, p2, p3, p4) is None

    # No intersection (not parallel, but segments don't meet)
    p1, p2 = (0, 0), (1, 1)
    p3, p4 = (0, 10), (1, 9)
    assert get_line_segment_intersection(p1, p2, p3, p4) is None

    # Collinear, overlapping
    p1, p2 = (0, 0), (5, 0)
    p3, p4 = (3, 0), (8, 0)
    # Our simple implementation returns None for collinear cases.
    assert get_line_segment_intersection(p1, p2, p3, p4) is None


def test_get_line_segment_polygon_intersections():
    p1 = (0.0, 50.0)
    p2 = (100.0, 50.0)
    region = [(40.0, 45.0), (60.0, 45.0), (60.0, 55.0), (40.0, 55.0)]

    # Test a simple crossing
    intersections = get_line_segment_polygon_intersections(p1, p2, [region])
    # Should find intersections at 40% and 60% of the line
    assert intersections == pytest.approx([0.0, 0.4, 0.6, 1.0])

    # Test a line fully outside
    p_out1 = (-20, 0)
    p_out2 = (-10, 0)
    intersections = get_line_segment_polygon_intersections(
        p_out1, p_out2, [region]
    )
    # Should only return the start and end points
    assert intersections == pytest.approx([0.0, 1.0])


def test_get_line_line_intersection():
    # Intersection
    p1, p2 = (0, 0), (10, 10)
    p3, p4 = (0, 10), (10, 0)
    assert get_line_line_intersection(p1, p2, p3, p4) == pytest.approx((5, 5))

    # Parallel lines
    p1, p2 = (0, 0), (10, 0)
    p3, p4 = (0, 1), (10, 1)
    assert get_line_line_intersection(p1, p2, p3, p4) is None

    # Intersection outside segments (infinite lines)
    p1, p2 = (0, 0), (1, 0)
    p3, p4 = (0, 1), (0, 2)
    # x-axis and y-axis intersect at 0,0
    assert get_line_line_intersection(p1, p2, p3, p4) == pytest.approx((0, 0))


def test_is_point_on_line_segment():
    p1, p2 = (0, 0), (10, 10)
    # Point in the middle
    assert is_point_on_line_segment((5, 5), p1, p2) is True
    # Endpoints
    assert is_point_on_line_segment((0, 0), p1, p2) is True
    assert is_point_on_line_segment((10, 10), p1, p2) is True
    # Point on the line, but outside segment
    assert is_point_on_line_segment((11, 11), p1, p2) is False
    assert is_point_on_line_segment((-1, -1), p1, p2) is False


def test_get_line_closest_point():
    # Case 1: Simple horizontal line
    p1, p2 = (0, 0), (10, 0)
    x, y = 5, 5
    assert get_line_closest_point(p1, p2, x, y) == pytest.approx((5, 0))

    # Case 2: Simple vertical line
    p1, p2 = (0, 0), (0, 10)
    x, y = 5, 5
    assert get_line_closest_point(p1, p2, x, y) == pytest.approx((0, 5))

    # Case 3: Diagonal line y=x
    p1, p2 = (0, 0), (10, 10)
    x, y = 0, 10
    assert get_line_closest_point(p1, p2, x, y) == pytest.approx((5, 5))

    # Case 4: Point is already on the line
    p1, p2 = (0, 0), (10, 10)
    x, y = 3, 3
    assert get_line_closest_point(p1, p2, x, y) == pytest.approx((3, 3))

    # Case 5: Point is outside the segment p1-p2
    # The function is for an *infinite* line, so it should still work
    p1, p2 = (0, 0), (10, 0)
    x, y = 20, 5
    assert get_line_closest_point(p1, p2, x, y) == pytest.approx((20, 0))

    # Case 6: Edge case - p1 and p2 are the same point
    p1, p2 = (5, 5), (5, 5)
    x, y = 10, 10
    # The function should return p1 in this case
    assert get_line_closest_point(p1, p2, x, y) == pytest.approx((5, 5))


def test_get_line_segment_closest_point():
    p1, p2 = (0, 0), (10, 0)

    # Closest point is projection
    t, pt, dist_sq = get_line_segment_closest_point(p1, p2, 5, 5)
    assert t == pytest.approx(0.5)
    assert pt == pytest.approx((5, 0))
    assert dist_sq == pytest.approx(25)

    # Closest point is p1
    t, pt, dist_sq = get_line_segment_closest_point(p1, p2, -5, 5)
    assert t == pytest.approx(0.0)
    assert pt == pytest.approx((0, 0))
    assert dist_sq == pytest.approx(50)

    # Closest point is p2
    t, pt, dist_sq = get_line_segment_closest_point(p1, p2, 15, 5)
    assert t == pytest.approx(1.0)
    assert pt == pytest.approx((10, 0))
    assert dist_sq == pytest.approx(50)

    # Point is on the segment
    t, pt, dist_sq = get_line_segment_closest_point(p1, p2, 7, 0)
    assert t == pytest.approx(0.7)
    assert pt == pytest.approx((7, 0))
    assert dist_sq == pytest.approx(0)


@pytest.fixture
def selection_rect():
    return (10.0, 10.0, 50.0, 50.0)


def test_is_point_inside_rect(selection_rect):
    # Inside
    assert is_point_inside_rect((25, 25), selection_rect)
    # On edge
    assert is_point_inside_rect((10, 25), selection_rect)
    assert is_point_inside_rect((25, 50), selection_rect)
    # Outside
    assert not is_point_inside_rect((5, 25), selection_rect)
    assert not is_point_inside_rect((60, 25), selection_rect)


def test_rect_a_contains_rect_b(selection_rect):
    contained_rect = (20, 20, 40, 40)
    touching_rect = (10, 20, 40, 40)
    intersecting_rect = (40, 40, 60, 60)
    outside_rect = (100, 100, 120, 120)
    assert does_rect_contain_rect(selection_rect, contained_rect)
    assert does_rect_contain_rect(selection_rect, touching_rect)
    assert not does_rect_contain_rect(selection_rect, intersecting_rect)
    assert not does_rect_contain_rect(selection_rect, outside_rect)


def test_does_line_segment_intersect_rect(selection_rect):
    # Fully contained
    assert does_line_segment_intersect_rect((20, 20), (40, 40), selection_rect)
    # One point in, one out
    assert does_line_segment_intersect_rect((25, 25), (60, 60), selection_rect)
    # Crossing through
    assert does_line_segment_intersect_rect((0, 25), (60, 25), selection_rect)
    # Touching edge
    assert does_line_segment_intersect_rect((0, 10), (20, 10), selection_rect)
    # Fully outside
    assert not does_line_segment_intersect_rect((0, 0), (5, 5), selection_rect)
    # Bbox intersects, and segment does too (diagonal case)
    assert does_line_segment_intersect_rect((0, 60), (60, 0), selection_rect)


def test_midpoint():
    a = (1.0, 2.0, 3.0)
    b = (5.0, 6.0, 7.0)
    assert midpoint(a, b) == (3.0, 4.0, 5.0)


def test_midpoint_negative():
    assert midpoint((-2.0, 0.0, 4.0), (2.0, 0.0, -4.0)) == (
        0.0,
        0.0,
        0.0,
    )


def test_get_line_segment_length():
    assert get_line_segment_length((0, 0), (3, 4)) == pytest.approx(5.0)
    assert get_line_segment_length((0, 0), (0, 0)) == pytest.approx(0.0)


def test_get_point_line_distance():
    d = get_point_line_distance((0, 1), (0, 0), (1, 0))
    assert d == pytest.approx(1.0)
    d = get_point_line_distance((0.5, 0), (0, 0), (1, 0))
    assert d == pytest.approx(0.0)
    d = get_point_line_distance((1, 1), (0, 0), (0, 0))
    assert d == pytest.approx(2**0.5)


def test_does_line_segment_intersect_circle():
    assert does_line_segment_intersect_circle((0, 0), (10, 0), (5, 0), 2)
    assert does_line_segment_intersect_circle((0, 0), (10, 0), (5, 2), 2)
    assert not does_line_segment_intersect_circle((0, 5), (10, 5), (5, 0), 2)


def test_transform_point():
    mat = [[1, 0, 0, 10], [0, 1, 0, 20], [0, 0, 1, 30], [0, 0, 0, 1]]
    result = transform_point(mat, 1, 2, 3)
    assert result == (11.0, 22.0, 33.0)

    scale_mat = [[2, 0, 0, 0], [0, 3, 0, 0], [0, 0, 4, 0], [0, 0, 0, 1]]
    result = transform_point(scale_mat, 1, 2, 3)
    assert result == (2.0, 6.0, 12.0)


class TestCircumcenter2D:
    def test_right_triangle(self):
        # 3-4-5 right triangle: circumcenter is midpoint of hypotenuse
        center, radius = circumcenter_2d((0, 0), (4, 0), (0, 3))
        assert center == pytest.approx((2.0, 1.5))
        assert radius == pytest.approx(2.5)

    def test_equilateral_triangle(self):
        center, radius = circumcenter_2d((0, 0), (2, 0), (1, 3**0.5))
        assert center == pytest.approx((1.0, 3**0.5 / 3))
        assert radius == pytest.approx(2 * 3**0.5 / 3)

    def test_collinear_returns_negative_radius(self):
        center, radius = circumcenter_2d((0, 0), (1, 1), (2, 2))
        assert center == (0.0, 0.0)
        assert radius == -1.0

    def test_center_is_equidistant(self):
        a, b, c = (1, 7), (-3, 2), (5, -1)
        center, radius = circumcenter_2d(a, b, c)
        d1 = ((center[0] - a[0]) ** 2 + (center[1] - a[1]) ** 2) ** 0.5
        d2 = ((center[0] - b[0]) ** 2 + (center[1] - b[1]) ** 2) ** 0.5
        d3 = ((center[0] - c[0]) ** 2 + (center[1] - c[1]) ** 2) ** 0.5
        assert d1 == pytest.approx(d2)
        assert d2 == pytest.approx(d3)
        assert d1 == pytest.approx(radius)


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
