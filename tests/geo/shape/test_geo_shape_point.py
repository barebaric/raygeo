import math

import pytest

from raygeo.geo.shape.point import (
    get_circumcenter,
    get_midpoint_3d,
    get_point_at_fraction,
    get_points_moving_average,
    rotate_point,
    transform_point_3d,
)


def test_midpoint_3d():
    a = (1.0, 2.0, 3.0)
    b = (5.0, 6.0, 7.0)
    assert get_midpoint_3d(a, b) == (3.0, 4.0, 5.0)


def test_midpoint_3d_negative():
    assert get_midpoint_3d((-2.0, 0.0, 4.0), (2.0, 0.0, -4.0)) == (
        0.0,
        0.0,
        0.0,
    )


def test_transform_point_3d():
    mat = [[1, 0, 0, 10], [0, 1, 0, 20], [0, 0, 1, 30], [0, 0, 0, 1]]
    result = transform_point_3d(mat, 1, 2, 3)
    assert result == (11.0, 22.0, 33.0)

    scale_mat = [[2, 0, 0, 0], [0, 3, 0, 0], [0, 0, 4, 0], [0, 0, 0, 1]]
    result = transform_point_3d(scale_mat, 1, 2, 3)
    assert result == (2.0, 6.0, 12.0)


class TestCircumcenter2D:
    def test_right_triangle(self):
        # 3-4-5 right triangle: get_circumcenter is midpoint of hypotenuse
        center, radius = get_circumcenter((0, 0), (4, 0), (0, 3))
        assert center == pytest.approx((2.0, 1.5))
        assert radius == pytest.approx(2.5)

    def test_equilateral_triangle(self):
        center, radius = get_circumcenter((0, 0), (2, 0), (1, 3**0.5))
        assert center == pytest.approx((1.0, 3**0.5 / 3))
        assert radius == pytest.approx(2 * 3**0.5 / 3)

    def test_collinear_returns_negative_radius(self):
        center, radius = get_circumcenter((0, 0), (1, 1), (2, 2))
        assert center == (0.0, 0.0)
        assert radius == -1.0

    def test_center_is_equidistant(self):
        a, b, c = (1, 7), (-3, 2), (5, -1)
        center, radius = get_circumcenter(a, b, c)
        d1 = ((center[0] - a[0]) ** 2 + (center[1] - a[1]) ** 2) ** 0.5
        d2 = ((center[0] - b[0]) ** 2 + (center[1] - b[1]) ** 2) ** 0.5
        d3 = ((center[0] - c[0]) ** 2 + (center[1] - c[1]) ** 2) ** 0.5
        assert d1 == pytest.approx(d2)
        assert d2 == pytest.approx(d3)
        assert d1 == pytest.approx(radius)


class TestRotatePoint:
    def test_zero_rotation(self):
        assert rotate_point((1.0, 0.0), 0.0) == pytest.approx((1.0, 0.0))
        assert rotate_point((0.0, 5.0), 0.0) == pytest.approx((0.0, 5.0))

    def test_quarter_turn_ccw(self):
        result = rotate_point((1.0, 0.0), math.pi / 2)
        assert result == pytest.approx((0.0, 1.0))

    def test_quarter_turn_cw(self):
        result = rotate_point((1.0, 0.0), -math.pi / 2)
        assert result == pytest.approx((0.0, -1.0))

    def test_half_turn(self):
        result = rotate_point((1.0, 2.0), math.pi)
        assert result == pytest.approx((-1.0, -2.0))

    def test_45_degrees(self):
        result = rotate_point((1.0, 0.0), math.pi / 4)
        s = math.sqrt(2) / 2
        assert result == pytest.approx((s, s))

    def test_origin_point(self):
        assert rotate_point((0.0, 0.0), 1.0) == pytest.approx((0.0, 0.0))
        assert rotate_point((0.0, 0.0), math.pi) == pytest.approx((0.0, 0.0))

    def test_full_turn(self):
        result = rotate_point((3.0, 4.0), 2 * math.pi)
        assert result == pytest.approx((3.0, 4.0))

    @pytest.mark.parametrize(
        "point, angle, expected",
        [
            (
                (1.0, 0.0),
                math.pi / 6,
                (math.cos(math.pi / 6), math.sin(math.pi / 6)),
            ),
            ((0.0, 1.0), math.pi / 2, (-1.0, 0.0)),
            ((-1.0, 0.0), math.pi, (1.0, 0.0)),
            ((1.0, 1.0), math.pi / 2, (-1.0, 1.0)),
        ],
    )
    def test_parametrized(self, point, angle, expected):
        assert rotate_point(point, angle) == pytest.approx(expected)


class TestGetPointAtFraction:
    POINTS = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]

    def test_fraction_zero_is_first(self):
        assert get_point_at_fraction(self.POINTS, 0.0) == pytest.approx(
            (0.0, 0.0)
        )

    def test_fraction_one_is_last(self):
        assert get_point_at_fraction(self.POINTS, 1.0) == pytest.approx(
            (10.0, 10.0)
        )

    def test_midpoint(self):
        assert get_point_at_fraction(self.POINTS, 0.5) == pytest.approx(
            (10.0, 0.0)
        )

    def test_interior_of_second_segment(self):
        # fraction 0.75 -> halfway along the (10,0)->(10,10) segment
        assert get_point_at_fraction(self.POINTS, 0.75) == pytest.approx(
            (10.0, 5.0)
        )

    def test_single_point(self):
        assert get_point_at_fraction([(3.0, 4.0)], 0.0) == pytest.approx(
            (3.0, 4.0)
        )
        assert get_point_at_fraction([(3.0, 4.0)], 1.0) == pytest.approx(
            (3.0, 4.0)
        )

    def test_two_points(self):
        assert get_point_at_fraction([(0.0, 0.0), (4.0, 0.0)], 0.25) == (
            pytest.approx((1.0, 0.0))
        )


class TestGetPointsMovingAverage:
    def test_identity_for_zero_radius(self):
        pts = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]
        assert get_points_moving_average(pts, 0) == pts

    def test_constant_sequence_unchanged(self):
        pts = [(5.0, 5.0)] * 5
        assert get_points_moving_average(pts, 2) == pts

    def test_interior_point_is_window_mean(self):
        pts = [(0.0, 0.0), (2.0, 0.0), (4.0, 0.0), (6.0, 0.0), (8.0, 0.0)]
        result = get_points_moving_average(pts, 1)
        # interior: mean of neighbours; ends: mean of in-bounds window
        assert result[2] == pytest.approx((4.0, 0.0))
        assert result[0] == pytest.approx((1.0, 0.0))
        assert result[1] == pytest.approx((2.0, 0.0))

    def test_ends_renormalize_over_shrinking_window(self):
        pts = [(0.0, 0.0), (3.0, 0.0), (6.0, 0.0), (9.0, 0.0)]
        result = get_points_moving_average(pts, 2)
        # first point: mean of (0,0),(3,0),(6,0)
        assert result[0] == pytest.approx((3.0, 0.0))
        # last point: mean of (3,0),(6,0),(9,0)
        assert result[3] == pytest.approx((6.0, 0.0))

    def test_empty_returns_empty(self):
        assert get_points_moving_average([], 1) == []
