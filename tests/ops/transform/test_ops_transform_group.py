from typing import List

from raygeo.ops import Ops
from raygeo.ops.state import AirAssistMode, CoolantMode, HeadCoolantMode, State
from raygeo.ops.types import CommandType


def test_group_by_command_type_empty():
    ops = Ops()
    assert list(ops.segment_indices()) == []


def test_group_by_command_type_single_move():
    ops = Ops()
    ops.move_to(0, 0)
    indices = list(ops.segment_indices())
    assert len(indices) == 1
    assert len(indices[0]) == 1
    assert ops.command_type(indices[0][0]) == CommandType.MOVE_TO


def test_group_by_command_type_move_and_line():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(1, 0)
    indices = list(ops.segment_indices())
    assert len(indices) == 1
    assert len(indices[0]) == 2


def test_group_by_command_type_move_and_arc():
    ops = Ops()
    ops.move_to(0, 0)
    ops.arc_to(1, 0, 1, 1, False)
    ops.line_to(2, 0)
    ops.line_to(3, 0)
    indices = list(ops.segment_indices())
    assert len(indices) == 1
    assert indices[0][0] == 0
    assert ops.command_type(indices[0][1]) == CommandType.ARC_TO


def test_group_by_command_type_state_commands():
    ops = Ops()
    ops.set_power(1.0)
    ops.move_to(0, 0)
    ops.line_to(1, 0)
    ops.set_coolant(CoolantMode.OFF)
    indices = list(ops.segment_indices())
    assert len(indices) == 3
    assert ops.is_state(indices[0][0])
    assert ops.is_travel(indices[1][0])
    assert ops.is_state(indices[2][0])


def _create_ops_with_states(states_config: List[bool]) -> Ops:
    """Helper to create ops with specified coolant states."""
    ops = Ops()
    for i, flood_on in enumerate(states_config):
        ops.line_to(float(i), float(i))
    for i, flood_on in enumerate(states_config):
        ops.set_state_at(
            i,
            State(
                power=1.0,
                coolant=CoolantMode.FLOOD if flood_on else CoolantMode.OFF,
            ),
        )
    return ops


def _create_ops_with_air_assist_states(states_config: List[bool]) -> Ops:
    """Helper to create ops with specified air assist states."""
    ops = Ops()
    for i, on in enumerate(states_config):
        ops.line_to(float(i), float(i))
    for i, on in enumerate(states_config):
        ops.set_state_at(
            i,
            State(
                power=1.0,
                air_assist=AirAssistMode.ON if on else AirAssistMode.OFF,
            ),
        )
    return ops


def _create_ops_with_head_coolant_states(states_config: List[bool]) -> Ops:
    """Helper to create ops with specified head coolant states."""
    ops = Ops()
    for i, on in enumerate(states_config):
        ops.line_to(float(i), float(i))
    for i, on in enumerate(states_config):
        ops.set_state_at(
            i,
            State(
                power=1.0,
                head_coolant=HeadCoolantMode.ON if on else HeadCoolantMode.OFF,
            ),
        )
    return ops


def test_group_by_state_continuity():
    """Test splitting commands by non-reorderable state changes."""
    # All same coolant state -> 1 segment
    ops1 = _create_ops_with_states([True, True, True])
    groups = ops1.group_by_state_continuity()
    assert len(groups) == 1
    assert groups[0].len() == 3

    # Coolant state change -> 2 segments
    ops2 = _create_ops_with_states([True, True, False])
    groups = ops2.group_by_state_continuity()
    assert len(groups) == 2
    assert groups[0].len() == 2
    assert groups[1].len() == 1

    # Multiple coolant state changes
    ops3 = _create_ops_with_states([False, True, True, False, False, True])
    groups = ops3.group_by_state_continuity()
    assert len(groups) == 4
    assert [g.len() for g in groups] == [1, 2, 2, 1]

    # Empty
    ops_empty = Ops()
    assert ops_empty.group_by_state_continuity() == []

    # Single command
    ops4 = _create_ops_with_states([True])
    assert len(ops4.group_by_state_continuity()) == 1

    # Test with marker commands
    ops_marker = Ops()
    ops_marker.line_to(0, 0)
    ops_marker.job_start()
    ops_marker.line_to(1, 1)
    ops_marker.set_state_at(
        0,
        State(
            power=1.0,
            coolant=CoolantMode.FLOOD,
        ),
    )
    ops_marker.set_state_at(
        2,
        State(
            power=1.0,
            coolant=CoolantMode.FLOOD,
        ),
    )
    groups_m = ops_marker.group_by_state_continuity()
    assert len(groups_m) == 3
    assert [g.len() for g in groups_m] == [1, 1, 1]
    assert groups_m[1].is_marker(0)


def test_group_by_state_continuity_air_assist():
    """Test splitting by air assist state changes."""
    # All same air assist -> 1 segment
    ops1 = _create_ops_with_air_assist_states([True, True, True])
    groups = ops1.group_by_state_continuity()
    assert len(groups) == 1

    # Air assist change -> 2 segments
    ops2 = _create_ops_with_air_assist_states([True, True, False])
    groups = ops2.group_by_state_continuity()
    assert len(groups) == 2
    assert groups[0].len() == 2
    assert groups[1].len() == 1


def test_group_by_state_continuity_head_coolant():
    """Test splitting by head coolant state changes."""
    # All same head coolant -> 1 segment
    ops1 = _create_ops_with_head_coolant_states([True, True, True])
    groups = ops1.group_by_state_continuity()
    assert len(groups) == 1

    # Head coolant change -> 2 segments
    ops2 = _create_ops_with_head_coolant_states([True, True, False])
    groups = ops2.group_by_state_continuity()
    assert len(groups) == 2
    assert groups[0].len() == 2
    assert groups[1].len() == 1


def test_group_by_path_continuity():
    """Test splitting a list of commands into re-orderable paths."""
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    ops.move_to(100, 100)
    ops.line_to(110, 100)
    indices = list(ops.segment_indices())
    assert len(indices) == 2
    assert len(indices[0]) == 3
    assert ops.is_travel(indices[0][0])
    assert len(indices[1]) == 2
    assert ops.is_travel(indices[1][0])

    # Test with a travel command at the end
    ops.move_to(0, 0)
    indices = list(ops.segment_indices())
    assert len(indices) == 3
    assert len(indices[2]) == 1


def test_segments():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.set_power(0.5)
    ops.set_air_assist(AirAssistMode.ON)
    ops.move_to(5, 5)
    segments = list(ops.segment_indices())
    assert len(segments) > 0
    # First segment should end before the travel command
    assert ops.is_cutting(segments[0][-1])


def test_without_state():
    ops = Ops()
    ops.set_power(1.0)
    ops.move_to(0, 0)
    ops.set_feed_rate(800)
    ops.line_to(10, 0)
    ops.set_air_assist(AirAssistMode.ON)

    filtered = ops.without_state()
    assert filtered.len() == 2
    assert filtered.command_type(0) == CommandType.MOVE_TO
    assert filtered.command_type(1) == CommandType.LINE_TO


def test_without_state_empty():
    ops = Ops()
    filtered = ops.without_state()
    assert filtered.len() == 0


def test_without_state_no_state_commands():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    filtered = ops.without_state()
    assert filtered.len() == 2
