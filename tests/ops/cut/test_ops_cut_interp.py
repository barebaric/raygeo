"""Tests for the interpolation helpers (``ops.cut.interp``).

Covers ``Interpolation`` bracket logic, ``point_in_valid_area``,
and ``rotate``.
"""

import math

import pytest

from raygeo.ops.cut.interp import Interpolation, point_in_valid_area, rotate


class TestInterpolationNew:
    """Construction and initial state."""

    def test_new_returns_empty(self):
        interp = Interpolation()
        assert interp.min_angle() == -math.pi / 4.0
        assert interp.max_angle() == math.pi / 4.0

    def test_new_not_valid(self):
        assert not Interpolation().joint_is_valid()

    def test_new_has_no_pos(self):
        assert not Interpolation().has_pos((0.0, 0.0))


class TestInterpolationAdd:
    """Adding samples to the bracket."""

    def test_single_sample(self):
        interp = Interpolation()
        interp.add(
            error=-1.0,
            angle=0.0,
            pos=(0.0, 0.0),
        )
        assert not interp.joint_is_valid()

    def test_two_samples_same_sign(self):
        interp = Interpolation()
        interp.add(
            error=-2.0,
            angle=-0.3,
            pos=(0.0, 0.0),
        )
        interp.add(
            error=-1.0,
            angle=-0.1,
            pos=(1.0, 0.0),
        )
        assert not interp.joint_is_valid()

    def test_valid_bracket_after_opposite_signs(self):
        interp = Interpolation()
        interp.add(
            error=-2.0,
            angle=-0.3,
            pos=(0.0, 0.0),
        )
        interp.add(
            error=1.0,
            angle=0.2,
            pos=(1.0, 0.0),
        )
        assert interp.joint_is_valid()

    def test_valid_bracket_zero_crossing_min_negative(self):
        interp = Interpolation()
        interp.add(
            error=-0.5,
            angle=-0.2,
            pos=(0.0, 0.0),
        )
        interp.add(
            error=0.0,
            angle=0.1,
            pos=(1.0, 0.0),
        )
        assert interp.joint_is_valid()

    def test_add_refines_closer_negative(self):
        interp = Interpolation()
        interp.add(
            error=-5.0,
            angle=-0.4,
            pos=(0.0, 0.0),
        )
        interp.add(
            error=3.0,
            angle=0.3,
            pos=(1.0, 0.0),
        )
        interp.add(
            error=-1.0,
            angle=-0.1,
            pos=(2.0, 0.0),
        )
        # min should now be -1.0 (closer to zero than -5.0)
        assert interp.joint_is_valid()
        # interpolate should give a value between -0.1 and 0.3
        angle = interp.interpolate()
        assert -0.1 <= angle <= 0.3

    def test_add_refines_closer_positive(self):
        interp = Interpolation()
        interp.add(
            error=-3.0,
            angle=-0.3,
            pos=(0.0, 0.0),
        )
        interp.add(
            error=5.0,
            angle=0.4,
            pos=(1.0, 0.0),
        )
        interp.add(
            error=1.0,
            angle=0.1,
            pos=(2.0, 0.0),
        )
        assert interp.joint_is_valid()
        angle = interp.interpolate()
        assert -0.3 <= angle <= 0.1

    def test_add_discards_worst_same_sign(self):
        interp = Interpolation()
        interp.add(
            error=-10.0,
            angle=-0.5,
            pos=(0.0, 0.0),
        )
        interp.add(
            error=-5.0,
            angle=-0.3,
            pos=(1.0, 0.0),
        )
        # add a third negative sample
        interp.add(
            error=-20.0,
            angle=-0.7,
            pos=(2.0, 0.0),
        )
        # should have kept -10.0 and -5.0 (closest to zero)
        assert interp.has_pos((0.0, 0.0))
        assert interp.has_pos((1.0, 0.0))
        assert not interp.has_pos((2.0, 0.0))


