"""Port of the reference symmetry constraint tests against the
raygeo bindings (entity-reference contract)."""

import pytest

from raygeo.sketch.constraints.symmetry import SymmetryConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def test_point_symmetry(setup_env):
    reg = setup_env
    p1 = reg.add_point(10, 10)
    p2 = reg.add_point(30, 30)
    center = reg.add_point(20, 20)

    c = SymmetryConstraint(p1, p2, center=center)
    assert c.error(reg, None) == pytest.approx([0.0, 0.0], abs=1e-6)


def test_point_symmetry_violated(setup_env):
    reg = setup_env
    p1 = reg.add_point(10, 10)
    p2 = reg.add_point(35, 30)
    center = reg.add_point(20, 20)

    c = SymmetryConstraint(p1, p2, center=center)
    assert c.error(reg, None) != pytest.approx([0.0, 0.0], abs=1e-6)


def test_line_symmetry(setup_env):
    reg = setup_env
    p1 = reg.add_point(20, 10)
    p2 = reg.add_point(20, 30)
    l1 = reg.add_point(10, 20)
    l2 = reg.add_point(30, 20)
    axis = reg.add_line(l1, l2)

    c = SymmetryConstraint(p1, p2, axis=axis)
    assert c.error(reg, None) == pytest.approx([0.0, 0.0], abs=1e-6)


def test_user_visible(setup_env):
    reg = setup_env
    p1 = reg.add_point(10, 10)
    p2 = reg.add_point(30, 30)
    center = reg.add_point(20, 20)

    c = SymmetryConstraint(p1, p2, center=center, user_visible=False)
    assert c.user_visible is False

    c2 = SymmetryConstraint(p1, p2, center=center, user_visible=True)
    assert c2.user_visible is True


def test_entity_refs(setup_env):
    reg = setup_env
    p1 = reg.add_point(10, 10)
    p2 = reg.add_point(30, 30)
    center = reg.add_point(20, 20)

    c = SymmetryConstraint(p1, p2, center=center)
    assert c.p1 == p1
    assert c.p2 == p2
    assert c.center == center
    assert c.axis is None
