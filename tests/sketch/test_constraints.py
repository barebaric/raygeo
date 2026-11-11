"""Residual tests for the individual sketch constraint types."""

import math

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.angle import AngleConstraint
from raygeo.sketch.constraints.aspect_ratio import AspectRatioConstraint
from raygeo.sketch.constraints.coincident import CoincidentConstraint
from raygeo.sketch.constraints.collinear import CollinearConstraint
from raygeo.sketch.constraints.diameter import DiameterConstraint
from raygeo.sketch.constraints.distance import DistanceConstraint
from raygeo.sketch.constraints.drag import DragConstraint
from raygeo.sketch.constraints.equal_distance import EqualDistanceConstraint
from raygeo.sketch.constraints.equal_length import EqualLengthConstraint
from raygeo.sketch.constraints.horizontal import HorizontalConstraint
from raygeo.sketch.constraints.parallelogram import ParallelogramConstraint
from raygeo.sketch.constraints.perpendicular import (
    PerpendicularConstraint,
    PerpendicularTarget,
)
from raygeo.sketch.constraints.point_on_curve import PointOnCurveConstraint
from raygeo.sketch.constraints.point_on_line import (
    PointOnLineConstraint,
    PointOnTarget,
)
from raygeo.sketch.constraints.radius import RadiusConstraint
from raygeo.sketch.constraints.rotational import RotationalConstraint
from raygeo.sketch.constraints.symmetry import (
    SymmetryConstraint,
    SymmetryTarget,
)
from raygeo.sketch.constraints.tangent import TangentConstraint
from raygeo.sketch.constraints.vertical import VerticalConstraint


def _max_residuals(points, hard):
    return Solver(points, hard, []).max_residuals()


def _assert_close(value, expected, tol=1e-9):
    assert math.isclose(value, expected, abs_tol=tol), f"{value} != {expected}"


def test_horizontal_vertical():
    points = [
        PointDef(0, 1.0, 2.0, False),
        PointDef(1, 5.0, 6.0, False),
    ]
    residuals = _max_residuals(
        points,
        [HorizontalConstraint(0, 1), VerticalConstraint(0, 1)],
    )
    _assert_close(residuals[0], abs(2.0 - 6.0))
    _assert_close(residuals[1], abs(1.0 - 5.0))


def test_coincident():
    points = [
        PointDef(0, 1.0, 2.0, False),
        PointDef(1, 4.0, -1.0, False),
    ]
    residuals = _max_residuals(points, [CoincidentConstraint(0, 1)])
    _assert_close(residuals[0], 3.0)


def test_distance():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 3.0, 4.0, False),
    ]
    residuals = _max_residuals(points, [DistanceConstraint(0, 1, 10.0)])
    _assert_close(residuals[0], 5.0)


def test_radius_diameter():
    points = [
        PointDef(0, 1.0, 1.0, False),
        PointDef(1, 4.0, 5.0, False),
    ]
    residuals = _max_residuals(points, [RadiusConstraint(0, 1, 10.0)])
    _assert_close(residuals[0], 5.0)
    residuals = _max_residuals(points, [DiameterConstraint(0, 1, 10.0)])
    _assert_close(residuals[0], 0.0)


def test_perpendicular_line_line():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 1.0, 1.0, False),
        PointDef(3, 1.0, 3.0, False),
    ]
    constraint = PerpendicularConstraint(
        PerpendicularTarget.LineLine(0, 1, 2, 3)
    )
    residuals = _max_residuals(points, [constraint])
    _assert_close(residuals[0], 0.0)
    points[3] = PointDef(3, 2.0, 3.0, False)
    residuals = _max_residuals(points, [constraint])
    _assert_close(residuals[0], 2.0 * 1.0 + 0.0 * 2.0)


def test_perpendicular_line_center():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 1.0, 0.0, False),
    ]
    constraint = PerpendicularConstraint(
        PerpendicularTarget.LineCenter(0, 1, 2)
    )
    residuals = _max_residuals(points, [constraint])
    _assert_close(residuals[0], 0.0)
    points[2] = PointDef(2, 1.0, 5.0, False)
    residuals = _max_residuals(points, [constraint])
    _assert_close(residuals[0], 10.0)


