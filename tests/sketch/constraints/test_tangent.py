"""Port of the reference tangent constraint tests against the raygeo
bindings."""

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.tangent import TangentConstraint
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


def test_tangent_constraint(setup_env):
    reg = setup_env

    # Arc: Center at (0,0), Radius 10
    start = reg.add_point(10, 0)
    end = reg.add_point(0, 10)
    center = reg.add_point(0, 0)
    reg.add_arc(start, end, center)

    # Line: Horizontal at y=10. dist_to_line = 10. radius = 10.
    # Error = 0.
    lp1 = reg.add_point(-5, 10)
    lp2 = reg.add_point(5, 10)
    reg.add_line(lp1, lp2)

    c = TangentConstraint(lp1, lp2, center, start)
    assert c.error(reg)[0] == pytest.approx(0.0)
    assert c.user_visible() is True

    # Move line to y=20. dist=20, radius=10. Error = 10.
    reg.set_point_position(lp1, -5, 20)
    reg.set_point_position(lp2, 5, 20)

    assert c.error(reg)[0] == pytest.approx(10.0)


def test_tangent_constraint_user_visible(setup_env):
    reg = setup_env
    start = reg.add_point(10, 0)
    end = reg.add_point(0, 10)
    center = reg.add_point(0, 0)
    reg.add_arc(start, end, center)

    lp1 = reg.add_point(-5, 10)
    lp2 = reg.add_point(5, 10)
    reg.add_line(lp1, lp2)

    c = TangentConstraint(lp1, lp2, center, start, user_visible=False)
    assert c.user_visible() is False

    c2 = TangentConstraint(lp1, lp2, center, start, user_visible=True)
    assert c2.user_visible() is True


def test_tangent_constraint_on_circle(setup_env):
    reg = setup_env

    # Circle: Center at (0,0), Radius 10
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(10, 0)
    reg.add_circle(center, radius_pt)

    # Line: Horizontal at y=10. dist=10, radius=10. Error = 0.
    lp1 = reg.add_point(-5, 10)
    lp2 = reg.add_point(5, 10)
    reg.add_line(lp1, lp2)

    c = TangentConstraint(lp1, lp2, center, radius_pt)
    assert c.error(reg)[0] == pytest.approx(0.0)

    # Move line to y=20. dist=20, radius=10, Error = 10.
    reg.set_point_position(lp1, -5, 20)
    reg.set_point_position(lp2, 5, 20)

    assert c.error(reg)[0] == pytest.approx(10.0)


def test_tangent_constraint_zero_length_line(setup_env):
    """Test denominator protection for Tangent constraint."""
    reg = setup_env
    p1 = reg.add_point(0, 0)
    reg.add_line(p1, p1)

    start = reg.add_point(10, 0)
    end = reg.add_point(0, 10)
    center = reg.add_point(0, 0)
    reg.add_arc(start, end, center)

    c_tan = TangentConstraint(p1, p1, center, start)
    # Error falls back to dist(center, line_pt) - radius
    # dist = 0. radius = 10. Error = 0 - 10 = -10.
    assert c_tan.error(reg)[0] == pytest.approx(-10.0)


# deferred: targets_segment is only bound on dimensional constraints


def test_tangent_constraint_gradient(setup_env):
    reg = setup_env
    lp1 = reg.add_point(0, 12)
    lp2 = reg.add_point(10, 12)
    cp = reg.add_point(5, 0)
    rp = reg.add_point(15, 0)
    reg.add_line(lp1, lp2)
    reg.add_circle(cp, rp)

    constraint = TangentConstraint(lp1, lp2, cp, rp)

    _assert_gradient_matches_fd(reg, constraint, [lp1, lp2, cp, rp])


def test_tangent_gradient_with_shared_points(setup_env):
    reg = setup_env
    p0 = reg.add_point(5, 0)  # circle center
    p1 = reg.add_point(0, 10)  # line start
    p2 = reg.add_point(10, 10)  # line end AND circle radius point
    reg.add_line(p1, p2)
    reg.add_circle(p0, p2)

    constraint = TangentConstraint(p1, p2, p0, p2)

    # Jacobian columns follow the constraint's own point order,
    # deduplicated: (l1, l2, center, radius_point).
    _assert_gradient_matches_fd(reg, constraint, [p1, p2, p0])


def test_tangent_constraint_serialization_round_trip(setup_env):
    reg = setup_env

    # Circle: Center at (0,0), Radius 10
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(10, 0)
    reg.add_circle(center, radius_pt)

    # Line: Horizontal at y=10
    lp1 = reg.add_point(-5, 10)
    lp2 = reg.add_point(5, 10)
    reg.add_line(lp1, lp2)

    # Create original constraint
    original = TangentConstraint(lp1, lp2, center, radius_pt)

    # Serialize to dict; the dict carries the resolved point ids,
    # from which the constraint is rebuilt (the reference deserializes
    # through entity ids, which the resolved constraint no longer
    # stores).
    serialized = original.to_dict()
    restored = TangentConstraint(
        serialized["l1_idx"],
        serialized["l2_idx"],
        serialized["center_idx"],
        serialized["radius_pt_idx"],
    )

    # Check that the restored constraint has the same error
    assert original.error(reg) == restored.error(reg)
    assert original.user_visible() == restored.user_visible()


def test_tangent_is_hit(setup_env):
    reg = setup_env
    center = reg.add_point(50, 50)
    radius_pt = reg.add_point(50, 60)  # radius=10
    reg.add_circle(center, radius_pt)

    lp1 = reg.add_point(0, 60)
    lp2 = reg.add_point(100, 60)
    reg.add_line(lp1, lp2)
    c = TangentConstraint(lp1, lp2, center, radius_pt)

    threshold = 15.0

    # Tangent point is (50, 60). Normal angle is PI/2. The symbol
    # sits 15px along the normal: (50, 75).
    symbol_x, symbol_y = 50, 72

    # Hit
    assert c.is_hit(symbol_x, symbol_y, threshold, reg) is True
    # Miss
    assert c.is_hit(0, 0, threshold, reg) is False


# deferred: draw — command/UI layer
# deferred: can_apply_to — command/UI layer
