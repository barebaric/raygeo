"""Port of the reference point-on-curve constraint tests against the
raygeo bindings."""

from unittest.mock import MagicMock

import pytest

from raygeo.geo.shape.bezier import get_bezier_closest_point
from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.point_on_curve import PointOnCurveConstraint
from raygeo.sketch.registry import EntityRegistry

# Hump-shaped curve: (0,0) -> (10,0) with both control points pulling
# the curve up to an apex of (5, 7.5).  Control points are offsets
# from their owning endpoints.
CP1_OFFSET = (0.0, 10.0)
CP2_OFFSET = (0.0, 10.0)


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


def _assert_gradient_matches_fd(reg, constraint, pids, tol=1e-4, eps=1e-6):
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


def _add_hump_bezier(reg, start=(0.0, 0.0), end=(10.0, 0.0)):
    p_start = reg.add_point(*start)
    p_end = reg.add_point(*end)
    bez_id = reg.add_bezier(p_start, p_end, cp1=CP1_OFFSET, cp2=CP2_OFFSET)
    return bez_id, p_start, p_end


def _closest_point_on_bezier(xs, ys, px, py):
    """Reference ``closest_point_on_bezier`` port: absolute control
    point coordinates given as x/y 4-tuples."""
    result = get_bezier_closest_point(
        (xs[0], ys[0], 0.0),
        (xs[1], ys[1], 0.0),
        (xs[2], ys[2], 0.0),
        (xs[3], ys[3], 0.0),
        px,
        py,
    )
    assert result is not None
    t, (bx, by), _dist_sq = result
    return t, bx, by


X = (0.0, 0.0, 10.0, 10.0)
Y = (0.0, 10.0, 10.0, 0.0)


def test_closest_point_apex():
    t, bx, by = _closest_point_on_bezier(X, Y, 5.0, 8.0)
    assert t == pytest.approx(0.5, abs=1e-6)
    assert bx == pytest.approx(5.0, abs=1e-6)
    assert by == pytest.approx(7.5, abs=1e-6)


def test_closest_point_clamps_to_start():
    t, bx, by = _closest_point_on_bezier(X, Y, -3.0, 0.0)
    assert t == pytest.approx(0.0)
    assert bx == pytest.approx(0.0)
    assert by == pytest.approx(0.0)


def test_closest_point_clamps_to_end():
    t, bx, by = _closest_point_on_bezier(X, Y, 14.0, 0.0)
    assert t == pytest.approx(1.0)
    assert bx == pytest.approx(10.0)
    assert by == pytest.approx(0.0)


def test_point_on_curve_error(setup_env):
    reg = setup_env
    _bez_id, p_start, p_end = _add_hump_bezier(reg)
    pt = reg.add_point(5.0, 8.0)

    c = PointOnCurveConstraint(
        pt, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )
    assert c.error(reg)[0] == pytest.approx(0.5, abs=1e-6)


def test_point_on_curve_error_zero_on_curve(setup_env):
    reg = setup_env
    _bez_id, p_start, p_end = _add_hump_bezier(reg)
    pt = reg.add_point(5.0, 7.5)

    c = PointOnCurveConstraint(
        pt, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )
    assert c.error(reg)[0] < 1e-9


def test_point_on_curve_gradient(setup_env):
    reg = setup_env
    _bez_id, p_start, p_end = _add_hump_bezier(reg)
    pt = reg.add_point(5.0, 8.0)

    constraint = PointOnCurveConstraint(
        pt, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )

    _assert_gradient_matches_fd(reg, constraint, [pt, p_start, p_end])


def test_point_on_curve_gradient_zero_at_solution(setup_env):
    """At the solution the analytic gradient is empty, so the
    Jacobian column of the constrained point is exactly zero."""
    reg = setup_env
    _bez_id, p_start, p_end = _add_hump_bezier(reg)
    pt = reg.add_point(5.0, 7.5)

    c = PointOnCurveConstraint(
        pt, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )
    solver = _solver(reg, c, [pt, p_start, p_end])
    jacobian = solver.jacobian()
    for row in jacobian:
        assert row[0] == 0.0
        assert row[1] == 0.0


def test_point_on_curve_pulls_point_onto_curve(setup_env):
    reg = setup_env
    _bez_id, p_start, p_end = _add_hump_bezier(reg)
    reg.set_point_position(p_start, 0.0, 0.0)
    pt = reg.add_point(5.0, 6.0)

    # The curve endpoints are pinned by making the problem anchored:
    # only the point moves here (the endpoints keep their positions
    # as the closest solution has them stationary).
    points = []
    for pid, fixed in ((p_start, True), (p_end, True), (pt, False)):
        position = reg.get_point(pid)
        assert position is not None
        points.append(PointDef(pid, position[0], position[1], fixed))
    constraint = PointOnCurveConstraint(
        pt, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )
    solver = Solver(points, [constraint], [])
    assert solver.solve(1e-10, False)

    x, y = solver.positions()[2]
    reg.set_point_position(pt, x, y)
    assert constraint.error(reg)[0] < 1e-4


def test_point_on_curve_pulls_curve_to_anchor(setup_env):
    reg = setup_env
    _bez_id, p_start, p_end = _add_hump_bezier(reg)
    anchor = reg.add_point(5.0, 6.5, fixed=True)

    points = []
    for pid, fixed in ((p_start, False), (p_end, False), (anchor, True)):
        position = reg.get_point(pid)
        assert position is not None
        points.append(PointDef(pid, position[0], position[1], fixed))
    constraint = PointOnCurveConstraint(
        anchor, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )
    solver = Solver(points, [constraint], [])
    assert solver.solve(1e-10, False)

    assert solver.max_residuals()[0] < 1e-4


# deferred: can_apply_to — command/UI layer

# deferred: serialization round trip — the constraint dict stores
# entity ids while the resolved constraint stores point ids, pending
# the json serialization alignment.

# deferred: references — get_referenced_point_ids and friends are not
# bound on the resolved constraint.


def test_point_on_curve_is_hit(setup_env):
    reg = setup_env
    _bez_id, p_start, p_end = _add_hump_bezier(reg)
    pt = reg.add_point(10, 20)

    c = PointOnCurveConstraint(
        pt, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )

    threshold = 15.0

    assert c.is_hit(10, 20, threshold, reg) is True
    assert c.is_hit(30, 20, threshold, reg) is False


def test_point_on_curve_sketch_end_to_end(setup_env):
    reg = setup_env
    p_start = reg.add_point(0, 0, fixed=True)
    p_end = reg.add_point(10, 0, fixed=True)
    reg.add_bezier(p_start, p_end, cp1=CP1_OFFSET, cp2=CP2_OFFSET)
    pt = reg.add_point(5, 6)

    constraint = PointOnCurveConstraint(
        pt, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )
    solver = _solver(reg, constraint, [p_start, p_end, pt])
    assert solver.solve(1e-10, False)

    assert solver.max_residuals()[0] < 1e-4


# deferred: ignores missing shape — the constraint references
# resolved points, so a missing entity cannot be represented.


def test_point_on_curve_draw(setup_env):
    reg = setup_env
    _bez_id, p_start, p_end = _add_hump_bezier(reg)
    pt = reg.add_point(0, 0)

    c = PointOnCurveConstraint(
        pt, p_start, p_end, CP1_OFFSET, CP2_OFFSET, False
    )

    ctx = MagicMock()

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)
