"""Tests for the SnapEngine query flow through its Python binding."""

import pytest

from raygeo.sketch.registry import EntityRegistry
from raygeo.sketch.snap.engine import SnapEngine, geometry_key
from raygeo.sketch.snap.producers import EntityPointsProducer
from raygeo.sketch.snap.types import DragContext


@pytest.fixture
def registry():
    reg = EntityRegistry()
    a = reg.add_point(0.0, 0.0, False)
    b = reg.add_point(10.0, 0.0, False)
    reg.add_line(a, b, False)
    return reg


def _engine(registry):
    """A engine with the entity-point producer registered, mirroring
    the host's setup."""
    engine = SnapEngine()
    engine.register_producer(EntityPointsProducer())
    return engine


def test_engine_disabled_returns_no_snap(registry):
    engine = _engine(registry)
    engine.enabled = False
    result = engine.query(registry, (1.0, 1.0))
    assert result.snapped is False
    assert result.distance == float("inf")


def test_engine_snaps_to_entity_point(registry):
    engine = _engine(registry)
    result = engine.query(registry, (0.3, 0.4))
    assert result.snapped is True
    assert result.primary_snap_point is not None
    point = result.primary_snap_point
    assert point.x == pytest.approx(0.0)
    assert point.y == pytest.approx(0.0)
    assert point.line_type.name == "ENTITY_POINT"


def test_engine_point_hysteresis_keeps_primary(registry):
    """Within the exit radius the previous primary stays selected even
    when another kind is nearer."""
    engine = _engine(registry)
    engine.threshold = 2.0
    first = engine.query(registry, (0.2, 0.2))
    assert first.primary_snap_point is not None
    assert first.primary_snap_point.line_type.name == "ENTITY_POINT"

    # Still inside the 1.5x exit radius around the primary point.
    second = engine.query(registry, (1.5, 1.5))
    assert second.snapped is True
    assert second.primary_snap_point is not None
    assert second.primary_snap_point.line_type.name == "ENTITY_POINT"


def test_engine_snap_lines_from_index(registry):
    engine = _engine(registry)
    result = engine.query(registry, (5.0, 0.5))
    # The line y=0 through the sketch's points is a horizontal guide.
    horizontal = [line for line in result.snap_lines if line.is_horizontal]
    assert any(abs(line.coordinate) < 1e-9 for line in horizontal)


def test_engine_rebuild_is_cached(registry):
    engine = _engine(registry)
    context = DragContext()
    engine.rebuild_index(registry, (0.0, 0.0), context)
    assert engine.is_snap_result_current(registry)
    engine.invalidate_index()
    assert engine.is_snap_result_current(registry) is False


def test_engine_detects_geometry_change(registry):
    engine = _engine(registry)
    engine.rebuild_index(registry, (0.0, 0.0), DragContext())
    pid = registry.points()[0].id
    registry.set_point_position(pid, 1.5, 2.5)
    assert engine.is_snap_result_current(registry) is False


def test_engine_visible_lines(registry):
    engine = _engine(registry)
    lines = engine.get_visible_snap_lines(registry, (0.0, 0.0))
    assert lines, "expected the entity-point guides of the sketch points"
    assert all(line.line_type.name == "ENTITY_POINT" for line in lines)


def test_geometry_key_tracks_points_and_construction(registry):
    base = geometry_key(registry)
    pid = registry.points()[0].id
    registry.set_point_position(pid, 3.0, 3.0)
    changed = geometry_key(registry)
    assert base != changed

    construction_line = registry.add_line(
        registry.add_point(1.0, 1.0, False),
        registry.add_point(2.0, 2.0, False),
        True,
    )
    assert construction_line is not None
    without_construction = geometry_key(registry, include_construction=False)
    with_construction = geometry_key(registry, include_construction=True)
    assert without_construction != with_construction
