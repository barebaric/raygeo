"""Port of the reference tangent constraint tests against the
raygeo bindings (entity-reference contract)."""

import pytest

from raygeo.sketch.constraints.tangent import TangentConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _tangent_setup(reg):
    """A horizontal line y=10 and a circle centered at (30, 30)
    with radius 20 (bottom of the circle touches the line)."""
    l1 = reg.add_point(0, 10)
    l2 = reg.add_point(60, 10)
    line_id = reg.add_line(l1, l2)
    center = reg.add_point(30, 30)
    radius_pt = reg.add_point(30, 10)
    circle_id = reg.add_circle(center, radius_pt)
    return line_id, circle_id


def test_tangent_constraint(setup_env):
    reg = setup_env
    line_id, circle_id = _tangent_setup(reg)

    c = TangentConstraint(line_id, circle_id)
    assert c.error(reg, None) == pytest.approx(0.0, abs=1e-6)
    assert c.user_visible is True


def test_tangent_constraint_non_tangent(setup_env):
    reg = setup_env
    l1 = reg.add_point(0, 10)
    l2 = reg.add_point(60, 10)
    line_id = reg.add_line(l1, l2)
    center = reg.add_point(30, 50)
    radius_pt = reg.add_point(30, 30)
    circle_id = reg.add_circle(center, radius_pt)

    c = TangentConstraint(line_id, circle_id)
    # Circle center is at y=50, radius=20, so bottom is at y=30.
    # Line is at y=10. Distance from center to line = 40 != radius.
    assert abs(c.error(reg, None)) > 1.0


def test_tangent_entity_ids(setup_env):
    reg = setup_env
    line_id, circle_id = _tangent_setup(reg)

    c = TangentConstraint(line_id, circle_id)
    assert c.line_id == line_id
    assert c.shape_id == circle_id
