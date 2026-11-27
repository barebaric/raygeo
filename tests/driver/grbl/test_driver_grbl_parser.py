"""Unit tests for the pure GRBL parsers (port of test_grbl_util.py)."""

import pytest

from raygeo.driver.core.types import DeviceState, DeviceStatus
from raygeo.driver.grbl.parser import (
    alarm_code_to_device_error,
    detect_unit_system_from_settings,
    error_code_to_device_error,
    extract_device_name,
    extract_device_name_from_output,
    gcode_to_p_number,
    is_grbl_output,
    is_report_in_inches,
    parse_board,
    parse_grbl_parser_state,
    parse_grbl_settings,
    parse_msg,
    parse_opt_flags,
    parse_opt_info,
    parse_probe_line,
    parse_setting_pairs,
    parse_state,
    parse_ver,
    parse_version,
    parse_wcs_line,
    split_realtime_commands,
    strip_gcode_comments,
    version_supports_single_axis_homing,
)


class TestParseState:
    def test_basic_mpos(self):
        state = parse_state("<Idle|MPos:1.0,2.0,3.0>")
        assert state.status == DeviceStatus.IDLE
        assert state.machine_pos == [1.0, 2.0, 3.0]

    def test_wpos_with_wco_recalculation(self):
        state = parse_state("<Idle|WPos:10.0,20.0,30.0|WCO:1.0,2.0,3.0>")
        assert state.work_pos == [10.0, 20.0, 30.0]
        assert state.wco == [1.0, 2.0, 3.0]
        assert state.machine_pos == [11.0, 22.0, 33.0]

    def test_mpos_with_wco_recalculates_wpos(self):
        state = parse_state("<Run|MPos:11.0,22.0,33.0|WCO:1.0,2.0,3.0>")
        assert state.machine_pos == [11.0, 22.0, 33.0]
        assert state.work_pos == [10.0, 20.0, 30.0]

    def test_wco_inferred_from_mpos_and_wpos(self):
        state = parse_state("<Idle|MPos:11.0,22.0,33.0|WPos:10.0,20.0,30.0>")
        assert state.wco == [1.0, 2.0, 3.0]

    def test_two_axis_position_padded_with_zero(self):
        state = parse_state("<Idle|MPos:1.5,2.5>")
        assert state.machine_pos == [1.5, 2.5, 0.0]

    def test_four_axis_position(self):
        state = parse_state("<Idle|MPos:1,2,3,4>")
        assert state.machine_pos == [1.0, 2.0, 3.0, 4.0]

    def test_feed_and_spindle(self):
        state = parse_state("<Run|MPos:0,0,0|FS:500,1000>")
        assert state.feed_rate == 500
        assert state.spindle_speed is None

    def test_buffer_state(self):
        state = parse_state("<Idle|MPos:0,0,0|Bf:15,127>")
        assert state.buffer_available == 15
        assert state.buffer_rx_available == 127

    def test_alarm_with_error_code(self):
        state = parse_state("<Alarm:1|MPos:0,0,0>")
        assert state.status == DeviceStatus.ALARM
        assert state.error is not None
        assert state.error.code == 1
        assert state.error.title == "Hard Limit"

    def test_hold_substate_is_not_an_error(self):
        state = parse_state("<Hold:0|MPos:0,0,0>")
        assert state.status == DeviceStatus.HOLD
        assert state.error is None

    def test_door_substate_is_not_an_error(self):
        state = parse_state("<Door:1|MPos:0,0,0>")
        assert state.status == DeviceStatus.DOOR
        assert state.error is None

    def test_unknown_status(self):
        state = parse_state("<Weird|MPos:0,0,0>")
        assert state.status == DeviceStatus.UNKNOWN

    def test_inch_report_converted_to_mm(self):
        state = parse_state("<Idle|MPos:1.0,2.0,3.0>", report_in_inches=True)
        assert state.machine_pos == pytest.approx([25.4, 50.8, 76.2])

    def test_default_state_used_as_base(self):
        default = DeviceState()
        state = parse_state("<Run|FS:100,0>", default=default)
        assert state.feed_rate == 100
        assert state.machine_pos == [None, None, None]

    def test_malformed_report_returns_default(self):
        default = DeviceState()
        state = parse_state("<garbage", default=default)
        assert state == default


