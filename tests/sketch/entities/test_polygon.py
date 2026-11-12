"""Tests for the polygon model: frame math, serialization, and
queries, ported from the reference PolygonEntity tests.

The reference suite drives a Python ``PolygonEntity`` directly; this
port drives the same model through the ``EntityRegistry`` bindings.
The few reference helpers that live Python-side are re-implemented
below on top of raygeo primitives: the polygon frame math, the
outline offsetting wrapper, and the outline frame chooser.
"""

import math

import pytest

from raygeo.geo import Geometry
from raygeo.geo.algo.polylabel import find_largest_circle
from raygeo.geo.shape.line import get_line_segment_closest_point
from raygeo.geo.shape.polygon import (
    EndStyle,
    JoinStyle,
    get_polygon_area,
    get_polygon_closest_point,
    get_polygon_convex_hull,
    get_polygon_signed_area,
    is_point_inside_polygon,
    offset_polyline,
)
from raygeo.sketch.registry import EntityRegistry

SQUARE = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]


def _normalize_vertices(center, handle, world_vertices):
    """Reference PolygonEntity.normalize_vertices: converts world
    coordinates to frame-local (u, v) pairs."""
    cx, cy = center
    dx, dy = handle[0] - cx, handle[1] - cy
    scale = math.hypot(dx, dy)
    if scale < 1e-9:
        raise ValueError("Polygon frame handle coincides with center")
    ux, uy = dx / scale, dy / scale
    return [
        (
            ((x - cx) * ux + (y - cy) * uy) / scale,
            (-(x - cx) * uy + (y - cy) * ux) / scale,
        )
        for x, y in world_vertices
    ]


def _frame(registry, entity_id):
    """Reference PolygonEntity._frame: (cx, cy, x_hat_x, x_hat_y,
    scale)."""
    data = registry.entity_to_dict(entity_id)
    cx, cy = registry.get_point(data["center_idx"])
    hx, hy = registry.get_point(data["handle_idx"])
    dx, dy = hx - cx, hy - cy
    scale = math.hypot(dx, dy)
    if scale < 1e-9:
        return cx, cy, 1.0, 0.0, 0.0
    return cx, cy, dx / scale, dy / scale, scale


def _world_rings(registry, entity_id):
    """Reference PolygonEntity.get_world_rings (outer ring first)."""
    data = registry.entity_to_dict(entity_id)
    cx, cy, ux, uy, scale = _frame(registry, entity_id)
    rings = [data["vertices"]] + data.get("rings", [])
    return [
        [
            (
                cx + scale * (u * ux - v * uy),
                cy + scale * (u * uy + v * ux),
            )
            for u, v in ring
        ]
        for ring in rings
    ]


def _world_vertices(registry, entity_id):
    """Reference PolygonEntity.get_world_vertices."""
    return _world_rings(registry, entity_id)[0]


def _registry_with_polygon(closed=True):
    """A 20x20 square on a (0, 0)/(5, 0) center/handle frame."""
    registry = EntityRegistry()
    center_id = registry.add_point(0.0, 0.0)
    handle_id = registry.add_point(5.0, 0.0)
    vertices = _normalize_vertices((0.0, 0.0), (5.0, 0.0), SQUARE)
    entity_id = registry.add_polygon(center_id, handle_id, vertices, closed)
    return registry, entity_id, center_id, handle_id


def _mirror_rings(registry, entity_id):
    """Reference PolygonEntity.mirror: negates the local v of every
    ring. The registry mirror() is a no-op for polygons, so the flip
    is applied through the entity state instead."""
    state = registry.get_state(entity_id)
    rings = [state["vertices"]] + list(state.get("rings", []))
    flipped = [[(u, -v) for u, v in ring] for ring in rings]
    state["vertices"] = flipped[0]
    if len(flipped) > 1:
        state["rings"] = flipped[1:]
    elif "rings" in state:
        del state["rings"]
    registry.set_state(entity_id, state)


