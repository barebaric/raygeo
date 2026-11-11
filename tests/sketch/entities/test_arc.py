"""Entity tests for the sketch arc model."""

import math

import pytest

from raygeo.geo import Arc as GeoArc
from raygeo.geo import Geometry, Move
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def registry():
    return EntityRegistry()


def test_arc_serialization_round_trip():
    """Tests the to_dict and from_dict methods for a single Arc."""
    data = {
        "points": [
            {"id": 3, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 4, "x": 10.0, "y": 0.0, "fixed": False},
            {"id": 5, "x": 5.0, "y": 5.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 20,
                "type": "arc",
                "construction": True,
                "start_idx": 3,
                "end_idx": 4,
                "center_idx": 5,
                "clockwise": True,
            },
        ],
        "id_counter": 21,
    }

    registry = EntityRegistry.from_dict(data)
    assert registry.entity_to_dict(20) == {
        "id": 20,
        "type": "arc",
        "construction": True,
        "start_idx": 3,
        "end_idx": 4,
        "center_idx": 5,
        "clockwise": True,
    }

    reborn = EntityRegistry.from_dict(registry.to_dict())
    assert reborn.entity_type(20) == "arc"
    assert reborn.entity_to_dict(20) == registry.entity_to_dict(20)


def test_arc_get_point_ids(registry):
    """Tests that an arc correctly reports its defining point IDs."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(5, 5)
    arc = registry.add_arc(p1, p2, p3)
    assert set(registry.entity_point_ids(arc)) == {p1, p2, p3}


def test_arc_get_endpoint_ids(registry):
    """Tests that an arc correctly reports its endpoint IDs."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(5, 5)
    arc = registry.add_arc(p1, p2, p3)
    assert registry.entity_endpoint_ids(arc) == [p1, p2]
    assert set(registry.entity_point_ids(arc)) == {p1, p2, p3}


def test_arc_to_polygon_vertices(registry):
    """Tests that an arc linearizes into polygon vertices."""
    center = registry.add_point(0, 0)
    start = registry.add_point(10, 0)
    end = registry.add_point(0, 10)
    arc = registry.add_arc(start, end, center, clockwise=False)

    vertices = registry.to_polygon_vertices(arc, True)
    assert len(vertices) == 9
    assert vertices[0] == pytest.approx((10.0, 0.0))
    assert vertices[-1] == pytest.approx((0.0, 10.0))

    vertices_rev = registry.to_polygon_vertices(arc, False)
    assert len(vertices_rev) == 25
    assert vertices_rev[0] == pytest.approx((0.0, 10.0))
    assert vertices_rev[-1] == pytest.approx((10.0, 0.0))


