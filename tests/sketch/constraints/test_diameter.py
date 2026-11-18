"""Port of the reference diameter constraint tests against the
raygeo bindings (entity-reference contract)."""

import pytest

from raygeo.sketch.constraints.diameter import DiameterConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _circle(reg):
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(10, 0)
    circle_id = reg.add_circle(center, radius_pt)
    return circle_id


def test_diameter_constraint(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    c = DiameterConstraint(circle_id, 20.0)
    assert c.error(reg, None) == pytest.approx(0.0)
    assert c.user_visible is True


def test_diameter_constraint_non_zero(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    c = DiameterConstraint(circle_id, 10.0)
    assert c.error(reg, None) == pytest.approx(10.0)


def test_diameter_constraint_user_visible(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    c = DiameterConstraint(circle_id, 20.0, user_visible=False)
    assert c.user_visible is False

    c2 = DiameterConstraint(circle_id, 20.0, user_visible=True)
    assert c2.user_visible is True


def test_diameter_entity_id(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    c = DiameterConstraint(circle_id, 20.0)
    assert c.circle_id == circle_id


def test_diameter_constrains_radius_method(setup_env):
    reg = setup_env
    circle_id = _circle(reg)
    other = reg.add_point(20, 0)
    other_circle = reg.add_circle(other, reg.add_point(25, 0))

    c = DiameterConstraint(circle_id, 20.0)

    assert c.constrains_radius(reg, circle_id) is True
    assert c.constrains_radius(reg, other_circle) is False
    assert c.constrains_radius(reg, 999) is False


def test_diameter_targets_segment(setup_env):
    reg = setup_env
    circle_id = _circle(reg)
    other_circle = reg.add_circle(reg.add_point(20, 0), reg.add_point(25, 0))

    c = DiameterConstraint(circle_id, 20.0)

    assert c.targets_segment(0, 0, circle_id) is True
    assert c.targets_segment(0, 0, other_circle) is False
    assert c.targets_segment(0, 0, None) is False


def test_diameter_constraint_serialization_round_trip(setup_env):
    reg = setup_env
    circle_id = _circle(reg)

    original = DiameterConstraint(circle_id, 20.0)
    restored = DiameterConstraint.from_dict(original.to_dict())
    assert original.error(reg, None) == pytest.approx(
        restored.error(reg, None)
    )
    assert original.user_visible == restored.user_visible


def test_diameter_constraint_with_expression(setup_env):
    reg = setup_env
    center = reg.add_point(0, 0)
    radius_pt = reg.add_point(5, 0)
    circle_id = reg.add_circle(center, radius_pt)

    c = DiameterConstraint(circle_id, 10.0)
    c.expression = "d_val"

    # Current d=10, target=10. Error = 0.
    assert c.error(reg, None) == pytest.approx(0.0)
    assert c.expression == "d_val"