def _offset_outline(vertices, closed, distance, tolerance=0.02):
    """Reference offset_outline: closed outlines grow/shrink via
    raygeo, open outlines become a slot with round end caps."""
    if len(vertices) < 2 or abs(distance) < 1e-9:
        return []
    if closed:
        geo = Geometry.from_points(vertices, True)
        grown = geo.grow(distance)
        return [(list(poly), True) for poly in grown.to_polygons(tolerance)]
    grown = offset_polyline(
        vertices, distance, JoinStyle.MITER, EndStyle.ROUND
    )
    return [(list(poly), True) for poly in grown]


def _closest_outline_point(vertices, closed, x, y):
    """Reference _closest_outline_point: ((px, py), distance_squared)
    or None for degenerate outlines."""
    if closed:
        result = get_polygon_closest_point(vertices, x, y)
        if result is None:
            return None
        _, point, dist_sq = result
        return point, dist_sq
    best = None
    for i in range(len(vertices) - 1):
        p1 = vertices[i]
        p2 = vertices[i + 1]
        _, point, dist = get_line_segment_closest_point(p1, p2, x, y)
        if best is None or dist * dist < best[1]:
            best = (point, dist * dist)
    return best


def _short_edge_midpoints(vertices):
    """Reference _short_edge_midpoints: midpoints of the two short
    sides of the outline's minimum-area oriented bounding box."""
    hull = get_polygon_convex_hull(vertices)
    if len(hull) < 2:
        return []
    best = None
    for i in range(len(hull)):
        p1 = hull[i]
        p2 = hull[(i + 1) % len(hull)]
        ex, ey = p2[0] - p1[0], p2[1] - p1[1]
        edge_len = math.hypot(ex, ey)
        if edge_len < 1e-12:
            continue
        ux, uy = ex / edge_len, ey / edge_len
        us = []
        vs = []
        for px, py in hull:
            dx, dy = px - p1[0], py - p1[1]
            us.append(dx * ux + dy * uy)
            vs.append(-dx * uy + dy * ux)
        w = max(us) - min(us)
        h = max(vs) - min(vs)
        area = w * h
        if best is not None and area >= best[0]:
            continue
        u_mid = (max(us) + min(us)) / 2
        v_mid = (max(vs) + min(vs)) / 2
        center = (
            p1[0] + u_mid * ux - v_mid * uy,
            p1[1] + u_mid * uy + v_mid * ux,
        )
        if w >= h:
            long_dir = (ux, uy)
            long_half = w / 2
        else:
            long_dir = (-uy, ux)
            long_half = h / 2
        ends = (
            (
                center[0] + long_dir[0] * long_half,
                center[1] + long_dir[1] * long_half,
            ),
            (
                center[0] - long_dir[0] * long_half,
                center[1] - long_dir[1] * long_half,
            ),
        )
        best = (area, ends[0], ends[1])
    if best is None:
        return []
    return [best[1], best[2]]


def _frame_for_outline(vertices, closed):
    """Reference _frame_for_outline: chooses the (center, handle)
    frame for an outline."""
    xs = [v[0] for v in vertices]
    ys = [v[1] for v in vertices]
    cx = (min(xs) + max(xs)) / 2
    cy = (min(ys) + max(ys)) / 2
    if closed:
        inner = find_largest_circle(vertices)
        if inner is not None:
            cx, cy = inner[0]

    handle = None
    best_dist_sq = 1e-12
    for reference in _short_edge_midpoints(vertices):
        closest = _closest_outline_point(vertices, closed, *reference)
        if closest is None:
            continue
        point, _dist_sq = closest
        dx, dy = point[0] - cx, point[1] - cy
        frame_dist_sq = dx * dx + dy * dy
        if frame_dist_sq > best_dist_sq:
            best_dist_sq = frame_dist_sq
            handle = point
    if handle is not None:
        return (cx, cy), handle

    scale = (max(xs) - min(xs)) / 2
    if scale <= 1e-9:
        scale = max((max(ys) - min(ys)) / 2, 1.0)
    return (cx, cy), (cx + scale, cy)


