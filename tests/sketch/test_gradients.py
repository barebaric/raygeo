"""Finite-difference checks of the analytic constraint gradients.

The first Jacobian row of a single-constraint problem is compared
against central finite differences of that constraint's residual at
a generic configuration.  This catches sign and scaling mistakes
that residual-value tests miss.
"""

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

EPS = 1e-6
TOL = 1e-4


def _check_gradient(constraint, coords):
    """Compares every Jacobian row against finite differences of the
    matching residual component, for every non-fixed point."""
    coords = [list(xy) for xy in coords]

    def residuals():
        points = [PointDef(i, x, y, False) for i, (x, y) in enumerate(coords)]
        solver = Solver(points, [constraint], [])
        return solver.residuals()

    points = [PointDef(i, x, y, False) for i, (x, y) in enumerate(coords)]
    solver = Solver(points, [constraint], [])
    jacobian = solver.jacobian()
    rows = solver.residuals()
    assert len(jacobian) == len(rows)

    state_index = 0
    for i, (x, y) in enumerate(coords):
        for axis in (0, 1):
            original = coords[i][axis]
            coords[i][axis] = original + EPS
            plus = residuals()
            coords[i][axis] = original - EPS
            minus = residuals()
            coords[i][axis] = original
            finite = [(vp - vm) / (2.0 * EPS) for vp, vm in zip(plus, minus)]

            column = 2 * state_index + axis
            for j, analytic in enumerate(jacobian):
                expected = finite[j] if j < len(finite) else 0.0
                value = analytic[column] if column < len(analytic) else 0.0
                assert abs(expected - value) < TOL, (
                    f"point {i} axis {axis} residual {j}: analytic "
                    f"{value} vs finite difference {expected}"
                )
        state_index += 1


def test_distance():
    _check_gradient(DistanceConstraint(0, 1, 10.0), [(1, 2), (8, 7)])


def test_horizontal_vertical():
    _check_gradient(HorizontalConstraint(0, 1), [(1, 2), (8, 7)])
    _check_gradient(VerticalConstraint(0, 1), [(1, 2), (8, 7)])


def test_coincident():
    _check_gradient(CoincidentConstraint(0, 1), [(1, 2), (8, 7)])


def test_radius_diameter():
    _check_gradient(RadiusConstraint(0, 1, 9.0), [(1, 1), (4, 5)])
    _check_gradient(DiameterConstraint(0, 1, 9.0), [(1, 1), (4, 5)])


def test_tangent_center_above_line():
    _check_gradient(
        TangentConstraint(0, 1, 2, 3),
        [(0, 0), (10, 0), (5, 2.5), (5, 5.5)],
    )


def test_tangent_center_below_line():
    _check_gradient(
        TangentConstraint(0, 1, 2, 3),
        [(0, 0), (10, 0), (5, -2.5), (5, -5.5)],
    )


def test_point_on_line():
    _check_gradient(
        PointOnLineConstraint(0, PointOnTarget.Line(1, 2)),
        [(1, 2), (0, 0), (2, 0)],
    )


def test_point_on_radius_curve():
    _check_gradient(
        PointOnLineConstraint(0, PointOnTarget.RadiusCurve(1, 2)),
        [(5, 0), (0, 0), (3, 0)],
    )


def test_point_on_bezier():
    _check_gradient(
        PointOnCurveConstraint(0, 1, 2, (0.5, 0.5), (-0.5, 0.5), False),
        [(1, 0.5), (0, 0), (2, 1)],
    )


def test_perpendicular_cases():
    _check_gradient(
        PerpendicularConstraint(PerpendicularTarget.LineLine(0, 1, 2, 3)),
        [(0, 0), (2, 0), (1, 1), (2, 3)],
    )
    _check_gradient(
        PerpendicularConstraint(PerpendicularTarget.LineCenter(0, 1, 2)),
        [(0, 0), (2, 0), (1, 1)],
    )
    _check_gradient(
        PerpendicularConstraint(PerpendicularTarget.Circles(0, 1, 2, 3)),
        [(0, 0), (3, 0), (5, 0), (5, 4)],
    )


def test_symmetry_cases():
    _check_gradient(
        SymmetryConstraint(0, 1, SymmetryTarget.Point(2)),
        [(0, 0), (3, 4), (1, 1)],
    )
    _check_gradient(
        SymmetryConstraint(0, 1, SymmetryTarget.Line(2, 3)),
        [(-1, 2), (1, 3), (0, 0), (0, 4)],
    )


def test_collinear():
    _check_gradient(CollinearConstraint(0, 1, 2), [(0, 0), (2, 0), (1, 1)])


def test_equal_distance():
    _check_gradient(
        EqualDistanceConstraint(0, 1, 2, 3),
        [(0, 0), (3, 4), (1, 1), (4, 1)],
    )


def test_equal_length():
    _check_gradient(
        EqualLengthConstraint([[0, 1, 2, 3]]),
        [(0, 0), (4, 0), (0, 1), (1, 1)],
    )


def test_aspect_ratio():
    _check_gradient(
        AspectRatioConstraint(0, 1, 2, 3, 2.0),
        [(0, 0), (4, 0), (0, 1), (0, 2)],
    )


def test_parallelogram():
    _check_gradient(
        ParallelogramConstraint(0, 1, 2, 3),
        [(0, 0), (4, 0), (0, 3), (4, 2)],
    )


def test_rotational():
    _check_gradient(
        RotationalConstraint(0, 1, 2, math.pi / 2),
        [(0, 0), (2, 0), (0, 2)],
    )


def test_angle():
    _check_gradient(
        AngleConstraint(0, 1, 2, 3, 45.0),
        [(0, 0), (4, 0), (1, 1), (3, 3)],
    )


def test_angle_shared_joint():
    """Adjacent lines sharing a joint point: the finite difference
    of the error captures the joint in both line roles, so the
    analytic gradient must contain one combined entry, not a sum of
    two.  Regression test for a double-counting bug that stalled
    chained-angle sketches."""
    _check_gradient(
        AngleConstraint(0, 1, 1, 2, 150.0),
        [(0, 0), (2, 0), (4, 2)],
    )


def test_drag():
    _check_gradient(DragConstraint(0, 4.0, 3.0, 0.5), [(2, 3)])
