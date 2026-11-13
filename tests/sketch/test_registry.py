"""Round-trip tests for the sketch data model against the reference
registry dictionary shapes (`.ryp` compatible)."""

from raygeo.sketch.registry import EntityRegistry


def _rect_registry_dict():
    """Registry dict shaped exactly like the Python EntityRegistry
    produces for a constrained 4-line square."""
    return {
        "points": [
            {"id": 0, "x": 0.0, "y": 0.0, "fixed": True},
            {"id": 1, "x": 100.0, "y": 0.0, "fixed": False},
            {"id": 2, "x": 100.0, "y": 50.0, "fixed": False},
            {
                "id": 3,
                "x": 0.0,
                "y": 50.0,
                "fixed": False,
                "waypoint_type": "smooth",
            },
        ],
        "entities": [
            {
                "id": 4,
                "type": "line",
                "construction": False,
                "p1_idx": 0,
                "p2_idx": 1,
            },
            {
                "id": 5,
                "type": "arc",
                "construction": False,
                "start_idx": 1,
                "end_idx": 2,
                "center_idx": 3,
                "clockwise": True,
                "invisible": True,
            },
            {
                "id": 6,
                "type": "circle",
                "construction": True,
                "center_idx": 0,
                "radius_pt_idx": 1,
                "array_copy": True,
            },
            {
                "id": 7,
                "type": "bezier",
                "construction": False,
                "start_idx": 2,
                "end_idx": 3,
                "cp1_dx": 5.0,
                "cp1_dy": 5.0,
            },
            {
                "id": 8,
                "type": "ellipse",
                "construction": False,
                "center_idx": 0,
                "radius_x_pt_idx": 1,
                "radius_y_pt_idx": 2,
                "helper_line_ids": [],
            },
            {
                "id": 9,
                "type": "polygon",
                "construction": False,
                "center_idx": 0,
                "handle_idx": 1,
                "vertices": [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]],
                "closed": True,
                "rings": [[[2.0, 2.0], [3.0, 2.0], [3.0, 3.0]]],
            },
            {
                "id": 10,
                "type": "text_box",
                "construction": False,
                "origin_id": 0,
                "width_id": 1,
                "height_id": 2,
                "content": "hello",
                "font_config": {
                    "font_family": "sans-serif",
                    "font_size": 12.0,
                    "bold": True,
                    "italic": False,
                    "custom_marker": 7,
                },
                "construction_line_ids": [3, 4],
                "fill_color": [1.0, 0.5, 0.0, 1.0],
            },
        ],
        "constraints": [],
        "id_counter": 11,
    }


def test_round_trip_preserves_registry_dict():
    original = _rect_registry_dict()
    model = EntityRegistry.from_dict(original)
    assert model.to_dict() == original


def test_from_dict_counts_and_ids():
    model = EntityRegistry.from_dict(_rect_registry_dict())
    assert model.point_count() == 4
    assert model.entity_count() == 7
    assert model.point_ids() == [0, 1, 2, 3]
    assert model.entity_ids() == [4, 5, 6, 7, 8, 9, 10]


def test_entity_queries():
    model = EntityRegistry.from_dict(_rect_registry_dict())
    assert model.entity_type(4) == "line"
    assert model.entity_type(5) == "arc"
    assert model.entity_type(6) == "circle"
    assert model.entity_type(7) == "bezier"
    assert model.entity_type(8) == "ellipse"
    assert model.entity_type(9) == "polygon"
    assert model.entity_type(10) == "text_box"

    assert model.entity_point_ids(4) == [0, 1]
    assert model.entity_point_ids(5) == [1, 2, 3]
    assert model.entity_point_ids(6) == [0, 1]
    assert model.entity_point_ids(8) == [0, 1, 2]
    assert model.entity_point_ids(9) == [0, 1]
    assert model.entity_point_ids(10) == [0, 1, 2]

    assert model.entity_endpoint_ids(4) == [0, 1]
    assert model.entity_endpoint_ids(6) == []


def test_point_queries_and_mutation():
    model = EntityRegistry.from_dict(_rect_registry_dict())
    assert model.get_point(0) == (0.0, 0.0)
    assert model.is_point_used(0)
    assert model.set_point_position(0, 5.0, 6.0)
    assert model.get_point(0) == (5.0, 6.0)

    unused = model.point_ids()[-1] + 1000
    assert not model.is_point_used(unused)
    model.remove_points([unused])


def test_add_methods_allocate_sequential_ids():
    model = EntityRegistry.from_dict(_rect_registry_dict())
    pid = model.add_point(1.0, 2.0, False)
    assert pid == 11
    line = model.add_line(pid, 0)
    assert line == 12
    arc = model.add_arc(0, 1, 2, True)
    assert arc == 13
    circle = model.add_circle(0, 1)
    assert circle == 14
    ellipse = model.add_ellipse(0, 1, 2)
    assert ellipse == 15
    bezier = model.add_bezier(0, 1, cp1=(1.0, 2.0))
    assert bezier == 16
    polygon = model.add_polygon(0, 1, [(0.0, 0.0), (5.0, 0.0)], True)
    assert polygon == 17
    text = model.add_text_box(0, 1, 2, "hi")
    assert text == 18
    assert model.entity_count() == 14


def test_remove_entities_updates_usage():
    model = EntityRegistry.from_dict(_rect_registry_dict())
    version = model.entity_version()
    model.remove_entities([4, 5, 6, 7, 8, 9, 10])
    assert model.entity_count() == 0
    assert model.entity_version() == version + 1
    assert model.is_point_used(0) is False


def test_build_and_serialize_matches_reference_shape():
    model = EntityRegistry()
    p0 = model.add_point(0.0, 0.0, True)
    p1 = model.add_point(100.0, 0.0)
    line = model.add_line(p0, p1)
    expected = {
        "points": [
            {"id": 0, "x": 0.0, "y": 0.0, "fixed": True},
            {"id": 1, "x": 100.0, "y": 0.0, "fixed": False},
        ],
        "entities": [
            {
                "id": line,
                "type": "line",
                "construction": False,
                "p1_idx": p0,
                "p2_idx": p1,
            }
        ],
        "constraints": [],
        "id_counter": 3,
    }
    assert model.to_dict() == expected


def test_constraints_round_trip():
    model = EntityRegistry()
    p0 = model.add_point(0.0, 0.0, True)
    p1 = model.add_point(100.0, 0.0)
    model.add_line(p0, p1)
    horizontal = {
        "type": "HorizontalConstraint",
        "p1": p0,
        "p2": p1,
        "user_visible": True,
    }
    distance = {
        "type": "DistanceConstraint",
        "p1": p0,
        "p2": p1,
        "value": 100.0,
    }
    h_index = model.add_constraint_json(horizontal)
    d_index = model.add_constraint_json(distance)
    assert h_index == 0
    assert d_index == 1
    assert model.constraint_count() == 2

    restored = EntityRegistry.from_dict(model.to_dict())
    assert restored.constraint_count() == 2
    assert restored.constraint_dict(h_index) == horizontal
    assert restored.constraint_dict(d_index) == distance
    assert restored.to_dict()["constraints"] == [horizontal, distance]
