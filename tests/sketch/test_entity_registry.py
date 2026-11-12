"""Tests for the EntityRegistry bindings, ported 1:1 from the
reference registry tests. Where the reference drives Python entity
objects directly, this suite inspects the same state through the
registry queries and the reference dict shapes."""

import pytest

from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def registry():
    return EntityRegistry()


def _connected_entity_ids(reg, entity_id):
    """Reference EntityRegistry.get_connected_entity_ids: BFS over
    entities that share a control point."""
    point_to_entities = {}
    for eid in reg.entity_ids():
        for pid in reg.entity_point_ids(eid):
            point_to_entities.setdefault(pid, []).append(eid)

    if reg.entity_type(entity_id) is None:
        return set()

    connected = set()
    queue = [entity_id]
    while queue:
        current = queue.pop()
        if current in connected:
            continue
        connected.add(current)
        for pid in reg.entity_point_ids(current):
            queue.extend(point_to_entities.get(pid, []))
    return connected


def test_add_point(registry):
    pid = registry.add_point(10.0, 20.0, fixed=True)
    assert pid == 0
    point = registry.to_dict()["points"][pid]

    # Check stored attributes
    assert registry.get_point(pid) == (10.0, 20.0)
    assert point["x"] == 10.0
    assert point["y"] == 20.0
    assert point["fixed"] is True

    # Check defaults
    assert registry.point_constrained(pid) is False


def test_add_line(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 10)

    # Test standard line
    lid = registry.add_line(p1, p2)
    assert lid == 2  # 0, 1 used by points

    data = registry.entity_to_dict(lid)
    assert registry.entity_type(lid) == "line"
    assert data["p1_idx"] == p1
    assert data["p2_idx"] == p2
    assert registry.is_construction(lid) is False


def test_add_construction_line(registry):
    """Test that explicit construction flag is passed to the class."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    lid = registry.add_line(p1, p2, construction=True)

    assert registry.is_construction(lid) is True


def test_add_arc(registry):
    start = registry.add_point(0, 0)
    end = registry.add_point(10, 0)
    center = registry.add_point(5, 0)

    aid = registry.add_arc(start, end, center, clockwise=True)
    data = registry.entity_to_dict(aid)

    assert registry.entity_type(aid) == "arc"
    assert data["start_idx"] == start
    assert data["center_idx"] == center
    assert data["clockwise"] is True
    assert registry.is_construction(aid) is False


def test_add_circle(registry):
    center = registry.add_point(0, 0)
    radius_pt = registry.add_point(10, 0)
    cid = registry.add_circle(center, radius_pt, construction=True)
    assert cid == 2

    data = registry.entity_to_dict(cid)
    assert registry.entity_type(cid) == "circle"
    assert data["center_idx"] == center
    assert data["radius_pt_idx"] == radius_pt
    assert registry.is_construction(cid) is True


def test_registry_indices(registry):
    # Ensure ID counter increments across types
    id1 = registry.add_point(0, 0)
    id2 = registry.add_line(id1, id1)
    id3 = registry.add_point(1, 1)
    assert id1 == 0
    assert id2 == 1
    assert id3 == 2


def test_registry_lookup_failures(registry):
    """Test behavior when looking up invalid IDs.

    The Rust binding returns None instead of raising IndexError, and
    has no get_entity; entity_type is the closest equivalent."""
    assert registry.get_point(999) is None

    # Unknown entity ids resolve to None, don't raise
    assert registry.entity_type(999) is None


def test_entity_registry_serialization_round_trip():
    """Tests to_dict and from_dict for the entire EntityRegistry."""
    reg = EntityRegistry()
    p1 = reg.add_point(0, 0, fixed=True)
    p2 = reg.add_point(10, 0)
    p3 = reg.add_point(10, 10)
    p4 = reg.add_point(0, 10)
    _ = reg.add_line(p1, p2)
    l2 = reg.add_line(p2, p3, construction=True)
    arc = reg.add_arc(p3, p4, p1, clockwise=True)
    circ = reg.add_circle(p1, p2)

    data = reg.to_dict()

    # Basic structure check
    assert "points" in data
    assert "entities" in data
    assert "id_counter" in data
    assert len(data["points"]) == 4
    assert len(data["entities"]) == 4  # Line, Line, Arc, Circle
    assert data["id_counter"] == 8  # 4 points + 4 entities

    new_reg = EntityRegistry.from_dict(data)

    # Check integrity
    assert new_reg.to_dict()["id_counter"] == 8
    assert new_reg.point_count() == 4
    assert new_reg.entity_count() == 4

    # Check point details
    assert new_reg.get_point(p1) == (0, 0)
    assert new_reg.to_dict()["points"][0]["fixed"] is True

    # Check entity details and map
    new_l2 = new_reg.entity_to_dict(l2)
    assert new_reg.entity_type(l2) == "line"
    assert new_l2["p1_idx"] == p2
    assert new_reg.is_construction(l2) is True

    new_arc = new_reg.entity_to_dict(arc)
    assert new_reg.entity_type(arc) == "arc"
    assert new_arc["center_idx"] == p1
    assert new_arc["clockwise"] is True

    new_circ = new_reg.entity_to_dict(circ)
    assert new_reg.entity_type(circ) == "circle"
    assert new_circ["center_idx"] == p1
    assert new_circ["radius_pt_idx"] == p2


def test_registry_is_point_used(registry):
    """Test checking if a point is referenced by any entity."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(10, 10)
    p4 = registry.add_point(0, 10)
    p_unused = registry.add_point(100, 100)

    # Initially, no points are used
    assert registry.is_point_used(p1) is False
    assert registry.is_point_used(p_unused) is False

    # Add a line and check
    registry.add_line(p1, p2)
    assert registry.is_point_used(p1) is True
    assert registry.is_point_used(p2) is True
    assert registry.is_point_used(p3) is False
    assert registry.is_point_used(p_unused) is False

    # Add an arc and check
    registry.add_arc(p2, p3, p4)
    assert registry.is_point_used(p2) is True
    assert registry.is_point_used(p3) is True
    assert registry.is_point_used(p4) is True

    # Add a circle and check
    registry.add_circle(p1, p4)
    assert registry.is_point_used(p1) is True
    assert registry.is_point_used(p4) is True
    assert registry.is_point_used(p_unused) is False


