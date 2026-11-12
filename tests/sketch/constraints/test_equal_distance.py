"""Port of the reference equal-distance constraint tests against the
raygeo bindings."""

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.equal_distance import EqualDistanceConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _solver(reg, constraint, pids):
    points = []
    for pid in pids:
        position = reg.get_point(pid)
        assert position is not None
        points.append(PointDef(pid, position[0], position[1], False))
    return Solver(points, [constraint], [])


def _assert_gradient_matches_fd(reg, constraint, pids, tol=1e-5, eps=1e-6):
    """Compares the assembled analytic Jacobian against a central
    finite difference of the residuals (check_grad port)."""
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
            for row, value in enumerate(plus):
                finite = (value - minus[row]) / (2.0 * eps)
                analytic = jacobian[row][2 * col + axis]
                assert analytic == pytest.approx(finite, abs=tol)


def test_equal_distance_constraint(setup_env):
    reg = setup_env
    # Segment 1: Length 10
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    # Segment 2: Length 4
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)

    c = EqualDistanceConstraint(p1, p2, p3, p4)

    # Error should be 10 - 4 = 6
    assert c.error(reg)[0] == pytest.approx(6.0)
    assert c.user_visible() is True


def test_equal_distance_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)

    c = EqualDistanceConstraint(p1, p2, p3, p4, user_visible=False)
    assert c.user_visible() is False

    c2 = EqualDistanceConstraint(p1, p2, p3, p4, user_visible=True)
    assert c2.user_visible() is True


def test_equal_distance_targets_segment(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)
    p5 = reg.add_point(5, 5)

    c = EqualDistanceConstraint(p1, p2, p3, p4)

    # Matches pair 1 (order independent)
    assert c.targets_segment(p1, p2, None, reg) is True
    assert c.targets_segment(p2, p1, None, reg) is True

    # Matches pair 2 (order independent)
    assert c.targets_segment(p3, p4, None, reg) is True
    assert c.targets_segment(p4, p3, None, reg) is True

    # No match
    assert c.targets_segment(p1, p3, None, reg) is False
    assert c.targets_segment(p1, p5, None, reg) is False


def test_equal_distance_constraint_gradient(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(15, 15)
    p4 = reg.add_point(15, 22)

    constraint = EqualDistanceConstraint(p1, p2, p3, p4)

    _assert_gradient_matches_fd(reg, constraint, [p1, p2, p3, p4])


def test_equal_distance_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    # Segment 1: Length 10
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    # Segment 2: Length 4
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 14)

    # Create original constraint
    original = EqualDistanceConstraint(p1, p2, p3, p4)

    # Serialize to dict
    serialized = original.to_dict()

    # Deserialize from dict through the registry and rebuild
    handle = reg.add_constraint_json(serialized)
    data = reg.constraint_dict(handle)
    restored = EqualDistanceConstraint(
        data["p1"], data["p2"], data["p3"], data["p4"]
    )

    # Check that the restored constraint has the same error
    assert original.error(reg) == restored.error(reg)
    assert original.user_visible() == restored.user_visible()


# deferred: draw — command/UI layer
