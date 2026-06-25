import io
import json
import math
from contextlib import redirect_stdout

import numpy as np
import pytest

from raygeo.geo import Geometry
from raygeo.ops import Ops
from raygeo.ops.axis import Axis
from raygeo.ops.state import CoolantMode, State
from raygeo.ops.types import CommandCategory, CommandType, SectionType


@pytest.fixture
def empty_ops():
    return Ops()


@pytest.fixture
def sample_ops():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.set_power(0.5)
    ops.set_coolant(CoolantMode.AIR)
    return ops


def test_initialization(empty_ops):
    assert len(empty_ops) == 0
    assert empty_ops.last_move_to == (0.0, 0.0, 0.0)


def test_add_commands(empty_ops):
    empty_ops.move_to(5, 5)
    assert empty_ops.len() == 1
    assert empty_ops.command_type(0) == CommandType.MOVE_TO

    empty_ops.line_to(10, 10)
    assert empty_ops.command_type(1) == CommandType.LINE_TO


def test_clear_commands(sample_ops):
    sample_ops.clear()
    assert len(sample_ops) == 0


def test_ops_addition(sample_ops):
    ops2 = Ops()
    ops2.move_to(20, 20)
    combined = sample_ops + ops2
    assert len(combined) == len(sample_ops) + len(ops2)


def test_ops_multiplication(sample_ops):
    multiplied = sample_ops * 3
    assert len(multiplied) == 3 * len(sample_ops)


def test_ops_extend(sample_ops):
    # Create another Ops object to extend with
    ops2 = Ops()
    ops2.move_to(20, 20)
    ops2.set_feed_rate(1000)

    original_len = len(sample_ops)
    len_to_add = len(ops2)

    # Perform the extend operation
    sample_ops.extend(ops2)

    # Verify the length has increased correctly
    assert len(sample_ops) == original_len + len_to_add

    # Verify the last two commands match what was appended
    assert sample_ops.command_type(-2) == CommandType.MOVE_TO
    assert sample_ops.endpoint(-2) == (20, 20, 0)
    assert sample_ops.command_type(-1) == CommandType.SET_FEED_RATE


def test_ops_extend_with_empty(sample_ops):
    empty_ops = Ops()
    original_len = len(sample_ops)
    sample_ops.extend(empty_ops)
    assert len(sample_ops) == original_len


def test_ops_extend_with_none(sample_ops):
    original_len = len(sample_ops)
    # This test is just to ensure it doesn't raise an exception.
    # The type hint is `Ops`, so this would be a type error, but
    # robust code should handle it.
    sample_ops.extend(None)  # type: ignore
    assert len(sample_ops) == original_len


def test_copy():
    ops_original = Ops()
    ops_original.move_to(10, 10)
    ops_original.line_to(20, 20)
    ops_original.last_move_to = (10, 10, 0)

    ops_copy = ops_original.copy()

    # Check for deepcopy: objects should not be the same instance
    assert ops_original is not ops_copy
    assert ops_original.last_move_to == ops_copy.last_move_to

    # Modify the copy and check that original is unchanged
    ops_copy.translate(5, 5)
    ops_copy.set_power(1.0)

    assert len(ops_original) == 2
    assert ops_original.endpoint(0) == (10, 10, 0)
    assert ops_original.endpoint(1) == (20, 20, 0)

    assert len(ops_copy) == 3
    assert ops_copy.endpoint(0) == (15, 15, 0)
    assert ops_copy.endpoint(1) == (25, 25, 0)


def test_preload_state(sample_ops):
    sample_ops.preload_state()

    # Verify that non-state commands have their state attribute set
    for i in range(sample_ops.len()):
        if sample_ops.category(i) != CommandCategory.STATE:
            info = sample_ops.inspect(i)
            assert info.state is not None
            assert isinstance(info.state, State)

    # Verify that state commands are still present in the commands list
    state_count = sum(
        1
        for i in range(sample_ops.len())
        if sample_ops.category(i) == CommandCategory.STATE
    )
    assert state_count > 0


def test_move_to(sample_ops):
    sample_ops.move_to(15, 15)
    last_idx = sample_ops.len() - 1
    assert sample_ops.command_type(last_idx) == CommandType.MOVE_TO
    assert sample_ops.endpoint(last_idx) == (15.0, 15.0, 0.0)


