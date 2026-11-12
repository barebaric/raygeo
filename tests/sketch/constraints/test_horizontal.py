"""Port of the reference horizontal constraint tests against the
raygeo bindings."""

from unittest.mock import MagicMock

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.horizontal import HorizontalConstraint
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


def test_horizontal_constraint(setup_env):
    reg = setup_env
    # p1 y=0, p2 y=5. Error should be 0 - 5 = -5
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 5)

    c = HorizontalConstraint(p1, p2)
    assert c.error(reg)[0] == pytest.approx(-5.0)
    assert c.user_visible() is True


def test_horizontal_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 5)

    c = HorizontalConstraint(p1, p2, user_visible=False)
    assert c.user_visible() is False

    c2 = HorizontalConstraint(p1, p2, user_visible=True)
    assert c2.user_visible() is True


# deferred: targets_segment is only bound on dimensional constraints


def test_horizontal_constraint_gradient(setup_env):
    reg = setup_env
    p1_id = reg.add_point(1, 2)
    p2_id = reg.add_point(5, 6)

    constraint = HorizontalConstraint(p1_id, p2_id)

    _assert_gradient_matches_fd(reg, constraint, [p1_id, p2_id])


def test_horizontal_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 5)

    # Create original constraint
    original = HorizontalConstraint(p1, p2)

    # Serialize to dict
    serialized = original.to_dict()

    # Deserialize from dict through the registry and rebuild
    handle = reg.add_constraint_json(serialized)
    restored = HorizontalConstraint(
        reg.constraint_dict(handle)["p1"],
        reg.constraint_dict(handle)["p2"],
    )

    # Check that the restored constraint has the same error
    assert original.error(reg) == restored.error(reg)
    assert original.user_visible() == restored.user_visible()


def test_horizontal_is_hit(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 50)
    p2 = reg.add_point(100, 50)
    c = HorizontalConstraint(p1, p2)

    threshold = 15.0

    # Symbol is at t=0.2 along line, offset by -10 in Y
    # Point is at x=20, y=50. Symbol at x=20, y=40.
    symbol_x, symbol_y = 20, 40

    # Hit
    assert c.is_hit(symbol_x, symbol_y, threshold, reg) is True
    # Miss
    assert c.is_hit(0, 0, threshold, reg) is False


# deferred: can_apply_to — command/UI layer


def test_horizontal_draw(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 5)

    c = HorizontalConstraint(p1, p2)

    ctx = MagicMock()

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)


def test_horizontal_draw_status_ordinals(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 5)

    c = HorizontalConstraint(p1, p2)

    def to_screen(pos):
        return pos

    c.status = 1
    ctx = MagicMock()
    c.draw(ctx, reg, to_screen)
    assert ctx.set_source_rgb.call_args_list[-1].args == (0.0, 0.6, 0.0)

    c.status = 2
    ctx = MagicMock()
    c.draw(ctx, reg, to_screen)
    assert ctx.set_source_rgb.call_args_list[-1].args == (1.0, 0.6, 0.0)

    c.status = 3
    ctx = MagicMock()
    c.draw(ctx, reg, to_screen)
    assert ctx.set_source_rgb.call_args_list[-1].args == (1.0, 0.2, 0.2)

    c.status = 4
    ctx = MagicMock()
    c.draw(ctx, reg, to_screen)
    assert ctx.set_source_rgb.call_args_list[-1].args == (1.0, 0.2, 0.2)
    assert any(
        name == "stroke_preserve" for name, _args, _kwargs in ctx.mock_calls
    )
