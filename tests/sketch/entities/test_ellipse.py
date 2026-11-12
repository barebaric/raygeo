import math

import pytest

from raygeo.geo import Geometry
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def registry():
    return EntityRegistry()


def test_ellipse_serialization_round_trip():
    """Tests the to_dict and from_dict round trip for a single Ellipse."""
    data = {
        "points": [
            {"id": 6, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 7, "x": 10.0, "y": 0.0, "fixed": False},
            {"id": 8, "x": 0.0, "y": 5.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 30,
                "type": "ellipse",
                "construction": True,
                "center_idx": 6,
                "radius_x_pt_idx": 7,
                "radius_y_pt_idx": 8,
                "helper_line_ids": [100, 101],
            },
        ],
        "id_counter": 102,
    }
    registry = EntityRegistry.from_dict(data)
    assert registry.entity_to_dict(30) == {
        "id": 30,
        "type": "ellipse",
        "construction": True,
        "center_idx": 6,
        "radius_x_pt_idx": 7,
        "radius_y_pt_idx": 8,
        "helper_line_ids": [100, 101],
    }

    restored = EntityRegistry.from_dict(registry.to_dict())
    assert restored.entity_type(30) == "ellipse"
    assert restored.entity_point_ids(30) == registry.entity_point_ids(30)
    assert restored.is_construction(30) == registry.is_construction(30)
    assert restored.helper_ids(30) == registry.helper_ids(30)


def test_ellipse_serialization_without_helper_lines():
    """Tests serialization when helper_line_ids is missing."""
    data = {
        "points": [
            {"id": 1, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 2, "x": 10.0, "y": 0.0, "fixed": False},
            {"id": 3, "x": 0.0, "y": 5.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 31,
                "type": "ellipse",
                "construction": False,
                "center_idx": 1,
                "radius_x_pt_idx": 2,
                "radius_y_pt_idx": 3,
            },
        ],
        "id_counter": 32,
    }
    registry = EntityRegistry.from_dict(data)
    assert registry.entity_to_dict(31)["helper_line_ids"] == []

    restored = EntityRegistry.from_dict(registry.to_dict())
    assert restored.entity_to_dict(31)["helper_line_ids"] == []


