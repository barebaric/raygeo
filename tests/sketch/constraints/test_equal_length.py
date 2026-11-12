"""Port of the reference equal-length constraint tests against the
raygeo bindings."""

import cairo
import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.equal_length import EqualLengthConstraint
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


def test_equal_length_constraint(setup_env):
    """Tests error calculation for EqualLengthConstraint."""
    reg = setup_env
    # Line 1: length 10
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    # Line 2: length 5
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)
    reg.add_line(p3, p4)

    # Arc 1: radius 4 (radius point is the arc start)
    c1_p = reg.add_point(20, 0)
    s1 = reg.add_point(24, 0)
    e1 = reg.add_point(20, 4)
    reg.add_arc(s1, e1, c1_p)

    # Circle 1: radius 3
    c2_p = reg.add_point(30, 0)
    r2 = reg.add_point(33, 0)
    reg.add_circle(c2_p, r2)

    # The binding takes the entities' characteristic-length pairs
    # (base_a, base_b, other_a, other_b) instead of entity ids.
    # Test Line-Line (error is [len2 - len1])
    c = EqualLengthConstraint([[p1, p2, p3, p4]])
    assert c.error(reg) == pytest.approx([5.0 - 10.0])
    assert c.user_visible() is True

    # Test Line-Arc (error is [rad1 - len1])
    c2 = EqualLengthConstraint([[p1, p2, c1_p, s1]])
    assert c2.error(reg) == pytest.approx([4.0 - 10.0])

    # Test Arc-Circle (error is [rad_circ - rad_arc])
    c3 = EqualLengthConstraint([[c1_p, s1, c2_p, r2]])
    assert c3.error(reg) == pytest.approx([3.0 - 4.0])

    # Test multi-entity constraint
    c_multi = EqualLengthConstraint(
        [[p1, p2, p3, p4], [p1, p2, c1_p, s1], [p1, p2, c2_p, r2]]
    )
    # Errors are [len2-len1, rad_arc-len1, rad_circ-len1]
    assert c_multi.error(reg) == pytest.approx(
        [5.0 - 10.0, 4.0 - 10.0, 3.0 - 10.0]
    )

    # Test edge cases (no error for < 2 entities: zero pairs)
    assert EqualLengthConstraint([]).error(reg) == []


def test_equal_length_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)
    reg.add_line(p3, p4)

    c = EqualLengthConstraint([[p1, p2, p3, p4]], user_visible=False)
    assert c.user_visible() is False

    c2 = EqualLengthConstraint([[p1, p2, p3, p4]], user_visible=True)
    assert c2.user_visible() is True


def test_equal_length_constrains_radius_method(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    l1 = reg.add_line(p1, p2)

    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)
    l2 = reg.add_line(p3, p4)

    c = EqualLengthConstraint([[p1, p2, p3, p4]])

    # Should return True for entities in the pairs
    assert c.constrains_radius(reg, l1) is True
    assert c.constrains_radius(reg, l2) is True

    # Should return False for entities not in the pairs
    assert c.constrains_radius(reg, 999) is False


def test_equal_length_targets_segment(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    l1 = reg.add_line(p1, p2)

    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)
    l2 = reg.add_line(p3, p4)

    p5 = reg.add_point(0, 20)
    l3 = reg.add_line(p1, p5)

    c = EqualLengthConstraint([[p1, p2, p3, p4]])

    # Should match entities in the pairs
    assert c.targets_segment(0, 0, l1, reg) is True
    assert c.targets_segment(0, 0, l2, reg) is True

    # Should not match entities not in the pairs or None
    assert c.targets_segment(0, 0, l3, reg) is False
    assert c.targets_segment(0, 0, None, reg) is False


def test_equal_length_constraint_gradient(setup_env):
    reg = setup_env
    p0 = reg.add_point(0, 0)
    p1 = reg.add_point(10, 0)
    p2 = reg.add_point(15, 15)
    p3 = reg.add_point(15, 22)
    reg.add_line(p0, p1)
    reg.add_line(p2, p3)

    constraint = EqualLengthConstraint([[p0, p1, p2, p3]])

    _assert_gradient_matches_fd(reg, constraint, [p0, p1, p2, p3])


# deferred: serialization round trip — the constraint dict stores
# entity ids while the resolved constraint stores expanded pairs,
# pending the json serialization alignment.


