from raygeo.sketch.constraints import CoincidentConstraint
from raygeo.sketch.contour import (
    build_offset_items,
    build_outline_item,
    chain_vertices,
    group_rings_into_solids,
    offset_outline,
    outline_frame,
    partition_components,
    rings_to_geometry,
    walk_component,
)
from raygeo.sketch.entities import Polygon, PolygonOutline
from raygeo.sketch.registry import EntityRegistry


def _line_chain():
    registry = EntityRegistry()
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    c = registry.add_point(20.0, 0.0, False)
    e1 = registry.add_line(a, b, False)
    e2 = registry.add_line(b, c, False)
    return registry, [e1, e2]


def test_partition_and_walk_chain():
    registry, edges = _line_chain()
    components = partition_components(registry, edges, [])
    assert components == [sorted(edges)]
    ordered, closed = walk_component(registry, edges, [])
    assert closed is False
    assert len(ordered) == 2
    vertices = chain_vertices(registry, ordered, False)
    assert len(vertices) == 3
    assert (0.0, 0.0) in vertices
    assert (10.0, 0.0) in vertices
    assert (20.0, 0.0) in vertices


def test_rings_to_geometry_and_solid_grouping():
    outer = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
    hole = [(2.0, 2.0), (4.0, 2.0), (4.0, 4.0), (2.0, 4.0)]
    geo = rings_to_geometry([outer, hole])
    assert not geo.is_empty()
    solids = group_rings_into_solids([outer, hole])
    assert len(solids) == 1
    assert len(solids[0][1]) == 1


def test_offset_outline_closed():
    square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
    result = offset_outline(square, True, 1.0)
    assert result
    assert all(closed for _ring, closed in result)


def test_outline_frame_is_inside_and_unique():
    square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
    center, handle = outline_frame(square, True)
    assert 0.0 < center[0] < 10.0
    assert 0.0 < center[1] < 10.0
    assert handle != center


def test_build_offset_items_joins_chain():
    registry = EntityRegistry()
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    c = registry.add_point(20.0, 10.0, False)
    e1 = registry.add_line(a, b, False)
    e2 = registry.add_line(b, c, False)

    items = build_offset_items(registry, [e1, e2], [])

    assert items is not None
    assert len(items) == 1
    assert isinstance(items[0], PolygonOutline)
    assert items[0].closed is False


def test_build_offset_items_joins_coincident_chain():
    registry = EntityRegistry()
    a = registry.add_point(0.0, 0.0, False)
    b = registry.add_point(10.0, 0.0, False)
    c = registry.add_point(20.0, 10.0, False)
    d = registry.add_point(10.0, 0.0, False)
    e1 = registry.add_line(a, b, False)
    e2 = registry.add_line(c, d, False)
    constraints = [CoincidentConstraint(b, d)]

    items = build_offset_items(registry, [e1, e2], constraints)

    assert items is not None
    assert len(items) == 1
    assert isinstance(items[0], PolygonOutline)
    assert items[0].source_ids == [e1, e2]


def test_build_offset_items_keeps_standalone_entity():
    registry = EntityRegistry()
    center = registry.add_point(10.0, 10.0, False)
    radius = registry.add_point(15.0, 10.0, False)
    circle = registry.add_circle(center, radius, False)

    items = build_offset_items(registry, [circle], [])

    assert items is not None
    assert items[0].id == circle


def test_build_outline_item():
    ids = iter([100, 101, 102])
    center, handle, polygon = build_outline_item(
        [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
        True,
        lambda: next(ids),
    )

    assert center.id == 100
    assert handle.id == 101
    assert isinstance(polygon, Polygon)
    assert polygon.id == 102