def test_perpendicular_circles():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 3.0, 0.0, False),
        PointDef(2, 5.0, 0.0, False),
        PointDef(3, 5.0, 4.0, False),
    ]
    residuals = _max_residuals(
        points,
        [PerpendicularConstraint(PerpendicularTarget.Circles(0, 1, 2, 3))],
    )
    _assert_close(residuals[0], 0.0)


def test_tangent():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 10.0, 0.0, False),
        PointDef(2, 5.0, 2.0, False),
        PointDef(3, 5.0, 5.0, False),
    ]
    residuals = _max_residuals(points, [TangentConstraint(0, 1, 2, 3)])
    _assert_close(residuals[0], 1.0)


def test_point_on_line():
    points = [
        PointDef(0, 1.0, 2.0, False),
        PointDef(1, 0.0, 0.0, False),
        PointDef(2, 2.0, 0.0, False),
    ]
    residuals = _max_residuals(
        points,
        [PointOnLineConstraint(0, PointOnTarget.Line(1, 2))],
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
        [PointOnLineConstraint(0, PointOnTarget.RadiusCurve(1, 2))],
    )
    _assert_close(residuals[0], 2.0)


def test_point_on_bezier():
    points = [
        PointDef(0, 1.0, 0.5, False),
        PointDef(1, 0.0, 0.0, False),
        PointDef(2, 2.0, 1.0, False),
    ]
    residuals = _max_residuals(
        points,
        [
            PointOnCurveConstraint(
                0,
                1,
                2,
                (0.5, 0.5),
                (-0.5, 0.5),
                False,
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
        points,
        [SymmetryConstraint(0, 1, SymmetryTarget.Point(2))],
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
        points,
        [SymmetryConstraint(0, 1, SymmetryTarget.Line(2, 3))],
    )
    _assert_close(residuals[0], 0.0)


def test_collinear():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 1.0, 1.0, False),
    ]
    residuals = _max_residuals(points, [CollinearConstraint(0, 1, 2)])
    _assert_close(residuals[0], 2.0 * 1.0 - 0.0 * 1.0)


def test_equal_distance():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 3.0, 4.0, False),
        PointDef(2, 1.0, 1.0, False),
        PointDef(3, 4.0, 1.0, False),
    ]
    residuals = _max_residuals(points, [EqualDistanceConstraint(0, 1, 2, 3)])
    _assert_close(residuals[0], 2.0)


def test_equal_length():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 4.0, 0.0, False),
        PointDef(2, 0.0, 1.0, False),
        PointDef(3, 1.0, 1.0, False),
    ]
    residuals = _max_residuals(points, [EqualLengthConstraint([[0, 1, 2, 3]])])
    _assert_close(residuals[0], 3.0)


def test_aspect_ratio():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 4.0, 0.0, False),
        PointDef(2, 0.0, 1.0, False),
        PointDef(3, 0.0, 2.0, False),
    ]
    residuals = _max_residuals(
        points, [AspectRatioConstraint(0, 1, 2, 3, 2.0)]
    )
    _assert_close(residuals[0], 2.0)


def test_parallelogram():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 4.0, 0.0, False),
        PointDef(2, 0.0, 3.0, False),
        PointDef(3, 4.0, 2.0, False),
    ]
    residuals = _max_residuals(points, [ParallelogramConstraint(0, 1, 2, 3)])
    _assert_close(residuals[0], 1.0)


def test_rotational():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 0.0, 2.0, False),
    ]
    residuals = _max_residuals(
        points,
        [RotationalConstraint(0, 1, 2, math.pi / 2)],
    )
    _assert_close(residuals[0], 0.0)


def test_angle():
    points = [
        PointDef(0, 0.0, 0.0, False),
        PointDef(1, 2.0, 0.0, False),
        PointDef(2, 2.0, 0.0, False),
        PointDef(3, 2.0, 2.0, False),
    ]
    residuals = _max_residuals(points, [AngleConstraint(0, 1, 2, 3, 90.0)])
    _assert_close(residuals[0], 0.0)


def test_drag():
    points = [PointDef(0, 2.0, 3.0, False)]
    residuals = _max_residuals(points, [DragConstraint(0, 4.0, 3.0, 0.5)])
    _assert_close(residuals[0], 1.0)