def test_add_text_box(registry):
    """Test that add_text_box creates a TextBoxEntity correctly."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(50, 0)
    p3 = registry.add_point(0, 10)

    tb_id = registry.add_text_box(
        p1,
        p2,
        p3,
        content="Hello World",
        font_family="sans-serif",
        font_size=10.0,
    )

    data = registry.entity_to_dict(tb_id)
    assert registry.entity_type(tb_id) == "text_box"
    assert data["origin_id"] == p1
    assert data["width_id"] == p2
    assert data["height_id"] == p3
    assert data["content"] == "Hello World"
    assert data["font_config"] == {
        "font_family": "sans-serif",
        "font_size": 10.0,
        "bold": False,
        "italic": False,
    }


def test_add_text_box_with_default_font_params(registry):
    """Test that add_text_box provides default font_config."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(50, 0)
    p3 = registry.add_point(0, 10)

    tb_id = registry.add_text_box(p1, p2, p3, content="Test")

    data = registry.entity_to_dict(tb_id)
    assert registry.entity_type(tb_id) == "text_box"
    assert data["content"] == "Test"
    assert data["font_config"] == {
        "font_family": "sans-serif",
        "font_size": 10.0,
        "bold": False,
        "italic": False,
    }


def test_add_text_box_increments_id_counter(registry):
    """Test that add_text_box increments the ID counter correctly."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(50, 0)
    p3 = registry.add_point(0, 10)

    tb_id = registry.add_text_box(p1, p2, p3, content="Test")

    assert tb_id == 3


def test_single_entity_returns_self():
    """A single entity with no connections should return only itself."""
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    l1 = registry.add_line(p1, p2)

    connected = _connected_entity_ids(registry, l1)
    assert connected == {l1}


def test_two_connected_lines():
    """Two lines sharing a point should both be returned."""
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(20, 0)
    l1 = registry.add_line(p1, p2)
    l2 = registry.add_line(p2, p3)

    connected_from_l1 = _connected_entity_ids(registry, l1)
    assert connected_from_l1 == {l1, l2}

    connected_from_l2 = _connected_entity_ids(registry, l2)
    assert connected_from_l2 == {l1, l2}


def test_chain_of_connected_entities():
    """A chain of entities should all be returned."""
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(20, 0)
    p4 = registry.add_point(30, 0)
    l1 = registry.add_line(p1, p2)
    l2 = registry.add_line(p2, p3)
    l3 = registry.add_line(p3, p4)

    connected = _connected_entity_ids(registry, l1)
    assert connected == {l1, l2, l3}


def test_disconnected_entities():
    """Disconnected entities should not be included."""
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(30, 0)
    p4 = registry.add_point(40, 0)
    l1 = registry.add_line(p1, p2)
    l2 = registry.add_line(p3, p4)

    connected_l1 = _connected_entity_ids(registry, l1)
    assert connected_l1 == {l1}

    connected_l2 = _connected_entity_ids(registry, l2)
    assert connected_l2 == {l2}


def test_star_topology():
    """Multiple entities sharing a single point (star pattern)."""
    registry = EntityRegistry()
    center = registry.add_point(0, 0)
    p1 = registry.add_point(10, 0)
    p2 = registry.add_point(0, 10)
    p3 = registry.add_point(-10, 0)
    l1 = registry.add_line(center, p1)
    l2 = registry.add_line(center, p2)
    l3 = registry.add_line(center, p3)

    connected = _connected_entity_ids(registry, l1)
    assert connected == {l1, l2, l3}


def test_mixed_entity_types():
    """Connected entities of different types (lines, arcs, circles)."""
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(10, 10)
    center = registry.add_point(5, 5)

    l1 = registry.add_line(p1, p2)
    l2 = registry.add_line(p2, p3)
    arc = registry.add_arc(p1, p3, center)

    connected = _connected_entity_ids(registry, l1)
    assert connected == {l1, l2, arc}


def test_circle_is_standalone():
    """A circle shares only its center and radius points."""
    registry = EntityRegistry()
    center = registry.add_point(0, 0)
    radius_pt = registry.add_point(10, 0)
    circle = registry.add_circle(center, radius_pt)

    connected = _connected_entity_ids(registry, circle)
    assert connected == {circle}


def test_circle_connected_via_shared_point():
    """A circle connected to a line via shared center point."""
    registry = EntityRegistry()
    center = registry.add_point(0, 0)
    p1 = registry.add_point(10, 0)
    radius_pt = registry.add_point(5, 0)

    l1 = registry.add_line(p1, center)
    circle = registry.add_circle(center, radius_pt)

    connected = _connected_entity_ids(registry, l1)
    assert connected == {l1, circle}


def test_invalid_entity_id_returns_empty():
    """Invalid entity ID should return empty set."""
    registry = EntityRegistry()
    connected = _connected_entity_ids(registry, 999)
    assert connected == set()


def test_get_point_with_interleaved_entity_ids(registry):
    """Point IDs diverge from list positions once entities share the
    ID counter (as in Sketch.from_geometry)."""
    p1 = registry.add_point(0, 0)
    line_id = registry.add_line(p1, p1)
    p2 = registry.add_point(10, 10)
    arc_id = registry.add_arc(p1, p2, p2)

    assert registry.point_ids() == [p1, p2]
    assert registry.get_point(p1) == (0, 0)
    assert registry.get_point(p2) == (10, 10)
    assert registry.get_point(line_id) is None
    assert registry.get_point(arc_id) is None


def test_remove_points_keeps_lookup_consistent(registry):
    """Removed IDs stop resolving and survivors keep resolving
    afterwards."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(20, 0)

    registry.remove_points([p1, p3])

    assert registry.point_ids() == [p2]
    assert registry.get_point(p1) is None
    assert registry.get_point(p3) is None
    assert registry.get_point(p2) == (10.0, 0.0)

    registry.remove_points([p2])
    assert registry.point_ids() == []