def test_equal_length_is_hit(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(100, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 50)
    p4 = reg.add_point(0, 150)
    reg.add_line(p3, p4)

    c = EqualLengthConstraint([[p1, p2, p3, p4]])

    threshold = 15.0

    # Symbol pos for line 1: midpoint (50, 0), normal is (0, -1),
    # offset 15 -> (50, -15)
    l1_symbol_x, l1_symbol_y = 50, -15
    # Symbol pos for line 2: midpoint (0, 100), tangent angle pi/2,
    # normal angle 0 -> (15, 100)
    l2_symbol_x, l2_symbol_y = 15, 100

    # Hit line 1
    assert c.is_hit(l1_symbol_x, l1_symbol_y, threshold, reg) is True
    # Hit line 2
    assert c.is_hit(l2_symbol_x, l2_symbol_y, threshold, reg) is True
    # Miss both
    assert c.is_hit(0, 0, threshold, reg) is False


# deferred: can_apply_to — command/UI layer


def test_equal_length_error_two_ellipses(setup_env):
    reg = setup_env
    c1 = reg.add_point(0, 0)
    rx1 = reg.add_point(5, 0)
    ry1 = reg.add_point(0, 3)
    reg.add_ellipse(c1, rx1, ry1)

    c2 = reg.add_point(20, 0)
    rx2 = reg.add_point(28, 0)
    ry2 = reg.add_point(20, 5)
    reg.add_ellipse(c2, rx2, ry2)

    constr = EqualLengthConstraint([[c1, rx1, c2, rx2], [c1, ry1, c2, ry2]])
    err = constr.error(reg)
    assert len(err) == 2
    assert err[0] == pytest.approx(8.0 - 5.0)
    assert err[1] == pytest.approx(5.0 - 3.0)


def test_equal_length_error_three_ellipses(setup_env):
    reg = setup_env
    c1 = reg.add_point(0, 0)
    rx1 = reg.add_point(10, 0)
    ry1 = reg.add_point(0, 4)
    reg.add_ellipse(c1, rx1, ry1)

    c2 = reg.add_point(30, 0)
    rx2 = reg.add_point(37, 0)
    ry2 = reg.add_point(30, 9)
    reg.add_ellipse(c2, rx2, ry2)

    c3 = reg.add_point(60, 0)
    rx3 = reg.add_point(65, 0)
    ry3 = reg.add_point(60, 2)
    reg.add_ellipse(c3, rx3, ry3)

    constr = EqualLengthConstraint(
        [
            [c1, rx1, c2, rx2],
            [c1, ry1, c2, ry2],
            [c1, rx1, c3, rx3],
            [c1, ry1, c3, ry3],
        ]
    )
    err = constr.error(reg)
    assert len(err) == 4
    assert err[0] == pytest.approx(7.0 - 10.0)
    assert err[1] == pytest.approx(9.0 - 4.0)
    assert err[2] == pytest.approx(5.0 - 10.0)
    assert err[3] == pytest.approx(2.0 - 4.0)


def test_equal_length_error_ellipse_and_circle(setup_env):
    reg = setup_env
    cc = reg.add_point(0, 0)
    cr = reg.add_point(6, 0)
    reg.add_circle(cc, cr)

    ec = reg.add_point(20, 0)
    erx = reg.add_point(25, 0)
    ery = reg.add_point(20, 3)
    reg.add_ellipse(ec, erx, ery)

    # The circle has one characteristic pair; the shorter-side
    # fallback pairs the circle length with both ellipse axes.
    constr = EqualLengthConstraint([[cc, cr, ec, erx], [cc, cr, ec, ery]])
    err = constr.error(reg)
    assert len(err) == 2
    assert err[0] == pytest.approx(5.0 - 6.0)
    assert err[1] == pytest.approx(3.0 - 6.0)


def test_equal_length_gradient_two_ellipses(setup_env):
    reg = setup_env
    c1 = reg.add_point(0, 0)
    rx1 = reg.add_point(5, 0)
    ry1 = reg.add_point(0, 3)
    reg.add_ellipse(c1, rx1, ry1)

    c2 = reg.add_point(20, 0)
    rx2 = reg.add_point(28, 0)
    ry2 = reg.add_point(20, 5)
    reg.add_ellipse(c2, rx2, ry2)

    constraint = EqualLengthConstraint(
        [[c1, rx1, c2, rx2], [c1, ry1, c2, ry2]]
    )

    _assert_gradient_matches_fd(reg, constraint, [c1, rx1, ry1, c2, rx2, ry2])


def test_equal_length_gradient_ellipse_circle(setup_env):
    reg = setup_env
    cc = reg.add_point(0, 0)
    cr = reg.add_point(6, 0)
    reg.add_circle(cc, cr)

    ec = reg.add_point(20, 0)
    erx = reg.add_point(25, 0)
    ery = reg.add_point(20, 3)
    reg.add_ellipse(ec, erx, ery)

    constraint = EqualLengthConstraint([[cc, cr, ec, erx], [cc, cr, ec, ery]])

    _assert_gradient_matches_fd(reg, constraint, [cc, cr, ec, erx, ery])


# deferred: reports referenced entities — entity reference reporting
# is not bound on the resolved constraint.


def test_equal_length_draw(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    reg.add_line(p1, p2)

    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)
    reg.add_line(p3, p4)

    c = EqualLengthConstraint([[p1, p2, p3, p4]])

    ctx = cairo.Context(cairo.ImageSurface(cairo.FORMAT_ARGB32, 200, 200))

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)
