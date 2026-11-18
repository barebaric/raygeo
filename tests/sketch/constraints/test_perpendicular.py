"""Port of the reference perpendicular constraint tests against the
raygeo bindings (entity-reference contract)."""

import pytest

from raygeo.sketch.constraints.perpendicular import PerpendicularConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def test_perpendicular_lines(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(5, 0)
    p4 = reg.add_point(5, 10)
    l1 = reg.add_line(p1, p2)
    l2 = reg.add_line(p3, p4)

    c = PerpendicularConstraint(l1, l2)
    assert c.error(reg, None) == pytest.approx(0.0, abs=1e-6)
    assert c.user_visible is True


def test_non_perpendicular_lines(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 0)
    p4 = reg.add_point(10, 10)
    l1 = reg.add_line(p1, p2)
    l2 = reg.add_line(p3, p4)

    c = PerpendicularConstraint(l1, l2)
    assert abs(c.error(reg, None)) > 1.0


def test_entity_refs(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(5, 0)
    p4 = reg.add_point(5, 10)
    l1 = reg.add_line(p1, p2)
    l2 = reg.add_line(p3, p4)

    c = PerpendicularConstraint(l1, l2)
    assert c.e1_id == l1
    assert c.e2_id == l2
