import pytest

import raygeo.ops as ops_mod
from raygeo.ops.assembly.polyline import polyline_to_ops
from raygeo.ops.types import CommandType

# Type aliases for readability
_MoveTo = ops_mod.types.CommandType.MOVE_TO
_LineTo = ops_mod.types.CommandType.LINE_TO


# ── polyline_to_ops ──


def test_polyline_to_ops_empty():
    result = polyline_to_ops([], move_first=True)
    assert len(result) == 0


def test_polyline_to_ops_single_point_with_move():
    result = polyline_to_ops([(10.0, 20.0, 0.0)], move_first=True)
    assert len(result) == 1
    assert result.command_type(0) == _MoveTo


def test_polyline_to_ops_single_point_no_move():
    result = polyline_to_ops([(10.0, 20.0, 0.0)], move_first=False)
    assert len(result) == 1
    assert result.command_type(0) == _LineTo


def test_polyline_to_ops_with_move():
    points = [(0.0, 0.0, 0.0), (10.0, 10.0, 0.0), (20.0, 0.0, 0.0)]
    result = polyline_to_ops(points, move_first=True)
    assert len(result) == 3
    assert result.command_type(0) == _MoveTo
    assert result.command_type(1) == _LineTo
    assert result.command_type(2) == _LineTo


def test_polyline_to_ops_no_move():
    points = [(0.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    result = polyline_to_ops(points, move_first=False)
    assert len(result) == 2
    assert result.command_type(0) == _LineTo
    assert result.command_type(1) == _LineTo


def test_polyline_to_ops_3d():
    points = [(0.0, 0.0, 5.0), (10.0, 0.0, 0.0), (20.0, 0.0, -5.0)]
    result = polyline_to_ops(points, move_first=True)
    assert result.endpoint(0)[2] == pytest.approx(5.0)
    assert result.endpoint(1)[2] == pytest.approx(0.0)
    assert result.endpoint(2)[2] == pytest.approx(-5.0)


# ── smoke tests from original test_ops_assembly ──


def test_assembly_polyline_to_ops():
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    ops = polyline_to_ops(points, move_first=True)
    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.LINE_TO


def test_assembly_polyline_no_move_first():
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    ops = polyline_to_ops(points, move_first=False)
    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.LINE_TO