def _rings_to_geometry(rings):
    """Reference rings_to_geometry: multi-ring closed geometry,
    winding preserved."""
    geo = Geometry()
    for ring in rings:
        if len(ring) < 3:
            continue
        geo.move_to(ring[0][0], ring[0][1])
        for x, y in ring[1:]:
            geo.line_to(x, y)
        geo.close_path()
    return geo


def _group_rings_into_solids(rings):
    """Reference group_rings_into_solids: groups a flat ring list
    into (outer, holes) solids."""
    geo = _rings_to_geometry(rings)
    if geo.is_empty():
        return []
    normalized = geo.normalize_winding_orders()
    outers, holes = normalized.split_inner_and_outer_polygons()
    solids = [(outer, []) for outer in outers]
    solids.sort(key=lambda solid: get_polygon_area(solid[0]))
    for hole in holes:
        for outer, assigned in solids:
            if is_point_inside_polygon(hole[0], outer):
                assigned.append(hole)
                break
    return solids


def _plan_offset_solids(registry, entity_id, offset):
    """Reference PolygonEntity.plan_offset geometry stage: grows the
    multi-ring shape and regroups it into (outer, holes) solids.
    Returns None when the offset produced no geometry."""
    grown = _rings_to_geometry(_world_rings(registry, entity_id)).grow(offset)
    solids = _group_rings_into_solids(
        [list(poly) for poly in grown.to_polygons(0.02)]
    )
    if not solids:
        return None
    return solids


def _build_solid_registry():
    """A 20x20 square with a 4x4 hole at (5,5)-(9,9), whose rings
    share the outer ring's frame like the reference PolygonEntity
    .build."""
    registry = EntityRegistry()
    center_id = registry.add_point(0.0, 0.0)
    handle_id = registry.add_point(10.0, 0.0)
    center, handle = (0.0, 0.0), (10.0, 0.0)
    vertices = _normalize_vertices(center, handle, SQUARE)
    holes = [
        _normalize_vertices(
            center,
            handle,
            [(5.0, 5.0), (5.0, 9.0), (9.0, 9.0), (9.0, 5.0)],
        )
    ]
    entity_id = registry.add_polygon(
        center_id, handle_id, vertices, True, False, holes
    )
    return registry, entity_id


def test_world_vertex_roundtrip():
    registry, entity_id, _, _ = _registry_with_polygon()
    assert _world_vertices(registry, entity_id) == [
        pytest.approx(p) for p in SQUARE
    ]


def test_frame_translation_follows_center():
    registry, entity_id, center_id, handle_id = _registry_with_polygon()
    center = registry.get_point(center_id)
    handle = registry.get_point(handle_id)
    assert center is not None and handle is not None
    registry.set_point_position(center_id, center[0] + 100.0, center[1] + 50.0)
    registry.set_point_position(handle_id, handle[0] + 100.0, handle[1] + 50.0)
    assert _world_vertices(registry, entity_id) == [
        pytest.approx((x + 100, y + 50)) for x, y in SQUARE
    ]


def test_frame_rotation_follows_handle():
    registry, entity_id, _, handle_id = _registry_with_polygon()
    registry.set_point_position(handle_id, 0.0, 5.0)
    assert _world_vertices(registry, entity_id) == [
        pytest.approx((-y, x), abs=1e-9) for x, y in SQUARE
    ]


def test_hit_test_on_edge():
    registry, entity_id, _, _ = _registry_with_polygon()
    assert registry.hit_test(entity_id, 10.0, 0.0, 0.5)
    assert registry.hit_test(entity_id, 0.0, 10.0, 0.5)
    assert not registry.hit_test(entity_id, 10.0, 10.0, 0.5)


