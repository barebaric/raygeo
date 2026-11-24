from pathlib import Path

import pytest

from raygeo.formats.gcode import parse_gcode

FIXTURES = Path(__file__).parent / "assets"


def _text(name: str) -> str:
    return (FIXTURES / name).read_bytes().decode("utf-8")


def _ends(geometry) -> list[tuple[float, float, float]]:
    return [tuple(round(v, 6) for v in cmd.end) for cmd in geometry.data]


def _types(geometry) -> list[str]:
    return [type(cmd).__name__ for cmd in geometry.data]


def _arcs(geometry):
    return [cmd for cmd in geometry.data if type(cmd).__name__ == "Arc"]


class TestBasics:
    def test_modal_motion_and_units(self):
        result = parse_gcode(_text("rs274ngc-basics.ngc"))
        geometry = result.geometry

        assert result.unsupported == []
        assert _types(geometry) == ["Move"] + ["Line"] * 5
        assert _ends(geometry) == [
            (0.0, 0.0, 0.0),
            (31.75, 0.0, 0.0),
            (31.75, 31.75, 0.0),
            (0.0, 31.75, 0.0),
            (0.0, 0.0, 0.0),
            (15.875, 15.875, 0.0),
        ]
        assert geometry.rect() == pytest.approx((0.0, 0.0, 31.75, 31.75))

    def test_dwell_does_not_change_motion_mode(self):
        result = parse_gcode(
            "G21 G90\nG0 X0 Y0\nG1 X10 Y0\nG4 P0.25\nX10 Y10\n"
        )

        assert _types(result.geometry) == ["Move", "Line", "Line"]
        assert len(result.geometry.split_into_contours()) == 1


class TestArcs:
    def test_arc_forms_and_centres(self):
        result = parse_gcode(_text("rs274ngc-arcs.ngc"))
        geometry = result.geometry

        assert result.unsupported == []
        assert _types(geometry) == [
            "Move",
            "Arc",
            "Arc",
            "Arc",
            "Arc",
            "Arc",
            "Move",
            "Arc",
        ]
        assert _ends(geometry) == [
            (0.0, 0.0, 0.0),
            (10.0, 10.0, 0.0),
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 0.0),
            (10.0, 10.0, 0.0),
            (0.0, 0.0, 0.0),
            (10.0, 10.0, 0.0),
            (15.0, 15.0, 0.0),
        ]
        offsets = [
            tuple(round(v, 6) for v in cmd.center_offset)
            for cmd in _arcs(geometry)
        ]
        assert offsets == [
            (10.0, 0.0, 0.0),
            (0.0, -10.0, 0.0),
            (10.0, 0.0, 0.0),
            (10.0, 0.0, 0.0),
            (-10.0, 0.0, 0.0),
            (5.0, 0.0, 0.0),
        ]
        clockwise = [cmd.clockwise for cmd in _arcs(geometry)]
        assert clockwise == [True, False, True, True, False, True]

    def test_full_circle_bounds(self):
        result = parse_gcode(_text("rs274ngc-arcs.ngc"))

        assert result.geometry.rect() == pytest.approx(
            (-10.0, -10.0, 20.0, 20.0)
        )

    def test_absolute_arc_centres(self):
        result = parse_gcode(
            "G21 G90\nG0 X10 Y10\nG90.1\nG3 X20 Y10 I15 J10\n"
        )
        arcs = _arcs(result.geometry)

        assert len(arcs) == 1
        assert arcs[0].center_offset == pytest.approx((5.0, 0.0, 0.0))
        assert arcs[0].clockwise is False

    def test_radius_full_circle_is_skipped(self):
        result = parse_gcode("G21 G90\nG0 X0 Y0\nG2 X0 Y0 R10\n")

        assert result.unsupported == ["G2"]
        assert result.geometry.is_empty()


class TestQuirks:
    def test_lexical_edge_cases(self):
        result = parse_gcode(_text("rs274ngc-quirks.ngc"))
        geometry = result.geometry

        assert result.unsupported == []
        assert _types(geometry) == [
            "Move",
            "Line",
            "Line",
            "Line",
            "Move",
            "Line",
        ]
        assert _ends(geometry) == [
            (0.0, 0.0, 0.0),
            (10.0, 0.0, 0.0),
            (0.0, 10.0, 0.0),
            (0.0, 0.0, 0.0),
            (0.5, -0.5, 0.0),
            (0.75, 1.0, 0.0),
        ]
        assert geometry.rect() == pytest.approx((0.0, -0.5, 10.0, 10.0))

    def test_block_delete_line_is_skipped(self):
        result = parse_gcode("/G20\nG21\nG1 X10 Y0\n")

        assert _ends(result.geometry)[-1] == (10.0, 0.0, 0.0)

    def test_z_only_rapid_does_not_break_the_path(self):
        result = parse_gcode(
            "G21 G90\nG0 X0 Y0\nG1 X10 Y0\nG0 Z5\nG1 X10 Y10\n"
        )
        geometry = result.geometry

        assert _types(geometry) == ["Move", "Line", "Line"]
        assert len(geometry.split_into_contours()) == 1


class TestLightBurn:
    def test_line_mode(self):
        result = parse_gcode(_text("lightburn-line.gc"))
        geometry = result.geometry

        assert result.unsupported == []
        assert _types(geometry) == ["Move"] + ["Line"] * 4
        assert _ends(geometry) == [
            (0.0, 0.0, 0.0),
            (10.0, 0.0, 0.0),
            (10.0, 10.0, 0.0),
            (0.0, 10.0, 0.0),
            (0.0, 0.0, 0.0),
        ]

    def test_fill_mode_splits_scanlines(self):
        result = parse_gcode(_text("lightburn-fill.gc"))
        geometry = result.geometry
        contours = geometry.split_into_contours()

        assert result.unsupported == []
        assert len(contours) == 3
        assert [len(contour) for contour in contours] == [2, 2, 2]
        assert geometry.rect() == pytest.approx((0.0, 0.0, 10.0, 0.4))

    def test_fill_mode_without_s0_heuristic(self):
        result = parse_gcode(_text("lightburn-fill.gc"), s0_is_travel=False)
        geometry = result.geometry

        assert len(geometry.split_into_contours()) == 1
        assert len(geometry) == 6


class TestUnsupported:
    def test_skipped_instructions_are_reported(self):
        result = parse_gcode("G18\nG81 X0 Y0\nG41\nO100\n#1 = 5\nM6 T1\n")

        assert result.unsupported == ["#", "G18", "G41", "G81", "M6", "O"]

    def test_unsupported_positioning_does_not_cut(self):
        result = parse_gcode(
            "G21 G90\nG1 X10 Y0\nG92 X0 Y0\nG81 X5 Y5\nG1 X10 Y10\n"
        )

        assert result.unsupported == ["G81", "G92"]
        assert _types(result.geometry) == ["Move", "Line", "Line"]
        assert _ends(result.geometry)[-1] == (10.0, 10.0, 0.0)

    def test_other_axes_are_reported(self):
        result = parse_gcode("G21 G90\nG1 X1 Y1 E2\nA10\n")

        assert result.unsupported == ["A", "E"]

    def test_safe_preambles_are_not_reported(self):
        result = parse_gcode(
            "G17 G20 G40 G49 G54 G80 G90 G94\n"
            "M3 S1000\nM8\nG1 X1 Y1\nM9\nM5\nM2\n"
        )

        assert result.unsupported == []
        assert _ends(result.geometry)[-1] == (25.4, 25.4, 0.0)
