"""Tests for the shared value types: DeviceState, DeviceStatus and
DeviceError.

Covers the Python-facing construction surface (setters and the
DeviceError constructor) and the merge/round-trip contract that
embedders rely on when they keep their own state object and feed it
back to ``parse_state`` as the default.
"""

from raygeo.driver.core.types import DeviceError, DeviceState, DeviceStatus
from raygeo.driver.grbl.parser import parse_state


def build_state(**overrides) -> DeviceState:
    """A fully populated DeviceState, built through the Python
    setters, with optional per-field overrides."""
    state = DeviceState()
    state.status = overrides.get("status", DeviceStatus.RUN)
    state.error = overrides.get("error", None)
    state.machine_pos = overrides.get("machine_pos", [11.0, 22.0, 33.0])
    state.work_pos = overrides.get("work_pos", [10.0, 20.0, 30.0])
    state.wco = overrides.get("wco", [1.0, 2.0, 3.0])
    state.feed_rate = overrides.get("feed_rate", 500)
    state.spindle_speed = overrides.get("spindle_speed", 1000)
    state.buffer_available = overrides.get("buffer_available", 62)
    state.buffer_rx_available = overrides.get("buffer_rx_available", 100)
    return state


class TestDeviceStateDefaults:
    def test_fresh_state_defaults(self):
        state = DeviceState()
        assert state.status == DeviceStatus.UNKNOWN
        assert state.error is None
        assert state.machine_pos == [None, None, None]
        assert state.work_pos == [None, None, None]
        assert state.wco == [0.0, 0.0, 0.0]
        assert state.feed_rate is None
        assert state.spindle_speed is None
        assert state.buffer_available is None
        assert state.buffer_rx_available is None

    def test_status_name(self):
        assert DeviceStatus.ALARM.name == "ALARM"
        assert parse_state("<Idle|MPos:1,2,3>").status.name == "IDLE"


class TestDeviceStateSetters:
    def test_set_all_fields(self):
        state = build_state()
        assert state.status == DeviceStatus.RUN
        assert state.machine_pos == [11.0, 22.0, 33.0]
        assert state.work_pos == [10.0, 20.0, 30.0]
        assert state.wco == [1.0, 2.0, 3.0]
        assert state.feed_rate == 500
        assert state.spindle_speed == 1000
        assert state.buffer_available == 62
        assert state.buffer_rx_available == 100

    def test_set_error(self):
        state = DeviceState()
        error = DeviceError(20, "Unsupported Command", "details")
        state.error = error
        assert state.error == error
        assert state.error.code == 20
        assert state.error.title == "Unsupported Command"
        assert state.error.description == "details"

    def test_clear_error_with_none(self):
        state = build_state(error=DeviceError(1, "t", "d"))
        state.error = None
        assert state.error is None

    def test_unknown_entries_survive_setter(self):
        state = DeviceState()
        state.machine_pos = [1.0, None, 3.0]
        assert state.machine_pos == [1.0, None, 3.0]

    def test_status_from_member_lookup(self):
        state = DeviceState()
        state.status = getattr(DeviceStatus, "HOLD")
        assert state.status == DeviceStatus.HOLD


class TestDeviceStateEquality:
    def test_equal_states_compare_equal(self):
        assert build_state() == build_state()

    def test_status_difference_compares_unequal(self):
        assert build_state() != build_state(status=DeviceStatus.IDLE)

    def test_error_difference_compares_unequal(self):
        plain = build_state()
        failing = build_state(error=DeviceError(9, "t", "d"))
        assert plain != failing

    def test_position_difference_compares_unequal(self):
        assert build_state() != build_state(machine_pos=[1.0, 2.0, 3.0])


class TestDeviceError:
    def test_constructor_and_getters(self):
        error = DeviceError(9, "Commands Locked", "Clear the alarm state.")
        assert error.code == 9
        assert error.title == "Commands Locked"
        assert error.description == "Clear the alarm state."

    def test_equality(self):
        assert DeviceError(1, "a", "b") == DeviceError(1, "a", "b")
        assert DeviceError(1, "a", "b") != DeviceError(2, "a", "b")

    def test_repr_contains_code_and_title(self):
        text = repr(DeviceError(7, "Memory Error", "d"))
        assert "7" in text
        assert "Memory Error" in text


class TestParseStateWithBuiltDefault:
    """parse_state merges the report into a caller-provided default;
    the default may be a state constructed through the Python
    setters (the round-trip embedders depend on)."""

    def test_report_updates_status_of_built_state(self):
        state = parse_state(
            "<Idle|MPos:1.0,2.0,3.0>", build_state(status=DeviceStatus.ALARM)
        )
        assert state.status == DeviceStatus.IDLE
        assert state.error is None

    def test_report_keeps_unreported_fields_of_built_state(self):
        state = parse_state(
            "<Idle|WPos:10.0,20.0,30.0>",
            build_state(feed_rate=500, wco=[1.0, 2.0, 3.0]),
        )
        assert state.feed_rate == 500
        assert state.machine_pos == [11.0, 22.0, 33.0]

    def test_alarm_report_sets_error_on_built_state(self):
        state = parse_state("<Alarm:1|FS:500,0>", build_state(feed_rate=100))
        assert state.status == DeviceStatus.ALARM
        assert state.error is not None
        assert state.error.code == 1
        assert state.feed_rate == 500

    def test_repeated_merge_is_stable(self):
        merged = build_state()
        for _ in range(3):
            merged = parse_state("<Hold|MPos:11.0,22.0,33.0>", merged)
        assert merged.status == DeviceStatus.HOLD
        assert merged.machine_pos == [11.0, 22.0, 33.0]
        assert merged.feed_rate == 500


class TestStateRoundTrip:
    """Copying a parsed state field-by-field through the setters
    reproduces it exactly — the contract the Rayforge conversion
    helpers implement."""

    def test_copy_via_setters_reproduces_parsed_state(self):
        parsed = parse_state("<Run|MPos:1.0,2.0,3.0|FS:800,50|Bf:14,120>")
        copy = DeviceState()
        copy.status = parsed.status
        copy.error = parsed.error
        copy.machine_pos = parsed.machine_pos
        copy.work_pos = parsed.work_pos
        copy.wco = parsed.wco
        copy.feed_rate = parsed.feed_rate
        copy.spindle_speed = parsed.spindle_speed
        copy.buffer_available = parsed.buffer_available
        copy.buffer_rx_available = parsed.buffer_rx_available
        assert copy == parsed
