from raygeo.ops.state import CoolantMode
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
    assert CommandType.SET_FEED_RATE.value == 11
    assert CommandType.SET_RAPID_RATE.value == 12
    assert CommandType.SET_HEAD.value == 15
    assert CommandType.SET_FREQUENCY.value == 16
    assert CommandType.SET_PULSE_WIDTH.value == 17
    assert CommandType.SET_SPINDLE_RPM.value == 18
    assert CommandType.SET_COOLANT.value == 20
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
        CommandType.SET_FEED_RATE,
        CommandType.SET_RAPID_RATE,
        CommandType.SET_FREQUENCY,
        CommandType.SET_PULSE_WIDTH,
        CommandType.SET_HEAD,
        CommandType.SET_SPINDLE_RPM,
        CommandType.SET_COOLANT,
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


def test_coolant_mode_constants():
    assert CoolantMode.OFF.name == "OFF"
    assert CoolantMode.FLOOD.name == "FLOOD"
    assert CoolantMode.MIST.name == "MIST"
    assert CoolantMode.AIR.name == "AIR"
    assert CoolantMode.OFF.value == 0
    assert CoolantMode.FLOOD.value == 1
    assert CoolantMode.MIST.value == 2
    assert CoolantMode.AIR.value == 3


def test_coolant_mode_repr():
    assert repr(CoolantMode.OFF) == "CoolantMode.OFF"
    assert repr(CoolantMode.FLOOD) == "CoolantMode.FLOOD"
