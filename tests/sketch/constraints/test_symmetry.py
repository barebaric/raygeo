"""Port of the reference symmetry constraint tests against the raygeo
bindings."""

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.symmetry import (
    SymmetryConstraint,
    SymmetryTarget,
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


def test_symmetry_constraint_point(setup_env):
    """Test symmetry between two points with respect to a center."""
    reg = setup_env
    # Center at (0,0)
    pc = reg.add_point(0, 0)
    # P1 at (-5, -2)
    p1 = reg.add_point(-5, -2)
    # P2 at (5, 2) (perfectly symmetric)
    p2 = reg.add_point(5, 2)

    c = SymmetryConstraint(p1, p2, SymmetryTarget.Point(pc))

    # Error vector: [(x1+x2) - 2xc, (y1+y2) - 2yc]
    # x: (-5 + 5) - 0 = 0
    # y: (-2 + 2) - 0 = 0
    assert c.error(reg) == pytest.approx([0.0, 0.0])
    assert c.user_visible() is True

    # Move P2 to (6, 2)
    # x: (-5 + 6) - 0 = 1
    reg.set_point_position(p2, 6.0, 2.0)
    assert c.error(reg) == pytest.approx([1.0, 0.0])


def test_symmetry_constraint_user_visible(setup_env):
    reg = setup_env
    pc = reg.add_point(0, 0)
    p1 = reg.add_point(-5, -2)
    p2 = reg.add_point(5, 2)

    c = SymmetryConstraint(
        p1, p2, SymmetryTarget.Point(pc), user_visible=False
    )
    assert c.user_visible() is False

    c2 = SymmetryConstraint(
        p1, p2, SymmetryTarget.Point(pc), user_visible=True
    )
    assert c2.user_visible() is True


# deferred: targets_segment is only bound on dimensional constraints


def test_symmetry_constraint_line(setup_env):
    """Test symmetry between two points with respect to an axis
    line."""
    reg = setup_env
    # Axis on Y-axis: (0, -10) -> (0, 10)
    l1 = reg.add_point(0, -10)
    l2 = reg.add_point(0, 10)
    reg.add_line(l1, l2)

    # P1 at (-5, 5), P2 at (5, 5) (perfectly symmetric)
    p1 = reg.add_point(-5, 5)
    p2 = reg.add_point(5, 5)

    c = SymmetryConstraint(p1, p2, SymmetryTarget.Line(l1, l2))
    assert c.error(reg) == pytest.approx([0.0, 0.0])

    # Move P2 up by 1 -> (5, 6)
    reg.set_point_position(p2, 5, 6)

    # 1. Perpendicularity check (Dot product)
    # Axis Vector: (0, 20)
    # Point Vector P1->P2: (10, 1)
    # Dot: 0*10 + 20*1 = 20
    expected_perp_err = 20.0

    # 2. Midpoint on line check (Cross product logic)
    # Midpoint of P1(-5, 5) and P2(5, 6) is (0, 5.5)
    # Line Start L1(0, -10). Vector L1->Mid is (0, 15.5)
    # Axis Vector L1->L2 is (0, 20)
    # Cross product 2D: (0 * 20) - (15.5 * 0) = 0
    expected_coll_err = 0.0

    err = c.error(reg)
    assert err[0] == pytest.approx(expected_perp_err)
    assert err[1] == pytest.approx(expected_coll_err)


def test_symmetry_constraint_point_gradient(setup_env):
    reg = setup_env
    p1 = reg.add_point(-5, 2)
    p2 = reg.add_point(5, -3)
    c = reg.add_point(1, -1)

    constraint = SymmetryConstraint(p1, p2, SymmetryTarget.Point(c))

    _assert_gradient_matches_fd(reg, constraint, [p1, p2, c])


def test_symmetry_constraint_line_gradient(setup_env):
    reg = setup_env
    p1 = reg.add_point(-5, 10)
    p2 = reg.add_point(6, 11)
    l1 = reg.add_point(0, 0)
    l2 = reg.add_point(0, 20)
    reg.add_line(l1, l2)

    constraint = SymmetryConstraint(p1, p2, SymmetryTarget.Line(l1, l2))

    _assert_gradient_matches_fd(reg, constraint, [p1, p2, l1, l2])


# deferred: serialization round trips — the constraint dict resolves
# center/axis through entity ids while the resolved constraint stores
# point ids, pending the json serialization alignment.


def test_symmetry_is_hit(setup_env):
    reg = setup_env
    p1 = reg.add_point(-50, 0)
    p2 = reg.add_point(50, 0)
    center = reg.add_point(0, 0)
    c = SymmetryConstraint(p1, p2, SymmetryTarget.Point(center))

    threshold = 15.0

    # Midpoint is (0,0). Angle is 0. Offset is 12.
    # Symbol points are at (-12, 0) and (12, 0)
    # Hit left symbol
    assert c.is_hit(-12, 0, threshold, reg) is True
    # Hit right symbol
    assert c.is_hit(12, 0, threshold, reg) is True
    # Miss
    assert c.is_hit(50, 50, threshold, reg) is False


# deferred: can_apply_to — command/UI layer
# deferred: draw — command/UI layer
