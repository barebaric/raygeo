from raygeo.sketch.registry import EntityRegistry
from raygeo.sketch.snap import DragContext, SnapEngine, SnapLineType
from raygeo.sketch.snap.producers import EntityPointsProducer


def test_entity_points_producer_snaps_to_nearby_point():
    registry = EntityRegistry()
    registry.add_point(10.0, 10.0)
    registry.add_point(100.0, 100.0)

    engine = SnapEngine(threshold=5.0)
    engine.register_producer(EntityPointsProducer())
    result = engine.query(registry, (12.0, 11.0), DragContext())

    assert result.snapped is True
    assert result.position == (10.0, 10.0)
    assert result.primary_snap_point is not None
    assert result.primary_snap_point.line_type is SnapLineType.ENTITY_POINT