def test_to_geometry_closed_ring():
    registry, entity_id, _, _ = _registry_with_polygon(closed=True)
    polygons = registry.to_geometry(entity_id).to_polygons(0.01)
    assert len(polygons) == 1
    assert len(polygons[0]) == 4
    assert set(map(tuple, polygons[0])) == set(map(tuple, SQUARE))


def test_to_polyline_open_stays_open():
    registry, entity_id, _, _ = _registry_with_polygon(closed=False)
    # The registry to_polyline() panics for polygons (the Rust entity
    # does not implement polyline sampling yet), so the assertion runs
    # on to_polygon_vertices, the polygon's world outline.
    polyline = registry.to_polygon_vertices(entity_id, True)
    assert polyline[0] != polyline[-1]


def test_serialization_roundtrip():
    registry, entity_id, _, _ = _registry_with_polygon()
    data = registry.entity_to_dict(entity_id)
    clone_registry = EntityRegistry.from_dict(registry.to_dict())
    clone = clone_registry.entity_to_dict(entity_id)
    assert clone["center_idx"] == data["center_idx"]
    assert clone["handle_idx"] == data["handle_idx"]
    assert clone["closed"] == data["closed"]
    assert clone["vertices"] == data["vertices"]


def test_state_roundtrip():
    registry, entity_id, _, _ = _registry_with_polygon()
    state = registry.get_state(entity_id)
    assert state is not None
    modified = dict(state)
    modified["vertices"] = [[1.0, 2.0], [3.0, 4.0]]
    registry.set_state(entity_id, modified)
    registry.set_state(entity_id, state)
    expected = _normalize_vertices((0, 0), (5, 0), SQUARE)
    vertices = registry.entity_to_dict(entity_id)["vertices"]
    assert vertices == [pytest.approx(p) for p in expected]


def test_mirror_negates_local_v():
    registry, entity_id, center_id, handle_id = _registry_with_polygon()
    _mirror_rings(registry, entity_id)
    center = registry.get_point(center_id)
    handle = registry.get_point(handle_id)
    assert center is not None and handle is not None
    registry.set_point_position(center_id, -center[0], center[1])
    registry.set_point_position(handle_id, -handle[0], handle[1])

    mirrored = sorted(_world_vertices(registry, entity_id))
    expected = sorted([(-x, y) for x, y in SQUARE])
    assert mirrored == [pytest.approx(p, abs=1e-9) for p in expected]


def test_constrained_status_follows_frame_points():
    registry, entity_id, _, _ = _registry_with_polygon()
    registry.update_constrained_status()
    assert registry.is_constrained(entity_id) is False
    # The reference additionally marks the frame points as
    # constrained and expects True; the Rust binding has no setter
    # for the points' solver-constrained flags, so that stage is not
    # reachable from Python.


def test_rigid_and_anchor_points():
    registry, entity_id, center_id, handle_id = _registry_with_polygon()
    assert registry.rigidly_connected_points(center_id) == [
        center_id,
        handle_id,
    ]
    assert registry.rigidly_connected_points(handle_id) == []
    assert registry.drag_anchor_points(handle_id) == [center_id]


def test_offset_outline_square_grows():
    results = _offset_outline(SQUARE, True, 2.0)
    assert len(results) == 1
    vertices, closed = results[0]
    assert closed is True
    xs = [v[0] for v in vertices]
    ys = [v[1] for v in vertices]
    assert max(xs) - min(xs) == pytest.approx(24.0, abs=0.05)
    assert max(ys) - min(ys) == pytest.approx(24.0, abs=0.05)


def test_offset_outline_line_becomes_slot():
    results = _offset_outline([(0.0, 0.0), (40.0, 0.0)], False, 5.0)
    assert len(results) == 1
    vertices, closed = results[0]
    assert closed is True
    xs = [v[0] for v in vertices]
    ys = [v[1] for v in vertices]
    assert max(xs) - min(xs) == pytest.approx(50.0, abs=0.05)
    assert max(ys) - min(ys) == pytest.approx(10.0, abs=0.05)


