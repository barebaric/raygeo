"""Port of the reference point-on-curve constraint tests against the
raygeo bindings (entity-reference contract)."""

import pytest

from raygeo.sketch.constraints.point_on_curve import PointOnCurveConstraint
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def setup_env():
    return EntityRegistry()


def _bezier(reg):
    start = reg.add_point(0, 0)
    end = reg.add_point(100, 0)
    bezier_id = reg.add_bezier(start, end, cp1=(30.0, 20.0), cp2=(-30.0, 20.0))
    return bezier_id, start, end


def test_point_on_curve(setup_env):
    reg = setup_env
    bezier_id, _start, _end = _bezier(reg)
    pt = reg.add_point(50, 15)

    c = PointOnCurveConstraint(pt, bezier_id)
    assert c.error(reg, None) == pytest.approx([0.0], abs=0.01)
    assert c.user_visible is True


def test_point_off_curve(setup_env):
    reg = setup_env
    bezier_id, _start, _end = _bezier(reg)
    pt = reg.add_point(50, 100)

    c = PointOnCurveConstraint(pt, bezier_id)
    assert abs(c.error(reg, None)[0]) > 1.0


def test_entity_refs(setup_env):
    reg = setup_env
    bezier_id, _start, _end = _bezier(reg)
    pt = reg.add_point(50, 15)

    c = PointOnCurveConstraint(pt, bezier_id)
    assert c.point_id == pt
    assert c.shape_id == bezier_id
