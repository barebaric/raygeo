"""Entity tests for the sketch circle model."""

import pytest

from raygeo.geo import Arc as GeoArc
from raygeo.geo import Geometry, Move
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def registry():
    return EntityRegistry()


def test_circle_serialization_round_trip():
    """Tests the to_dict and from_dict methods for a single Circle."""
    data = {
        "points": [
            {"id": 6, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 7, "x": 10.0, "y": 0.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 30,
                "type": "circle",
                "construction": True,
                "center_idx": 6,
                "radius_pt_idx": 7,
            },
        ],
        "id_counter": 31,
    }

    registry = EntityRegistry.from_dict(data)
    assert registry.entity_to_dict(30) == {
        "id": 30,
        "type": "circle",
        "construction": True,
        "center_idx": 6,
        "radius_pt_idx": 7,
    }

    reborn = EntityRegistry.from_dict(registry.to_dict())
    assert reborn.entity_type(30) == "circle"
    assert reborn.entity_to_dict(30) == registry.entity_to_dict(30)


def test_circle_get_point_ids(registry):
    """Tests that a circle correctly reports its defining point IDs."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    circle = registry.add_circle(p1, p2)
    assert set(registry.entity_point_ids(circle)) == {p1, p2}


def test_circle_get_endpoint_ids(registry):
    """Tests that a circle has no endpoints (it's a closed loop)."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    circle = registry.add_circle(p1, p2)
    assert registry.entity_endpoint_ids(circle) == []


def test_circle_to_polygon_vertices(registry):
    """Tests that a circle returns empty list (handled specially for fills)."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    circle = registry.add_circle(p1, p2)
    assert registry.to_polygon_vertices(circle, True) == []


def test_circle_get_junction_point_ids(registry):
    """Tests that a circle reports its defining points as junctions."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    circle = registry.add_circle(p1, p2)
    assert sorted(registry.junction_point_ids(circle)) == sorted([p1, p2])


def test_circle_hit_test(registry):
    """Tests Circle.hit_test method."""
    center = registry.add_point(0, 0)
    radius_pt = registry.add_point(10, 0)
    circle = registry.add_circle(center, radius_pt)
    threshold = 2.0

    # Point on the circle (radius 10)
    assert registry.hit_test(circle, 10, 0, threshold) is True
    assert registry.hit_test(circle, -10, 0, threshold) is True
    assert registry.hit_test(circle, 0, 10, threshold) is True
    assert registry.hit_test(circle, 0, -10, threshold) is True

    # Point within threshold distance
    assert registry.hit_test(circle, 11, 0, threshold) is True
    assert registry.hit_test(circle, 9, 0, threshold) is True

    # Point outside threshold
    assert registry.hit_test(circle, 15, 0, threshold) is False
    assert registry.hit_test(circle, 5, 0, threshold) is False
    assert registry.hit_test(circle, 0, 0, threshold) is False


def test_circle_update_constrained_status(registry):
    """
    Test Circle.update_constrained_status logic.
    Circle requires center point constrained AND radius defined.
    """
    c = registry.add_point(0, 0)
    r = registry.add_point(10, 0)
    cid = registry.add_circle(c, r)

    # Case 1: Nothing constrained
    assert registry.point_constrained(c) is False
    assert registry.point_constrained(r) is False
    registry.update_constrained_status()
    assert registry.is_constrained(cid) is False

    # Case 5: Only a radius constraint exists, center is not constrained
    registry.update_constrained_status(radius_constrained_entity_ids=[cid])
    assert registry.is_constrained(cid) is False


def test_circle_get_midpoint(registry):
    """Test getting a point on the circle's circumference."""
    center = registry.add_point(5, 5)
    radius_pt_idx = registry.add_point(15, 5)

    cid = registry.add_circle(center, radius_pt_idx)

    geo = registry.to_geometry(cid)
    midpoint = geo.data[0].end
    radius_pt = registry.get_point(radius_pt_idx)

    assert midpoint is not None
    assert midpoint[:2] == radius_pt
    assert midpoint[:2] == (15.0, 5.0)


def test_circle_get_ignorable_unconstrained_points(registry):
    """
    Tests that a Circle correctly identifies its radius point as ignorable
    only when the circle itself is constrained.
    """
    center = registry.add_point(0, 0)
    radius_pt = registry.add_point(10, 0)
    cid = registry.add_circle(center, radius_pt)

    # When circle is not constrained, list should be empty
    registry.update_constrained_status()
    assert registry.ignorable_unconstrained_points(cid) == []


@pytest.fixture
def selection_setup(registry):
    """Fixture for setting up circle entities for selection tests."""
    rect = (20, 20, 80, 80)

    # Circle fully inside
    c_in = registry.add_point(50, 50)
    r_in = registry.add_point(60, 50)  # radius 10
    circle_in = registry.add_circle(c_in, r_in)

    # Circle intersecting
    c_cross = registry.add_point(85, 50)
    r_cross = registry.add_point(95, 50)  # radius 10
    circle_cross = registry.add_circle(c_cross, r_cross)

    # Circle outside
    c_out = registry.add_point(0, 0)
    r_out = registry.add_point(5, 0)  # radius 5
    circle_out = registry.add_circle(c_out, r_out)

    return (
        registry,
        rect,
        {
            "circle_in": circle_in,
            "circle_cross": circle_cross,
            "circle_out": circle_out,
        },
    )


def test_circle_is_contained_by(selection_setup):
    """Test the is_contained_by method for Circle entities."""
    registry, rect, entities = selection_setup
    assert registry.is_contained_by(entities["circle_in"], rect) is True
    assert registry.is_contained_by(entities["circle_cross"], rect) is False
    assert registry.is_contained_by(entities["circle_out"], rect) is False


def test_circle_intersects_rect(selection_setup):
    """Test of intersects_rect method for Circle entities."""
    registry, rect, entities = selection_setup
    assert registry.intersects_rect(entities["circle_in"], rect) is True
    assert registry.intersects_rect(entities["circle_cross"], rect) is True
    assert registry.intersects_rect(entities["circle_out"], rect) is False


def test_circle_to_geometry(registry):
    """Test Circle.to_geometry method."""
    center = registry.add_point(0, 0)
    radius_pt = registry.add_point(10, 0)
    circle = registry.add_circle(center, radius_pt)
    geo = registry.to_geometry(circle)
    assert isinstance(geo, Geometry)
    assert len(geo) == 3
    assert geo.data is not None
    assert isinstance(geo.data[0], Move)
    assert isinstance(geo.data[1], GeoArc)
    assert isinstance(geo.data[2], GeoArc)


def test_circle_create_fill_geometry(registry):
    """Test Circle.create_fill_geometry method."""
    center = registry.add_point(0, 0)
    radius_pt = registry.add_point(10, 0)
    circle = registry.add_circle(center, radius_pt)
    geo = registry.to_geometry(circle)
    assert isinstance(geo, Geometry)
    assert len(geo) == 3
    assert geo.data is not None
    assert isinstance(geo.data[0], Move)
    assert isinstance(geo.data[1], GeoArc)
    assert isinstance(geo.data[2], GeoArc)


def test_circle_get_set_state(registry):
    """Test state capture and restoration for Undo/Redo."""
    c = registry.add_point(0, 0)
    r = registry.add_point(10, 0)
    cid = registry.add_circle(c, r)

    # Verify initial state
    state = registry.get_state(cid)
    assert state == {"construction": False}

    # Modify state
    registry.set_construction(cid, True)

    # Restore state
    registry.set_state(cid, state)
    assert registry.is_construction(cid) is False
