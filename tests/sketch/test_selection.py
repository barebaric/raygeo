from raygeo.sketch.registry import EntityRegistry
from raygeo.sketch.selection import Selection


def test_clear_clears_all_selections():
    selection = Selection()
    selection.point_ids = [1, 2]
    selection.entity_ids = [3, 4]
    selection.constraint_idx = 5
    selection.junction_pid = 6

    selection.clear()

    assert selection.point_ids == []
    assert selection.entity_ids == []
    assert selection.constraint_idx is None
    assert selection.junction_pid is None


def test_copy_is_independent():
    selection = Selection()
    selection.point_ids = [1]
    selection.entity_ids = [2]

    copy = selection.copy()
    copy.point_ids = [1, 3]
    copy.entity_ids = [2, 4]

    assert selection.point_ids == [1]
    assert selection.entity_ids == [2]


def test_select_constraint_single_mode():
    selection = Selection()
    selection.point_ids = [1]
    selection.entity_ids = [2]
    selection.junction_pid = 3

    selection.select_constraint(5, is_multi=False)

    assert selection.constraint_idx == 5
    assert selection.point_ids == []
    assert selection.entity_ids == []
    assert selection.junction_pid is None


def test_select_constraint_multi_mode():
    selection = Selection()
    selection.point_ids = [1]
    selection.entity_ids = [2]
    selection.junction_pid = 3

    selection.select_constraint(5, is_multi=True)

    assert selection.constraint_idx == 5
    assert selection.point_ids == [1]
    assert selection.entity_ids == [2]
    assert selection.junction_pid == 3


def test_select_junction_single_mode():
    selection = Selection()
    selection.point_ids = [1]
    selection.entity_ids = [2]
    selection.constraint_idx = 3

    selection.select_junction(5, is_multi=False)

    assert selection.junction_pid == 5
    assert selection.point_ids == []
    assert selection.entity_ids == []
    assert selection.constraint_idx is None


def test_select_junction_multi_mode():
    selection = Selection()
    selection.point_ids = [1]
    selection.entity_ids = [2]
    selection.constraint_idx = 3

    selection.select_junction(5, is_multi=True)

    assert selection.junction_pid == 5
    assert selection.point_ids == [1]
    assert selection.entity_ids == [2]
    assert selection.constraint_idx == 3


def test_select_point_single_mode():
    selection = Selection()
    selection.entity_ids = [2]
    selection.constraint_idx = 3
    selection.junction_pid = 4

    selection.select_point(5, is_multi=False)

    assert selection.point_ids == [5]
    assert selection.entity_ids == []
    assert selection.constraint_idx is None
    assert selection.junction_pid is None


def test_select_point_multi_mode_adds_and_toggles():
    selection = Selection()
    selection.select_point(1, is_multi=True)
    selection.select_point(2, is_multi=True)
    assert selection.point_ids == [1, 2]

    selection.select_point(1, is_multi=True)
    assert selection.point_ids == [2]


def test_select_point_single_mode_no_change_if_same():
    selection = Selection()
    selection.select_point(1, is_multi=False)
    selection.select_point(1, is_multi=False)
    assert selection.point_ids == [1]


def test_select_entity_single_mode():
    selection = Selection()
    selection.point_ids = [1]
    selection.constraint_idx = 2
    selection.junction_pid = 3

    selection.select_entity(10, is_multi=False)

    assert selection.entity_ids == [10]
    assert selection.point_ids == []
    assert selection.constraint_idx is None
    assert selection.junction_pid is None


def test_select_entity_multi_mode_adds_and_toggles():
    selection = Selection()
    selection.select_entity(1, is_multi=True)
    selection.select_entity(2, is_multi=True)
    assert selection.entity_ids == [1, 2]

    selection.select_entity(1, is_multi=True)
    assert selection.entity_ids == [2]


def test_select_connected_entities_adds_to_existing_selection():
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    p3 = registry.add_point(20, 0)
    p4 = registry.add_point(30, 0)
    l1 = registry.add_line(p1, p2)
    l2 = registry.add_line(p2, p3)
    l3 = registry.add_line(p3, p4)

    selection = Selection()
    selection.entity_ids = [l3]
    selection.point_ids = [p1]

    selection.select_connected_entities(l1, registry)

    assert set(selection.entity_ids) == {l1, l2, l3}
    assert selection.point_ids == []
    assert selection.constraint_idx is None
    assert selection.junction_pid is None


def test_select_connected_entities_clears_constraint_and_junction():
    registry = EntityRegistry()
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(10, 0)
    l1 = registry.add_line(p1, p2)

    selection = Selection()
    selection.constraint_idx = 0
    selection.junction_pid = p1

    selection.select_connected_entities(l1, registry)

    assert selection.constraint_idx is None
    assert selection.junction_pid is None


def test_add_point_and_add_entity_do_not_clear():
    selection = Selection()
    selection.constraint_idx = 7
    selection.junction_pid = 8

    selection.add_point(1)
    selection.add_point(1)
    selection.add_entity(2)

    assert selection.point_ids == [1, 1]
    assert selection.entity_ids == [2]
    assert selection.constraint_idx == 7
    assert selection.junction_pid == 8


def test_clear_points_and_entities_are_separate():
    selection = Selection()
    selection.point_ids = [1]
    selection.entity_ids = [2]
    selection.constraint_idx = 3
    selection.junction_pid = 4

    selection.clear_points()
    assert selection.point_ids == []
    assert selection.entity_ids == [2]
    assert selection.constraint_idx == 3
    assert selection.junction_pid == 4

    selection.clear_entities()
    assert selection.entity_ids == []


def test_is_empty():
    selection = Selection()
    assert selection.is_empty() is True

    for setter in (
        lambda: setattr(selection, "point_ids", [1]),
        lambda: setattr(selection, "entity_ids", [1]),
        lambda: setattr(selection, "constraint_idx", 0),
        lambda: setattr(selection, "junction_pid", 1),
    ):
        selection.clear()
        setter()
        assert selection.is_empty() is False
