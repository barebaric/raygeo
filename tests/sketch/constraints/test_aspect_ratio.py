"""Port of the reference aspect-ratio constraint tests against the
raygeo bindings."""

from unittest.mock import MagicMock

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.aspect_ratio import AspectRatioConstraint
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


def test_aspect_ratio_constraint(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)

    c = AspectRatioConstraint(p1, p2, p3, p4, 2.0)
    dist1 = 10.0
    dist2 = 5.0
    expected_error = dist1 - dist2 * 2.0
    assert c.error(reg)[0] == pytest.approx(expected_error)
    assert c.user_visible() is True


def test_aspect_ratio_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)

    c = AspectRatioConstraint(p1, p2, p3, p4, 2.0, user_visible=False)
    assert c.user_visible() is False

    c2 = AspectRatioConstraint(p1, p2, p3, p4, 2.0, user_visible=True)
    assert c2.user_visible() is True


def test_aspect_ratio_constraint_perfect_match(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)

    c = AspectRatioConstraint(p1, p2, p3, p4, 2.0)
    assert c.error(reg)[0] == pytest.approx(0.0)


def test_aspect_ratio_constraint_diagonal_lines(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(6, 8)
    p3 = reg.add_point(10, 10)
    p4 = reg.add_point(13, 14)

    c = AspectRatioConstraint(p1, p2, p3, p4, 1.0)
    dist1 = 10.0
    dist2 = 5.0
    expected_error = dist1 - dist2 * 1.0
    assert c.error(reg)[0] == pytest.approx(expected_error)


def test_aspect_ratio_constraint_zero_ratio(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)

    c = AspectRatioConstraint(p1, p2, p3, p4, 0.0)
    dist1 = 10.0
    expected_error = dist1
    assert c.error(reg)[0] == pytest.approx(expected_error)


def test_aspect_ratio_constraint_zero_second_distance(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(0, 10)

    c = AspectRatioConstraint(p1, p2, p3, p4, 2.0)
    dist1 = 10.0
    expected_error = dist1
    assert c.error(reg)[0] == pytest.approx(expected_error)


# deferred: constrains_radius — the base-class flag is not bound on
# this constraint.

# deferred: targets_segment is only bound on dimensional constraints


def test_aspect_ratio_constraint_gradient(setup_env):
    reg = setup_env
    p1_id = reg.add_point(1, 2)
    p2_id = reg.add_point(5, 6)
    p3_id = reg.add_point(0, 0)
    p4_id = reg.add_point(3, 0)

    constraint = AspectRatioConstraint(p1_id, p2_id, p3_id, p4_id, 2.0)

    _assert_gradient_matches_fd(reg, constraint, [p1_id, p2_id, p3_id, p4_id])


def test_aspect_ratio_constraint_gradient_zero_first_distance(setup_env):
    reg = setup_env
    p1_id = reg.add_point(0, 0)
    p2_id = reg.add_point(0, 0)
    p3_id = reg.add_point(0, 0)
    p4_id = reg.add_point(3, 0)

    constraint = AspectRatioConstraint(p1_id, p2_id, p3_id, p4_id, 2.0)

    # The analytic gradient omits the degenerate pair, so its
    # Jacobian columns are exactly zero.
    jacobian = _solver(
        reg, constraint, [p1_id, p2_id, p3_id, p4_id]
    ).jacobian()
    for row in jacobian:
        assert row[0] == 0.0
        assert row[1] == 0.0
        assert row[2] == 0.0
        assert row[3] == 0.0


def test_aspect_ratio_constraint_gradient_zero_second_distance(setup_env):
    reg = setup_env
    p1_id = reg.add_point(0, 0)
    p2_id = reg.add_point(3, 0)
    p3_id = reg.add_point(0, 0)
    p4_id = reg.add_point(0, 0)

    constraint = AspectRatioConstraint(p1_id, p2_id, p3_id, p4_id, 2.0)

    jacobian = _solver(
        reg, constraint, [p1_id, p2_id, p3_id, p4_id]
    ).jacobian()
    for row in jacobian:
        assert row[4] == 0.0
        assert row[5] == 0.0
        assert row[6] == 0.0
        assert row[7] == 0.0


def test_aspect_ratio_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)

    original = AspectRatioConstraint(p1, p2, p3, p4, 2.5)

    serialized = original.to_dict()

    handle = reg.add_constraint_json(serialized)
    data = reg.constraint_dict(handle)
    restored = AspectRatioConstraint(
        data["p1"], data["p2"], data["p3"], data["p4"], data["ratio"]
    )

    assert original.error(reg) == restored.error(reg)
    assert original.p1 == restored.p1
    assert original.p2 == restored.p2
    assert original.p3 == restored.p3
    assert original.p4 == restored.p4
    assert original.ratio == restored.ratio
    assert original.user_visible() == restored.user_visible()


def test_aspect_ratio_constraint_serialization_dict_format(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)

    c = AspectRatioConstraint(p1, p2, p3, p4, 2.5)

    data = c.to_dict()

    assert data["type"] == "AspectRatioConstraint"
    assert data["p1"] == p1
    assert data["p2"] == p2
    assert data["p3"] == p3
    assert data["p4"] == p4
    assert data["ratio"] == 2.5
    assert data["user_visible"] is True


# deferred: depends_on_points / depends_on_entities — reference
# reporting is not bound on the resolved constraint.

# deferred: can_apply_to — command/UI layer


def test_aspect_ratio_draw(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)

    c = AspectRatioConstraint(p1, p2, p3, p4, 2.0)

    ctx = MagicMock()

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)