def test_line_to(sample_ops):
    sample_ops.line_to(20, 20)
    last_idx = sample_ops.len() - 1
    assert sample_ops.command_type(last_idx) == CommandType.LINE_TO
    assert sample_ops.endpoint(last_idx) == (20.0, 20.0, 0.0)


def test_close_path(sample_ops):
    sample_ops.move_to(5, 5, -1.0)
    sample_ops.close_path()
    last_idx = sample_ops.len() - 1
    assert sample_ops.command_type(last_idx) == CommandType.LINE_TO
    assert sample_ops.endpoint(last_idx) == sample_ops.last_move_to
    assert sample_ops.endpoint(last_idx) == (5.0, 5.0, -1.0)


def test_arc_to(sample_ops):
    sample_ops.arc_to(5, 5, 2, 3, clockwise=False)
    last_idx = sample_ops.len() - 1
    assert sample_ops.command_type(last_idx) == CommandType.ARC_TO
    assert sample_ops.endpoint(last_idx) == (5.0, 5.0, 0.0)
    i, j, cw = sample_ops.arc_params(last_idx)
    assert cw is False


def test_bezier_to():
    ops = Ops()
    ops.move_to(0.0, 0.0, 10.0)
    ops.bezier_to(
        control1=(1.0, 1.0, 10.0),
        control2=(2.0, 1.0, 20.0),
        end=(3.0, 0.0, 20.0),
    )

    assert len(ops) == 2  # move_to + bezier_to
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.BEZIER_TO
    info = ops.inspect(1)
    assert info.end == (3.0, 0.0, 20.0)
    assert info.control1 == (1.0, 1.0, 10.0)
    assert info.control2 == (2.0, 1.0, 20.0)


def test_bezier_to_no_start_point():
    ops = Ops()
    ops.bezier_to((1, 1, 1), (2, 2, 2), (3, 3, 3))
    assert not ops.is_empty()
    assert ops.command_type(0) == CommandType.BEZIER_TO
    info = ops.inspect(0)
    assert info.end == (3.0, 3.0, 3.0)
    assert info.control1 == (1.0, 1.0, 1.0)
    assert info.control2 == (2.0, 2.0, 2.0)


def test_set_power(sample_ops):
    sample_ops.set_power(0.8)
    last_idx = sample_ops.len() - 1
    assert sample_ops.command_type(last_idx) == CommandType.SET_POWER
    assert sample_ops.inspect(last_idx).power == 0.8


def test_set_feed_rate(sample_ops):
    sample_ops.set_feed_rate(300)
    last_idx = sample_ops.len() - 1
    assert sample_ops.command_type(last_idx) == CommandType.SET_FEED_RATE
    assert sample_ops.inspect(last_idx).feed_rate == 300


def test_set_rapid_rate(sample_ops):
    sample_ops.set_rapid_rate(2000)
    last_idx = sample_ops.len() - 1
    assert sample_ops.command_type(last_idx) == CommandType.SET_RAPID_RATE
    assert sample_ops.inspect(last_idx).rapid_rate == 2000.0


def test_set_head():
    ops = Ops()
    ops.set_head("head-abc")
    last_idx = ops.len() - 1
    assert ops.command_type(last_idx) == CommandType.SET_HEAD
    assert ops.inspect(last_idx).head_uid == "head-abc"


def test_set_frequency():
    ops = Ops()
    ops.set_frequency(20000)
    last_idx = ops.len() - 1
    assert ops.command_type(last_idx) == CommandType.SET_FREQUENCY
    assert ops.inspect(last_idx).frequency == 20000


def test_set_pulse_width():
    ops = Ops()
    ops.set_pulse_width(5.0)
    last_idx = ops.len() - 1
    assert ops.command_type(last_idx) == CommandType.SET_PULSE_WIDTH
    assert ops.inspect(last_idx).pulse_width == 5.0


def test_set_spindle_rpm():
    ops = Ops()
    ops.set_spindle_rpm(12000)
    assert ops.command_type(0) == CommandType.SET_SPINDLE_RPM
    assert ops.spindle_rpm(0) == 12000