def test_arc_get_junction_point_ids(registry):
    """Tests that an arc correctly reports its junction point IDs."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(5, 5)
    arc = registry.add_arc(p1, p2, p3)
    assert set(registry.junction_point_ids(arc)) == {p1, p2, p3}


def test_arc_hit_test(registry):
    """Tests Arc.hit_test method."""
    center = registry.add_point(0, 0)
    start = registry.add_point(10, 0)
    end = registry.add_point(0, 10)
    arc = registry.add_arc(start, end, center, clockwise=False)
    threshold = 2.0

    # Point on the arc (radius 10, 45 degrees)
    assert registry.hit_test(arc, 10, 0, threshold) is True
    assert registry.hit_test(arc, 0, 10, threshold) is True
    angle_45 = 10 * math.sqrt(2) / 2
    assert registry.hit_test(arc, angle_45, angle_45, threshold) is True

    # Point within threshold distance
    assert registry.hit_test(arc, 10 + 1, 0, threshold) is True
    assert registry.hit_test(arc, 10 - 1, 0, threshold) is True

    # Point outside threshold
    assert registry.hit_test(arc, 15, 0, threshold) is False
    assert registry.hit_test(arc, 5, 0, threshold) is False

    # Point outside arc sweep (CW arc would include this, CCW does not)
    assert registry.hit_test(arc, -10, 0, threshold) is False
    assert registry.hit_test(arc, 0, -10, threshold) is False


def test_arc_update_constrained_status(registry):
    """Test Arc.update_constrained_status logic."""
    s = registry.add_point(10, 0)
    e = registry.add_point(0, 10)
    c = registry.add_point(0, 0)
    aid = registry.add_arc(s, e, c)

    # Initial state: no defining point is constrained
    assert registry.point_constrained(s) is False
    assert registry.point_constrained(e) is False
    assert registry.point_constrained(c) is False
    registry.update_constrained_status()
    assert registry.is_constrained(aid) is False


def test_arc_get_midpoint(registry):
    """Test calculation of the arc's midpoint."""
    center = registry.add_point(0, 0)
    start = registry.add_point(10, 0)
    end = registry.add_point(-10, 0)

    # Counter-clockwise arc (upper semi-circle)
    aid_ccw = registry.add_arc(start, end, center, clockwise=False)
    vertices_ccw = registry.to_polygon_vertices(aid_ccw, True)
    mid_ccw = vertices_ccw[len(vertices_ccw) // 2]
    assert mid_ccw[0] == pytest.approx(0)
    assert mid_ccw[1] == pytest.approx(10)

    # Clockwise arc (lower semi-circle)
    aid_cw = registry.add_arc(start, end, center, clockwise=True)
    vertices_cw = registry.to_polygon_vertices(aid_cw, True)
    mid_cw = vertices_cw[len(vertices_cw) // 2]
    assert mid_cw[0] == pytest.approx(0)
    assert mid_cw[1] == pytest.approx(-10)


def test_arc_is_angle_within_sweep(registry):
    """Test checking if an angle is within the arc's sweep."""
    center = registry.add_point(0, 0)
    start = registry.add_point(10, 0)  # 0 degrees
    end = registry.add_point(0, 10)  # 90 degrees (pi/2)
    threshold = 0.5

    def point_at_angle(angle):
        return (10.0 * math.cos(angle), 10.0 * math.sin(angle))

    # Counter-clockwise from 0 to 90 degrees
    arc_ccw = registry.add_arc(start, end, center, clockwise=False)

    assert (
        registry.hit_test(arc_ccw, *point_at_angle(math.pi / 4), threshold)
        is True
    )  # 45 deg
    assert (
        registry.hit_test(arc_ccw, *point_at_angle(math.pi), threshold)
        is False
    )  # 180 deg
    assert (
        registry.hit_test(arc_ccw, *point_at_angle(0), threshold) is True
    )  # on start boundary
    assert (
        registry.hit_test(arc_ccw, *point_at_angle(math.pi / 2), threshold)
        is True
    )  # on end boundary

    # Clockwise from 0 to 90 (sweep is the long way around)
    arc_cw = registry.add_arc(start, end, center, clockwise=True)

    assert (
        registry.hit_test(arc_cw, *point_at_angle(math.pi / 4), threshold)
        is False
    )  # 45 deg
    assert (
        registry.hit_test(arc_cw, *point_at_angle(math.pi), threshold) is True
    )  # 180 deg
    assert (
        registry.hit_test(arc_cw, *point_at_angle(-math.pi / 2), threshold)
        is True
    )  # -90 deg


@pytest.fixture
def selection_setup(registry):
    """Fixture for setting up arc entities for selection tests."""
    rect = (20, 20, 80, 80)

    # Arc fully inside (semi-circle with radius 10)
    s_arc_in = registry.add_point(40, 50)
    e_arc_in = registry.add_point(60, 50)
    c_arc_in = registry.add_point(50, 50)
    arc_in = registry.add_arc(s_arc_in, e_arc_in, c_arc_in)

    # Arc intersecting
    s_arc_cross = registry.add_point(70, 50)
    e_arc_cross = registry.add_point(90, 50)
    c_arc_cross = registry.add_point(80, 50)
    arc_cross = registry.add_arc(s_arc_cross, e_arc_cross, c_arc_cross)

    # Arc outside
    s_arc_out = registry.add_point(0, 0)
    e_arc_out = registry.add_point(10, 0)
    c_arc_out = registry.add_point(5, 0)
    arc_out = registry.add_arc(s_arc_out, e_arc_out, c_arc_out)

    return (
        registry,
        rect,
        {
            "arc_in": arc_in,
            "arc_cross": arc_cross,
            "arc_out": arc_out,
        },
    )


def test_arc_is_contained_by(selection_setup):
    """Test the is_contained_by method for Arc entities."""
    registry, rect, entities = selection_setup
    assert registry.is_contained_by(entities["arc_in"], rect) is True
    assert registry.is_contained_by(entities["arc_cross"], rect) is False
    assert registry.is_contained_by(entities["arc_out"], rect) is False


def test_arc_intersects_rect(selection_setup):
    """Test of intersects_rect method for Arc entities."""
    registry, rect, entities = selection_setup
    assert registry.intersects_rect(entities["arc_in"], rect) is True
    assert registry.intersects_rect(entities["arc_cross"], rect) is True
    assert registry.intersects_rect(entities["arc_out"], rect) is False


def test_arc_to_geometry(registry):
    """Test Arc.to_geometry method."""
    center = registry.add_point(0, 0)
    start = registry.add_point(10, 0)
    end = registry.add_point(-10, 0)
    arc = registry.add_arc(start, end, center, clockwise=False)
    geo = registry.to_geometry(arc)
    assert isinstance(geo, Geometry)
    assert len(geo) == 2
    assert geo.data is not None
    assert isinstance(geo.data[0], Move)
    assert isinstance(geo.data[1], GeoArc)


def test_arc_append_to_geometry(registry):
    """Test Arc.append_to_geometry method."""
    center = registry.add_point(0, 0)
    start = registry.add_point(10, 0)
    end = registry.add_point(-10, 0)
    arc_ccw = registry.add_arc(start, end, center, clockwise=False)
    arc_cw = registry.add_arc(start, end, center, clockwise=True)

    sx, sy = registry.get_point(start)
    ex, ey = registry.get_point(end)

    geo = Geometry()
    geo.move_to(sx, sy)
    assert geo.data is not None

    registry.append_to_geometry(geo, arc_ccw, True)
    assert len(geo) == 2
    assert isinstance(geo.data[0], Move)
    assert isinstance(geo.data[1], GeoArc)
    assert geo.data[1].clockwise is False

    geo2 = Geometry()
    geo2.move_to(ex, ey)
    assert geo2.data is not None
    registry.append_to_geometry(geo2, arc_cw, True)
    assert len(geo2) == 2
    assert isinstance(geo2.data[0], Move)
    assert isinstance(geo2.data[1], GeoArc)
    assert geo2.data[1].clockwise is True

    geo3 = Geometry()
    geo3.move_to(sx, sy)
    assert geo3.data is not None
    registry.append_to_geometry(geo3, arc_ccw, False)
    assert len(geo3) == 2
    assert isinstance(geo3.data[0], Move)
    assert isinstance(geo3.data[1], GeoArc)
    assert geo3.data[1].clockwise is True


def test_arc_get_set_state(registry):
    """Test state capture and restoration for Undo/Redo."""
    s = registry.add_point(10, 0)
    e = registry.add_point(0, 10)
    c = registry.add_point(0, 0)
    aid = registry.add_arc(s, e, c, clockwise=False)

    # Verify initial state
    state = registry.get_state(aid)
    assert state == {"construction": False, "clockwise": False}

    # Modify state
    registry.set_state(aid, {"construction": True, "clockwise": True})
    assert registry.is_construction(aid) is True
    assert registry.get_state(aid) == {
        "construction": True,
        "clockwise": True,
    }

    # Restore state
    registry.set_state(aid, state)
    assert registry.is_construction(aid) is False
    assert registry.get_state(aid) == state
