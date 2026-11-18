"""Port of the reference distance constraint tests against the
raygeo bindings."""

import cairo
import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.distance import DistanceConstraint
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
    return Solver(points, reg, [constraint], [])


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


def test_distance_constraint(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    # Target is 10, actual is 10. Error should be 10 - 10 = 0.
    c = DistanceConstraint(p1, p2, 10.0)
    assert c.error(reg) == pytest.approx(0.0)
    assert c.user_visible is True

    # Target is 5, actual is 10. Error should be 10 - 5 = 5.
    c2 = DistanceConstraint(p1, p2, 5.0)
    assert c2.error(reg) == pytest.approx(5.0)


def test_distance_constraint_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    c = DistanceConstraint(p1, p2, 10.0, user_visible=False)
    assert c.user_visible is False

    c2 = DistanceConstraint(p1, p2, 10.0, user_visible=True)
    assert c2.user_visible is True


def test_distance_constrains_radius_method(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    c = DistanceConstraint(p1, p2, 10.0)

    # Distance constraint doesn't constrain radius of an entity
    assert c.constrains_radius(reg, 999) is False
    assert c.constrains_radius(reg, p1) is False


def test_distance_targets_segment(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(5, 5)

    c = DistanceConstraint(p1, p2, 10.0)

    # Should match exact pair (order independent)
    assert c.targets_segment(p1, p2) is True
    assert c.targets_segment(p2, p1) is True

    # Should not match subset or disjoint
    assert c.targets_segment(p1, p3) is False
    assert c.targets_segment(p3, p2) is False

    # Entity ID is ignored by DistanceConstraint logic (it only
    # tracks points)
    assert c.targets_segment(p1, p2, 999) is True


def test_distance_constraint_with_expression(setup_env):
    reg = setup_env
    # The binding resolves expressions caller-side, so the context
    # value 20.0 for "width" is passed as the resolved value and the
    # expression string is stored alongside (no update_from_context).
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    c = DistanceConstraint(p1, p2, 20.0)
    c.expression = "width"

    # Actual dist is 10, target dist is 20. Error is 10 - 20 = -10.
    assert c.error(reg) == pytest.approx(-10.0)
    assert c.expression == "width"


def test_distance_constraint_gradient(setup_env):
    """Verifies the analytic Jacobian of DistanceConstraint against a
    central finite difference of the residuals."""
    reg = setup_env
    p1_id = reg.add_point(1, 2)
    p2_id = reg.add_point(5, 6)

    constraint = DistanceConstraint(p1_id, p2_id, 10.0)

    _assert_gradient_matches_fd(reg, constraint, [p1_id, p2_id])


def test_distance_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    # Create original constraint
    original = DistanceConstraint(p1, p2, 5.0)

    # Serialize to dict and rebuild through the model layer
    restored = DistanceConstraint.from_dict(original.to_dict())
    assert restored.to_dict() == original.to_dict()


def test_distance_constraint_serialization_round_trip_with_expression(
    setup_env,
):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)

    # Create original constraint with expression; the value is
    # pre-resolved from the context ("width" -> 20.0)
    original = DistanceConstraint(p1, p2, 20.0)
    original.expression = "width"

    # Serialize to dict
    serialized = original.to_dict()

    # Deserialize from dict through the model layer,
    # re-attaching the stored expression
    restored = DistanceConstraint.from_dict(serialized)
    assert restored.to_dict() == serialized


def test_distance_is_hit(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(100, 0)
    c = DistanceConstraint(p1, p2, 100)

    threshold = 15.0

    # Hit on the line segment (at midpoint)
    assert c.is_hit(50, 0, reg, lambda pos: pos, None, threshold) is True

    # Hit on the line segment (not at midpoint)
    assert c.is_hit(70, 0, reg, lambda pos: pos, None, threshold) is True

    # Hit near the line segment (within threshold)
    assert c.is_hit(50, 10, reg, lambda pos: pos, None, threshold) is True

    # Miss - far from the line segment (beyond threshold)
    assert c.is_hit(50, 20, reg, lambda pos: pos, None, threshold) is False


# deferred: can_apply_to — command/UI layer


@pytest.mark.parametrize(
    "draw_options",
    [{}, {"is_selected": True}, {"is_hovered": True}, {"point_radius": 10.0}],
)
def test_distance_draw(setup_env, draw_options):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(100, 0)
    c = DistanceConstraint(p1, p2, 100)

    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 200, 100)
    ctx = cairo.Context(surface)

    def to_screen(pos):
        return pos[0] + 50, pos[1] + 50

    c.draw(ctx, reg, to_screen, **draw_options)
    assert any(surface.get_data())