def test_set_coolant():
    ops = Ops()
    ops.set_coolant(CoolantMode.FLOOD)
    assert ops.command_type(0) == CommandType.SET_COOLANT
    assert ops.coolant(0) == "Flood"


def test_spindle_speed_and_coolant_inspect():
    ops = Ops()
    ops.set_spindle_rpm(8000)
    ops.set_coolant(CoolantMode.MIST)
    si = ops.inspect(0)
    assert si.type_ == CommandType.SET_SPINDLE_RPM
    assert si.spindle_rpm == 8000
    assert si.coolant is None
    ci = ops.inspect(1)
    assert ci.type_ == CommandType.SET_COOLANT
    assert ci.coolant == "Mist"
    assert ci.spindle_rpm is None


def test_spindle_speed_and_coolant_are_state():
    ops = Ops()
    ops.set_spindle_rpm(5000)
    assert ops.is_state(0)
    ops.set_coolant(CoolantMode.AIR)
    assert ops.is_state(1)


def test_default_spindle_speed_and_coolant():
    ops = Ops()
    with pytest.raises(IndexError):
        ops.spindle_rpm(0)
    with pytest.raises(IndexError):
        ops.coolant(0)


def test_spindle_speed_type_error():
    ops = Ops()
    ops.move_to(0, 0, 0)
    with pytest.raises(TypeError):
        ops.spindle_rpm(0)


def test_coolant_type_error():
    ops = Ops()
    ops.move_to(0, 0, 0)
    with pytest.raises(TypeError):
        ops.coolant(0)


def test_scan_to(empty_ops):
    """Test the scan_to method with default and custom power values."""
    # Test with default power values
    empty_ops.scan_to(10, 20, 5)
    last_idx = empty_ops.len() - 1
    assert empty_ops.command_type(last_idx) == CommandType.SCAN_LINE
    assert empty_ops.endpoint(last_idx) == (10.0, 20.0, 5.0)
    assert bytes(empty_ops.scanline_data(last_idx)) == bytearray([255])

    # Test with custom power values
    custom_power = bytearray([100, 150, 200, 150, 100])
    empty_ops.scan_to(30, 40, 2, custom_power)
    last_idx = empty_ops.len() - 1
    assert empty_ops.command_type(last_idx) == CommandType.SCAN_LINE
    assert empty_ops.endpoint(last_idx) == (30.0, 40.0, 2.0)
    assert bytes(empty_ops.scanline_data(last_idx)) == custom_power


def test_rect_default_ignores_travel():
    """Tests that Ops.rect() ignores travel moves by default."""
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.move_to(100, 100)  # This move should be ignored
    min_x, min_y, max_x, max_y = ops.rect()
    assert (min_x, min_y, max_x, max_y) == (0.0, 0.0, 10.0, 10.0)


def test_rect_includes_travel():
    """Tests that Ops.rect(include_travel=True) includes travel moves."""
    ops = Ops()
    ops.move_to(-20, -20)
    ops.line_to(10, 10)
    ops.move_to(100, 100)  # This move should be included
    min_x, min_y, max_x, max_y = ops.rect(include_travel=True)
    # Points considered: (-20,-20), (10,10) from first segment,
    # and (10,10), (100,100) from second
    assert (min_x, min_y, max_x, max_y) == (-20.0, -20.0, 100.0, 100.0)


def test_distance(sample_ops):
    sample_ops.move_to(20, 20, -5)  # Travel with Z change
    distance = sample_ops.distance()
    # Distance should be 2D
    expected = math.dist((0, 0), (10, 10)) + math.dist((10, 10), (20, 20))
    assert distance == pytest.approx(expected)


def test_cut_distance(sample_ops):
    # Add a travel move to ensure it's not counted
    sample_ops.move_to(100, 100)
    cut_dist = sample_ops.cut_distance()
    # Only the initial line_to(10, 10) from (0,0) should be counted
    expected = math.hypot(10, 10)
    assert cut_dist == pytest.approx(expected)


def test_segments(sample_ops):
    sample_ops.move_to(5, 5)  # Travel command
    segments = list(sample_ops.segment_indices())
    assert len(segments) > 0
    # First segment should end before the travel command
    assert sample_ops.is_cutting(segments[0][-1])