class TestInterpolationQuery:
    """Query methods."""

    def test_has_pos_true(self):
        interp = Interpolation()
        interp.add(
            error=-1.0,
            angle=-0.2,
            pos=(3.0, 4.0),
        )
        interp.add(
            error=1.0,
            angle=0.2,
            pos=(5.0, 6.0),
        )
        assert interp.has_pos((3.0, 4.0))
        assert interp.has_pos((5.0, 6.0))
        assert not interp.has_pos((0.0, 0.0))

    def test_clamp_angle_within_bounds(self):
        interp = Interpolation()
        assert interp.clamp_angle(0.1, 0.5) == pytest.approx(0.1)

    def test_clamp_angle_exceeds_max(self):
        interp = Interpolation()
        assert interp.clamp_angle(1.0, 0.3) == pytest.approx(0.3)

    def test_clamp_angle_below_min(self):
        interp = Interpolation()
        assert interp.clamp_angle(-1.0, 0.3) == pytest.approx(-0.3)

    def test_clamp_angle_zero_deflection(self):
        interp = Interpolation()
        assert interp.clamp_angle(0.5, 0.0) == pytest.approx(0.0)

    def test_interpolate_empty_fallback_to_min_angle(self):
        interp = Interpolation()
        assert interp.interpolate() == interp.min_angle()

    def test_interpolate_one_sample_fallback_to_max_angle(self):
        interp = Interpolation()
        interp.add(
            error=-1.0,
            angle=-0.2,
            pos=(0.0, 0.0),
        )
        assert interp.interpolate() == interp.max_angle()

    def test_interpolate_linear(self):
        interp = Interpolation()
        interp.add(
            error=-1.0,
            angle=-0.3,
            pos=(0.0, 0.0),
        )
        interp.add(
            error=1.0,
            angle=0.3,
            pos=(1.0, 0.0),
        )
        # error = 0 at angle = 0.0
        assert interp.interpolate() == pytest.approx(0.0)

    def test_interpolate_asymmetric(self):
        interp = Interpolation()
        interp.add(
            error=-2.0,
            angle=-0.4,
            pos=(0.0, 0.0),
        )
        interp.add(
            error=1.0,
            angle=0.2,
            pos=(1.0, 0.0),
        )
        # zero crossing at angle where error = 0:
        # p = (0 - (-2)) / (1 - (-2)) = 2/3
        # angle = -0.4 * (1 - 2/3) + 0.2 * 2/3 = -0.4/3 + 0.4/3 = 0.0
        actual = interp.interpolate()
        # with clamp in [0.2, 0.8], p = 0.666... stays, so angle = 0.0
        assert actual == pytest.approx(0.0, abs=1e-10)


class TestPointInValidArea:
    """point_in_valid_area function."""

    def _square_cw(self):
        return [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)]

    def _square_ccw(self):
        return [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]

    def test_point_inside_ccw(self):
        area = [self._square_ccw()]
        assert point_in_valid_area((0.5, 0.5), area)

    def test_point_outside_ccw(self):
        area = [self._square_ccw()]
        assert not point_in_valid_area((2.0, 2.0), area)

    def test_point_inside_hole_invalid(self):
        hole = self._square_cw()  # CW = hole
        shell = self._square_ccw()
        area = [shell, hole]
        # point at center is inside shell but also inside hole → invalid
        assert not point_in_valid_area((0.5, 0.5), area)

    def test_point_outside_hole_valid(self):
        hole = self._square_cw()
        shell = [(0.0, 0.0), (3.0, 0.0), (3.0, 3.0), (0.0, 3.0)]
        area = [shell, hole]
        # point inside shell but outside hole → valid
        assert point_in_valid_area((2.0, 0.5), area)

    def test_no_ccw_shell_returns_false(self):
        area = [self._square_cw()]  # only a hole
        assert not point_in_valid_area((0.5, 0.5), area)

    def test_degenerate_polygon_skipped(self):
        """Polygons with < 3 vertices are skipped."""
        area = [[(0.0, 0.0), (1.0, 0.0)]]  # line, not a real polygon
        assert not point_in_valid_area((0.5, 0.5), area)

    def test_multiple_ccw_shells_any_works(self):
        shell_a = self._square_ccw()
        shell_b = [(5.0, 5.0), (6.0, 5.0), (6.0, 6.0), (5.0, 6.0)]
        area = [shell_a, shell_b]
        assert point_in_valid_area((5.5, 5.5), area)
        assert point_in_valid_area((0.5, 0.5), area)


class TestRotate:
    """rotate function."""

    def test_rotate_zero(self):
        x, y = rotate((1.0, 0.0), 0.0)
        assert (x, y) == pytest.approx((1.0, 0.0))

    def test_rotate_90(self):
        x, y = rotate((1.0, 0.0), math.pi / 2.0)
        assert (x, y) == pytest.approx((0.0, 1.0))

    def test_rotate_180(self):
        x, y = rotate((1.0, 0.0), math.pi)
        assert (x, y) == pytest.approx((-1.0, 0.0))

    def test_rotate_360(self):
        x, y = rotate((1.0, 0.0), 2.0 * math.pi)
        assert (x, y) == pytest.approx((1.0, 0.0))

    def test_rotate_vector_not_unit(self):
        x, y = rotate((2.0, 0.0), math.pi / 2.0)
        assert (x, y) == pytest.approx((0.0, 2.0))

    def test_rotate_negative_angle(self):
        x, y = rotate((0.0, 1.0), -math.pi / 2.0)
        assert (x, y) == pytest.approx((1.0, 0.0))
