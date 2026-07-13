from raygeo.ops.state import AirAssistMode, CoolantMode, HeadCoolantMode
from raygeo.ops.types import (
    CommandCategory,
    CommandType,
    RasterMode,
    SectionType,
    category,
)


def test_state_block_command_type_values():
    assert CommandType.STATE_BLOCK_START.value == 108
    assert CommandType.STATE_BLOCK_END.value == 109


def test_state_block_category():
    assert category(CommandType.STATE_BLOCK_START) == CommandCategory.MARKER
    assert category(CommandType.STATE_BLOCK_END) == CommandCategory.MARKER


def test_state_block_command_type_names():
    assert CommandType.STATE_BLOCK_START.name == "STATE_BLOCK_START"
    assert CommandType.STATE_BLOCK_END.name == "STATE_BLOCK_END"


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
    assert CommandType.SET_AIR_ASSIST.value == 21
    assert CommandType.SET_HEAD_COOLANT.value == 22
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
        CommandType.SET_AIR_ASSIST,
        CommandType.SET_HEAD_COOLANT,
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
    assert CoolantMode.OFF.value == 0
    assert CoolantMode.FLOOD.value == 1
    assert CoolantMode.MIST.value == 2


def test_coolant_mode_repr():
    assert repr(CoolantMode.OFF) == "CoolantMode.OFF"
    assert repr(CoolantMode.FLOOD) == "CoolantMode.FLOOD"


def test_air_assist_mode_constants():
    assert AirAssistMode.OFF.name == "OFF"
    assert AirAssistMode.ON.name == "ON"
    assert AirAssistMode.OFF.value == 0
    assert AirAssistMode.ON.value == 1


def test_air_assist_mode_repr():
    assert repr(AirAssistMode.OFF) == "AirAssistMode.OFF"
    assert repr(AirAssistMode.ON) == "AirAssistMode.ON"


def test_head_coolant_mode_constants():
    assert HeadCoolantMode.OFF.name == "OFF"
    assert HeadCoolantMode.ON.name == "ON"
    assert HeadCoolantMode.OFF.value == 0
    assert HeadCoolantMode.ON.value == 1


def test_head_coolant_mode_repr():
    assert repr(HeadCoolantMode.OFF) == "HeadCoolantMode.OFF"
    assert repr(HeadCoolantMode.ON) == "HeadCoolantMode.ON"


def test_raster_mode_values():
    assert RasterMode.VARIABLE_POWER.value == 1
    assert RasterMode.CONSTANT_POWER.value == 2
    assert RasterMode.DEPTH_MAP.value == 3


def test_raster_mode_names():
    assert RasterMode.VARIABLE_POWER.name == "VARIABLE_POWER"
    assert RasterMode.CONSTANT_POWER.name == "CONSTANT_POWER"
    assert RasterMode.DEPTH_MAP.name == "DEPTH_MAP"


def test_raster_mode_repr():
    assert repr(RasterMode.VARIABLE_POWER) == "RasterMode.VARIABLE_POWER"
    assert repr(RasterMode.CONSTANT_POWER) == "RasterMode.CONSTANT_POWER"
    assert repr(RasterMode.DEPTH_MAP) == "RasterMode.DEPTH_MAP"