def test_preload_state_application():
    ops = Ops()
    ops.set_power(0.3)
    ops.line_to(5, 5)
    ops.set_feed_rate(200)
    ops.preload_state()

    state1 = ops.inspect(1).state
    assert state1 is not None
    assert state1.power == 0.3

    state_count = sum(
        1 for i in range(ops.len()) if ops.category(i) == CommandCategory.STATE
    )
    assert state_count == 2

    for i in range(ops.len()):
        if ops.category(i) != CommandCategory.STATE:
            info = ops.inspect(i)
            assert info.state is not None
            assert isinstance(info.state, State)


def test_from_geometry():
    # Use the actual Geometry class instead of mocks to ensure correct types
    geo_obj = Geometry()
    geo_obj.move_to(10, 10, 0)
    geo_obj.line_to(20, 20, 0)
    geo_obj.arc_to(30, 10, -10, 0, clockwise=False, z=0)

    ops = Ops.from_geometry(geo_obj)

    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == (10, 10, 0)
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.endpoint(1) == (20, 20, 0)
    assert ops.command_type(2) == CommandType.ARC_TO
    assert ops.endpoint(2) == (30, 10, 0)
    i, j, cw = ops.arc_params(2)
    assert (i, j) == (-10, 0)
    assert cw is False
    assert ops.last_move_to == geo_obj.last_move_to


def test_from_geometry_with_bezier():
    geo_obj = Geometry()
    geo_obj.move_to(10, 10, 0)
    geo_obj.line_to(20, 20, 0)
    geo_obj.arc_to_as_bezier(30, 10, -10, 0, clockwise=False, z=0)

    ops = Ops.from_geometry(geo_obj)

    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == pytest.approx((10, 10, 0))
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.endpoint(1) == pytest.approx((20, 20, 0))
    assert all(
        ops.command_type(i) == CommandType.BEZIER_TO
        for i in range(2, ops.len())
    )
    assert ops.endpoint(ops.len() - 1) == pytest.approx((30, 10, 0))
    assert ops.last_move_to == geo_obj.last_move_to


def test_serialization_deserialization_all_types():
    """Tests that all command types can be serialized and deserialized."""
    ops = Ops()
    ops.job_start()
    ops.layer_start("layer-1")
    ops.workpiece_start("wp-1")
    ops.ops_section_start(SectionType.RASTER_FILL, "wp-1")
    ops.set_rapid_rate(5000)
    ops.set_feed_rate(1000)
    ops.set_power(0.8)
    ops.set_coolant(CoolantMode.AIR)
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


def test_dump(sample_ops):
    """Ensures dump() runs without error and produces output."""
    f = io.StringIO()
    with redirect_stdout(f):
        sample_ops.dump()
    output = f.getvalue()
    assert "MOVE_TO" in output
    assert "LINE_TO" in output


def test_numpy_serialization_round_trip_all_commands():
    """
    Tests the NumPy serialization round trip with all command types to ensure
    the hybrid serialization strategy works correctly.
    """
    ops = Ops()
    # Add one of each command type
    ops.job_start()  # Marker
    ops.layer_start("layer-1")  # Marker with data
    ops.set_rapid_rate(6000)  # State with data
    ops.set_feed_rate(1500)  # State with data
    ops.set_power(0.75)  # State with data
    ops.set_coolant(CoolantMode.AIR)  # State
    ops.set_head("head-xyz")  # State with data
    ops.move_to(1, 2, 3)  # Geometric
    ops.line_to(4, 5, 6)  # Geometric
    ops.arc_to(x=7, y=8, z=9, i=1, j=-1, clockwise=False)  # Geometric
    ops.bezier_to(
        control1=(8.0, 9.0, 10.0),
        control2=(9.0, 10.0, 11.0),
        end=(10.0, 11.0, 12.0),
    )
    ops.scan_to(10, 11, 12, bytearray([10, 20, 30]))  # Geometric
    ops.set_coolant(CoolantMode.OFF)  # State
    ops.layer_end("layer-1")  # Marker with data
    ops.job_end()  # Marker

    # Perform the round trip
    arrays = ops.to_numpy_arrays()
    reconstructed_ops = Ops.from_numpy_arrays(arrays)

    # Assertions
    assert len(reconstructed_ops) == len(ops)
    for i in range(ops.len()):
        assert ops.inspect(i) == reconstructed_ops.inspect(i)


