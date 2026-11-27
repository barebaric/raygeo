"""Unit tests for the pure Marlin response parsers."""

from raygeo.driver.marlin.parser import (
    detect_unit_system_from_m149,
    extract_marlin_banner_from_output,
    extract_marlin_device_name,
    is_boot_message,
    is_error_response,
    is_marlin_output,
    is_ok_response,
    parse_error_message,
    parse_m114_line,
    parse_m114_position,
    parse_m115_firmware_info,
    parse_m211_endstops,
    parse_m503_settings,
    parse_marlin_version,
)


class TestAckClassification:
    def test_plain_ok(self):
        assert is_ok_response("ok")

    def test_ok_with_temperature_payload(self):
        assert is_ok_response("ok T:19.5 /200.0 B:60.0 /60.0")

    def test_ok_with_positions(self):
        assert is_ok_response("ok X:1 Y:2 Z:3")

    def test_ok_prefix_is_not_ok(self):
        assert not is_ok_response("oks")

    def test_error_response(self):
        assert is_error_response("Error:checksum mismatch")

    def test_other_line_is_not_ok(self):
        assert not is_ok_response("echo:done")


class TestBootMessages:
    def test_boot_prefixes(self):
        assert is_boot_message("start")
        assert is_boot_message("Marlin 2.1.2")
        assert is_boot_message("echo:Last Updated")
        assert is_boot_message("External Reset")

    def test_gcode_is_not_boot(self):
        assert not is_boot_message("G1 X10")

    def test_is_marlin_output(self):
        assert is_marlin_output(b"start\r\nok\r\n")
        assert not is_marlin_output(b"ok\r\n")

    def test_banner_extraction_skips_acks(self):
        banner = extract_marlin_banner_from_output(
            b"ok\r\nError:bad\r\nstart\r\nMarlin 2.1.2\r\n"
        )
        assert banner == "start"


class TestM114:
    def test_position_line(self):
        line = "X:10.00 Y:-2.50 Z:0.00 E:0.00 Count X:1000 Y:-250 Z:0"
        assert parse_m114_line(line) == (10.0, -2.5, 0.0)

    def test_position_from_lines(self):
        lines = ["echo:busy", "X:1.00 Y:2.00 Z:3.00", "ok"]
        assert parse_m114_position(lines) == (1.0, 2.0, 3.0)

    def test_no_position(self):
        assert parse_m114_position(["ok"]) is None


class TestProbes:
    def test_m115_firmware_info(self):
        info = parse_m115_firmware_info(
            ["FIRMWARE_NAME:Marlin 2.1.2 (Github)", "MACHINE_TYPE:Laser"]
        )
        assert info["firmware_name"] == "Marlin"
        assert info["machine_type"] == "Laser"

    def test_m211_endstops(self):
        assert parse_m211_endstops(
            ["echo:Maximum (mm): X:200.00 Y:200.00"]
        ) == (200.0, 200.0)

    def test_m503_settings(self):
        settings = dict(
            parse_m503_settings(
                ["echo:  M203 X50.00 Y50.00", "echo:  M204 S1000.00"]
            )
        )
        assert settings["max_feedrate_x"] == 50.0
        assert settings["max_feedrate_y"] == 50.0
        assert settings["acceleration"] == 1000.0

    def test_m149_metric(self):
        assert (
            detect_unit_system_from_m149(["echo: M149 Units in mm", "ok"])
            == "metric"
        )

    def test_m149_imperial(self):
        assert (
            detect_unit_system_from_m149(["echo: M149 Units in inches"])
            == "imperial"
        )

    def test_m149_absent(self):
        assert detect_unit_system_from_m149(["ok"]) is None


class TestDeviceName:
    def test_machine_type_preferred(self):
        name = extract_marlin_device_name(
            ["FIRMWARE_NAME:Marlin 2.1.2", "MACHINE_TYPE:Custom Cutter"],
            [],
        )
        assert name == "Custom Cutter"

    def test_generic_machine_type_falls_back(self):
        name = extract_marlin_device_name(
            ["FIRMWARE_NAME:Marlin 2.1.2", "MACHINE_TYPE:3D Printer"],
            [],
        )
        assert name == "Marlin"

    def test_boot_version_fallback(self):
        name = extract_marlin_device_name(
            ["MACHINE_TYPE:Default"], ["start", "Marlin 2.1.2.7"]
        )
        assert name == "Marlin 2.1.2.7"

    def test_unknown(self):
        assert extract_marlin_device_name(["ok"], []) == (
            "Unknown Marlin Device"
        )


class TestVersionAndErrors:
    def test_version(self):
        assert parse_marlin_version("Marlin 2.1.2.7") == "2.1.2.7"

    def test_error_message(self):
        assert parse_error_message(
            "Error:Line number is not a line number"
        ) == ("Line number is not a line number")
