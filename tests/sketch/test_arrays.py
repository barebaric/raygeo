from raygeo.sketch.arrays import Array


def test_circular_to_dict_from_dict():
    definition = Array.circular(
        "arr-1",
        3,
        members=[(0, [10, 11]), (1, [12])],
        count=4,
        total_angle_deg=180.0,
        rotate_copies=False,
    )
    definition.standalone_pids = [(0, [20])]
    data = definition.to_dict()
    assert data == {
        "uid": "arr-1",
        "mode": "circular",
        "guide_circle_id": 3,
        "members": [[0, [10, 11]], [1, [12]]],
        "standalone_pids": [[0, [20]]],
        "count": 4,
        "rotate_copies": False,
        "total_angle_deg": 180.0,
    }
    restored = Array.from_dict(data)
    assert restored.to_dict() == data


def test_curve_along_to_dict_from_dict():
    definition = Array.curve_along(
        "arr-2",
        5,
        members=[(0, [1])],
        count=3,
        rotate_copies=True,
        path_entity_id=7,
        align_to_tangent=False,
        offset_to_start=1.5,
        spacing=2.5,
        template_anchor=((3.0, 4.0), 0.5),
    )
    data = definition.to_dict()
    assert data == {
        "uid": "arr-2",
        "mode": "curve_along",
        "guide_circle_id": 5,
        "members": [[0, [1]]],
        "standalone_pids": [],
        "count": 3,
        "rotate_copies": True,
        "path_entity_id": 7,
        "align_to_tangent": False,
        "offset_to_start": 1.5,
        "spacing": 2.5,
        "template_anchor": [[3.0, 4.0], 0.5],
    }
    restored = Array.from_dict(data)
    assert restored.to_dict() == data


def test_circular_snapshot_restore():
    definition = Array.circular("arr-3", 1, count=2)
    state = definition.snapshot()
    assert state == {
        "members": [],
        "standalone_pids": [],
        "count": 2,
        "total_angle_deg": 360.0,
        "rotate_copies": True,
    }

    definition.members = [(0, [9])]
    definition.count = 8
    definition.total_angle_deg = 90.0
    definition.rotate_copies = False

    definition.restore(state)
    assert definition.members == []
    assert definition.count == 2
    assert definition.total_angle_deg == 360.0
    assert definition.rotate_copies is True


def test_curve_along_snapshot_restore():
    definition = Array.curve_along(
        "arr-4",
        1,
        count=2,
        path_entity_id=5,
        template_anchor=((1.0, 2.0), 0.25),
    )
    state = definition.snapshot()
    definition.path_entity_id = 9
    definition.spacing = 4.0
    definition.template_anchor = None
    definition.restore(state)
    assert definition.path_entity_id == 5
    assert definition.spacing == 0.0
    assert definition.template_anchor == ((1.0, 2.0), 0.25)


def test_unsupported_mode_is_rejected():
    try:
        Array.from_dict({"uid": "x", "mode": "spiral"})
    except ValueError:
        pass
    else:
        raise AssertionError("expected ValueError")
