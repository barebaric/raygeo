import math

from raygeo.sketch.arrays import (
    ArrayStrategy,
    CircularArrayStrategy,
    CurveAlongArrayStrategy,
    path_length,
    sample_path,
)
from raygeo.sketch.registry import EntityRegistry


def test_circular_template_placement_projects_onto_orbit():
    strategy = CircularArrayStrategy(count=4, center=(0.0, 0.0), radius=2.0)
    placement = strategy.template_placement((1.0, 0.0))
    x, y = placement.transform_point(1.0, 0.0)
    assert math.isclose(x, 2.0)
    assert math.isclose(y, 0.0)


def test_circular_member_placements_rotate_about_center():
    strategy = CircularArrayStrategy(count=4, center=(0.0, 0.0), radius=2.0)
    placements = strategy.member_placements((2.0, 0.0))
    assert len(placements) == 3
    x, y = placements[0].transform_point(2.0, 0.0)
    assert math.isclose(x, 0.0, abs_tol=1e-9)
    assert math.isclose(y, 2.0, abs_tol=1e-9)


def test_curve_strategy_samples_a_line():
    registry = EntityRegistry()
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    line = registry.add_line(a, b, False)
    strategy = CurveAlongArrayStrategy(count=3, path_entity_id=line)
    placement = strategy.template_placement((1.0, 2.0), registry)
    x, y = placement.transform_point(1.0, 2.0)
    assert math.isclose(x, 0.0, abs_tol=1e-9)
    assert math.isclose(y, 0.0, abs_tol=1e-9)
    assert len(strategy.member_placements((1.0, 2.0), registry)) == 2


def test_strategies_share_polymorphic_base():
    circular = CircularArrayStrategy(count=3, center=(0.0, 0.0))
    curve = CurveAlongArrayStrategy(count=3, path_entity_id=-1)
    assert isinstance(circular, ArrayStrategy)
    assert isinstance(curve, ArrayStrategy)
    assert circular.needs_center_point is True
    assert curve.uses_template_anchor is True
    assert isinstance(circular.template_placement((1.0, 1.0)), object)


def test_guide_sampling_matches_entity_discretization():
    """The curve-along guide polyline uses the canonical entity
    discretization, so its arc length matches the entity's own
    polyline (array guides and contour edges agree)."""
    registry = EntityRegistry()
    center = registry.add_point(0.0, 0.0, False)
    start = registry.add_point(10.0, 0.0, False)
    end = registry.add_point(-10.0, 0.0, False)
    arc = registry.add_arc(start, end, center, clockwise=False)

    guide_length = path_length(registry, arc)
    polyline = registry.get_entity(arc).to_polyline(registry, 0.02)
    entity_length = sum(
        math.dist(polyline[i], polyline[i + 1])
        for i in range(len(polyline) - 1)
    )
    assert math.isclose(guide_length, entity_length, rel_tol=1e-9)

    # Sampled placements sit on the arc's circumference within the
    # chord sag of the 16-segment half-circle polyline
    # (r * (1 - cos(theta/2)) ~= 0.048 here).
    samples = sample_path(registry, arc, 5, 0.0)
    assert len(samples) == 5
    for (x, y), _tangent in samples:
        assert math.isclose(math.hypot(x, y), 10.0, rel_tol=0.0, abs_tol=0.05)
