"""Port of the reference equal-length constraint tests against the
raygeo bindings (entity-reference contract)."""

import pytest

from raygeo.sketch.constraints.equal_length import EqualLengthConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def test_equal_length_lines(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(10, 10)
    l1 = reg.add_line(p1, p2)
    l2 = reg.add_line(p3, p4)

    c = EqualLengthConstraint([l1, l2])
    assert c.error(reg, None) == pytest.approx([0.0], abs=1e-6)
    assert c.user_visible is True


def test_equal_length_non_equal(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(5, 10)
    l1 = reg.add_line(p1, p2)
    l2 = reg.add_line(p3, p4)

    c = EqualLengthConstraint([l1, l2])
    assert abs(c.error(reg, None)[0]) > 1.0


def test_equal_length_entity_ids(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(0, 10)
    p4 = reg.add_point(10, 10)
    l1 = reg.add_line(p1, p2)
    l2 = reg.add_line(p3, p4)

    c = EqualLengthConstraint([l1, l2])
    assert c.entity_ids == [l1, l2]


def test_equal_length_single_entity(setup_env):
    reg = setup_env
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    l1 = reg.add_line(p1, p2)

    c = EqualLengthConstraint([l1])
    assert c.error(reg, None) == []
