"""Port of the reference radius constraint tests against the raygeo
bindings."""

import pytest

from raygeo.sketch import PointDef, Solver
from raygeo.sketch.constraints.radius import RadiusConstraint
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


def test_radius_constraint(setup_env):
    reg = setup_env
    start = reg.add_point(10, 0)
    end = reg.add_point(0, 10)
    center = reg.add_point(0, 0)
    reg.add_arc(start, end, center)

    # Current radius is 10. Target is 10. Error = 10 - 10 = 0
    c = RadiusConstraint(center, start, 10.0)
    assert c.error(reg)[0] == pytest.approx(0.0)
    assert c.user_visible() is True

    # Target is 5. Error = 10 - 5 = 5
    c2 = RadiusConstraint(center, start, 5.0)
    assert c2.error(reg)[0] == pytest.approx(5.0)


def test_radius_constraint_user_visible(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(10, 0)
    reg.add_circle(center, radius_pt)

    c = RadiusConstraint(center, radius_pt, 10.0, user_visible=False)
    assert c.user_visible() is False

    c2 = RadiusConstraint(center, radius_pt, 10.0, user_visible=True)
    assert c2.user_visible() is True


def test_radius_constraint_on_circle(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(10, 0)
    reg.add_circle(center, radius_pt)

    # Current radius is 10. Target is 10. Error = 10 - 10 = 0
    c = RadiusConstraint(center, radius_pt, 10.0)
    assert c.error(reg)[0] == pytest.approx(0.0)

    # Target is 5. Error = 10 - 5 = 5
    c2 = RadiusConstraint(center, radius_pt, 5.0)
    assert c2.error(reg)[0] == pytest.approx(5.0)


def test_radius_constrains_radius_method(setup_env):
    reg = setup_env
    start = reg.add_point(10, 0)
    end = reg.add_point(0, 10)
    center = reg.add_point(0, 0)
    arc_id = reg.add_arc(start, end, center)
    # The binding resolves entities to points, so entity identity is
    # matched through the defining points: a different entity must
    # use different points.
    other_start = reg.add_point(7, 0)
    other_end = reg.add_point(0, 7)
    other_id = reg.add_arc(other_start, other_end, center)

    c = RadiusConstraint(center, start, 10.0)

    # Should return True for the constrained entity
    assert c.constrains_radius(reg, arc_id) is True
    # Should return False for others
    assert c.constrains_radius(reg, other_id) is False
    assert c.constrains_radius(reg, 999) is False


def test_radius_targets_segment(setup_env):
    reg = setup_env
    start = reg.add_point(10, 0)
    end = reg.add_point(0, 10)
    center = reg.add_point(0, 0)
    arc_id = reg.add_arc(start, end, center)
    other_start = reg.add_point(7, 0)
    other_end = reg.add_point(0, 7)
    other_id = reg.add_arc(other_start, other_end, center)

    c = RadiusConstraint(center, start, 10.0)

    # Should match based on entity ID
    assert c.targets_segment(0, 0, arc_id, reg) is True

    # Should not match others
    assert c.targets_segment(0, 0, other_id, reg) is False
    assert c.targets_segment(0, 0, None, reg) is False


# deferred: test_radius_constraint_invalid_entity — the binding
# resolves constraints to points, so a radius constraint on a
# non-radius entity is not expressible.


def test_radius_constraint_gradient(setup_env):
    reg = setup_env
    c_id = reg.add_point(5, 5)
    s_id = reg.add_point(15, 5)
    e_id = reg.add_point(5, 15)
    reg.add_arc(s_id, e_id, c_id)

    constraint = RadiusConstraint(c_id, s_id, 12.0)

    _assert_gradient_matches_fd(reg, constraint, [c_id, s_id])


def test_radius_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(10, 0)
    reg.add_circle(center, radius_pt)

    # Create original constraint
    original = RadiusConstraint(center, radius_pt, 10.0)

    # Serialize to dict
    serialized = original.to_dict()

    # Deserialize from dict through the registry and rebuild
    handle = reg.add_constraint_json(serialized)
    data = reg.constraint_dict(handle)
    restored = RadiusConstraint(
        data["center_idx"],
        data["radius_pt_idx"],
        data["value"],
    )

    # Check that the restored constraint has the same error
    assert original.error(reg) == restored.error(reg)
    assert original.user_visible() == restored.user_visible()


def test_radius_constraint_with_expression(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(5, 0)
    reg.add_circle(center, radius_pt)

    # The binding resolves expressions caller-side, so the context
    # value 10.0 for "r_val" is passed as the resolved value and the
    # expression string is stored alongside (no update_from_context).
    c = RadiusConstraint(center, radius_pt, 10.0)
    c.set_expression("r_val")

    # Current r=5, target=10. Error = 5 - 10 = -5
    assert c.error(reg)[0] == pytest.approx(-5.0)
    assert c.expression() == "r_val"


def test_radius_is_hit(setup_env):
    reg = setup_env
    # The reference hits an arc whose label sits at the arc
    # midpoint angle; the binding resolves the constraint to
    # (center, radius point) only, so a circle with the label on the
    # radius direction is used here.
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(100, 0)
    reg.add_circle(center, radius_pt)

    c = RadiusConstraint(center, radius_pt, 100)

    threshold = 15.0

    # Label pos is at (radius + 20) along the radius vector:
    # (100 + 20, 0) = (120, 0)
    label_pos_x, label_pos_y = 120, 0

    # Hit
    assert c.is_hit(label_pos_x, label_pos_y, threshold, reg) is True
    # Miss
    assert c.is_hit(0, 0, threshold, reg) is False


# deferred: draw — command/UI layer
# deferred: can_apply_to — command/UI layer
