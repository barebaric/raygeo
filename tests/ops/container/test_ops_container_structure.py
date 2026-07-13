import pytest

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, RasterMode, SectionType


def test_subpath_indices():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    ops.move_to(100, 100)
    ops.line_to(110, 100)

    result = ops.subpath_indices()
    assert len(result) == 2
    assert result[0] == [0, 1, 2]
    assert result[1] == [3, 4]


def test_subpath_indices_empty():
    ops = Ops()
    assert ops.subpath_indices() == []


def test_subpath_indices_single():
    ops = Ops()
    ops.move_to(0, 0)
    assert ops.subpath_indices() == [[0]]


def test_section_raster_mode_with_mode():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    sections = ops.sections()
    assert len(sections) == 1
    assert sections[0].raster_mode == RasterMode.VARIABLE_POWER


def test_section_raster_mode_none_with_old_api():
    ops = Ops()
    ops.ops_section_start(SectionType.RASTER_FILL, "wp-1")
    ops.move_to(0, 0)
    ops.ops_section_end(SectionType.RASTER_FILL)

    sections = ops.sections()
    assert len(sections) == 1
    assert sections[0].raster_mode is None


def test_section_raster_mode_none_for_vector():
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp-1")
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    sections = ops.sections()
    assert len(sections) == 1
    assert sections[0].raster_mode is None


def test_section_ranges_raster_mode():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.CONSTANT_POWER
    )
    ops.move_to(0, 0)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.CONSTANT_POWER
    )

    ranges = ops.section_ranges()
    assert len(ranges) == 1
    assert ranges[0].raster_mode == RasterMode.CONSTANT_POWER


def test_multiple_sections_with_different_raster_modes():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.CONSTANT_POWER
    )
    ops.move_to(10, 10)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.CONSTANT_POWER
    )

    sections = ops.sections()
    assert len(sections) == 2
    assert sections[0].raster_mode == RasterMode.VARIABLE_POWER
    assert sections[1].raster_mode == RasterMode.CONSTANT_POWER


def test_validation_vector_outline_with_mode_rejected():
    with pytest.raises(ValueError):
        ops = Ops()
        ops.ops_section_start_with_mode(
            SectionType.VECTOR_OUTLINE,
            "wp-1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )


def test_validation_vector_outline_end_with_mode_rejected():
    with pytest.raises(ValueError):
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp-1")
        ops.ops_section_end_with_mode(
            SectionType.VECTOR_OUTLINE, raster_mode=RasterMode.VARIABLE_POWER
        )


def test_validation_raster_fill_without_mode_rejected():
    with pytest.raises(ValueError):
        ops = Ops()
        ops.ops_section_start_with_mode(
            SectionType.RASTER_FILL, "wp-1", raster_mode=None
        )


def test_validation_raster_fill_end_without_mode_rejected():
    with pytest.raises(ValueError):
        ops = Ops()
        ops.ops_section_start_with_mode(
            SectionType.RASTER_FILL,
            "wp-1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.ops_section_end_with_mode(
            SectionType.RASTER_FILL, raster_mode=None
        )


def test_validation_valid_combos_accepted():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )
    sections = ops.sections()
    assert len(sections) == 1
    assert sections[0].raster_mode == RasterMode.VARIABLE_POWER


def test_validation_old_api_bypasses():
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp-1")
    ops.move_to(0, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    sections = ops.sections()
    assert len(sections) == 1
    assert sections[0].raster_mode is None


def test_validation_raster_fill_with_mode_accepted():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.DEPTH_MAP
    )
    ops.move_to(0, 0)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.DEPTH_MAP
    )
    sections = ops.sections()
    assert len(sections) == 1
    assert sections[0].raster_mode == RasterMode.DEPTH_MAP


def test_state_block_markers():
    ops = Ops()
    ops.state_block_start("test-block")
    ops.set_power(0.5)
    ops.state_block_end()
    assert ops.command_type(0) == CommandType.STATE_BLOCK_START
    assert ops.command_type(2) == CommandType.STATE_BLOCK_END