def test_numpy_serialization_structure_hybrid():
    """
    Verifies the internal structure of the serialized arrays for a hybrid
    set of commands.
    """
    ops = Ops()
    ops.move_to(1, 1, 1)  # Geometric, index 0
    ops.set_power(0.5)  # State, index 1
    ops.line_to(2, 2, 2)  # Geometric, index 2

    arrays = ops.to_numpy_arrays()

    # Check that the JSON byte array exists and has content
    assert "state_marker_json_bytes" in arrays
    json_bytes = arrays["state_marker_json_bytes"]
    assert json_bytes.size > 0

    # Decode and verify the content
    json_str = json_bytes.tobytes().decode("utf-8")
    data = json.loads(json_str)

    # The dictionary should contain the data for the command at index 1
    assert "1" in data
    assert "0" not in data
    assert "2" not in data
    assert data["1"]["type"] == "SET_POWER"
    assert data["1"]["power"] == 0.5

    # Verify that geometric data is still correctly placed
    assert np.allclose(arrays["endpoints"][0], [1, 1, 1])
    assert np.allclose(arrays["endpoints"][2], [2, 2, 2])
    # The endpoint for the state command should be zero, as it's not used
    assert np.allclose(arrays["endpoints"][1], [0, 0, 0])


def test_numpy_serialization_round_trip_only_state():
    """Tests round-trip with only state/marker commands."""
    ops = Ops()
    ops.set_power(0.9)
    ops.set_head("head-abc")
    ops.layer_start("my-layer")

    arrays = ops.to_numpy_arrays()
    reconstructed_ops = Ops.from_numpy_arrays(arrays)

    assert len(reconstructed_ops) == 3
    for i in range(ops.len()):
        assert ops.inspect(i) == reconstructed_ops.inspect(i)


def test_numpy_serialization_round_trip_empty():
    """Tests round-trip with an empty Ops object."""
    ops = Ops()
    arrays = ops.to_numpy_arrays()
    reconstructed_ops = Ops.from_numpy_arrays(arrays)
    assert reconstructed_ops.is_empty()


def test_numpy_serialization_bezier_arrays():
    """Verifies bezier_data and bezier_map are populated correctly."""
    ops = Ops()
    ops.move_to(0, 0, 0)  # index 0
    ops.bezier_to(
        control1=(1, 2, 3), control2=(4, 5, 6), end=(7, 8, 9)
    )  # index 1
    ops.line_to(10, 10, 10)  # index 2
    ops.bezier_to(
        control1=(11, 12, 13), control2=(14, 15, 16), end=(17, 18, 19)
    )  # index 3

    arrays = ops.to_numpy_arrays()

    assert "bezier_data" in arrays
    assert "bezier_map" in arrays
    assert arrays["bezier_data"].shape == (2, 6)
    assert arrays["bezier_map"][0] == -1
    assert arrays["bezier_map"][1] == 0
    assert arrays["bezier_map"][2] == -1
    assert arrays["bezier_map"][3] == 1

    np.testing.assert_allclose(arrays["bezier_data"][0], [1, 2, 3, 4, 5, 6])
    np.testing.assert_allclose(
        arrays["bezier_data"][1], [11, 12, 13, 14, 15, 16]
    )


def test_numpy_serialization_bezier_round_trip():
    """Tests that BezierToCommand survives numpy serialization."""
    ops = Ops()
    ops.move_to(0, 0, 0)
    ops.bezier_to(
        control1=(1.5, 2.5, 3.5), control2=(4.5, 5.5, 6.5), end=(7.5, 8.5, 9.5)
    )
    ops.set_power(0.5)
    ops.bezier_to(
        control1=(10, 20, 30),
        control2=(40, 50, 60),
        end=(70, 80, 90),
    )

    arrays = ops.to_numpy_arrays()
    reconstructed = Ops.from_numpy_arrays(arrays)

    assert len(reconstructed) == 4
    assert reconstructed.command_type(0) == CommandType.MOVE_TO
    assert reconstructed.command_type(1) == CommandType.BEZIER_TO
    assert reconstructed.command_type(2) == CommandType.SET_POWER
    assert reconstructed.command_type(3) == CommandType.BEZIER_TO

    info1 = reconstructed.inspect(1)
    assert info1.end == pytest.approx((7.5, 8.5, 9.5))
    assert info1.control1 == pytest.approx((1.5, 2.5, 3.5))
    assert info1.control2 == pytest.approx((4.5, 5.5, 6.5))

    info3 = reconstructed.inspect(3)
    assert info3.end == pytest.approx((70, 80, 90))
    assert info3.control1 == pytest.approx((10, 20, 30))
    assert info3.control2 == pytest.approx((40, 50, 60))


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


