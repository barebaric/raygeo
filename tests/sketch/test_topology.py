"""Tests for sketch topology: chain assembly and loop detection."""

import math

import pytest

from raygeo.sketch.topology import (
    Edge,
    LoopShape,
    find_closed_loops,
    plan_chains,
)


def line_angle(p_from, p_to):
    return math.atan2(p_to[1] - p_from[1], p_to[0] - p_from[0])


def _rectangle_edges(width=10.0, height=5.0):
    """CCW rectangle: p1(0,0) p2(w,0) p3(w,h) p4(0,h)."""
    p1, p2, p3, p4 = (0, 0), (width, 0), (width, height), (0, height)
    edges = [
        Edge(1, 1, 2, line_angle(p1, p2), line_angle(p2, p1)),
        Edge(2, 2, 3, line_angle(p2, p3), line_angle(p3, p2)),
        Edge(3, 3, 4, line_angle(p3, p4), line_angle(p4, p3)),
        Edge(4, 4, 1, line_angle(p4, p1), line_angle(p1, p4)),
    ]
    points = [
        (1, 0.0, 0.0),
        (2, width, 0.0),
        (3, width, height),
        (4, 0.0, height),
    ]
    return edges, points


def test_plan_chains_single_closed_rectangle():
    edges, _ = _rectangle_edges()
    chains = plan_chains(edges, [])
    assert len(chains) == 1
    assert chains[0] == [(1, True), (2, True), (3, True), (4, True)]


def test_plan_chains_two_islands():
    edges, _ = _rectangle_edges()
    spur = Edge(5, 100, 101, 0.0, math.pi)
    chains = plan_chains(edges + [spur], [])
    assert len(chains) == 2
    assert chains[1] == [(5, True)]


def test_plan_chains_bridged_by_coincidence():
    edges, _ = _rectangle_edges()
    spur = Edge(5, 100, 101, 0.0, math.pi)
    chains = plan_chains(edges + [spur], [(100, 1), (101, 2)])
    assert len(chains) == 1
    assert len(chains[0]) == 5
    assert {entity_id for entity_id, _ in chains[0]} == {1, 2, 3, 4, 5}


def test_plan_chains_respects_forward_flags():
    p1, p2, p3, p4 = (0, 0), (10, 0), (10, 5), (0, 5)
    edges = [
        Edge(1, 1, 2, line_angle(p1, p2), line_angle(p2, p1)),
        Edge(2, 3, 2, line_angle(p3, p2), line_angle(p2, p3)),
        Edge(3, 3, 4, line_angle(p3, p4), line_angle(p4, p3)),
        Edge(4, 4, 1, line_angle(p4, p1), line_angle(p1, p4)),
    ]
    chains = plan_chains(edges, [])
    assert len(chains) == 1
    directions = dict(chains[0])
    assert directions[2] is False
    assert directions[1] is True and directions[3] is True


def _shoelace(polygon):
    area = 0.0
    for i in range(len(polygon)):
        x1, y1 = polygon[i]
        x2, y2 = polygon[(i + 1) % len(polygon)]
        area += x1 * y2 - x2 * y1
    return area * 0.5


def test_find_closed_loops_rectangle():
    edges, points = _rectangle_edges()
    loops = find_closed_loops(edges, points, [], [])
    assert len(loops) == 1
    assert loops[0] == [(1, True), (2, True), (3, True), (4, True)]


