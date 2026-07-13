import pytest

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, RasterMode, SectionType


def _count(ops, ct):
    return sum(1 for i in range(ops.len()) if ops.command_type(i) == ct)


def test_split_at_layer():
    ops = Ops()
    ops.layer_start("rough")
    ops.move_to(10.0, 20.0, 0.0)
    ops.line_to(30.0, 40.0, 0.0)
    ops.layer_end("rough")

    parts = ops.split_at(CommandType.LAYER_START)
    assert len(parts) == 1
    assert _count(parts[0], CommandType.LAYER_START) == 1
    assert _count(parts[0], CommandType.LAYER_END) == 1


def test_split_at_layer_with_gaps():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)  # before first layer
    ops.layer_start("a")
    ops.line_to(10.0, 10.0, 0.0)
    ops.layer_end("a")
    ops.line_to(20.0, 20.0, 0.0)  # gap between layers
    ops.layer_start("b")
    ops.line_to(30.0, 30.0, 0.0)
    ops.layer_end("b")
    ops.line_to(40.0, 40.0, 0.0)  # after last layer

    parts = ops.split_at(CommandType.LAYER_START)
    assert len(parts) == 5
    # gap before first layer
    assert _count(parts[0], CommandType.MOVE_TO) == 1
    assert _count(parts[0], CommandType.LAYER_START) == 0
    # layer a
    assert _count(parts[1], CommandType.LAYER_START) == 1
    # gap between
    assert _count(parts[2], CommandType.LINE_TO) == 1
    assert _count(parts[2], CommandType.LAYER_START) == 0
    # layer b
    assert _count(parts[3], CommandType.LAYER_START) == 1
    # gap after
    assert _count(parts[4], CommandType.LINE_TO) == 1
    assert _count(parts[4], CommandType.LAYER_START) == 0


def test_split_at_workpiece():
    ops = Ops()
    ops.workpiece_start("wp1")
    ops.move_to(10.0, 10.0, 0.0)
    ops.workpiece_end("wp1")
    ops.workpiece_start("wp2")
    ops.move_to(20.0, 20.0, 0.0)
    ops.workpiece_end("wp2")

    parts = ops.split_at(CommandType.WORKPIECE_START)
    assert len(parts) == 2
    for p in parts:
        assert _count(p, CommandType.WORKPIECE_START) == 1
        assert _count(p, CommandType.WORKPIECE_END) == 1


