"""Port of the reference collinear constraint tests against the
raygeo bindings."""

from unittest.mock import MagicMock

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.collinear import CollinearConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def sketch_with_points():
    """Create a registry with three points for testing."""
    reg = EntityRegistry()
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 10)
    p3 = reg.add_point(20, 0)
    return reg, p1, p2, p3


def _solver(reg, constraint, pids):
    points = []
    for pid in pids:
        position = reg.get_point(pid)
        assert position is not None
        points.append(PointDef(pid, position[0], position[1], False))
    return Solver(points, [constraint], [])


def test_collinear_constraint_serialization():
    """Test that the constraint can be serialized and deserialized."""
    constraint = CollinearConstraint(1, 2, 3)
    data = constraint.to_dict()
    assert data == {
        "type": "CollinearConstraint",
        "p1": 1,
        "p2": 2,
        "p3": 3,
        "user_visible": True,
    }
    reg = EntityRegistry()
    handle = reg.add_constraint_json(data)
    restored_data = reg.constraint_dict(handle)
    new_constraint = CollinearConstraint(
        restored_data["p1"], restored_data["p2"], restored_data["p3"]
    )
    assert new_constraint.p1 == 1
    assert new_constraint.p2 == 2
    assert new_constraint.p3 == 3
    assert new_constraint.user_visible() is True


def test_collinear_constraint_user_visible():
    """Test that user_visible can be set."""
    c = CollinearConstraint(1, 2, 3, user_visible=False)
    assert c.user_visible() is False

    c2 = CollinearConstraint(1, 2, 3, user_visible=True)
    assert c2.user_visible() is True


# deferred: targets_segment is only bound on dimensional constraints


def test_collinear_constraint_error_zero(sketch_with_points):
    """Test the error is zero for perfectly collinear points."""
    reg, p1, p2, p3 = sketch_with_points
    # Move p3 to be on the line p1-p2
    reg.set_point_position(p3, 5, 5)

    constraint = CollinearConstraint(p1, p2, p3)
    error = constraint.error(reg)[0]
    assert abs(error) < 1e-9


def test_collinear_constraint_error_non_zero(sketch_with_points):
    """Test the error is non-zero for non-collinear points."""
    reg, p1, p2, p3 = sketch_with_points
    # p1=(0,0), p2=(10,10), p3=(20,0)
    constraint = CollinearConstraint(p1, p2, p3)
    error = constraint.error(reg)[0]

    # Expected error:
    #   (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x)
    # = (10 - 0) * (0 - 0) - (10 - 0) * (20 - 0) = 0 - 10 * 20 = -200
    assert pytest.approx(error) == -200.0


def test_collinear_constraint_gradient(sketch_with_points):
    """Test the gradient calculation against a finite difference of
    the error."""
    reg, p1, p2, p3 = sketch_with_points
    constraint = CollinearConstraint(p1, p2, p3)

    pids = [p1, p2, p3]
    eps = 1e-6
    jacobian = _solver(reg, constraint, pids).jacobian()
    for col, pid in enumerate(pids):
        x, y = reg.get_point(pid)
        for axis, px, py in (
            (0, x + eps, y),
            (1, x, y + eps),
        ):
            reg.set_point_position(pid, px, py)
            plus = _solver(reg, constraint, pids).residuals()
            dx, dy = (x - eps, y) if axis == 0 else (x, y - eps)
            reg.set_point_position(pid, dx, dy)
            minus = _solver(reg, constraint, pids).residuals()
            reg.set_point_position(pid, x, y)
            finite = (plus[0] - minus[0]) / (2.0 * eps)
            assert jacobian[0][2 * col + axis] == pytest.approx(
                finite, abs=1e-5
            )


def test_collinear_solver_integration(sketch_with_points):
    """Test that applying the constraint and solving moves the
    point."""
    reg, p1, p2, p3 = sketch_with_points

    # Fix p1 and p2 to define the line
    constraint = CollinearConstraint(p1, p2, p3)
    points = []
    for pid, fixed in ((p1, True), (p2, True), (p3, False)):
        position = reg.get_point(pid)
        assert position is not None
        points.append(PointDef(pid, position[0], position[1], fixed))
    solver = Solver(points, [constraint], [])
    assert solver.solve(1e-10, False)

    # The exact final position depends on the solver's path; we
    # verify the error is now zero.
    x, y = solver.positions()[2]
    reg.set_point_position(p3, x, y)
    error = constraint.error(reg)[0]
    assert abs(error) < 1e-6


def test_collinear_draw(sketch_with_points):
    """Test that the draw method can be called without errors."""
    reg, p1, p2, p3 = sketch_with_points
    constraint = CollinearConstraint(p1, p2, p3)

    ctx = MagicMock()

    def to_screen(pos):
        return pos

    constraint.draw(ctx, reg, to_screen)
    constraint.draw(ctx, reg, to_screen, is_selected=True)
    constraint.draw(ctx, reg, to_screen, is_hovered=True)
    constraint.draw(ctx, reg, to_screen, point_radius=10.0)
