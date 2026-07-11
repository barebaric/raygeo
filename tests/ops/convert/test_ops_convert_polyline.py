import pytest

import raygeo.ops as ops_mod
from raygeo.ops import Ops
from raygeo.ops.state import State
from raygeo.ops.types import CommandType

# Type aliases for readability
_MoveTo = ops_mod.types.CommandType.MOVE_TO
_LineTo = ops_mod.types.CommandType.LINE_TO


# ── polyline_to_ops ──


def test_polyline_to_ops_empty():
    result = Ops.from_polyline([], move_first=True)
    assert len(result) == 0


def test_polyline_to_ops_single_point_with_move():
    result = Ops.from_polyline([(10.0, 20.0, 0.0)], move_first=True)
    assert len(result) == 1
    assert result.command_type(0) == _MoveTo


def test_polyline_to_ops_single_point_no_move():
    result = Ops.from_polyline([(10.0, 20.0, 0.0)], move_first=False)
    assert len(result) == 1
    assert result.command_type(0) == _LineTo


def test_polyline_to_ops_with_move():
    points = [(0.0, 0.0, 0.0), (10.0, 10.0, 0.0), (20.0, 0.0, 0.0)]
    result = Ops.from_polyline(points, move_first=True)
    assert len(result) == 3
    assert result.command_type(0) == _MoveTo
    assert result.command_type(1) == _LineTo
    assert result.command_type(2) == _LineTo


def test_polyline_to_ops_no_move():
    points = [(0.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    result = Ops.from_polyline(points, move_first=False)
    assert len(result) == 2
    assert result.command_type(0) == _LineTo
    assert result.command_type(1) == _LineTo


def test_polyline_to_ops_3d():
    points = [(0.0, 0.0, 5.0), (10.0, 0.0, 0.0), (20.0, 0.0, -5.0)]
    result = Ops.from_polyline(points, move_first=True)
    assert result.endpoint(0)[2] == pytest.approx(5.0)
    assert result.endpoint(1)[2] == pytest.approx(0.0)
    assert result.endpoint(2)[2] == pytest.approx(-5.0)


# ── smoke tests from original test_ops_assembly ──


def test_assembly_from_polyline():
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    ops = Ops.from_polyline(points, move_first=True)
    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.LINE_TO


def test_assembly_polyline_no_move_first():
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    ops = Ops.from_polyline(points, move_first=False)
    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.LINE_TO


def test_polyline_to_ops_with_state_prepends_commands():
    points = [(10.0, 20.0, 0.0), (30.0, 40.0, 0.0)]
    st = State(power=0.5, feed_rate=1200)
    result = Ops.from_polyline(points, move_first=True, state=st)
    assert result.len() == 4
    assert result.command_type(0) == CommandType.SET_POWER
    assert result.command_type(1) == CommandType.SET_FEED_RATE
    assert result.command_type(2) == CommandType.MOVE_TO
    assert result.command_type(3) == CommandType.LINE_TO


def test_polyline_to_ops_with_state_empty_path():
    st = State(power=0.5)
    result = Ops.from_polyline([], move_first=True, state=st)
    assert result.len() >= 1
    assert result.command_type(0) == CommandType.SET_POWER


def test_polyline_to_ops_state_none_is_backward_compat():
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)]
    result = Ops.from_polyline(points, move_first=True)
    assert result.len() == 2
    assert result.command_type(0) == CommandType.MOVE_TO
    assert result.command_type(1) == CommandType.LINE_TO


def test_polyline_to_ops_state_power_applied():
    points = [(10.0, 20.0, 0.0)]
    st = State(power=0.75, feed_rate=800)
    result = Ops.from_polyline(points, move_first=True, state=st)
    assert result.state_at(0).power == 0.75
    assert result.state_at(1).feed_rate == 800


def test_polyline_to_ops_state_no_move_first():
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)]
    st = State(power=1.0)
    result = Ops.from_polyline(points, move_first=False, state=st)
    assert result.command_type(0) == CommandType.SET_POWER
    assert result.command_type(1) == CommandType.LINE_TO
    assert result.command_type(2) == CommandType.LINE_TO