class TestParseVer:
    def test_standard_format(self):
        assert parse_ver("[VER:1.1h.ORTUR:]") == ("1.1h", "ORTUR")

    def test_comma_format(self):
        assert parse_ver("[VER:1.0.15,20240923]") == ("1.0.15", None)

    def test_two_part_format(self):
        assert parse_ver("[VER:1.1h:]") == ("1.1h", None)

    def test_not_a_ver_line(self):
        assert parse_ver("[OPT:V,15,127]") is None

    def test_parse_version_finds_first(self):
        lines = ["[MSG:hello]", "[VER:1.1f.ROM:]"]
        assert parse_version(lines) == "1.1f"


class TestSettingsParsing:
    def test_parse_grbl_settings(self):
        lines = ["$0=10", "$13=0", "$110=500.000"]
        assert parse_grbl_settings(lines) == {
            "0": 10.0,
            "13": 0.0,
            "110": 500.0,
        }

    def test_parse_setting_pairs_preserves_raw_values(self):
        lines = ["$0=10", "$110=500.000"]
        assert parse_setting_pairs(lines) == [
            ("0", "10"),
            ("110", "500.000"),
        ]

    def test_detect_unit_system_metric(self):
        assert detect_unit_system_from_settings(["$13=0"]) == "metric"

    def test_detect_unit_system_imperial(self):
        assert detect_unit_system_from_settings(["$13=1"]) == "imperial"

    def test_detect_unit_system_absent(self):
        assert detect_unit_system_from_settings(["$0=10"]) is None

    def test_is_report_in_inches(self):
        assert is_report_in_inches(["$13=1"]) is True
        assert is_report_in_inches(["$13=0"]) is False
        assert is_report_in_inches(["$0=10"]) is False


class TestDeviceName:
    def test_machine_msg(self):
        lines = ["[MSG:machine:Sculpfun iCube]"]
        assert extract_device_name(lines) == "Sculpfun iCube"

    def test_machine_msg_typo(self):
        lines = ["[MSG:mechine:Ortur]"]
        assert extract_device_name(lines) == "Ortur"

    def test_board_line(self):
        lines = ["[BOARD:A1 Pro Laser Master]"]
        assert extract_device_name(lines) == "A1 Pro Laser Master"

    def test_board_precedes_ver(self):
        lines = ["[BOARD:A1 Pro Laser Master]", "[VER:1.1h.ORTUR:]"]
        assert extract_device_name(lines) == "A1 Pro Laser Master"

    def test_machine_msg_precedes_board(self):
        lines = ["[BOARD:A1 Pro Laser Master]", "[MSG:machine:Ortur]"]
        assert extract_device_name(lines) == "Ortur"

    def test_ver_build_name(self):
        lines = ["[VER:1.1h.ORTUR:]"]
        assert extract_device_name(lines) == "ORTUR"

    def test_unknown(self):
        assert extract_device_name(["[VER:1.1h:]"]) == "Unknown Grbl Device"

    def test_parse_board(self):
        assert parse_board("[BOARD:A1 Pro Laser Master]") == (
            "A1 Pro Laser Master"
        )
        assert parse_board("[BOARD: ]") is None
        assert parse_board("[VER:1.1h:]") is None

    def test_from_output_banner_fallback(self):
        assert (
            extract_device_name_from_output(b"Grbl 1.1h ['$' for help]\r\n")
            == "Grbl 1.1h ['$' for help]"
        )

    def test_from_output_skips_acks(self):
        assert extract_device_name_from_output(b"ok\r\nerror:20\r\n") is None

    def test_from_output_skips_status_reports(self):
        assert extract_device_name_from_output(b"<Idle|MPos:0,0,0>\r\n") is (
            None
        )


class TestRawOutputClassification:
    def test_grbl_banner(self):
        assert is_grbl_output(b"Grbl 1.1h")

    def test_grblhal_banner(self):
        assert is_grbl_output(b"GrblHAL 1.1h")

    def test_status_report(self):
        assert is_grbl_output(b"<Idle|MPos:0,0,0>")

    def test_build_info(self):
        assert is_grbl_output(b"[VER:1.1h:]")
        assert is_grbl_output(b"[OPT:V,15,127]")
        assert is_grbl_output(b"[MSG:hi]")

    def test_non_grbl(self):
        assert not is_grbl_output(b"Marlin 2.0.9")
        assert not is_grbl_output(b"")


class TestCommentStripping:
    def test_semicolon(self):
        assert strip_gcode_comments("G1 X10 ; move") == "G1 X10"

    def test_parens(self):
        assert strip_gcode_comments("G1 (comment) X10") == "G1  X10"

    def test_both(self):
        assert strip_gcode_comments("(a) G1 X1 ; tail") == "G1 X1"

    def test_whitespace_trimmed(self):
        assert strip_gcode_comments("  G1 X10  ") == "G1 X10"


