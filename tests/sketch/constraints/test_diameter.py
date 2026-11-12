"""Port of the reference diameter constraint tests against the
raygeo bindings."""

import cairo
import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.diameter import DiameterConstraint
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


def test_diameter_constraint(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    # Radius is 5, so diameter is 10
    radius_pt = reg.add_point(5, 0)
    reg.add_circle(center, radius_pt)

    # Target diam is 10, actual is 10. Error = 10 - 10 = 0
    c = DiameterConstraint(center, radius_pt, 10.0)
    assert c.error(reg)[0] == pytest.approx(0.0)
    assert c.user_visible() is True

    # Target diam is 20, actual is 10. Error = 10 - 20 = -10
    c2 = DiameterConstraint(center, radius_pt, 20.0)
    assert c2.error(reg)[0] == pytest.approx(-10.0)


def test_diameter_constraint_user_visible(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(5, 0)
    reg.add_circle(center, radius_pt)

    c = DiameterConstraint(center, radius_pt, 10.0, user_visible=False)
    assert c.user_visible() is False

    c2 = DiameterConstraint(center, radius_pt, 10.0, user_visible=True)
    assert c2.user_visible() is True


def test_diameter_constrains_radius_method(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(5, 0)
    circ_id = reg.add_circle(center, radius_pt)
    # The binding resolves entities to points, so entity identity is
    # matched through the defining points: a different entity must
    # use different points.
    other_radius_pt = reg.add_point(7, 0)
    other_circ_id = reg.add_circle(center, other_radius_pt)

    c = DiameterConstraint(center, radius_pt, 10.0)

    # Should return True for the constrained circle
    assert c.constrains_radius(reg, circ_id) is True
    # Should return False for any other entity
    assert c.constrains_radius(reg, other_circ_id) is False
    assert c.constrains_radius(reg, 999) is False


def test_diameter_targets_segment(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(5, 0)
    circ_id = reg.add_circle(center, radius_pt)
    other_radius_pt = reg.add_point(7, 0)
    other_circ_id = reg.add_circle(center, other_radius_pt)

    c = DiameterConstraint(center, radius_pt, 10.0)

    # Should match based on entity ID (points ignored for this check)
    assert c.targets_segment(0, 0, circ_id, reg) is True

    # Should not match other entities or None
    assert c.targets_segment(0, 0, other_circ_id, reg) is False
    assert c.targets_segment(0, 0, None, reg) is False


def test_diameter_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(5, 0)
    reg.add_circle(center, radius_pt)

    # Create original constraint
    original = DiameterConstraint(center, radius_pt, 10.0)

    # Serialize to dict
    serialized = original.to_dict()

    # Deserialize from dict through the registry and rebuild
    handle = reg.add_constraint_json(serialized)
    data = reg.constraint_dict(handle)
    restored = DiameterConstraint(
        data["center_idx"],
        data["radius_pt_idx"],
        data["value"],
    )

    # Check that the restored constraint has the same error
    assert original.error(reg) == restored.error(reg)
    assert original.user_visible() == restored.user_visible()


def test_diameter_constraint_with_expression(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    # Radius is 5, so diameter is 10
    radius_pt = reg.add_point(5, 0)
    reg.add_circle(center, radius_pt)

    # The binding resolves expressions caller-side: the context
    # value 20.0 for "target_diam" is passed as the resolved value
    # and the expression string is stored alongside (the reference
    # update_from_context evaluation has no binding counterpart).
    # Actual diam is 10, target is 20. Error = 10 - 20 = -10
    c = DiameterConstraint(center, radius_pt, 20.0)
    c.set_expression("target_diam")

    assert c.error(reg)[0] == pytest.approx(-10.0)
    assert c.value == 20.0
    assert c.expression() == "target_diam"


def test_diameter_constraint_gradient(setup_env):
    reg = setup_env
    c_id = reg.add_point(0, 0)
    r_id = reg.add_point(5, 0)
    reg.add_circle(c_id, r_id)

    constraint = DiameterConstraint(c_id, r_id, 12.0)

    _assert_gradient_matches_fd(reg, constraint, [c_id, r_id])


def test_diameter_is_hit(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(100, 0)  # radius=100
    reg.add_circle(center, radius_pt)

    c = DiameterConstraint(center, radius_pt, 200)

    threshold = 15.0

    # Label pos logic places it at (radius + 20) along radius vector
    # (100 + 20, 0) = (120, 0)
    label_pos_x, label_pos_y = 120, 0

    # Hit
    assert c.is_hit(label_pos_x, label_pos_y, threshold, reg) is True
    # Miss
    assert c.is_hit(0, 0, threshold, reg) is False


# deferred: can_apply_to — command/UI layer


def test_diameter_draw(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(5, 0)
    reg.add_circle(center, radius_pt)

    c = DiameterConstraint(center, radius_pt, 10.0)

    ctx = cairo.Context(cairo.ImageSurface(cairo.FORMAT_ARGB32, 200, 200))

    def to_screen(pos):
        return pos

    c.draw(ctx, reg, to_screen)
    c.draw(ctx, reg, to_screen, is_selected=True)
    c.draw(ctx, reg, to_screen, is_hovered=True)
    c.draw(ctx, reg, to_screen, point_radius=10.0)