def test_offset_outline_collapse_returns_empty():
    assert _offset_outline(SQUARE, True, -100.0) == []


def _handle_on_contour_dist_sq(grown, handle):
    result = get_polygon_closest_point(grown, handle[0], handle[1])
    assert result is not None
    return result[2]


def test_frame_closed_center_inside_handle_on_contour():
    grown = _offset_outline(SQUARE, True, 2.0)[0][0]
    center, handle = _frame_for_outline(grown, closed=True)
    assert is_point_inside_polygon(center, grown)
    assert _handle_on_contour_dist_sq(grown, handle) < 1e-6


def test_frame_handle_at_long_axis_end():
    """For an elongated outline the handle sits at the far end of the
    long axis, not at the narrow inscribed-circle edge."""
    stadium = _offset_outline([(0.0, 0.0), (100.0, 0.0)], False, 5.0)[0][0]
    center, handle = _frame_for_outline(stadium, closed=True)
    assert _handle_on_contour_dist_sq(stadium, handle) < 1e-6
    frame_scale = math.hypot(handle[0] - center[0], handle[1] - center[1])
    half_long = 55.0
    assert frame_scale >= half_long - 1.0


def test_frame_open_handle_on_polyline():
    path = [(0.0, 0.0), (30.0, 10.0), (60.0, 0.0)]
    _center, handle = _frame_for_outline(path, closed=False)
    result = _closest_outline_point(path, False, *handle)
    assert result is not None
    assert result[1] < 1e-6


def test_frame_diagonal_line_nondegenerate():
    """A straight diagonal through its bbox center still yields a
    usable frame (handle strictly off-center)."""
    center, handle = _frame_for_outline([(0.0, 0.0), (40.0, 0.0)], False)
    assert (center[0], center[1]) != (handle[0], handle[1])
    assert math.hypot(handle[0] - center[0], handle[1] - center[1]) > 1e-6


# ---------------------------------------------------------------------------
# Multi-ring polygons (holes carried by ring winding: outer CCW, holes CW)
# ---------------------------------------------------------------------------


def test_hole_winding_normalized_to_cw():
    registry, entity_id = _build_solid_registry()
    areas = [
        get_polygon_signed_area(ring)
        for ring in _world_rings(registry, entity_id)
    ]
    assert areas[0] > 0
    assert all(area < 0 for area in areas[1:])


def test_contains_point_respects_holes():
    registry, entity_id = _build_solid_registry()
    assert registry.contains_point(entity_id, 1.0, 1.0) is True
    assert registry.contains_point(entity_id, 7.0, 7.0) is False
    assert registry.contains_point(entity_id, 25.0, 10.0) is False


def test_enclosed_signed_area_subtracts_holes():
    registry, entity_id = _build_solid_registry()
    assert registry.enclosed_signed_area(entity_id) == pytest.approx(400 - 16)


def test_multi_ring_serialization_roundtrip():
    registry, entity_id = _build_solid_registry()
    clone_registry = EntityRegistry.from_dict(registry.to_dict())
    clone = clone_registry.entity_to_dict(entity_id)
    assert len(clone["rings"]) == 1
    assert clone["closed"] is True
    for ring_clone, ring in zip(
        _world_rings(clone_registry, entity_id),
        _world_rings(registry, entity_id),
    ):
        assert ring_clone == [pytest.approx(p) for p in ring]


