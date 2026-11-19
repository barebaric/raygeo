"""Tests for the constraint-component bindings."""

from raygeo.sketch.components import (
    compute_constraint_components,
    get_referenced_points,
)
from raygeo.sketch.constraints.distance import DistanceConstraint
from raygeo.sketch.constraints.tangent import TangentConstraint
from raygeo.sketch.registry import EntityRegistry


def test_components_separate_disjoint_constraints():
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0, fixed=True)
    p2 = registry.add_point(5, 0)
    p3 = registry.add_point(200, 0, fixed=True)
    p4 = registry.add_point(205, 0)

    components = compute_constraint_components(
        registry,
        [
            DistanceConstraint(p1, p2, 10.0),
            DistanceConstraint(p3, p4, 50.0),
        ],
    )

    assert {frozenset(component) for component in components} == {
        frozenset({p1, p2}),
        frozenset({p3, p4}),
    }


def test_referenced_points_expands_entity_references():
    registry = EntityRegistry()
    center = registry.add_point(0, 0)
    radius_point = registry.add_point(5, 0)
    line_p1 = registry.add_point(10, 10)
    line_p2 = registry.add_point(20, 10)
    circle = registry.add_circle(center, radius_point)
    line = registry.add_line(line_p1, line_p2)

    constraint = TangentConstraint(line, circle)

    assert set(get_referenced_points(registry, constraint)) == {
        line_p1,
        line_p2,
        center,
        radius_point,
    }
    assert compute_constraint_components(registry, [constraint]) == [
        {center, radius_point, line_p1, line_p2}
    ]