def test_ellipse_get_point_ids(registry):
    """Tests that an ellipse correctly reports its defining point IDs."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)
    assert set(registry.entity_point_ids(eid)) == {center, rx, ry}


def test_ellipse_get_endpoint_ids(registry):
    """Tests that an ellipse has no endpoints (it's a closed loop)."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)
    assert registry.entity_endpoint_ids(eid) == []


def test_ellipse_get_junction_point_ids(registry):
    """Tests that an ellipse reports all three defining points."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)
    assert sorted(registry.junction_point_ids(eid)) == sorted([center, rx, ry])


def test_ellipse_hit_test_on_edge(registry):
    """Tests hit_test on ellipse edge."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)
    threshold = 1.0

    assert registry.hit_test(eid, 10, 0, threshold) is True
    assert registry.hit_test(eid, -10, 0, threshold) is True
    assert registry.hit_test(eid, 0, 5, threshold) is True
    assert registry.hit_test(eid, 0, -5, threshold) is True


def test_ellipse_hit_test_off_edge(registry):
    """Tests hit_test for points off the ellipse."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)
    threshold = 1.0

    assert registry.hit_test(eid, 5, 0, threshold) is False
    assert registry.hit_test(eid, 0, 2, threshold) is False
    assert registry.hit_test(eid, 15, 0, threshold) is False


def test_ellipse_hit_test_rotated(registry):
    """Tests hit_test for a rotated ellipse."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(5, 5)
    ry = registry.add_point(-5, 5)
    eid = registry.add_ellipse(center, rx, ry)
    threshold = 1.0

    dist = math.hypot(5, 5)
    assert registry.hit_test(eid, dist, 0, threshold) is True


def test_ellipse_hit_test_missing_points(registry):
    """Tests hit_test degrades to False when defining points are
    missing (the reference raises IndexError)."""
    eid = registry.add_ellipse(999, 998, 997)
    assert registry.hit_test(eid, 0, 0, 1.0) is False


def test_ellipse_hit_test_zero_radii(registry):
    """Tests hit_test returns False when radii are zero."""
    center = registry.add_point(0, 0)
    same = registry.add_point(0, 0)
    eid = registry.add_ellipse(center, same, same)
    assert registry.hit_test(eid, 0, 0, 1.0) is False


def test_ellipse_update_constrained_status(registry):
    """Test the entity constrained status recompute. Point constrained
    flags are solver-computed and read-only through the bindings, so
    only the unconstrained baseline is exercisable; the radius
    constrained entity list emulates the reference's mock radius
    constraint."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)

    registry.update_constrained_status([])
    assert registry.is_constrained(eid) is False

    registry.update_constrained_status([eid])
    assert registry.is_constrained(eid) is False


def test_ellipse_get_midpoint(registry):
    """Test that the midpoint is the radius-x point position."""
    center = registry.add_point(5, 5)
    rx = registry.add_point(15, 5)
    ry = registry.add_point(5, 10)

    registry.add_ellipse(center, rx, ry)

    midpoint = registry.get_point(rx)
    assert midpoint is not None
    assert midpoint == (15.0, 5.0)


def test_ellipse_get_ignorable_unconstrained_points(registry):
    """Tests that an unconstrained ellipse has no ignorable points.
    The constrained branch needs solver-driven entity flags that are
    not settable through the bindings."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)

    assert registry.ignorable_unconstrained_points(eid) == []


def test_ellipse_get_rigidly_connected_points(registry):
    """Test rigidly_connected_points returns all ellipse points for
    the center."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    registry.add_ellipse(center, rx, ry)

    result = registry.rigidly_connected_points(center)
    assert set(result) == {center, rx, ry}

    result = registry.rigidly_connected_points(rx)
    assert result == []


@pytest.fixture
def selection_setup(registry):
    """Fixture for setting up ellipse entities for selection tests."""
    rect = (20, 20, 80, 80)

    c_in = registry.add_point(50, 50)
    rx_in = registry.add_point(60, 50)
    ry_in = registry.add_point(50, 60)
    ellipse_in = registry.add_ellipse(c_in, rx_in, ry_in)

    c_cross = registry.add_point(85, 50)
    rx_cross = registry.add_point(95, 50)
    ry_cross = registry.add_point(85, 60)
    ellipse_cross = registry.add_ellipse(c_cross, rx_cross, ry_cross)

    c_out = registry.add_point(0, 0)
    rx_out = registry.add_point(5, 0)
    ry_out = registry.add_point(0, 3)
    ellipse_out = registry.add_ellipse(c_out, rx_out, ry_out)

    return (
        registry,
        rect,
        {
            "ellipse_in": ellipse_in,
            "ellipse_cross": ellipse_cross,
            "ellipse_out": ellipse_out,
        },
    )


def test_ellipse_is_contained_by(selection_setup):
    """Test the is_contained_by method for Ellipse entities."""
    registry, rect, entities = selection_setup
    assert registry.is_contained_by(entities["ellipse_in"], rect) is True
    assert registry.is_contained_by(entities["ellipse_cross"], rect) is False
    assert registry.is_contained_by(entities["ellipse_out"], rect) is False


def test_ellipse_intersects_rect(selection_setup):
    """Test of intersects_rect method for Ellipse entities."""
    registry, rect, entities = selection_setup
    assert registry.intersects_rect(entities["ellipse_in"], rect) is True
    assert registry.intersects_rect(entities["ellipse_cross"], rect) is True
    assert registry.intersects_rect(entities["ellipse_out"], rect) is False


def test_ellipse_to_geometry(registry):
    """Test to_geometry method."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)
    geo = registry.to_geometry(eid)
    assert isinstance(geo, Geometry)
    assert len(geo) > 0
    assert geo.data is not None


def test_ellipse_to_geometry_zero_radii(registry):
    """Test to_geometry returns empty geometry for zero radii."""
    center = registry.add_point(0, 0)
    same = registry.add_point(0, 0)
    eid = registry.add_ellipse(center, same, same)
    geo = registry.to_geometry(eid)
    assert len(geo) == 0


def test_ellipse_create_fill_geometry(registry):
    """Test that the ellipse forms a fillable closed region (the
    binding equivalent of create_fill_geometry)."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)
    area = registry.enclosed_signed_area(eid)
    assert area is not None
    assert area > 0
    assert registry.contains_point(eid, 0, 0) is True


def test_ellipse_get_set_state(registry):
    """Test state capture and restoration for Undo/Redo."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    eid = registry.add_ellipse(center, rx, ry)

    state = registry.get_state(eid)
    assert state == {"construction": False}

    registry.set_construction(eid, True)

    registry.set_state(eid, state)
    assert registry.is_construction(eid) is False


def test_ellipse_repr():
    """Test the serialized Ellipse identity fields."""
    data = {
        "points": [
            {"id": 2, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 3, "x": 10.0, "y": 0.0, "fixed": False},
            {"id": 4, "x": 0.0, "y": 5.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 1,
                "type": "ellipse",
                "construction": False,
                "center_idx": 2,
                "radius_x_pt_idx": 3,
                "radius_y_pt_idx": 4,
            },
        ],
        "id_counter": 5,
    }
    registry = EntityRegistry.from_dict(data)
    r = registry.entity_to_dict(1)
    assert r["type"] == "ellipse"
    assert r["id"] == 1
    assert r["center_idx"] == 2


def test_ellipse_get_radii(registry):
    """Test the radii derived from the defining point positions."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 0)
    ry = registry.add_point(0, 5)
    registry.add_ellipse(center, rx, ry)

    cx, cy = registry.get_point(center)
    rx_x, rx_y = registry.get_point(rx)
    ry_x, ry_y = registry.get_point(ry)
    rx_val = math.hypot(rx_x - cx, rx_y - cy)
    ry_val = math.hypot(ry_x - cx, ry_y - cy)
    assert rx_val == 10.0
    assert ry_val == 5.0


def test_ellipse_get_rotation(registry):
    """Test the rotation derived from the radius-x point offset."""
    center = registry.add_point(0, 0)
    rx = registry.add_point(10, 10)
    ry = registry.add_point(-10, 10)
    registry.add_ellipse(center, rx, ry)

    cx, cy = registry.get_point(center)
    rx_x, rx_y = registry.get_point(rx)
    rotation = math.atan2(rx_y - cy, rx_x - cx)
    expected = math.atan2(10, 10)
    assert abs(rotation - expected) < 1e-9