# --- Extra Axes Numpy Serialization Tests ---


def test_extra_axes_numpy_no_extra_axes_produces_no_key():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    arrays = ops.to_numpy_arrays()
    assert "extra_axes_json" not in arrays


def test_extra_axes_numpy_round_trip_with_extra_axes():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10, extra={Axis.A: 45.0})
    ops.arc_to(
        20,
        20,
        5,
        5,
        extra={Axis.B: 90.0},
    )
    ops.line_to(30, 30)

    arrays = ops.to_numpy_arrays()
    assert "extra_axes_json" in arrays

    restored = Ops.from_numpy_arrays(arrays)
    assert len(restored) == 4
    assert restored.inspect(0).extra_axes is None
    assert restored.inspect(1).extra_axes == {Axis.A: 45.0}
    assert restored.inspect(2).extra_axes == {Axis.B: 90.0}
    assert restored.inspect(3).extra_axes is None


def test_extra_axes_numpy_old_arrays_deserialize():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    arrays = ops.to_numpy_arrays()
    assert "extra_axes_json" not in arrays

    restored = Ops.from_numpy_arrays(arrays)
    for i in range(restored.len()):
        if restored.category(i) == CommandCategory.MOVING:
            assert restored.inspect(i).extra_axes is None


def test_extra_axes_numpy_round_trip_preserves_all_data():
    ops = Ops()
    ops.set_power(0.5)
    ops.move_to(0, 0)
    ops.line_to(10, 10, extra={Axis.A: 30.0})
    ops.scan_to(
        20,
        20,
        0,
        bytearray([100, 200]),
        extra={Axis.A: 60.0},
    )
    ops.set_power(0.8)

    arrays = ops.to_numpy_arrays()
    restored = Ops.from_numpy_arrays(arrays)

    for i in range(ops.len()):
        assert ops.inspect(i) == restored.inspect(i)


# --- Extra Axes Convenience Tests ---


def test_move_to_no_extra():
    ops = Ops()
    ops.move_to(10, 20)
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == (10.0, 20.0, 0.0)
    assert ops.inspect(0).extra_axes is None


def test_move_to_with_extra():
    ops = Ops()
    ops.move_to(10, 20, 0, extra={Axis.A: 45.0})
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.inspect(0).extra_axes == {Axis.A: 45.0}


def test_line_to_with_extra():
    ops = Ops()
    ops.line_to(10, 20, extra={Axis.A: 90.0})
    assert ops.command_type(0) == CommandType.LINE_TO
    assert ops.inspect(0).extra_axes == {Axis.A: 90.0}


def test_arc_to_with_extra():
    ops = Ops()
    ops.arc_to(5, 5, 2, 3, extra={Axis.A: 45.0})
    assert ops.command_type(0) == CommandType.ARC_TO
    assert ops.inspect(0).extra_axes == {Axis.A: 45.0}


def test_bezier_to_with_extra():
    ops = Ops()
    ops.move_to(0, 0)
    ops.bezier_to(
        control1=(1, 1, 0),
        control2=(2, 2, 0),
        end=(3, 3, 0),
        extra={Axis.A: 45.0},
    )
    assert ops.command_type(1) == CommandType.BEZIER_TO
    assert ops.inspect(1).extra_axes == {Axis.A: 45.0}


def test_scan_to_with_extra():
    ops = Ops()
    ops.scan_to(
        10,
        20,
        0,
        bytearray([100, 200]),
        extra={Axis.A: 45.0},
    )
    assert ops.command_type(0) == CommandType.SCAN_LINE
    assert ops.inspect(0).extra_axes == {Axis.A: 45.0}