def test_find_closed_loops_with_diagonal_yields_two_triangles():
    edges, points = _rectangle_edges()
    diagonal = Edge(
        5,
        1,
        3,
        line_angle((0, 0), (10, 5)),
        line_angle((10, 5), (0, 0)),
    )
    loops = find_closed_loops(edges + [diagonal], points, [], [])
    assert len(loops) == 2
    coords = {pid: (x, y) for pid, x, y in points}
    for loop in loops:
        polygon = [coords[point] for _entity, _fwd in loop for point in ()]
        # Triangles (1,2,3) and (1,3,4) each enclose half the square.
        corners = sorted(
            {coords[point] for entity_id, _ in loop for point in ()}
        )
        del polygon, corners
    # Verify by area: reconstruct each triangle from its loop.
    edge_lookup = {e.entity_id: e for e in edges + [diagonal]}
    areas = []
    for loop in loops:
        current = None
        polygon = []
        for entity_id, fwd in loop:
            edge = edge_lookup[entity_id]
            if current is None:
                current = edge.p1 if fwd else edge.p2
            polygon.append(coords[current])
            current = edge.p2 if current == edge.p1 else edge.p1
        areas.append(abs(_shoelace(polygon)))
    assert sorted(areas) == [pytest.approx(25.0), pytest.approx(25.0)]


def test_find_closed_loops_zero_area_dropped():
    edges = [
        Edge(1, 1, 2, 0.0, math.pi),
        Edge(2, 2, 3, 0.0, math.pi),
        Edge(3, 3, 1, 0.0, 0.0),
    ]
    points = [(1, 0.0, 0.0), (2, 5.0, 0.0), (3, 9.0, 0.0)]
    assert find_closed_loops(edges, points, [], []) == []


def test_find_closed_loops_appends_closed_entities():
    edges, points = _rectangle_edges()
    closed = LoopShape.Closed(entity_id=99, signed_area=42.0)
    loops = find_closed_loops(edges, points, [], [closed])
    assert len(loops) == 2
    assert loops[-1] == [(99, True)]


def test_find_closed_loops_arc_chirality_flips_area():
    """A 2x10 rectangle whose left edge is a semicircular arc: the
    inward (clockwise) bulge subtracts a 0.5*pi*r^2 segment that
    outweighs the +20 shoelace, dropping the loop; the outward
    (counter-clockwise) bulge adds it and keeps the loop.  This pins
    the circular-segment correction's sign and magnitude."""

    def tangent(center, point, clockwise):
        ux = (point[0] - center[0]) / 5.0
        uy = (point[1] - center[1]) / 5.0
        if clockwise:
            return (uy, -ux)
        return (-uy, ux)

    center = (0.0, 5.0)
    p4, p1 = (0.0, 10.0), (0.0, 0.0)
    points = [(1, 0.0, 0.0), (2, 2.0, 0.0), (3, 2.0, 10.0), (4, 0.0, 10.0)]
    base = [
        Edge(1, 1, 2, line_angle(p1, p2 := (2, 0)), line_angle(p2, p1)),
        Edge(2, 2, 3, line_angle(p2, (2, 10)), line_angle((2, 10), p2)),
        Edge(3, 3, 4, line_angle((2, 10), p4), line_angle(p4, (2, 10))),
    ]

    def build(clockwise):
        t4 = tangent(center, p4, clockwise)
        t1 = tangent(center, p1, clockwise)
        arc = Edge(4, 4, 1, math.atan2(t4[1], t4[0]), math.atan2(t1[1], t1[0]))
        return base + [arc]

    points_with_center = points + [(5, center[0], center[1])]
    arc_shape = LoopShape.Arc(
        entity_id=4, start=4, end=1, center=5, clockwise=False
    )
    ccw_loops = find_closed_loops(
        build(False), points_with_center, [], [arc_shape]
    )
    assert len(ccw_loops) == 1
    assert (4, True) in ccw_loops[0]

    cw_shape = LoopShape.Arc(
        entity_id=4, start=4, end=1, center=5, clockwise=True
    )
    cw_loops = find_closed_loops(
        build(True), points_with_center, [], [cw_shape]
    )
    # The reference traversal traces both faces of the cycle.  For
    # the inward bulge the forward face is negative (20 - 39.27) and
    # dropped, but its reversed face flips both the shoelace and the
    # segment correction (-20 + 39.27 > 0) and is kept.
    assert len(cw_loops) == 1
    assert cw_loops[0] == [(4, False), (3, False), (2, False), (1, False)]
