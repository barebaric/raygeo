"""Port of the reference point-on-line constraint tests against the
raygeo bindings (entity-reference contract)."""

import pytest

from raygeo.sketch.constraints.point_on_line import PointOnLineConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _line(reg):
    p1 = reg.add_point(0, 0)
    p2 = reg.add_point(10, 0)
    line_id = reg.add_line(p1, p2)
    return line_id, p1, p2


def _circle(reg):
    center = reg.add_point(30, 0)
    radius_pt = reg.add_point(40, 0)
    circle_id = reg.add_circle(center, radius_pt)
    return circle_id


def test_point_on_line(setup_env):
    reg = setup_env
    line_id, p1, p2 = _line(reg)
    pt = reg.add_point(5, 0)

    c = PointOnLineConstraint(pt, line_id)
    assert c.error(reg, None) == pytest.approx(0.0, abs=1e-6)
    assert c.user_visible is True


def test_point_off_line(setup_env):
    reg = setup_env
    line_id, _p1, _p2 = _line(reg)
    pt = reg.add_point(5, 5)

    c = PointOnLineConstraint(pt, line_id)
    assert abs(c.error(reg, None)) > 1.0


def test_point_on_circle(setup_env):
    reg = setup_env
    circle_id = _circle(reg)
    pt = reg.add_point(40, 0)

    c = PointOnLineConstraint(pt, circle_id)
    assert c.error(reg, None) == pytest.approx(0.0, abs=1e-6)


def test_point_off_circle(setup_env):
    reg = setup_env
    circle_id = _circle(reg)
    pt = reg.add_point(10, 0)

    c = PointOnLineConstraint(pt, circle_id)
    assert abs(c.error(reg, None)) > 1.0


def test_constrains_radius_when_point_constrained(setup_env):
    reg = setup_env
    circle_id = _circle(reg)
    pt = reg.add_point(40, 0)

    c = PointOnLineConstraint(pt, circle_id)
    # The point is unconstrained, so it does not define the radius.
    assert c.constrains_radius(reg, circle_id) is False