def test_scanline_count_empty():
    assert Ops().scanline_count == 0


def test_scanline_count_no_scanlines():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    assert ops.scanline_count == 0


def test_scanline_count_with_scanlines():
    ops = Ops()
    ops.move_to(0, 0)
    ops.scan_to(10, 0, 0, bytearray([100, 200]))
    ops.line_to(20, 0)
    ops.scan_to(30, 0, 0, bytearray([50]))
    assert ops.scanline_count == 2


def test_to_geometry():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.arc_to(20, 0, 5, 0, False)
    ops.set_power(1.0)
    ops.bezier_to((15, 15, 0), (25, 15, 0), (30, 0, 0))
    geo = ops.to_geometry()
    assert isinstance(geo, Geometry)


def test_to_geometry_empty():
    ops = Ops()
    geo = ops.to_geometry()
    assert isinstance(geo, Geometry)


def test_is_scanline():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.scan_to(20, 0, 0, power_values=bytearray([128] * 10))

    assert not ops.is_scanline(0)
    assert not ops.is_scanline(1)
    assert ops.is_scanline(2)


def test_distance_at():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)

    assert ops.distance_at(1, (0.0, 0.0, 0.0)) == pytest.approx(10.0)


def test_distance_at_diagonal():
    ops = Ops()
    ops.line_to(3, 4)
    dist = ops.distance_at(0, (0.0, 0.0, 0.0))
    assert dist == pytest.approx(5.0)


def test_distance_at_state_command():
    ops = Ops()
    ops.set_power(1.0)
    assert ops.distance_at(0, (0.0, 0.0, 0.0)) == 0.0


def test_distance_at_none_last_point():
    ops = Ops()
    ops.line_to(10, 0)
    assert ops.distance_at(0, None) == 0.0


def test_state_at():
    ops = Ops()
    ops.set_power(0.5)
    ops.set_feed_rate(800)
    ops.set_coolant(CoolantMode.AIR)
    ops.move_to(0, 0)

    state = ops.state_at(3)
    assert state.power == 0.5
    assert state.feed_rate == 800
    assert state.coolant == CoolantMode.AIR


def test_state_at_no_state_commands():
    ops = Ops()
    ops.move_to(0, 0)
    state = ops.state_at(0)
    assert state.power == 0.0
    assert state.coolant is None


def test_state_at_mid_sequence():
    ops = Ops()
    ops.set_power(0.3)
    ops.set_feed_rate(500)
    ops.set_power(0.8)
    ops.move_to(0, 0)

    state_0 = ops.state_at(0)
    assert state_0.power == 0.3
    assert state_0.feed_rate is None

    state_1 = ops.state_at(1)
    assert state_1.feed_rate == 500

    state_2 = ops.state_at(2)
    assert state_2.power == 0.8
    assert state_2.feed_rate == 500


def test_copy_command_from():
    src = Ops()
    src.move_to(5, 5)
    src.line_to(10, 10)

    dst = Ops()
    dst.copy_command_from(src, 1)
    assert dst.len() == 1
    assert dst.command_type(0) == CommandType.LINE_TO
    assert dst.endpoint(0) == (10.0, 10.0, 0.0)


def test_copy_command_from_is_deep():
    src = Ops()
    src.line_to(10, 0)
    dst = Ops()
    dst.copy_command_from(src, 0)
    dst.move_to(99, 99)
    assert src.len() == 1


def test_transfer_command_from():
    src = Ops()
    src.move_to(5, 5)
    src.line_to(10, 10)

    dst = Ops()
    dst.transfer_command_from(src, 1)
    assert dst.len() == 1
    assert dst.command_type(0) == CommandType.LINE_TO


def test_dwell():
    ops = Ops()
    ops.dwell(250.0)
    assert ops.len() == 1
    assert ops.command_type(0) == CommandType.DWELL


def test_dwell_duration():
    ops = Ops()
    ops.dwell(150.0)
    assert ops.dwell_duration(0) == 150.0


def test_dwell_duration_wrong_type():
    ops = Ops()
    ops.move_to(0, 0)
    with pytest.raises(TypeError):
        ops.dwell_duration(0)