def test_state_blocks_inside_section():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.state_block_start("labels")
    ops.set_power(0.3)
    ops.state_block_end()
    ops.state_block_start("cell-r0-c0")
    ops.set_power(0.5)
    ops.set_feed_rate(100)
    ops.state_block_end()
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    sections = ops.sections()
    assert len(sections) == 1
    blocks = sections[0].state_blocks(ops)
    assert len(blocks) == 2
    assert blocks[0].name == "labels"
    assert blocks[1].name == "cell-r0-c0"


def test_state_blocks_by_name_prefix():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.CONSTANT_POWER
    )
    ops.state_block_start("cell-r0-c0")
    ops.set_power(0.1)
    ops.state_block_end()
    ops.state_block_start("cell-r0-c1")
    ops.set_power(0.5)
    ops.state_block_end()
    ops.state_block_start("labels")
    ops.set_power(0.3)
    ops.state_block_end()
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.CONSTANT_POWER
    )

    sections = ops.sections()
    assert len(sections) == 1
    cell_blocks = sections[0].state_blocks_by_name(ops, "cell-*")
    assert len(cell_blocks) == 2
    assert cell_blocks[0].name == "cell-r0-c0"
    assert cell_blocks[1].name == "cell-r0-c1"

    labels = sections[0].state_blocks_by_name(ops, "labels")
    assert len(labels) == 1
    assert labels[0].name == "labels"


def test_state_block_content():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.state_block_start("block1")
    ops.set_power(0.5)
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.state_block_end()
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    sections = ops.sections()
    blocks = sections[0].state_blocks(ops)
    assert len(blocks) == 1
    content = sections[0].state_block_content(ops, blocks[0])
    assert len(content) == 3
    assert content.command_type(0) == CommandType.SET_POWER
    assert content.command_type(1) == CommandType.MOVE_TO
    assert content.command_type(2) == CommandType.LINE_TO


def test_state_blocks_all():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.state_block_start("a")
    ops.set_power(0.5)
    ops.state_block_end()
    ops.state_block_start("b")
    ops.set_power(0.7)
    ops.state_block_end()
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    all_blocks = ops.state_blocks()
    assert len(all_blocks) == 2
    assert all_blocks[0].name == "a"
    assert all_blocks[1].name == "b"


def test_state_block_start_outside_section():
    ops = Ops()
    ops.state_block_start("orphan")
    assert ops.command_type(0) == CommandType.STATE_BLOCK_START


def test_sections_by_mode():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.CONSTANT_POWER
    )
    ops.move_to(10, 10)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.CONSTANT_POWER
    )

    var_sections = ops.sections_by_mode(RasterMode.VARIABLE_POWER)
    assert len(var_sections) == 1
    assert var_sections[0].raster_mode == RasterMode.VARIABLE_POWER

    const_sections = ops.sections_by_mode(RasterMode.CONSTANT_POWER)
    assert len(const_sections) == 1
    assert const_sections[0].raster_mode == RasterMode.CONSTANT_POWER

    depth_sections = ops.sections_by_mode(RasterMode.DEPTH_MAP)
    assert len(depth_sections) == 0


def test_sections_by_type():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp-1")
    ops.move_to(10, 10)
    ops.line_to(20, 20)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    raster_sections = ops.sections_by_type(SectionType.RASTER_FILL)
    assert len(raster_sections) == 1
    assert raster_sections[0].section_type == SectionType.RASTER_FILL

    vector_sections = ops.sections_by_type(SectionType.VECTOR_OUTLINE)
    assert len(vector_sections) == 1
    assert vector_sections[0].section_type == SectionType.VECTOR_OUTLINE


def test_section_content():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    sections = ops.sections()
    assert len(sections) == 1

    content = ops.section_content(sections[0])
    assert len(content) == 2
    assert content.command_type(0) == CommandType.MOVE_TO
    assert content.command_type(1) == CommandType.LINE_TO


def test_section_content_via_section_method():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    sections = ops.sections()
    assert len(sections) == 1

    content = sections[0].content(ops)
    assert len(content) == 1
    assert content.command_type(0) == CommandType.MOVE_TO


def test_section_range_content():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    ranges = ops.section_ranges()
    assert len(ranges) == 1

    content = ranges[0].content(ops)
    assert len(content) == 2
