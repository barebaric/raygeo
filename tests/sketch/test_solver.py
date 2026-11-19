"""Tests for the high-level registry solver binding."""

import math

import pytest

from raygeo.sketch import conflicting_constraints, solve_registry
from raygeo.sketch.constraints.distance import DistanceConstraint
from raygeo.sketch.constraints.horizontal import HorizontalConstraint
from raygeo.sketch.constraints.radius import RadiusConstraint
from raygeo.sketch.registry import EntityRegistry


def _distance(reg, p1, p2):
    a = reg.get_point(p1)
    b = reg.get_point(p2)
    assert a is not None and b is not None
    return math.dist(a, b)


def test_solve_registry_updates_positions_and_flags():
    reg = EntityRegistry()
    p1 = reg.add_point(0, 0, fixed=True)
    p2 = reg.add_point(5, 0)

    constraints = [
        HorizontalConstraint(p1, p2),
        DistanceConstraint(p1, p2, 10.0),
    ]
    assert solve_registry(reg, constraints) is True

    assert reg.get_point(p2) == pytest.approx((10.0, 0.0), abs=1e-4)
    assert reg.point_constrained(p2) is True


def test_solve_registry_can_skip_dof_flags():
    reg = EntityRegistry()
    p1 = reg.add_point(0, 0, fixed=True)
    p2 = reg.add_point(5, 0)

    constraints = [DistanceConstraint(p1, p2, 10.0)]
    assert solve_registry(reg, constraints, update_dof=False) is True

    assert _distance(reg, p1, p2) == pytest.approx(10.0, abs=1e-4)
    assert reg.point_constrained(p2) is False


def test_solve_registry_updates_entity_flags():
    reg = EntityRegistry()
    p1 = reg.add_point(0, 0, fixed=True)
    p2 = reg.add_point(10, 0, fixed=True)
    line = reg.add_line(p1, p2)
    assert reg.is_constrained(line) is False

    assert solve_registry(reg, []) is True

    assert reg.is_constrained(line) is True


def test_solve_registry_uses_radius_constraints_for_entity_flags():
    reg = EntityRegistry()
    center = reg.add_point(0, 0, fixed=True)
    radius_point = reg.add_point(5, 0)
    circle = reg.add_circle(center, radius_point)
    constraints = [RadiusConstraint(circle, 10.0)]

    assert solve_registry(reg, constraints) is True

    assert reg.is_constrained(circle) is True


def test_solve_registry_respects_point_filter():
    reg = EntityRegistry()
    p1 = reg.add_point(0, 0, fixed=True)
    p2 = reg.add_point(5, 0)
    p3 = reg.add_point(100, 100)

    constraints = [DistanceConstraint(p1, p2, 10.0)]
    assert solve_registry(reg, constraints, point_filter=[p3]) is True

    assert reg.get_point(p2) == pytest.approx((5.0, 0.0))
    assert reg.get_point(p3) == pytest.approx((100.0, 100.0))


def test_conflicting_constraints_uses_current_positions():
    reg = EntityRegistry()
    p1 = reg.add_point(0, 0, fixed=True)
    p2 = reg.add_point(15, 0)
    constraints = [
        DistanceConstraint(p1, p2, 10.0),
        DistanceConstraint(p1, p2, 20.0),
    ]

    assert solve_registry(reg, constraints) is False
    assert conflicting_constraints(reg, constraints) == [0, 1]
