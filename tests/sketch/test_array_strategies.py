import math

from raygeo.sketch.arrays import (
    ArrayStrategy,
    CircularArrayStrategy,
    CurveAlongArrayStrategy,
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
