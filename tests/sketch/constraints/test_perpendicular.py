"""Port of the reference perpendicular constraint tests against the
raygeo bindings."""

import math
from unittest.mock import MagicMock

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.perpendicular import (
    PerpendicularConstraint,
    PerpendicularTarget,
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


def test_perpendicular_constraint_line_line(setup_env):
    reg = setup_env

    # Line 1: Horizontal (0,0) -> (10,0)
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    # Line 2: Vertical (5,5) -> (5,15). Vector is (0, 10)
    p3 = reg.add_point(5, 5)
    p4 = reg.add_point(5, 15)
    reg.add_line(p3, p4)

    c = PerpendicularConstraint(PerpendicularTarget.LineLine(p1, p2, p3, p4))
    # Dot product: (10, 0) . (0, 10) = 0
    assert c.error(reg)[0] == pytest.approx(0.0)
    assert c.user_visible() is True

    # Make Line 2 NOT perpendicular (Slope 1)
    # Move p4 to (15, 15). Vector (10, 10)
    reg.set_point_position(p4, 15, 15)

    # Dot product: (10, 0) . (10, 10) = 100
    assert c.error(reg)[0] == pytest.approx(100.0)


def test_perpendicular_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(5, 5)
    p4 = reg.add_point(5, 15)
    reg.add_line(p3, p4)

    c = PerpendicularConstraint(
        PerpendicularTarget.LineLine(p1, p2, p3, p4),
        user_visible=False,
    )
    assert c.user_visible() is False

    c2 = PerpendicularConstraint(
        PerpendicularTarget.LineLine(p1, p2, p3, p4),
        user_visible=True,
    )
    assert c2.user_visible() is True


def test_perpendicular_constraint_extended(setup_env):
    """Tests perpendicular constraint for Line-Circle and
    Circle-Circle."""
    reg = setup_env

    # --- Line-Circle Test ---
    # Circle at (10,10), radius 5
    c1_p = reg.add_point(10, 10)
    c1_r = reg.add_point(15, 10)
    reg.add_circle(c1_p, c1_r)

    # Line passing through center (0,10) -> (20,10)
    l1_p1 = reg.add_point(0, 10)
    l1_p2 = reg.add_point(20, 10)
    reg.add_line(l1_p1, l1_p2)

    # Error is cross product of (L2-L1) and (C-L1)
    # (20,0) x (10-0, 10-10) = 20*0 - 10*0 = 0
    lc_constraint = PerpendicularConstraint(
        PerpendicularTarget.LineCenter(l1_p1, l1_p2, c1_p)
    )
    assert lc_constraint.error(reg)[0] == pytest.approx(0.0)

    # Move line so it doesn't pass through center
    reg.set_point_position(l1_p1, 0, 0)
    reg.set_point_position(l1_p2, 20, 0)
    # Line is now (0,0)->(20,0). Center is (10,10).
    # Vector L2-L1: (20, 0)
    # Vector C-L1: (10, 10)
    # Cross product: (20 * 10) - (10 * 0) = 200
    assert lc_constraint.error(reg)[0] == pytest.approx(200.0)

    # --- Circle-Circle Test ---
    # C1 at (0,0), radius 3 (r^2=9)
    c2_p = reg.add_point(0, 0)
    c2_r = reg.add_point(3, 0)
    reg.add_circle(c2_p, c2_r)

    # C2 at (5,0), radius 4 (r^2=16), distance between centers = 5
    c3_p = reg.add_point(5, 0)
    c3_r = reg.add_point(9, 0)  # 5+4
    reg.add_circle(c3_p, c3_r)

    # Error is r1^2 + r2^2 - d^2 = 9 + 16 - 25 = 0
    cc_constraint = PerpendicularConstraint(
        PerpendicularTarget.Circles(c2_p, c2_r, c3_p, c3_r)
    )
    assert cc_constraint.error(reg)[0] == pytest.approx(0.0)

    # Move C2 center to (6,0), d^2 = 36
    reg.set_point_position(c3_p, 6, 0)
    # Radius of circ3 changes! New radius is (9-6)=3, so r2^2=9.
    # Error = r1^2 + new_r2^2 - d^2 = 9 + 9 - 36 = -18
    assert cc_constraint.error(reg)[0] == pytest.approx(-18.0)


# deferred: test_perpendicular_constraint_invalid_type — the binding
# resolves entities to points, so an invalid entity type cannot be
# passed.


# deferred: targets_segment is only bound on dimensional constraints


def test_perpendicular_constraint_gradient(setup_env):
    reg = setup_env
    p0 = reg.add_point(0, 0)
    p1 = reg.add_point(10, 2)
    p2 = reg.add_point(5, 10)
    p3 = reg.add_point(3, -5)
    reg.add_line(p0, p1)
    reg.add_line(p2, p3)

    constraint = PerpendicularConstraint(
        PerpendicularTarget.LineLine(p0, p1, p2, p3)
    )

    _assert_gradient_matches_fd(reg, constraint, [p0, p1, p2, p3])


def test_perpendicular_gradient_with_shared_points(setup_env):
    reg = setup_env
    p0 = reg.add_point(0, 0)  # p0: C1 center AND C2 radius point
    p1 = reg.add_point(3, 0)  # p1: C1 radius point
    p2 = reg.add_point(5, 0)  # p2: C2 center
    reg.add_circle(p0, p1)
    reg.add_circle(p2, p0)

    constraint = PerpendicularConstraint(
        PerpendicularTarget.Circles(p0, p1, p2, p0)
    )

    _assert_gradient_matches_fd(reg, constraint, [p0, p1, p2])


def test_perpendicular_constraint_line_circle_gradient(setup_env):
    reg = setup_env
    l1p1 = reg.add_point(0, 5)
    l1p2 = reg.add_point(20, 5)
    c1p = reg.add_point(10, 10)
    c1r = reg.add_point(15, 10)
    reg.add_line(l1p1, l1p2)
    reg.add_circle(c1p, c1r)

    constraint = PerpendicularConstraint(
        PerpendicularTarget.LineCenter(l1p1, l1p2, c1p)
    )

    _assert_gradient_matches_fd(reg, constraint, [l1p1, l1p2, c1p])


def test_perpendicular_constraint_serialization_round_trip(setup_env):
    reg = setup_env

    # Line 1: Horizontal (0,0) -> (10,0)
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    # Line 2: Vertical (5,5) -> (5,15)
    p3 = reg.add_point(5, 5)
    p4 = reg.add_point(5, 15)
    reg.add_line(p3, p4)

    # Create original constraint
    original = PerpendicularConstraint(
        PerpendicularTarget.LineLine(p1, p2, p3, p4)
    )

    # Serialize to dict
    serialized = original.to_dict()

    # Deserialize from dict through the registry and rebuild
    handle = reg.add_constraint_json(serialized)
    data = reg.constraint_dict(handle)
    restored = PerpendicularConstraint(
        PerpendicularTarget.LineLine(
            data["a1_idx"], data["a2_idx"], data["b1_idx"], data["b2_idx"]
        )
    )

    # Check that the restored constraint has the same error
    assert original.error(reg) == restored.error(reg)
    assert original.user_visible() == restored.user_visible()


def test_perpendicular_is_hit(setup_env):
    reg = setup_env

    threshold = 15.0

    # --- Test Line-Line case ---
    l1p1 = reg.add_point(0, 50)
    l1p2 = reg.add_point(100, 50)
    reg.add_line(l1p1, l1p2)

    l2p1 = reg.add_point(50, 0)
    l2p2 = reg.add_point(50, 100)
    reg.add_line(l2p1, l2p2)

    c_ll = PerpendicularConstraint(
        PerpendicularTarget.LineLine(l1p1, l1p2, l2p1, l2p2)
    )

    # Visuals are centered at intersection (50, 50)
    # The dot is placed at mid-angle with radius 0.6 * 16 = 9.6
    # ang1 ~ 0 (from (100,50)), ang2 ~ PI/2 (from (50,100))
    # mid-angle is ~PI/4. Dot pos ~ (50+9.6*cos(PI/4), 50+9.6*sin(PI/4))
    hit_x = 50 + 9.6 * math.cos(math.pi / 4)
    hit_y = 50 + 9.6 * math.sin(math.pi / 4)
    assert c_ll.is_hit(hit_x, hit_y, threshold, reg) is True
    assert c_ll.is_hit(0, 0, threshold, reg) is False

    # --- Test Line-Circle case ---
    c1p = reg.add_point(150, 50)
    c1r = reg.add_point(160, 50)
    reg.add_circle(c1p, c1r)
    c_lc = PerpendicularConstraint(
        PerpendicularTarget.LineCenter(l1p1, l1p2, c1p)
    )

    # Visuals are a box at the intersection of line and circle.
    # No intersection, so it defaults to the center of the circle
    # (150, 50).
    assert c_lc.is_hit(150, 50, threshold, reg) is True
    assert c_lc.is_hit(0, 0, threshold, reg) is False


# deferred: can_apply_to — command/UI layer


def test_perpendicular_draw(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(5, 5)
    p4 = reg.add_point(5, 15)
    reg.add_line(p1, p2)
    reg.add_line(p3, p4)

    c = PerpendicularConstraint(PerpendicularTarget.LineLine(p1, p2, p3, p4))

    ctx = MagicMock()

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)
