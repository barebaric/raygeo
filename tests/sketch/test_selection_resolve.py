from raygeo.sketch.registry import EntityRegistry
from raygeo.sketch.selection import Selection


def test_resolve_entities_returns_objects():
    registry = EntityRegistry()
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(1.0, 0.0, False)
    line = registry.add_line(a, b, False)
    selection = Selection()
    selection.entity_ids = [line]
    entities = selection.resolve_entities(registry)
    assert entities is not None
    assert len(entities) == 1
    assert entities[0].id == line
