"""Port of the reference point-on-line constraint tests against the
raygeo bindings."""

from unittest.mock import MagicMock

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.point_on_line import (
    PointOnLineConstraint,
    PointOnTarget,
)
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


def test_point_on_line_constraint(setup_env):
    reg = setup_env

    # Line along X-axis: (0,0) -> (10,0)
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    # Point at (5, 5). Distance is 5.
    # Error is signed distance: cross product / length
    # cross = (10-0)*(5-0) - (5-0)*(0-0) = 50
    # length = 10. Error = 50/10 = 5.
    p3 = reg.add_point(5, 5)

    c = PointOnLineConstraint(p3, PointOnTarget.Line(p1, p2))
    assert c.error(reg)[0] == pytest.approx(5.0)
    assert c.user_visible() is True

    # Move point to (5, 0). Error should be 0.
    reg.set_point_position(p3, 5, 0)
    assert c.error(reg)[0] == pytest.approx(0.0)


def test_point_on_line_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(5, 5)

    c = PointOnLineConstraint(
        p3, PointOnTarget.Line(p1, p2), user_visible=False
    )
    assert c.user_visible() is False

    c2 = PointOnLineConstraint(
        p3, PointOnTarget.Line(p1, p2), user_visible=True
    )
    assert c2.user_visible() is True


def test_point_on_arc_circle_constraint(setup_env):
    """Tests PointOnLine constraint for Arc and Circle types."""
    reg = setup_env

    # Circle at (0,0) with radius 10
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(10, 0)
    reg.add_circle(center, radius_pt)

    # Point on the circle circumference
    pt_on = reg.add_point(0, 10)
    c1 = PointOnLineConstraint(
        pt_on, PointOnTarget.RadiusCurve(center, radius_pt)
    )
    # Error: dist(pt, center) - radius = 10 - 10 = 0
    assert c1.error(reg)[0] == pytest.approx(0.0)

    # Point outside the circle
    pt_off = reg.add_point(0, 12)
    c2 = PointOnLineConstraint(
        pt_off, PointOnTarget.RadiusCurve(center, radius_pt)
    )
    # Error: dist(pt, center) - radius = 12 - 10 = 2
    assert c2.error(reg)[0] == pytest.approx(2.0)


def test_point_on_line_constrains_radius(setup_env):
    """Test that PointOnLineConstraint correctly reports constraining
    the radius when the point on the circle is itself constrained."""
    reg = setup_env

    # Setup Circle
    c = reg.add_point(0, 0, fixed=True)
    r_pt = reg.add_point(10, 0, fixed=False)  # Not fixed initially
    circ_id = reg.add_circle(c, r_pt)

    # Setup Constrained Point on Circle
    p_constrained = reg.add_point(0, 10, fixed=False)
    reg.set_point_constrained(p_constrained, True)

    # Setup Unconstrained Point on Circle
    p_unconstrained = reg.add_point(0, -10, fixed=False)
    reg.set_point_constrained(p_unconstrained, False)

    # Constraint 1: Constrained Point -> Circle
    c1 = PointOnLineConstraint(
        p_constrained, PointOnTarget.RadiusCurve(c, r_pt)
    )
    assert c1.constrains_radius(reg, circ_id) is True

    # Constraint 2: Unconstrained Point -> Circle
    c2 = PointOnLineConstraint(
        p_unconstrained, PointOnTarget.RadiusCurve(c, r_pt)
    )
    assert c2.constrains_radius(reg, circ_id) is False

    # Constraint 3: Wrong Entity ID
    assert c1.constrains_radius(reg, 999) is False


# deferred: targets_segment is only bound on dimensional constraints

# deferred: test_point_on_line_invalid_entity — the binding resolves
# constraints to points, so an invalid entity id cannot be passed.


def test_point_on_line_zero_length(setup_env):
    """Test denominator protection for PointOnLine with a zero-length
    line."""
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(0, 0)
    reg.add_line(p1, p2)
    p3 = reg.add_point(5, 5)
    c_pol = PointOnLineConstraint(p3, PointOnTarget.Line(p1, p2))
    expected_dist = (5**2 + 5**2) ** 0.5
    assert c_pol.error(reg)[0] == pytest.approx(expected_dist)


def test_point_on_line_gradient(setup_env):
    reg = setup_env
    pt_id = reg.add_point(5, 6)
    l1_id = reg.add_point(0, 0)
    l2_id = reg.add_point(10, 2)
    reg.add_line(l1_id, l2_id)

    constraint = PointOnLineConstraint(pt_id, PointOnTarget.Line(l1_id, l2_id))

    _assert_gradient_matches_fd(reg, constraint, [pt_id, l1_id, l2_id])


def test_point_on_circle_gradient(setup_env):
    reg = setup_env
    pt_id = reg.add_point(10, 12)
    c_id = reg.add_point(5, 5)
    r_id = reg.add_point(15, 5)
    reg.add_circle(c_id, r_id)

    constraint = PointOnLineConstraint(
        pt_id, PointOnTarget.RadiusCurve(c_id, r_id)
    )

    _assert_gradient_matches_fd(reg, constraint, [pt_id, c_id, r_id])


# deferred: serialization round trip — the constraint dict stores
# entity ids while the resolved constraint stores point ids, pending
# the json serialization alignment.


def test_point_on_line_is_hit(setup_env):
    reg = setup_env
    p1 = reg.add_point(10, 20)
    l1 = reg.add_point(0, 0)
    l2 = reg.add_point(100, 100)
    reg.add_line(l1, l2)
    c = PointOnLineConstraint(p1, PointOnTarget.Line(l1, l2))

    threshold = 15.0

    # Hit the point
    assert c.is_hit(10, 20, threshold, reg) is True
    # Miss the point
    assert c.is_hit(30, 20, threshold, reg) is False


# deferred: can_apply_to — command/UI layer


def test_point_on_line_draw(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(5, 5)
    reg.add_line(p1, p2)

    c = PointOnLineConstraint(p3, PointOnTarget.Line(p1, p2))

    ctx = MagicMock()

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)
