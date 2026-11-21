"""Port of the reference angle constraint tests against the
raygeo bindings (entity-reference contract)."""

import pytest

from raygeo.sketch.constraints.angle import AngleConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _perpendicular_lines(reg):
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(0, 10)
    l1 = reg.add_line(p1, p2)
    l2 = reg.add_line(p3, p4)
    return l1, l2


def test_angle_90_degrees(setup_env):
    reg = setup_env
    l1, l2 = _perpendicular_lines(reg)

    c = AngleConstraint(l2, l1, 90.0)
    assert c.error(reg, None) == pytest.approx(0.0, abs=1e-4)
    assert c.user_visible is True


def test_angle_45_degrees(setup_env):
    reg = setup_env
    l1, l2 = _perpendicular_lines(reg)

    c = AngleConstraint(l2, l1, 45.0)
    assert abs(c.error(reg, None)) > 1.0


def test_angle_non_zero_error(setup_env):
    reg = setup_env
    l1, l2 = _perpendicular_lines(reg)

    c = AngleConstraint(l2, l1, 30.0)
    assert abs(c.error(reg, None)) > 1.0


def test_angle_user_visible(setup_env):
    reg = setup_env
    l1, l2 = _perpendicular_lines(reg)

    c = AngleConstraint(l2, l1, 90.0, user_visible=False)
    assert c.user_visible is False

    c2 = AngleConstraint(l2, l1, 90.0, user_visible=True)
    assert c2.user_visible is True


def test_angle_entity_refs(setup_env):
    reg = setup_env
    l1, l2 = _perpendicular_lines(reg)

    c = AngleConstraint(l2, l1, 90.0)
    assert c.e1_id == l2
    assert c.e2_id == l1


def test_json_value_is_degrees(setup_env):
    """The serialized "value" is degrees (reference .ryp parity);
    RotationalConstraint serializes radians under the same key."""
    reg = setup_env
    l1, l2 = _perpendicular_lines(reg)

    c = AngleConstraint(l2, l1, 45.0)
    assert c.to_dict()["value"] == pytest.approx(45.0)


def test_error_with_unresolved_entities_stays_row_consistent(setup_env):
    """A constraint whose entities cannot be resolved imposes nothing:
    its static row count still holds, with zero-valued residuals."""
    reg = setup_env
    c = AngleConstraint(999, 998, 30.0)
    assert c.error(reg, None) == pytest.approx(0.0)
