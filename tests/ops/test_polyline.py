import pytest

import raygeo.ops as ops_mod
from raygeo.ops.polyline import LinkStrategy, link_passes, polyline_to_ops

# Type aliases for readability
_MoveTo = ops_mod.types.CommandType.MOVE_TO
_LineTo = ops_mod.types.CommandType.LINE_TO


def make_pass(start, end, z=0.0):
    ops = ops_mod.Ops()
    ops.move_to(start[0], start[1], z)
    ops.line_to(end[0], end[1], z)
    return ops


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


# ── link_passes ──


def test_link_passes_empty():
    result = link_passes([], safe_z=10.0, strategy="retract")
    assert len(result) == 0


def test_link_passes_single():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    result = link_passes([p1], safe_z=10.0, strategy="retract")
    assert len(result) == 2


def test_link_passes_stay_down():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    p2 = make_pass((20.0, 0.0), (30.0, 0.0), 0.0)
    result = link_passes([p1, p2], safe_z=10.0, strategy="stay_down")
    # pass1 + travel MoveTo + pass2 = 5
    assert len(result) == 5
    travel_end = result.endpoint(2)
    assert travel_end[0] == pytest.approx(20.0)
    assert travel_end[2] == pytest.approx(0.0)


def test_link_passes_retract():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    p2 = make_pass((20.0, 0.0), (30.0, 0.0), -5.0)
    result = link_passes([p1, p2], safe_z=10.0, strategy="retract")
    # pass1 (2) + retract + XY + descend (3) + pass2 (2) = 7
    assert len(result) == 7
    # index 2: retract to safe_z
    assert result.endpoint(2)[2] == pytest.approx(10.0)
    # index 3: XY move at safe_z
    assert result.endpoint(3)[0] == pytest.approx(20.0)
    assert result.endpoint(3)[2] == pytest.approx(10.0)
    # index 4: descend to pass2 Z
    assert result.endpoint(4)[2] == pytest.approx(-5.0)


def test_link_passes_three_passes():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    p2 = make_pass((10.0, 10.0), (20.0, 10.0), 0.0)
    p3 = make_pass((20.0, 0.0), (30.0, 0.0), 0.0)
    result = link_passes([p1, p2, p3], safe_z=5.0, strategy="retract")
    assert len(result) == 12


def test_link_passes_retract_same_xy():
    """XY unchanged between passes — no redundant position change."""
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    p2 = make_pass((10.0, 0.0), (20.0, 0.0), 0.0)
    result = link_passes([p1, p2], safe_z=0.0, strategy="retract")
    assert len(result) >= 4


def test_link_passes_invalid_strategy():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    with pytest.raises(ValueError):
        link_passes([p1], safe_z=10.0, strategy="invalid")


def test_link_strategy_constants():
    assert LinkStrategy.RETRACT == "retract"
    assert LinkStrategy.STAY_DOWN == "stay_down"
