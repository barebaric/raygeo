"""Writing DXF files."""

import pytest

from raygeo.dxf.reader import read
from raygeo.dxf.writer import write
from raygeo.geo import Geometry


def _rectangle(width=100.0, height=50.0) -> Geometry:
    geometry = Geometry()
    geometry.move_to(0, 0)
    geometry.line_to(width, 0)
    geometry.line_to(width, height)
    geometry.line_to(0, height)
    geometry.close_path()
    return geometry


class TestDocument:
    def test_header_and_sections(self):
        text = write(_rectangle())

        assert "$ACADVER" in text
        assert "AC1009" in text
        assert "ENTITIES" in text
        assert text.rstrip().endswith("EOF")

    def test_declares_millimetres(self):
        text = write(_rectangle())

        lines = text.splitlines()
        index = lines.index("$INSUNITS")
        assert lines[index + 1].strip() == "70"
        assert lines[index + 2].strip() == "4"

    def test_output_is_group_code_pairs(self):
        lines = write(_rectangle()).splitlines()

        assert len(lines) % 2 == 0
        for index in range(0, len(lines), 2):
            int(lines[index].strip())


class TestEntities:
    def test_lines_become_one_polyline(self):
        text = write(_rectangle())

        assert text.count("LWPOLYLINE") == 1

    def test_separate_contours_stay_separate(self):
        geometry = _rectangle()
        geometry.move_to(200, 0)
        geometry.line_to(250, 0)

        text = write(geometry)

        assert text.count("LWPOLYLINE") == 2

    def test_arcs_stay_arcs(self):
        geometry = Geometry()
        geometry.move_to(10, 0)
        geometry.arc_to(0, 10, -10, 0, False, 0)

        text = write(geometry)

        assert "ARC" in text
        assert "LWPOLYLINE" not in text

    def test_beziers_become_splines(self):
        geometry = Geometry()
        geometry.move_to(0, 0)
        # bezier_to takes the end point first, then both control points.
        geometry.bezier_to(100, 0, 25, 50, 75, 50)

        text = write(geometry)

        assert "SPLINE" in text


class TestRoundTrip:
    def test_rectangle_survives(self):
        original = _rectangle()

        back = read(write(original)).layers[0][1]

        assert back.is_closed(1e-6)
        assert back.area() == pytest.approx(original.area())
        assert back.rect() == pytest.approx(original.rect(), abs=1e-6)

    def test_arc_survives(self):
        original = Geometry()
        original.move_to(10, 0)
        original.arc_to(0, 10, -10, 0, False, 0)

        back = read(write(original)).layers[0][1]

        assert type(back.data[1]).__name__ == "Arc"
        assert back.data[1].end[0] == pytest.approx(0.0, abs=1e-6)
        assert back.data[1].end[1] == pytest.approx(10.0)

    def test_beziers_survive_approximately(self):
        original = Geometry()
        original.move_to(0, 0)
        original.bezier_to(100, 0, 25, 50, 75, 50)

        back = read(write(original)).layers[0][1]

        assert back.rect()[2] == pytest.approx(100.0, abs=1e-6)
        assert back.rect()[3] == pytest.approx(37.5, abs=1e-6)

    def test_multiple_contours_do_not_join(self):
        original = _rectangle()
        original.move_to(200, 0)
        original.line_to(250, 0)
        original.line_to(250, 40)

        back = read(write(original)).layers[0][1]

        assert len(back.split_into_contours()) == 2

    def test_units_are_millimetres(self):
        back = read(write(_rectangle()))

        assert back.units_to_mm == 1.0

    def test_written_file_reports_no_issues(self):
        back = read(write(_rectangle()))

        assert back.issues == []


class TestDegenerate:
    def test_empty_geometry_writes_a_valid_file(self):
        text = write(Geometry())

        back = read(text)
        assert back.is_empty()
