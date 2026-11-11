"""Tests for the sketch constraint solver."""

import math

from raygeo.sketch import Constraint, PointDef, Solver


def _stabilizers(points, weight=1e-4):
    return [
        Constraint.Drag(point=p.id, target_x=p.x, target_y=p.y, weight=weight)
        for p in points
        if not p.fixed
    ]


def _assert_close(value, expected, tol=1e-5):
    assert math.isclose(value[0], expected[0], abs_tol=tol)
    assert math.isclose(value[1], expected[1], abs_tol=tol)


def _rectangle_points():
    return [
        PointDef(0, 0.0, 0.0, True),
        PointDef(1, 90.0, 0.0, False),
        PointDef(2, 90.0, 60.0, False),
        PointDef(3, 0.0, 55.0, False),
    ]


def _rectangle_constraints():
    return [
        Constraint.Horizontal(p1=0, p2=1),
        Constraint.Vertical(p1=1, p2=2),
        Constraint.Horizontal(p1=2, p2=3),
        Constraint.Vertical(p1=3, p2=0),
        Constraint.Distance(p1=0, p2=1, value=100.0),
        Constraint.Distance(p1=1, p2=2, value=50.0),
    ]


def test_rectangle_converges():
    points = _rectangle_points()
    solver = Solver(points, _rectangle_constraints(), _stabilizers(points))
    assert solver.solve(1e-5, True)
    positions = solver.positions()
    assert positions[0] == (0.0, 0.0)
    _assert_close(positions[1], (100.0, 0.0))
    _assert_close(positions[2], (100.0, 50.0))
    _assert_close(positions[3], (0.0, 50.0))


def test_rectangle_fully_constrained():
    points = _rectangle_points()
    solver = Solver(points, _rectangle_constraints(), _stabilizers(points))
    assert solver.solve(1e-5, True)
    assert solver.constrained() == [True, True, True, True]
    assert solver.conflicting(1e-3) == []


def test_conflicting_constraint_detection():
    points = [
        PointDef(0, 0.0, 0.0, True),
        PointDef(1, 90.0, 0.0, False),
        PointDef(2, 90.0, 60.0, False),
        PointDef(3, 0.0, 55.0, False),
    ]
    hard = [
        Constraint.Horizontal(p1=0, p2=1),
        Constraint.Vertical(p1=1, p2=2),
        Constraint.Horizontal(p1=2, p2=3),
        Constraint.Coincident(p1=0, p2=3),
        Constraint.Distance(p1=0, p2=1, value=100.0),
        Constraint.Distance(p1=1, p2=2, value=50.0),
    ]
    solver = Solver(points, hard, _stabilizers(points))
    assert not solver.solve(1e-5, True)
    assert 5 in solver.conflicting(1e-3)


def test_under_constrained_dof():
    points = [
        PointDef(0, 0.0, 0.0, True),
        PointDef(1, 10.0, 0.0, False),
    ]
    hard = [Constraint.Distance(p1=0, p2=1, value=10.0)]
    solver = Solver(points, hard, _stabilizers(points))
    assert solver.solve(1e-5, True)
    constrained = solver.constrained()
    assert constrained[0] is True
    assert constrained[1] is False


def test_fully_constrained_segment():
    points = [
        PointDef(0, 0.0, 0.0, True),
        PointDef(1, 10.0, 5.0, False),
    ]
    hard = [
        Constraint.Horizontal(p1=0, p2=1),
        Constraint.Distance(p1=0, p2=1, value=10.0),
    ]
    solver = Solver(points, hard, _stabilizers(points))
    assert solver.solve(1e-5, True)
    _assert_close(solver.positions()[1], (10.0, 0.0))
    assert solver.constrained() == [True, True]


def test_scoped_solve_moves_only_scope_points():
    points = _rectangle_points()
    solver = Solver(
        points,
        _rectangle_constraints(),
        _stabilizers(points),
        point_filter=[1, 2, 3],
    )
    assert solver.solve(1e-5, False)
    positions = solver.positions()
    assert positions[0] == (0.0, 0.0)
    _assert_close(positions[1], (100.0, 0.0))


def test_decomposed_components_solve_independently():
    points = [
        PointDef(0, 0.0, 0.0, True),
        PointDef(1, 5.0, 3.0, False),
        PointDef(2, 40.0, 40.0, True),
        PointDef(3, 60.0, 40.0, False),
    ]
    hard = [
        Constraint.Distance(p1=0, p2=1, value=10.0),
        Constraint.Distance(p1=2, p2=3, value=15.0),
    ]
    solver = Solver(points, hard, _stabilizers(points))
    assert solver.solve(1e-5, True)
    positions = solver.positions()
    assert math.isclose(
        math.dist(positions[0], positions[1]), 10.0, abs_tol=1e-5
    )
    assert math.isclose(
        math.dist(positions[2], positions[3]), 15.0, abs_tol=1e-5
    )


def test_max_residuals_aligned_with_hard_constraints():
    points = [
        PointDef(0, 0.0, 0.0, True),
        PointDef(1, 10.0, 3.0, False),
    ]
    hard = [
        Constraint.Horizontal(p1=0, p2=1),
        Constraint.Distance(p1=0, p2=1, value=12.0),
    ]
    solver = Solver(points, hard, [])
    residuals = solver.max_residuals()
    assert len(residuals) == 2
    assert math.isclose(residuals[0], 3.0, abs_tol=1e-9)
    assert math.isclose(
        residuals[1], abs(math.hypot(10.0, 3.0) - 12.0), abs_tol=1e-9
    )


def test_no_constraints_succeeds():
    points = [
        PointDef(0, 1.0, 2.0, True),
        PointDef(1, 3.0, 4.0, False),
    ]
    solver = Solver(points, [], [])
    assert solver.solve(1e-5, True)
    assert solver.positions() == [(1.0, 2.0), (3.0, 4.0)]
