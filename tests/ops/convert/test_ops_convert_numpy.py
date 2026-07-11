import json

import numpy as np

from raygeo.ops import Ops
from raygeo.ops.axis import Axis
from raygeo.ops.state import AirAssistMode, CoolantMode
from raygeo.ops.types import CommandCategory, CommandType


def test_numpy_round_trip_all_commands():
    ops = Ops()
    ops.job_start()
    ops.layer_start("layer-1")
    ops.set_rapid_rate(6000)
    ops.set_feed_rate(1500)
    ops.set_power(0.75)
    ops.set_air_assist(AirAssistMode.ON)
    ops.set_head("head-xyz")
    ops.move_to(1, 2, 3)
    ops.line_to(4, 5, 6)
    ops.arc_to(x=7, y=8, z=9, i=1, j=-1, clockwise=False)
    ops.bezier_to(
        control1=(8.0, 9.0, 10.0),
        control2=(9.0, 10.0, 11.0),
        end=(10.0, 11.0, 12.0),
    )
    ops.scan_to(10, 11, 12, bytearray([10, 20, 30]))
    ops.set_coolant(CoolantMode.OFF)
    ops.layer_end("layer-1")
    ops.job_end()

    arrays = ops.to_numpy_arrays()
    reconstructed_ops = Ops.from_numpy_arrays(arrays)

    assert len(reconstructed_ops) == len(ops)
    for i in range(ops.len()):
        assert ops.inspect(i) == reconstructed_ops.inspect(i)


def test_structure_hybrid():
    ops = Ops()
    ops.move_to(1, 1, 1)
    ops.set_power(0.5)
    ops.line_to(2, 2, 2)

    arrays = ops.to_numpy_arrays()

    assert "state_marker_json_bytes" in arrays
    json_bytes = arrays["state_marker_json_bytes"]
    assert json_bytes.size > 0

    json_str = json_bytes.tobytes().decode("utf-8")
    data = json.loads(json_str)

    assert "1" in data
    assert "0" not in data
    assert "2" not in data
    assert data["1"]["type"] == "SET_POWER"
    assert data["1"]["power"] == 0.5

    assert np.allclose(arrays["endpoints"][0], [1, 1, 1])
    assert np.allclose(arrays["endpoints"][2], [2, 2, 2])
    assert np.allclose(arrays["endpoints"][1], [0, 0, 0])


def test_round_trip_only_state():
    ops = Ops()
    ops.set_power(0.9)
    ops.set_head("head-abc")
    ops.layer_start("my-layer")

    arrays = ops.to_numpy_arrays()
    reconstructed_ops = Ops.from_numpy_arrays(arrays)

    assert len(reconstructed_ops) == 3
    for i in range(ops.len()):
        assert ops.inspect(i) == reconstructed_ops.inspect(i)


def test_round_trip_empty():
    ops = Ops()
    arrays = ops.to_numpy_arrays()
    reconstructed_ops = Ops.from_numpy_arrays(arrays)
    assert reconstructed_ops.is_empty()


def test_bezier_arrays():
    ops = Ops()
    ops.move_to(0, 0, 0)
    ops.bezier_to(control1=(1, 2, 3), control2=(4, 5, 6), end=(7, 8, 9))
    ops.line_to(10, 10, 10)
    ops.bezier_to(
        control1=(11, 12, 13), control2=(14, 15, 16), end=(17, 18, 19)
    )

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


def test_bezier_round_trip():
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


def test_numpy_no_extra_axes_produces_no_key():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    arrays = ops.to_numpy_arrays()
    assert "extra_axes_json" not in arrays


def test_numpy_round_trip_with_extra_axes():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10, extra={Axis.A: 45.0})
    ops.arc_to(20, 20, 5, 5, extra={Axis.B: 90.0})
    ops.line_to(30, 30)

    arrays = ops.to_numpy_arrays()
    assert "extra_axes_json" in arrays

    restored = Ops.from_numpy_arrays(arrays)
    assert len(restored) == 4
    assert restored.inspect(0).extra_axes is None
    assert restored.inspect(1).extra_axes == {Axis.A: 45.0}
    assert restored.inspect(2).extra_axes == {Axis.B: 90.0}
    assert restored.inspect(3).extra_axes is None


def test_numpy_old_arrays_deserialize():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    arrays = ops.to_numpy_arrays()
    assert "extra_axes_json" not in arrays

    restored = Ops.from_numpy_arrays(arrays)
    for i in range(restored.len()):
        if restored.category(i) == CommandCategory.MOVING:
            assert restored.inspect(i).extra_axes is None


def test_numpy_round_trip_preserves_all_data():
    ops = Ops()
    ops.set_power(0.5)
    ops.move_to(0, 0)
    ops.line_to(10, 10, extra={Axis.A: 30.0})
    ops.scan_to(20, 20, 0, bytearray([100, 200]), extra={Axis.A: 60.0})
    ops.set_power(0.8)

    arrays = ops.to_numpy_arrays()
    restored = Ops.from_numpy_arrays(arrays)

    for i in range(ops.len()):
        assert ops.inspect(i) == restored.inspect(i)
