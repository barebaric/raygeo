from raygeo.sketch.registry import EntityRegistry


def _connected_bezier_ids(registry, point_id):
    """Registry equivalent of Point.get_connected_beziers."""
    return [
        entity_id
        for entity_id in registry.entity_ids()
        if registry.entity_type(entity_id) == "bezier"
        and point_id in registry.entity_point_ids(entity_id)
    ]


def _paired_bezier_ids(registry, point_id):
    """Registry equivalent of Point.get_paired_beziers."""
    connected = _connected_bezier_ids(registry, point_id)
    first = connected[0] if len(connected) > 0 else None
    second = connected[1] if len(connected) > 1 else None
    return first, second


def test_point_instantiation():
    """Tests basic point creation and attribute access."""
    registry = EntityRegistry()
    pid = registry.add_point(10.5, -20.0, fixed=True)
    assert pid == 0
    assert registry.get_point(pid) == (10.5, -20.0)
    assert registry.point_constrained(pid) is False
    assert registry.to_dict()["points"][0]["fixed"] is True


def test_point_serialization_round_trip():
    """Tests the to_dict and from_dict methods for a Point."""
    registry = EntityRegistry.from_dict(
        {
            "points": [{"id": 1, "x": 1.2, "y": 3.4, "fixed": True}],
            "entities": [],
            "id_counter": 2,
        }
    )
    data = registry.to_dict()["points"][0]
    assert data == {"id": 1, "x": 1.2, "y": 3.4, "fixed": True}

    new_registry = EntityRegistry.from_dict(registry.to_dict())
    assert new_registry.get_point(1) == (1.2, 3.4)
    assert new_registry.to_dict()["points"][0] == {
        "id": 1,
        "x": 1.2,
        "y": 3.4,
        "fixed": True,
    }


def test_point_is_in_rect():
    """Tests the Point.is_in_rect method."""
    registry = EntityRegistry()
    pt_inside = registry.add_point(5, 5)
    pt_outside = registry.add_point(15, 15)
    pt_on_edge = registry.add_point(10, 5)
    rect = (0, 0, 10, 10)  # min_x, min_y, max_x, max_y

    inside = registry.add_line(pt_inside, pt_inside)
    outside = registry.add_line(pt_outside, pt_outside)
    on_edge = registry.add_line(pt_on_edge, pt_on_edge)

    # Edges are inclusive
    assert registry.is_contained_by(inside, rect) is True
    assert registry.is_contained_by(outside, rect) is False
    assert registry.is_contained_by(on_edge, rect) is True


def test_point_get_connected_beziers():
    """Tests finding beziers connected to a point."""
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(20, 0)

    b1_id = registry.add_bezier(p1, p2)
    b2_id = registry.add_bezier(p2, p3)

    connected_to_p1 = _connected_bezier_ids(registry, p1)
    assert len(connected_to_p1) == 1
    assert connected_to_p1[0] == b1_id

    connected_to_p2 = _connected_bezier_ids(registry, p2)
    assert len(connected_to_p2) == 2
    assert set(connected_to_p2) == {b1_id, b2_id}

    connected_to_p3 = _connected_bezier_ids(registry, p3)
    assert len(connected_to_p3) == 1
    assert connected_to_p3[0] == b2_id


def test_point_get_paired_beziers():
    """Tests getting paired beziers (first two connected)."""
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(20, 0)

    b1_id = registry.add_bezier(p1, p2)
    b2_id = registry.add_bezier(p2, p3)

    b1, b2 = _paired_bezier_ids(registry, p2)
    assert b1 is not None
    assert b2 is not None
    assert b1 == b1_id
    assert b2 == b2_id


def test_point_paired_beziers_single():
    """Tests paired beziers when only one is connected."""
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)

    registry.add_bezier(p1, p2)

    b1, b2 = _paired_bezier_ids(registry, p1)
    assert b1 is not None
    assert b2 is None


def test_point_apply_constraint_symmetric():
    """Tests symmetric waypoint type: mirrored control points."""
    registry = EntityRegistry.from_dict(
        {
            "points": [
                {"id": 0, "x": 0.0, "y": 0.0, "fixed": False},
                {
                    "id": 1,
                    "x": 10.0,
                    "y": 10.0,
                    "fixed": False,
                    "waypoint_type": "symmetric",
                },
                {"id": 2, "x": 20.0, "y": 0.0, "fixed": False},
            ],
            "entities": [
                {
                    "id": 3,
                    "type": "bezier",
                    "construction": False,
                    "start_idx": 0,
                    "end_idx": 1,
                    "cp2_dx": 5.0,
                    "cp2_dy": 3.0,
                },
                {
                    "id": 4,
                    "type": "bezier",
                    "construction": False,
                    "start_idx": 1,
                    "end_idx": 2,
                },
            ],
            "id_counter": 5,
        }
    )
    assert registry.entity_type(3) == "bezier"
    assert registry.entity_type(4) == "bezier"
    assert registry.waypoint_type(1) == "symmetric"
    assert registry.get_state(3)["cp2"] == [5.0, 3.0]


def test_point_apply_constraint_smooth():
    """Tests smooth waypoint type: collinear with preserved length."""
    registry = EntityRegistry.from_dict(
        {
            "points": [
                {"id": 0, "x": 0.0, "y": 0.0, "fixed": False},
                {
                    "id": 1,
                    "x": 10.0,
                    "y": 10.0,
                    "fixed": False,
                    "waypoint_type": "smooth",
                },
                {"id": 2, "x": 20.0, "y": 0.0, "fixed": False},
            ],
            "entities": [
                {
                    "id": 3,
                    "type": "bezier",
                    "construction": False,
                    "start_idx": 0,
                    "end_idx": 1,
                    "cp2_dx": 6.0,
                    "cp2_dy": 8.0,
                },
                {
                    "id": 4,
                    "type": "bezier",
                    "construction": False,
                    "start_idx": 1,
                    "end_idx": 2,
                    "cp1_dx": 2.0,
                    "cp1_dy": 4.0,
                },
            ],
            "id_counter": 5,
        }
    )
    assert registry.entity_type(3) == "bezier"
    assert registry.entity_type(4) == "bezier"
    assert registry.waypoint_type(1) == "smooth"
    assert registry.get_state(3)["cp2"] == [6.0, 8.0]
    assert registry.get_state(4)["cp1"] == [2.0, 4.0]


def test_point_apply_constraint_sharp_noop():
    """Tests that SHARP waypoint type doesn't apply constraints."""
    registry = EntityRegistry.from_dict(
        {
            "points": [
                {"id": 0, "x": 0.0, "y": 0.0, "fixed": False},
                {
                    "id": 1,
                    "x": 10.0,
                    "y": 10.0,
                    "fixed": False,
                    "waypoint_type": "sharp",
                },
                {"id": 2, "x": 20.0, "y": 0.0, "fixed": False},
            ],
            "entities": [
                {
                    "id": 3,
                    "type": "bezier",
                    "construction": False,
                    "start_idx": 0,
                    "end_idx": 1,
                    "cp2_dx": 5.0,
                    "cp2_dy": 3.0,
                },
                {
                    "id": 4,
                    "type": "bezier",
                    "construction": False,
                    "start_idx": 1,
                    "end_idx": 2,
                },
            ],
            "id_counter": 5,
        }
    )
    assert registry.entity_type(3) == "bezier"
    assert registry.entity_type(4) == "bezier"
    assert registry.waypoint_type(1) == "sharp"
    assert registry.get_state(3)["cp2"] == [5.0, 3.0]
    assert registry.get_state(4).get("cp1") is None
