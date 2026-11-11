"""Residual tests for the individual sketch constraint types."""

import math

from raygeo.sketch import Constraint, PointDef, Solver


def _max_residuals(points, hard):
    return Solver(points, hard, []).max_residuals()


def _assert_close(value, expected, tol=1e-9):
    assert math.isclose(value, expected, abs_tol=tol), f"{value} != {expected}"


def test_horizontal_vertical():
    points = [
        PointDef(0, 1.0, 2.0, False),
        PointDef(1, 5.0, 6.0, False),
    ]
    hard = [
        Constraint.Horizontal(p1=0, p2=1),
        Constraint.Vertical(p1=0, p2=1),
    ]
    residuals = _max_residuals(points, hard)
    _assert_close(residuals[0], abs(2.0 - 6.0))
    _assert_close(residuals[1], abs(1.0 - 5.0))


def test_coincident():
    points = [
        PointDef(0, 1.0, 2.0, False),
        PointDef(1, 4.0, -1.0, False),
    ]
    residuals = _max_residuals(points, [Constraint.Coincident(p1=0, p2=1)])
    _assert_close(residuals[0], 3.0)


def test_distance():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 3.0, 4.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.Distance(p1=0, p2=1, value=10.0)]
    )
    _assert_close(residuals[0], 5.0)


def test_radius_diameter():
    points = [
        PointDef(0, 1.0, 1.0, False),
        PointDef(1, 4.0, 5.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.Radius(center=0, radius_point=1, value=10.0)]
    )
    _assert_close(residuals[0], 5.0)
    residuals = _max_residuals(
        points, [Constraint.Diameter(center=0, radius_point=1, value=10.0)]
    )
    _assert_close(residuals[0], 10.0 - 10.0)


def test_perp_line_line():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 1.0, 1.0, False),
        PointDef(3, 1.0, 3.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.PerpLineLine(a1=0, a2=1, b1=2, b2=3)]
    )
    _assert_close(residuals[0], 0.0)
    points[3] = PointDef(3, 2.0, 3.0, False)
    residuals = _max_residuals(
        points, [Constraint.PerpLineLine(a1=0, a2=1, b1=2, b2=3)]
    )
    _assert_close(residuals[0], 2.0 * 1.0 + 0.0 * 2.0)


def test_perp_line_center():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 1.0, 0.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.PerpLineCenter(l1=0, l2=1, center=2)]
    )
    _assert_close(residuals[0], 0.0)
    points[2] = PointDef(2, 1.0, 5.0, False)
    residuals = _max_residuals(
        points, [Constraint.PerpLineCenter(l1=0, l2=1, center=2)]
    )
    _assert_close(residuals[0], 10.0)


def test_perp_circles():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 3.0, 0.0, False),
        PointDef(2, 5.0, 0.0, False),
        PointDef(3, 5.0, 4.0, False),
    ]
    residuals = _max_residuals(
        points,
        [Constraint.PerpCircles(c1=0, r1_point=1, c2=2, r2_point=3)],
    )
    _assert_close(residuals[0], 9.0 + 16.0 - 25.0)


def test_tangent():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 10.0, 0.0, False),
        PointDef(2, 5.0, 2.0, False),
        PointDef(3, 5.0, 5.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.Tangent(l1=0, l2=1, center=2, radius_point=3)]
    )
    _assert_close(residuals[0], 1.0)


def test_point_on_line():
    points = [
        PointDef(0, 1.0, 2.0, False),
        PointDef(1, 0.0, 0.0, False),
        PointDef(2, 2.0, 0.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.PointOnLine(point=0, l1=1, l2=2)]
    )
    _assert_close(residuals[0], 2.0)


def test_point_on_radius_curve():
    points = [
        PointDef(0, 5.0, 0.0, False),
        PointDef(1, 0.0, 0.0, False),
        PointDef(2, 3.0, 0.0, False),
    ]
    residuals = _max_residuals(
        points,
        [Constraint.PointOnRadiusCurve(point=0, center=1, radius_point=2)],
    )
    _assert_close(residuals[0], 5.0 - 3.0)