def test_split_at_ops_section():
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(0.0, 0.0, 0.0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    ops.ops_section_start(
        SectionType.RASTER_FILL, "wp1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(10.0, 10.0, 0.0)
    ops.ops_section_end(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    parts = ops.split_at(CommandType.OPS_SECTION_START)
    assert len(parts) == 2
    for p in parts:
        assert _count(p, CommandType.OPS_SECTION_START) == 1
        assert _count(p, CommandType.OPS_SECTION_END) == 1


def test_split_at_job():
    ops = Ops()
    ops.job_start()
    ops.move_to(0.0, 0.0, 0.0)
    ops.job_end()

    parts = ops.split_at(CommandType.JOB_START)
    assert len(parts) == 1
    assert _count(parts[0], CommandType.JOB_START) == 1
    assert _count(parts[0], CommandType.JOB_END) == 1


def test_split_at_no_markers_returns_whole():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 10.0, 0.0)

    parts = ops.split_at(CommandType.LAYER_START)
    assert len(parts) == 1
    assert parts[0].len() == 2


def test_split_at_empty_ops():
    ops = Ops()
    parts = ops.split_at(CommandType.LAYER_START)
    assert parts == []


def test_split_at_unmatched_start():
    ops = Ops()
    ops.layer_start("orphan")
    ops.move_to(10.0, 10.0, 0.0)

    parts = ops.split_at(CommandType.LAYER_START)
    assert len(parts) == 1
    assert _count(parts[0], CommandType.LAYER_START) == 1
    assert _count(parts[0], CommandType.MOVE_TO) == 1


def test_split_at_stray_end():
    ops = Ops()
    ops.layer_end("stray")  # no matching start
    ops.move_to(0.0, 0.0, 0.0)

    parts = ops.split_at(CommandType.LAYER_START)
    assert len(parts) == 1
    assert _count(parts[0], CommandType.LAYER_END) == 1


def test_split_at_invalid_type():
    ops = Ops()
    with pytest.raises(ValueError, match="unsupported marker type"):
        ops.split_at(CommandType.MOVE_TO)

    with pytest.raises(ValueError, match="unsupported marker type"):
        ops.split_at(CommandType.LAYER_END)


def test_split_at_reassemble():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)  # gap before
    ops.layer_start("a")
    ops.line_to(10.0, 10.0, 0.0)
    ops.layer_end("a")
    ops.line_to(20.0, 20.0, 0.0)  # gap between
    ops.layer_start("b")
    ops.line_to(30.0, 30.0, 0.0)
    ops.layer_end("b")
    ops.line_to(40.0, 40.0, 0.0)  # gap after

    parts = ops.split_at(CommandType.LAYER_START)
    reassembled = Ops()
    for p in parts:
        reassembled.extend(p)

    assert reassembled.len() == ops.len()
    for i in range(ops.len()):
        assert reassembled.command_type(i) == ops.command_type(i)
        if ops.command_type(i) == CommandType.MOVE_TO:
            assert reassembled.endpoint(i) == ops.endpoint(i)


def test_split_at_layer_extents():
    ops = Ops()
    ops.layer_start("wide")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(100.0, 200.0, 0.0)
    ops.layer_end("wide")
    ops.layer_start("small")
    ops.move_to(10.0, 10.0, 0.0)
    ops.line_to(20.0, 30.0, 0.0)
    ops.layer_end("small")

    parts = ops.split_at(CommandType.LAYER_START)
    assert len(parts) == 2
    bbox0 = parts[0].rect(include_travel=False)
    assert bbox0 == (0.0, 0.0, 100.0, 200.0)
    bbox1 = parts[1].rect(include_travel=False)
    assert bbox1 == (10.0, 10.0, 20.0, 30.0)


def test_sub_ops():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(20, 0)

    sub = ops.sub_ops([0, 2])
    assert sub.len() == 2
    assert sub.command_type(0) == CommandType.MOVE_TO
    assert sub.command_type(1) == CommandType.LINE_TO
    assert sub.endpoint(1) == (20.0, 0.0, 0.0)


def test_sub_ops_is_deep_copy():
    ops = Ops()
    ops.line_to(10, 0)
    sub = ops.sub_ops([0])
    sub.move_to(99, 99)
    assert ops.len() == 1


def test_split_into_subpaths_empty():
    ops = Ops()
    assert ops.split_into_subpaths() == []


def test_split_into_subpaths_single_move():
    ops = Ops()
    ops.move_to(0, 0)
    result = ops.split_into_subpaths()
    assert len(result) == 1
    assert result[0].len() == 1
    assert result[0].is_travel(0)


def test_split_into_subpaths_single_subpath():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    result = ops.split_into_subpaths()
    assert len(result) == 1
    assert result[0].len() == 3


def test_split_into_subpaths_two_subpaths():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.move_to(20, 20)
    ops.line_to(30, 30)
    result = ops.split_into_subpaths()
    assert len(result) == 2
    assert result[0].is_travel(0)
    assert result[1].is_travel(0)


def test_split_into_subpaths_state_grouped():
    ops = Ops()
    ops.move_to(0, 0)
    ops.set_power(0.5)
    ops.line_to(10, 0)
    result = ops.split_into_subpaths()
    assert len(result) == 1
    assert result[0].len() == 3
    assert result[0].command_type(1) == CommandType.SET_POWER
    assert result[0].is_state(1)


def test_split_into_subpaths_starting_with_lineto():
    ops = Ops()
    ops.line_to(5, 5)
    ops.line_to(10, 10)
    result = ops.split_into_subpaths()
    assert len(result) == 1
    assert result[0].command_type(0) == CommandType.LINE_TO
    assert not result[0].is_travel(0)


def test_split_into_subpaths_three():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(1, 1)
    ops.move_to(2, 2)
    ops.line_to(3, 3)
    ops.move_to(4, 4)
    ops.line_to(5, 5)
    result = ops.split_into_subpaths()
    assert len(result) == 3


def test_split_into_subpaths_preserves_arc():
    ops = Ops()
    ops.move_to(0, 0)
    ops.arc_to(10, 0, 5, 0, clockwise=True)
    result = ops.split_into_subpaths()
    assert len(result) == 1
    assert result[0].len() == 2
    assert result[0].command_type(1) == CommandType.ARC_TO


def test_split_into_subpaths_single_moveto_no_draw():
    ops = Ops()
    ops.move_to(5, 5)
    ops.move_to(10, 10)
    result = ops.split_into_subpaths()
    assert len(result) == 2
    assert result[0].len() == 1
    assert result[1].len() == 1
