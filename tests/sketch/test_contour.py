from raygeo.sketch.contour import (
    chain_vertices,
    group_rings_into_solids,
    offset_outline,
    outline_frame,
    partition_components,
    rings_to_geometry,
    walk_component,
)
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