def test_point_on_bezier():
    points = [
        PointDef(0, 1.0, 0.5, False),
        PointDef(1, 0.0, 0.0, False),
        PointDef(2, 2.0, 1.0, False),
    ]
    residuals = _max_residuals(
        points,
        [
            Constraint.PointOnBezier(
                point=0,
                start=1,
                end=2,
                cp1_offset=(0.5, 0.5),
                cp2_offset=(-0.5, 0.5),
                is_line=False,
            )
        ],
    )
    assert residuals[0] > 0.0


def test_symmetry_point():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 3.0, 4.0, False),
        PointDef(2, 1.0, 1.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.SymmetryPoint(p1=0, p2=1, center=2)]
    )
    _assert_close(residuals[0], 2.0)


def test_symmetry_line():
    points = [
        PointDef(0, -1.0, 2.0, False),
        PointDef(1, 1.0, 2.0, False),
        PointDef(2, 0.0, 0.0, False),
        PointDef(3, 0.0, 4.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.SymmetryLine(p1=0, p2=1, l1=2, l2=3)]
    )
    _assert_close(residuals[0], 0.0)


def test_collinear():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 1.0, 1.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.Collinear(p1=0, p2=1, p3=2)]
    )
    _assert_close(residuals[0], 2.0 * 1.0 - 0.0 * 1.0)


def test_equal_distance():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 3.0, 4.0, False),
        PointDef(2, 1.0, 1.0, False),
        PointDef(3, 4.0, 1.0, False),
    ]
    residuals = _max_residuals(
        points, [Constraint.EqualDistance(p1=0, p2=1, p3=2, p4=3)]
    )
    _assert_close(residuals[0], 5.0 - 3.0)


def test_equal_length():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 4.0, 0.0, False),
        PointDef(2, 0.0, 1.0, False),
        PointDef(3, 1.0, 1.0, False),
    ]
    residuals = _max_residuals(
        points,
        [Constraint.EqualLength(pairs=[[0, 1, 2, 3]])],
    )
    _assert_close(residuals[0], 3.0)


def test_aspect_ratio():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 4.0, 0.0, False),
        PointDef(2, 0.0, 1.0, False),
        PointDef(3, 0.0, 2.0, False),
    ]
    residuals = _max_residuals(
        points,
        [Constraint.AspectRatio(p1=0, p2=1, p3=2, p4=3, ratio=2.0)],
    )
    _assert_close(residuals[0], 4.0 - 2.0 * 1.0)


def test_parallelogram():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 4.0, 0.0, False),
        PointDef(2, 0.0, 3.0, False),
        PointDef(3, 4.0, 2.0, False),
    ]
    residuals = _max_residuals(
        points,
        [Constraint.Parallelogram(p_origin=0, p_width=1, p_height=2, p4=3)],
    )
    _assert_close(residuals[0], 1.0)


def test_rotational():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 0.0, 2.0, False),
    ]
    residuals = _max_residuals(
        points,
        [Constraint.Rotational(center=0, p1=1, p2=2, angle_rad=math.pi / 2)],
    )
    _assert_close(residuals[0], 0.0)


def test_angle():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 2.0, 0.0, False),
        PointDef(3, 2.0, 2.0, False),
    ]
    residuals = _max_residuals(
        points,
        [
            Constraint.Angle(
                p1=0,
                p2=1,
                p3=2,
                p4=3,
                e1_far=None,
                e2_far=None,
                value_deg=90.0,
            )
        ],
    )
    _assert_close(residuals[0], 0.0)


def test_drag():
    points = [PointDef(0, 2.0, 3.0, False)]
    residuals = _max_residuals(
        points,
        [Constraint.Drag(point=0, target_x=4.0, target_y=3.0, weight=0.5)],
    )
    _assert_close(residuals[0], 1.0)