def test_rate_feed():
    ops = Ops()
    ops.set_feed_rate(1200)
    assert ops.rate(0) == 1200


def test_rate_rapid():
    ops = Ops()
    ops.set_rapid_rate(3000)
    assert ops.rate(0) == 3000


def test_rate_wrong_type():
    ops = Ops()
    ops.move_to(0, 0)
    with pytest.raises(TypeError):
        ops.rate(0)


def test_head_uid():
    ops = Ops()
    ops.set_head("head_42")
    assert ops.head_uid(0) == "head_42"


def test_head_uid_wrong_type():
    ops = Ops()
    ops.move_to(0, 0)
    with pytest.raises(TypeError):
        ops.head_uid(0)


def test_section_params_start():
    ops = Ops()
    ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
    st, uid = ops.section_params(0)
    assert st == SectionType.RASTER_FILL
    assert uid == "wp1"


def test_section_params_end():
    ops = Ops()
    ops.ops_section_end(SectionType.RASTER_FILL)
    st, uid = ops.section_params(0)
    assert st == SectionType.RASTER_FILL
    assert uid is None


def test_section_params_wrong_type():
    ops = Ops()
    ops.move_to(0, 0)
    with pytest.raises(TypeError):
        ops.section_params(0)


def test_replace_with():
    src = Ops()
    src.move_to(5, 5)
    src.line_to(10, 10)
    src.last_move_to = (5.0, 5.0, 0.0)

    dst = Ops()
    dst.move_to(0, 0)
    dst.replace_with(src)
    assert dst.len() == 2
    assert dst.command_type(0) == CommandType.MOVE_TO
    assert dst.endpoint(0) == (5.0, 5.0, 0.0)
    assert dst.last_move_to == (5.0, 5.0, 0.0)


def test_without_state():
    ops = Ops()
    ops.set_power(1.0)
    ops.move_to(0, 0)
    ops.set_feed_rate(800)
    ops.line_to(10, 0)
    ops.set_coolant(CoolantMode.AIR)

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


# --- apply_state ---


def test_apply_state_full():
    """All set fields are emitted as state commands."""
    ops = Ops()
    state = State(
        power=0.7,
        feed_rate=1200,
        rapid_rate=4000,
        spindle_rpm=18000,
        coolant=CoolantMode.FLOOD,
        frequency=5000,
        pulse_width=12.5,
        active_head_uid="head-1",
    )
    ops.apply_state(state)
    assert ops.len() == 8
    assert ops.power(0) == pytest.approx(0.7)
    assert ops.rate(1) == 1200
    assert ops.rate(2) == 4000
    assert ops.spindle_rpm(3) == 18000
    assert ops.coolant(4) == "Flood"
    assert ops.frequency(5) == 5000
    assert ops.pulse_width(6) == pytest.approx(12.5)
    assert ops.head_uid(7) == "head-1"


def test_apply_state_default():
    """Default State (power=0.0, rest None) emits only SetPower."""
    ops = Ops()
    state = State()
    ops.apply_state(state)
    assert ops.len() == 1
    assert ops.command_type(0) == CommandType.SET_POWER
    assert ops.power(0) == 0.0


def test_apply_state_power_always_emitted():
    """Power is emitted even when 0.0 (it has no Option wrapper)."""
    ops = Ops()
    state = State(power=0.0)
    ops.apply_state(state)
    assert ops.len() == 1
    assert ops.power(0) == 0.0


def test_apply_state_partial():
    """Only set fields are emitted; None fields produce no command."""
    ops = Ops()
    state = State(power=0.3, feed_rate=600, coolant=CoolantMode.MIST)
    ops.apply_state(state)
    assert ops.len() == 3
    assert ops.power(0) == pytest.approx(0.3)
    assert ops.rate(1) == 600
    assert ops.coolant(2) == "Mist"


def test_apply_state_accumulates():
    """apply_state appends; existing commands are preserved."""
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(5, 5)
    state = State(power=1.0, feed_rate=1000)
    ops.apply_state(state)
    assert ops.len() == 4
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.power(2) == pytest.approx(1.0)
    assert ops.rate(3) == 1000
