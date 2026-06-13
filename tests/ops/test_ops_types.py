from raygeo.ops.types import (
    CommandCategory,
    CommandType,
    SectionType,
    category,
)


def test_command_type_values():
    assert CommandType.MOVE_TO.value == 1
    assert CommandType.LINE_TO.value == 2
    assert CommandType.ARC_TO.value == 3
    assert CommandType.SCAN_LINE.value == 4
    assert CommandType.DWELL.value == 5
    assert CommandType.BEZIER_TO.value == 6
    assert CommandType.QUADRATIC_BEZIER_TO.value == 7
    assert CommandType.SET_POWER.value == 10
    assert CommandType.SET_CUT_SPEED.value == 11
    assert CommandType.SET_TRAVEL_SPEED.value == 12
    assert CommandType.ENABLE_AIR_ASSIST.value == 13
    assert CommandType.DISABLE_AIR_ASSIST.value == 14
    assert CommandType.SET_LASER.value == 15
    assert CommandType.SET_FREQUENCY.value == 16
    assert CommandType.SET_PULSE_WIDTH.value == 17
    assert CommandType.JOB_START.value == 100
    assert CommandType.JOB_END.value == 101
    assert CommandType.LAYER_START.value == 102
    assert CommandType.LAYER_END.value == 103
    assert CommandType.WORKPIECE_START.value == 104
    assert CommandType.WORKPIECE_END.value == 105
    assert CommandType.OPS_SECTION_START.value == 106
    assert CommandType.OPS_SECTION_END.value == 107


def test_category_moving():
    for ct in [
        CommandType.MOVE_TO,
        CommandType.LINE_TO,
        CommandType.ARC_TO,
        CommandType.BEZIER_TO,
        CommandType.QUADRATIC_BEZIER_TO,
        CommandType.SCAN_LINE,
    ]:
        assert category(ct) == CommandCategory.MOVING


def test_category_state():
    for ct in [
        CommandType.DWELL,
        CommandType.SET_POWER,
        CommandType.SET_CUT_SPEED,
        CommandType.SET_TRAVEL_SPEED,
        CommandType.SET_FREQUENCY,
        CommandType.SET_PULSE_WIDTH,
        CommandType.ENABLE_AIR_ASSIST,
        CommandType.DISABLE_AIR_ASSIST,
        CommandType.SET_LASER,
    ]:
        assert category(ct) == CommandCategory.STATE


def test_category_marker():
    for ct in [
        CommandType.JOB_START,
        CommandType.JOB_END,
        CommandType.LAYER_START,
        CommandType.LAYER_END,
        CommandType.WORKPIECE_START,
        CommandType.WORKPIECE_END,
        CommandType.OPS_SECTION_START,
        CommandType.OPS_SECTION_END,
    ]:
        assert category(ct) == CommandCategory.MARKER


def test_command_type_names():
    assert CommandType.MOVE_TO.name == "MOVE_TO"
    assert CommandType.LINE_TO.name == "LINE_TO"
    assert CommandType.ARC_TO.name == "ARC_TO"
    assert CommandType.SCAN_LINE.name == "SCAN_LINE"
    assert CommandType.SET_POWER.name == "SET_POWER"
    assert CommandType.JOB_START.name == "JOB_START"
    assert CommandType.OPS_SECTION_END.name == "OPS_SECTION_END"


def test_section_type_names():
    assert SectionType.VECTOR_OUTLINE.name == "VECTOR_OUTLINE"
    assert SectionType.RASTER_FILL.name == "RASTER_FILL"


def test_command_category_values():
    assert CommandCategory.MOVING.value == 0
    assert CommandCategory.STATE.value == 1
    assert CommandCategory.MARKER.value == 2