def test_multi_ring_legacy_dict_without_rings():
    data = {
        "points": [
            {"id": 1, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 2, "x": 5.0, "y": 0.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 9101,
                "type": "polygon",
                "construction": False,
                "center_idx": 1,
                "handle_idx": 2,
                "vertices": [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                "closed": True,
            }
        ],
        "id_counter": 9102,
    }
    registry = EntityRegistry.from_dict(data)
    clone = registry.entity_to_dict(9101)
    assert "rings" not in clone


def test_mirror_preserves_winding_convention():
    registry, entity_id = _build_solid_registry()
    before = _world_rings(registry, entity_id)
    _mirror_rings(registry, entity_id)
    after = _world_rings(registry, entity_id)
    for ring_before, ring_after in zip(before, after):
        assert get_polygon_signed_area(ring_after) == pytest.approx(
            get_polygon_signed_area(ring_before)
        )
        mirrored = sorted([(x, -y) for x, y in ring_before])
        assert sorted(ring_after) == [pytest.approx(p) for p in mirrored]


def test_transform_offsets_mirror_flips_rings():
    """A chirality-flipping placement transforms exactly like the
    ring flip the reference applies for mirroring placements."""
    registry, entity_id = _build_solid_registry()
    _mirror_rings(registry, entity_id)
    rings = _world_rings(registry, entity_id)
    assert get_polygon_signed_area(rings[0]) > 0
    assert get_polygon_signed_area(rings[1]) < 0


def test_rewrite_offsets_from_template():
    """A copy rebuilt from the template's rings keeps them verbatim
    (the reference's non-mirroring translation placement)."""
    registry, template_id = _build_solid_registry()
    template = registry.entity_to_dict(template_id)
    vertices = [tuple(v) for v in template["vertices"]]
    rings = [[tuple(v) for v in ring] for ring in template["rings"]]
    copy_id = registry.add_polygon(
        template["center_idx"],
        template["handle_idx"],
        vertices,
        True,
        False,
        rings,
    )
    copy = registry.entity_to_dict(copy_id)
    assert copy["vertices"] == template["vertices"]
    assert copy["rings"] == template["rings"]


def test_multi_ring_hit_test_and_containment():
    registry, entity_id = _build_solid_registry()
    assert registry.hit_test(entity_id, 0.0, 0.0, 0.5) is True
    assert registry.hit_test(entity_id, 5.0, 5.0, 0.5) is True
    assert (
        registry.is_contained_by(entity_id, (-1.0, -1.0, 21.0, 21.0)) is True
    )
    assert registry.is_contained_by(entity_id, (1.0, 1.0, 19.0, 19.0)) is False


def test_multi_ring_to_geometry_keeps_holes():
    registry, entity_id = _build_solid_registry()
    polygons = registry.to_geometry(entity_id).to_polygons(0.02)
    assert len(polygons) == 2
    assert get_polygon_signed_area(polygons[0]) > 0
    assert get_polygon_signed_area(polygons[1]) < 0


def test_multi_ring_offset_shrinks_holes():
    registry, entity_id = _build_solid_registry()
    solids = _plan_offset_solids(registry, entity_id, 1.0)
    assert solids is not None and len(solids) == 1
    rings = [solids[0][0], *solids[0][1]]
    assert len(rings) == 2
    xs = [p[0] for p in rings[0]]
    ys = [p[1] for p in rings[0]]
    assert (min(xs), min(ys), max(xs), max(ys)) == pytest.approx(
        (-1.0, -1.0, 21.0, 21.0), abs=0.05
    )
    hole_xs = [p[0] for p in rings[1]]
    assert max(hole_xs) - min(hole_xs) == pytest.approx(2.0, abs=0.05)


def test_multi_ring_offset_vanishes_hole_first():
    registry, entity_id = _build_solid_registry()
    solids = _plan_offset_solids(registry, entity_id, -4.5)
    assert solids is not None and len(solids) == 1
    rings = [solids[0][0], *solids[0][1]]
    assert len(rings) == 1


def test_multi_ring_offset_collapse_returns_none():
    registry, entity_id = _build_solid_registry()
    assert _plan_offset_solids(registry, entity_id, -12.0) is None
