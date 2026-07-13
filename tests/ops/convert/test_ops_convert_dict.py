from raygeo.ops import Ops
from raygeo.ops.axis import Axis
from raygeo.ops.state import AirAssistMode, HeadCoolantMode
from raygeo.ops.types import CommandType, RasterMode, SectionType


def test_serialization_deserialization_all_types():
    ops = Ops()
    ops.job_start()
    ops.layer_start("layer-1")
    ops.workpiece_start("wp-1")
    ops.ops_section_start(SectionType.RASTER_FILL, "wp-1")
    ops.set_rapid_rate(5000)
    ops.set_feed_rate(1000)
    ops.set_power(0.8)
    ops.set_air_assist(AirAssistMode.ON)
    ops.set_head_coolant(HeadCoolantMode.ON)
    ops.set_head("head-2")
    ops.move_to(1, 1, 1)
    ops.line_to(2, 2, 2)
    ops.arc_to(3, 1, 1, 1, clockwise=False)
    ops.scan_to(12, 2, 2, bytearray([50, 150]))
    ops.ops_section_end(SectionType.RASTER_FILL)
    ops.workpiece_end("wp-1")
    ops.layer_end("layer-1")
    ops.job_end()
    ops.last_move_to = (1, 1, 1)

    data = ops.to_dict()
    new_ops = Ops.from_dict(data)

    assert len(ops) == len(new_ops)
    assert new_ops.last_move_to == (1, 1, 1)

    for i in range(ops.len()):
        assert ops.inspect(i) == new_ops.inspect(i)


def test_extra_axes_to_dict_no_extra_axes():
    ops = Ops()
    ops.move_to(1, 2, 3)
    data = ops.to_dict()
    assert "extra_axes" not in data["commands"][0]


def test_extra_axes_to_dict_with_extra_axes():
    ops = Ops()
    ops.move_to(1, 2, 3, extra={Axis.A: 45.0})
    data = ops.to_dict()
    assert data["commands"][0]["extra_axes"] == {"A": 45.0}


def test_extra_axes_from_dict_no_extra_axes():
    data = {
        "commands": [
            {"type": "MOVE_TO", "end": [1, 2, 3]},
        ],
        "last_move_to": [0, 0, 0],
    }
    ops = Ops.from_dict(data)
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.inspect(0).extra_axes is None


def test_extra_axes_from_dict_with_extra_axes():
    data = {
        "commands": [
            {
                "type": "MOVE_TO",
                "end": [1, 2, 3],
                "extra_axes": {"A": 45.0},
            },
        ],
        "last_move_to": [0, 0, 0],
    }
    ops = Ops.from_dict(data)
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.inspect(0).extra_axes == {Axis.A: 45.0}


def test_extra_axes_round_trip_mixed():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10, extra={Axis.A: 45.0})
    ops.move_to(20, 20)
    ops.line_to(30, 30)

    data = ops.to_dict()
    restored = Ops.from_dict(data)

    assert len(restored) == 4
    assert restored.inspect(0).extra_axes is None
    assert restored.inspect(1).extra_axes == {Axis.A: 45.0}
    assert restored.inspect(2).extra_axes is None
    assert restored.inspect(3).extra_axes is None


def test_raster_mode_dict_round_trip():
    ops = Ops()
    ops.ops_section_start_with_mode(
        SectionType.RASTER_FILL, "wp-1", raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.move_to(0, 0)
    ops.scan_to(10, 10, power_values=bytearray([255]))
    ops.ops_section_end_with_mode(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )

    data = ops.to_dict()
    restored = Ops.from_dict(data)

    assert len(ops) == len(restored)
    sections = restored.sections()
    assert len(sections) == 1
    assert sections[0].raster_mode == RasterMode.VARIABLE_POWER


def test_state_block_dict_round_trip():
    ops = Ops()
    ops.state_block_start("my-block")
    ops.set_power(0.5)
    ops.state_block_end()

    data = ops.to_dict()
    restored = Ops.from_dict(data)

    assert len(ops) == len(restored)
    assert restored.command_type(0) == CommandType.STATE_BLOCK_START
    assert restored.command_type(2) == CommandType.STATE_BLOCK_END


def test_state_block_dict_round_trip_no_name():
    ops = Ops()
    ops.state_block_start(None)
    ops.set_power(0.5)
    ops.state_block_end()

    data = ops.to_dict()
    restored = Ops.from_dict(data)

    assert len(ops) == len(restored)
    assert restored.command_type(0) == CommandType.STATE_BLOCK_START
    assert restored.command_type(2) == CommandType.STATE_BLOCK_END


def test_raster_mode_dict_round_trip_old_format():
    data = {
        "commands": [
            {
                "type": "OPS_SECTION_START",
                "section_type": "RASTER_FILL",
                "workpiece_uid": "wp-1",
            },
            {"type": "MOVE_TO", "end": [0, 0, 0]},
            {"type": "OPS_SECTION_END", "section_type": "RASTER_FILL"},
        ],
        "last_move_to": [0, 0, 0],
    }
    ops = Ops.from_dict(data)
    assert ops.sections()[0].raster_mode is None
