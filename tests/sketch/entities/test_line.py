import pytest

from raygeo.geo import Geometry, Move
from raygeo.geo import Line as GeoLine
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def registry():
    return EntityRegistry()


def test_line_serialization_round_trip():
    """Tests the to_dict and from_dict methods for a single Line."""
    registry = EntityRegistry.from_dict(
        {
            "points": [
                {"id": 1, "x": 0.0, "y": 0.0, "fixed": False},
                {"id": 2, "x": 10.0, "y": 0.0, "fixed": False},
            ],
            "entities": [
                {
                    "id": 10,
                    "type": "line",
                    "construction": True,
                    "p1_idx": 1,
                    "p2_idx": 2,
                }
            ],
            "id_counter": 11,
        }
    )

    data = registry.entity_to_dict(10)
    assert data == {
        "id": 10,
        "type": "line",
        "construction": True,
        "p1_idx": 1,
        "p2_idx": 2,
    }

    new_registry = EntityRegistry.from_dict(registry.to_dict())
    assert new_registry.entity_type(10) == "line"
    assert new_registry.entity_point_ids(10) == [1, 2]
    assert new_registry.is_construction(10) is True


def test_line_get_point_ids(registry):
    """Tests that a line correctly reports its defining point IDs."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    line = registry.add_line(p1, p2)
    assert set(registry.entity_point_ids(line)) == {p1, p2}


def test_line_get_endpoint_ids(registry):
    """Tests that a line correctly reports its endpoint IDs."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    line = registry.add_line(p1, p2)
    assert registry.entity_endpoint_ids(line) == [p1, p2]


def test_line_to_polygon_vertices(registry):
    """Tests that a line returns its start point as polygon vertex."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 5)
    line = registry.add_line(p1, p2)

    vertices_forward = registry.to_polygon_vertices(line, True)
    assert len(vertices_forward) == 1
    assert vertices_forward[0] == (0.0, 0.0)

    vertices_backward = registry.to_polygon_vertices(line, False)
    assert len(vertices_backward) == 1
    assert vertices_backward[0] == (10.0, 5.0)


def test_line_get_junction_point_ids(registry):
    """Tests that a line correctly reports its junction point IDs."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    line = registry.add_line(p1, p2)
    assert set(registry.junction_point_ids(line)) == {p1, p2}


def test_line_hit_test(registry):
    """Tests Line.hit_test method."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 0)
    line = registry.add_line(p1, p2)
    threshold = 5.0

    # Point on the line
    assert registry.hit_test(line, 50, 0, threshold) is True
    assert registry.hit_test(line, 0, 0, threshold) is True
    assert registry.hit_test(line, 100, 0, threshold) is True

    # Point within threshold distance
    assert registry.hit_test(line, 50, 3, threshold) is True
    assert registry.hit_test(line, 50, -4, threshold) is True

    # Point outside threshold
    assert registry.hit_test(line, 50, 10, threshold) is False
    assert registry.hit_test(line, 50, -10, threshold) is False

    # Point beyond line segment endpoints
    assert registry.hit_test(line, -10, 0, threshold) is False
    assert registry.hit_test(line, 110, 0, threshold) is False


def test_line_update_constrained_status(registry):
    """Test Line.update_constrained_status logic."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 10)
    lid = registry.add_line(p1, p2)

    # Initially unconstrained
    assert registry.point_constrained(p1) is False
    assert registry.point_constrained(p2) is False
    registry.update_constrained_status([])
    assert registry.is_constrained(lid) is False


@pytest.fixture
def selection_setup(registry):
    """Fixture for setting up line entities for selection tests."""
    rect = (20, 20, 80, 80)

    # Line fully inside
    p_in1 = registry.add_point(30, 30)
    p_in2 = registry.add_point(70, 70)
    line_in = registry.add_line(p_in1, p_in2)

    # Line intersecting
    p_cross1 = registry.add_point(10, 50)
    p_cross2 = registry.add_point(90, 50)
    line_cross = registry.add_line(p_cross1, p_cross2)

    # Line outside
    p_out1 = registry.add_point(0, 0)
    p_out2 = registry.add_point(10, 10)
    line_out = registry.add_line(p_out1, p_out2)

    return (
        registry,
        rect,
        {
            "line_in": line_in,
            "line_cross": line_cross,
            "line_out": line_out,
        },
    )


def test_line_is_contained_by(selection_setup):
    """Test the is_contained_by method for Line entities."""
    registry, rect, entities = selection_setup
    assert registry.is_contained_by(entities["line_in"], rect) is True
    assert registry.is_contained_by(entities["line_cross"], rect) is False
    assert registry.is_contained_by(entities["line_out"], rect) is False


def test_line_intersects_rect(selection_setup):
    """Test of intersects_rect method for Line entities."""
    registry, rect, entities = selection_setup
    assert registry.intersects_rect(entities["line_in"], rect) is True
    assert registry.intersects_rect(entities["line_cross"], rect) is True
    assert registry.intersects_rect(entities["line_out"], rect) is False


def test_line_to_geometry(registry):
    """Test Line.to_geometry method."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    line = registry.add_line(p1, p2)
    geo = registry.to_geometry(line)
    assert isinstance(geo, Geometry)
    assert len(geo) == 2
    assert geo.data is not None
    assert isinstance(geo.data[0], Move)
    assert isinstance(geo.data[1], GeoLine)


def test_line_append_to_geometry(registry):
    """Test Line.append_to_geometry method."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    line = registry.add_line(p1, p2)
    pt1 = registry.get_point(p1)
    pt2 = registry.get_point(p2)
    geo = Geometry()
    geo.move_to(pt1[0], pt1[1])

    registry.append_to_geometry(geo, line, True)
    assert len(geo) == 2
    assert geo.data is not None
    assert isinstance(geo.data[0], Move)
    assert isinstance(geo.data[1], GeoLine)

    geo2 = Geometry()
    geo2.move_to(pt2[0], pt2[1])
    registry.append_to_geometry(geo2, line, False)
    assert geo2.data is not None
    assert len(geo2) == 2
    assert isinstance(geo2.data[0], Move)
    assert isinstance(geo2.data[1], GeoLine)


def test_line_get_set_state(registry):
    """Test state capture and restoration for Undo/Redo."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 10)
    lid = registry.add_line(p1, p2)

    # Verify initial state
    state = registry.get_state(lid)
    assert state == {"construction": False}

    # Modify state
    registry.set_construction(lid, True)

    # Restore state
    registry.set_state(lid, state)
    assert registry.is_construction(lid) is False