class TestRealtimeSplitting:
    def test_split(self):
        gcode, realtime = split_realtime_commands(
            ["G1 X10", "?", "!", "M5", "~"]
        )
        assert gcode == ["G1 X10", "M5"]
        assert realtime == ["?", "!", "~"]


class TestWcsHelpers:
    def test_gcode_to_p_number(self):
        assert gcode_to_p_number("G54") == 1
        assert gcode_to_p_number("G55") == 2
        assert gcode_to_p_number("G59") == 6
        assert gcode_to_p_number("G53") is None
        assert gcode_to_p_number("G60") is None
        assert gcode_to_p_number("X54") is None
        assert gcode_to_p_number("Gabc") is None

    def test_parse_wcs_line(self):
        assert parse_wcs_line("[G54:1.0,2.0,3.0]") == (
            "G54",
            (1.0, 2.0, 3.0),
        )

    def test_parse_wcs_line_two_axes(self):
        assert parse_wcs_line("[G55:1.0,2.0]") == ("G55", (1.0, 2.0, 0.0))

    def test_parse_wcs_line_no_match(self):
        assert parse_wcs_line("[G53:1,2,3]") is None

    def test_parse_grbl_parser_state(self):
        lines = ["[G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]"]
        assert parse_grbl_parser_state(lines) == "G54"

    def test_parse_grbl_parser_state_none(self):
        assert parse_grbl_parser_state(["ok"]) is None


class TestProbeParsing:
    def test_success(self):
        assert parse_probe_line("[PRB:10.0,20.0,5.0:1]") == (
            (10.0, 20.0, 5.0),
            True,
        )

    def test_failure(self):
        assert parse_probe_line("[PRB:0.0,0.0,0.0:0]") == (
            (0.0, 0.0, 0.0),
            False,
        )

    def test_negative_values(self):
        assert parse_probe_line("[PRB:-1.5,-2.5,-0.5:1]") == (
            (-1.5, -2.5, -0.5),
            True,
        )

    def test_no_match(self):
        assert parse_probe_line("ok") is None


class TestVersionHoming:
    def test_newer_than_1_1(self):
        assert version_supports_single_axis_homing(1.2, "")

    def test_1_1g_and_newer(self):
        assert version_supports_single_axis_homing(1.1, "g")
        assert version_supports_single_axis_homing(1.1, "h")

    def test_1_1f_and_older(self):
        assert not version_supports_single_axis_homing(1.1, "f")
        assert not version_supports_single_axis_homing(1.1, "")

    def test_older_versions(self):
        assert not version_supports_single_axis_homing(0.9, "j")


class TestErrorTables:
    def test_known_error(self):
        error = error_code_to_device_error("20")
        assert error.code == 20
        assert error.title == "Unsupported Command"

    def test_unknown_error(self):
        error = error_code_to_device_error("99")
        assert error.code == 99
        assert error.title == "Unknown Error"

    def test_invalid_error(self):
        error = error_code_to_device_error("abc")
        assert error.code == -1

    def test_known_alarm(self):
        alarm = alarm_code_to_device_error("3")
        assert alarm.code == 3
        assert alarm.title == "Abort Cycle"

    def test_unknown_alarm(self):
        alarm = alarm_code_to_device_error("99")
        assert alarm.code == 99
        assert alarm.title == "Unknown Alarm"

    def test_invalid_alarm(self):
        alarm = alarm_code_to_device_error("x")
        assert alarm.code == -1


class TestOptInfo:
    def test_valid(self):
        assert parse_opt_info("[OPT:V,15,127]") == 127
        assert parse_opt_info("[OPT:VMPH,63,511]") == 511

    def test_invalid(self):
        assert parse_opt_info("[VER:1.1h:]") is None
        assert parse_opt_info("ok") is None


class TestOptFlags:
    def test_valid(self):
        assert parse_opt_flags("[OPT:V,15,127]") == "V"
        assert parse_opt_flags("[OPT:VMPH,63,511]") == "VMPH"

    def test_invalid(self):
        assert parse_opt_flags("[VER:1.1h:]") is None
        assert parse_opt_flags("ok") is None


class TestParseMsg:
    def test_valid(self):
        assert parse_msg("[MSG:machine:Ortur]") == ("machine", "Ortur")

    def test_no_colon(self):
        assert parse_msg("[MSG:hello]") is None

    def test_not_msg(self):
        assert parse_msg("[VER:1.1h:]") is None