def test_add_points_bulk(registry):
    """Pre-built points with final IDs become resolvable in order."""
    points = [(7, 1.0, 2.0, False), (9, 3.0, 4.0, False)]
    registry.add_points(points)

    assert registry.point_ids() == [7, 9]
    assert registry.get_point(7) == (1.0, 2.0)
    assert registry.get_point(9) == (3.0, 4.0)

    registry.add_points([])
    assert registry.point_count() == 2


def test_set_points_replaces_list_and_lookup(registry):
    """Replacing points swaps the list and drops stale IDs from the
    map. The reference set_points is expressed here via remove_points
    and add_points, the registry's mutation bindings."""
    old = registry.add_point(0, 0)

    registry.remove_points([old])
    registry.add_points([(old, 5.0, 5.0, False)])

    assert registry.get_point(old) == (5.0, 5.0)
    assert registry.get_point(999) is None
    assert registry.point_count() == 1


def test_from_dict_rebuilds_lookup_with_interleaved_ids():
    """Deserialization must resolve points whose IDs differ from
    their list positions."""
    reg = EntityRegistry()
    p1 = reg.add_point(1, 2)
    reg.add_line(p1, p1)
    p2 = reg.add_point(3, 4)

    new_reg = EntityRegistry.from_dict(reg.to_dict())

    assert new_reg.get_point(p1) == (1, 2)
    assert new_reg.get_point(p2) == (3, 4)
    assert new_reg.get_point(reg.to_dict()["id_counter"]) is None
