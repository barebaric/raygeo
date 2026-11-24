"""Tests for the CorelDRAW writer."""

import pytest

from raygeo.formats.coreldraw import CdrFormat, detect_format, parse, write
from raygeo.geo import Geometry, Line, Move

MM_PER_INCH = 25.4


def rectangle() -> Geometry:
    geometry = Geometry()
    geometry.move_to(0, 0, 0)
    geometry.line_to(20, 0, 0)
    geometry.line_to(20, 10, 0)
    geometry.line_to(0, 10, 0)
    geometry.line_to(0, 0, 0)
    return geometry


def round_trip(geometry: Geometry):
    document = parse(write(geometry))
    name, read_back = document.layers[0]
    rect = tuple(value * MM_PER_INCH for value in read_back.rect())
    return document, name, read_back, rect


class TestWrite:
    def test_output_is_a_riff_drawing(self):
        data = write(rectangle())

        assert data.startswith(b"RIFF")
        assert data[8:12] == b"CDRC"
        assert detect_format(data) is CdrFormat.CDR

    def test_page_and_layer(self):
        document, name, geometry, _ = round_trip(rectangle())

        assert document.page_width == pytest.approx(20 / MM_PER_INCH)
        assert document.page_height == pytest.approx(10 / MM_PER_INCH)
        assert document.page_count == 1
        assert document.issues == []
        assert name == "Layer 1"
        assert len(geometry) == 5

    def test_rectangle_round_trips(self):
        _, _, geometry, rect = round_trip(rectangle())

        assert isinstance(geometry.data[-1], Line)
        assert geometry.data[-1].end[:2] == pytest.approx((0.0, 0.0))
        assert rect == pytest.approx((0.0, 0.0, 20.0, 10.0), abs=1e-6)
        assert all(
            isinstance(command, (Move, Line)) for command in geometry.data
        )

    def test_arcs_and_beziers_round_trip(self):
        geometry = Geometry()
        geometry.move_to(5, 0, 0)
        for end, offset in [
            ((10, 5), (0, 5)),
            ((5, 10), (-5, 0)),
            ((0, 5), (0, -5)),
            ((5, 0), (5, 0)),
        ]:
            geometry.arc_to(end[0], end[1], offset[0], offset[1], False)
        geometry.move_to(15, 0, 0)
        geometry.bezier_to(20, 10, 15, 10, 20, 0)

        document, _, read_back, rect = round_trip(geometry)

        assert document.issues == []
        assert rect == pytest.approx((0.0, 0.0, 20.0, 10.0), abs=1e-6)
        assert any(
            type(command).__name__ == "Bezier" for command in read_back.data
        )

    def test_drawing_is_moved_to_the_page_origin(self):
        geometry = Geometry()
        geometry.move_to(-10, -5, 0)
        geometry.line_to(10, -5, 0)
        geometry.line_to(10, 5, 0)
        geometry.line_to(-10, 5, 0)
        geometry.line_to(-10, -5, 0)

        document, _, _, rect = round_trip(geometry)

        assert document.page_width == pytest.approx(20 / MM_PER_INCH)
        assert document.page_height == pytest.approx(10 / MM_PER_INCH)
        assert rect == pytest.approx((0.0, 0.0, 20.0, 10.0), abs=1e-6)

    def test_multiple_subpaths_stay_in_one_object(self):
        geometry = Geometry()
        for x in (0, 30):
            geometry.move_to(x, 0, 0)
            geometry.line_to(x + 10, 0, 0)
            geometry.line_to(x + 10, 10, 0)
            geometry.line_to(x, 10, 0)
            geometry.line_to(x, 0, 0)

        document, _, read_back, rect = round_trip(geometry)

        assert len(document.layers) == 1
        assert rect == pytest.approx((0.0, 0.0, 40.0, 10.0), abs=1e-6)
        assert (
            sum(isinstance(command, Move) for command in read_back.data) == 2
        )

    def test_zero_width_page_becomes_one_millimetre(self):
        geometry = Geometry()
        geometry.move_to(5, 0, 0)
        geometry.line_to(5, 20, 0)

        document, _, read_back, rect = round_trip(geometry)

        assert document.page_width == pytest.approx(1 / MM_PER_INCH)
        assert document.page_height == pytest.approx(20 / MM_PER_INCH)
        assert rect == pytest.approx((0.0, 0.0, 0.0, 20.0), abs=1e-6)

    def test_empty_geometry_raises(self):
        with pytest.raises(RuntimeError):
            write(Geometry())
