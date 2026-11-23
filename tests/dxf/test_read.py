"""Reading DXF files."""

import math

import pytest
from conftest import arc, asset, document, ends, line, rectangle, types

from raygeo.dxf import read


class TestRealFiles:
    def test_minimal_circle(self):
        output = read(asset("circle.dxf"))

        assert len(output.layers) == 1
        name, geometry = output.layers[0]
        assert name == "0"
        assert geometry.is_empty() is False
        assert geometry.is_closed(1e-6)

    def test_circle_radius_and_bounds(self):
        output = read(asset("circle.dxf"))

        min_x, min_y, max_x, max_y = output.layers[0][1].rect()
        assert (min_x, min_y, max_x, max_y) == (-50.0, -50.0, 50.0, 50.0)
        assert output.bounds.width == pytest.approx(100.0)
        assert output.bounds.height == pytest.approx(100.0)

    def test_circle_is_arc_commands_not_lines(self):
        geometry = read(asset("circle.dxf")).layers[0][1]

        assert types(geometry) == ["Move"] + ["Arc"] * 3

    def test_acdb_circle_with_handle_and_extrusion(self):
        output = read(asset("acdbcircle.dxf"))

        min_x, min_y, max_x, max_y = output.layers[0][1].rect()
        assert (min_x, min_y, max_x, max_y) == (-25.0, -25.0, 25.0, 25.0)

    def test_acdb_arc_angles(self):
        geometry = read(asset("acdbarc.dxf")).layers[0][1]

        commands = geometry.data
        assert types(geometry) == ["Move", "Arc"]
        start = commands[0].end
        end = commands[1].end
        assert start[0] == pytest.approx(50 * math.cos(math.radians(45)))
        assert start[1] == pytest.approx(50 * math.sin(math.radians(45)))
        assert end[0] == pytest.approx(50 * math.cos(math.radians(135)))
        assert end[1] == pytest.approx(50 * math.sin(math.radians(135)))

    def test_arc_is_counter_clockwise(self):
        geometry = read(asset("acdbarc.dxf")).layers[0][1]

        assert geometry.data[1].clockwise is False


class TestEntities:
    def test_line(self):
        output = read(document([line(0, 0, 100, 50)]))
        geometry = output.layers[0][1]

        assert types(geometry) == ["Move", "Line"]
        assert ends(geometry) == [(0.0, 0.0, 0.0), (100.0, 50.0, 0.0)]

    def test_arc_direction(self):
        output = read(document([arc(0, 0, 10, 0, 90)]))
        geometry = output.layers[0][1]

        assert types(geometry) == ["Move", "Arc"]
        assert geometry.data[1].clockwise is False
        assert ends(geometry) == [(10.0, 0.0, 0.0), (0.0, 10.0, 0.0)]

    def test_arc_more_than_half_turn(self):
        output = read(document([arc(0, 0, 10, 0, 270)]))
        command = output.layers[0][1].data[1]

        assert command.clockwise is False
        assert command.end[0] == pytest.approx(0.0, abs=1e-6)
        assert command.end[1] == pytest.approx(-10.0)

    def test_clockwise_extrusion_reverses_arc(self):
        values = [
            (10, 0),
            (20, 0),
            (30, 0),
            (40, 10),
            (50, 0),
            (51, 90),
            (210, 0),
            (220, 0),
            (230, -1),
        ]
        output = read(document([("ARC", "0", values)]))
        command = output.layers[0][1].data[1]

        assert command.clockwise is True

    def test_ellipse(self):
        values = [
            (10, 0),
            (20, 0),
            (30, 0),
            (11, 40),
            (21, 0),
            (31, 0),
            (40, 0.5),
            (41, 0.0),
            (42, 2 * math.pi),
        ]
        geometry = read(document([("ELLIPSE", "0", values)])).layers[0][1]

        min_x, min_y, max_x, max_y = geometry.rect()
        assert max_x == pytest.approx(40.0, abs=0.5)
        assert max_y == pytest.approx(20.0, abs=0.5)
        assert min_x == pytest.approx(-40.0, abs=0.5)

    def test_lwpolyline_closed(self):
        geometry = read(document([rectangle(0, 0, 100, 50)])).layers[0][1]

        assert geometry.is_closed(1e-6)
        assert geometry.area() == pytest.approx(5000.0)

    def test_lwpolyline_bulge_becomes_arc(self):
        values = [
            (90, 2),
            (70, 0),
            (10, 0),
            (20, 0),
            (42, 1.0),
            (10, 10),
            (20, 0),
        ]
        geometry = read(document([("LWPOLYLINE", "0", values)])).layers[0][1]

        assert types(geometry) == ["Move", "Arc"]
        assert geometry.rect()[1] == pytest.approx(-5.0, abs=1e-6)

    def test_old_style_polyline(self):
        entities = [
            ("POLYLINE", "0", [(66, 1), (70, 9), (10, 0), (20, 0), (30, 0)]),
            ("VERTEX", "0", [(10, 0), (20, 0), (30, 0)]),
            ("VERTEX", "0", [(10, 10), (20, 0), (30, 0)]),
            ("VERTEX", "0", [(10, 10), (20, 10), (30, 0)]),
            ("SEQEND", "0", []),
        ]
        geometry = read(document(entities)).layers[0][1]

        assert geometry.is_closed(1e-6)
        assert geometry.area() == pytest.approx(50.0)

    def test_spline_with_control_points(self):
        values = [
            (70, 8),
            (71, 3),
            (72, 4),
            (73, 4),
            (74, 0),
            (40, 0),
            (40, 0),
            (40, 0),
            (40, 0),
            (40, 1),
            (40, 1),
            (40, 1),
            (40, 1),
            (10, 0),
            (20, 0),
            (30, 0),
            (10, 10),
            (20, 40),
            (30, 0),
            (10, 30),
            (20, 40),
            (30, 0),
            (10, 40),
            (20, 0),
            (30, 0),
        ]
        geometry = read(document([("SPLINE", "0", values)])).layers[0][1]

        assert not geometry.is_empty()
        assert "Bezier" in types(geometry)
        assert geometry.rect()[2] == pytest.approx(40.0, abs=1e-6)

    def test_unsupported_entity_is_reported(self):
        output = read(document([("OLYMPIC", "0", [(10, 0)])]))

        assert output.issues
        assert "OLYMPIC" in output.issues[0]

    def test_hatch_is_ignored_silently(self):
        output = read(document([("HATCH", "0", [])]))

        assert output.issues == []

    def test_text_is_reported(self):
        output = read(
            document([("TEXT", "0", [(10, 0), (20, 0), (40, 5), (1, "Hi")])])
        )

        assert any("TEXT" in issue for issue in output.issues)


class TestLayers:
    def test_layers_grouped_in_order(self):
        output = read(
            document(
                [
                    line(0, 0, 10, 0, layer="cut"),
                    line(0, 10, 10, 10, layer="engrave"),
                    line(10, 20, 20, 20, layer="cut"),
                ]
            )
        )

        names = [name for name, _ in output.layers]
        assert names == ["cut", "engrave"]
        assert len(output.layers[0][1].data) == 4

    def test_missing_layer_falls_back_to_zero(self):
        output = read(
            document([("LINE", "", [(10, 0), (20, 0), (11, 1), (21, 1)])])
        )

        assert output.layers[0][0] == "0"

    def test_block_entities_inherit_insert_layer(self):
        lines = [
            "0",
            "SECTION",
            "2",
            "BLOCKS",
            "0",
            "BLOCK",
            "2",
            "PART",
            "10",
            "0",
            "20",
            "0",
            "30",
            "0",
        ]
        lines += [
            "0",
            "LINE",
            "8",
            "0",
            "10",
            "0",
            "20",
            "0",
            "11",
            "5",
            "21",
            "0",
        ]
        lines += ["0", "ENDBLK", "0", "ENDSEC"]
        lines += ["0", "SECTION", "2", "ENTITIES"]
        lines += ["0", "INSERT", "8", "CUT", "2", "PART", "10", "0", "20", "0"]
        lines += ["0", "ENDSEC", "0", "EOF"]
        output = read("\n".join(lines) + "\n")

        assert [name for name, _ in output.layers] == ["CUT"]

    def test_block_insertion_is_translated(self):
        lines = ["0", "SECTION", "2", "BLOCKS"]
        lines += ["0", "BLOCK", "2", "PART", "10", "0", "20", "0", "30", "0"]
        lines += [
            "0",
            "LINE",
            "8",
            "0",
            "10",
            "0",
            "20",
            "0",
            "11",
            "5",
            "21",
            "0",
        ]
        lines += ["0", "ENDBLK", "0", "ENDSEC"]
        lines += ["0", "SECTION", "2", "ENTITIES"]
        lines += [
            "0",
            "INSERT",
            "8",
            "0",
            "2",
            "PART",
            "10",
            "100",
            "20",
            "50",
            "41",
            "2",
            "42",
            "2",
        ]
        lines += ["0", "ENDSEC", "0", "EOF"]
        geometry = read("\n".join(lines) + "\n").layers[0][1]

        assert ends(geometry) == [(100.0, 50.0, 0.0), (110.0, 50.0, 0.0)]

    def test_missing_block_is_reported(self):
        output = read(
            document([("INSERT", "0", [(2, "NOPE"), (10, 0), (20, 0)])])
        )

        assert any("NOPE" in issue for issue in output.issues)


class TestUnitsAndZ:
    def test_millimetres(self):
        output = read(document([line(0, 0, 10, 0)], header={"$INSUNITS": 4}))

        assert output.units_to_mm == 1.0

    def test_inches(self):
        output = read(document([line(0, 0, 10, 0)], header={"$INSUNITS": 1}))

        assert output.units_to_mm == 25.4

    def test_unitless_defaults_to_millimetres(self):
        output = read(document([line(0, 0, 10, 0)]))

        assert output.units_to_mm == 1.0

    def test_flat_drawing_has_no_z_variation(self):
        output = read(document([line(0, 0, 10, 0)]))

        assert output.has_z_variation() is False
        assert output.min_z == 0.0
        assert output.max_z == 0.0

    def test_z_variation_detected(self):
        output = read(document([line(0, 0, 10, 0, z1=0.0, z2=-10.0)]))

        assert output.has_z_variation() is True
        assert output.min_z == pytest.approx(-10.0)
        assert output.max_z == 0.0

    def test_small_z_noise_is_not_variation(self):
        output = read(document([line(0, 0, 10, 0, z1=0.0, z2=1e-6)]))

        assert output.has_z_variation() is False


class TestErrors:
    def test_non_dxf_input_raises(self):
        with pytest.raises(ValueError):
            read("this is not a dxf file")

    def test_empty_document(self):
        output = read("0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n")

        assert output.is_empty()
        assert output.layers == []

    def test_truncated_file_keeps_parsed_entities(self):
        text = document([line(0, 0, 10, 0)])
        output = read(text.rstrip()[:-20])

        assert not output.is_empty()

    def test_empty_string_is_an_empty_drawing(self):
        output = read("")

        assert output.is_empty()


class TestBulges:
    """A bulge is tan(included angle / 4); its exact sagitta is |b| * L / 2.

    These cases pin that identity for minor and major arcs alike. The bug
    they guard against rendered every bulged segment as a near-full circle.
    """

    CHORD = 18.0

    def _rect(self, bulge):
        values = [
            (90, 2),
            (70, 0),
            (10, 12),
            (20, 12),
            (42, bulge),
            (10, 12 + self.CHORD),
            (20, 12),
        ]
        geometry = read(document([("LWPOLYLINE", "0", values)]))
        return geometry.layers[0][1].rect()

    @pytest.mark.parametrize("bulge", [0.1, 0.2, 0.4, 0.6, 1.0, 1.5, 2.0])
    def test_positive_bulge_sagitta(self, bulge):
        rect = self._rect(bulge)

        assert rect[3] == pytest.approx(12.0, abs=1e-6)
        assert rect[1] == pytest.approx(
            12.0 - bulge * self.CHORD / 2, abs=1e-6
        )

    @pytest.mark.parametrize("bulge", [0.4, 1.0, 2.0])
    def test_negative_bulge_mirrors(self, bulge):
        rect = self._rect(-bulge)

        assert rect[1] == pytest.approx(12.0, abs=1e-6)
        assert rect[3] == pytest.approx(
            12.0 + bulge * self.CHORD / 2, abs=1e-6
        )

    def test_zero_bulge_is_a_line(self):
        geometry = read(
            document(
                [
                    (
                        "LWPOLYLINE",
                        "0",
                        [
                            (90, 2),
                            (70, 0),
                            (10, 0),
                            (20, 0),
                            (42, 0.0),
                            (10, 10),
                            (20, 0),
                        ],
                    )
                ]
            )
        ).layers[0][1]

        assert types(geometry) == ["Move", "Line"]

    def test_quarter_turn_is_a_semicircle_spanned_correctly(self):
        rect = self._rect(1.0)

        assert rect[1] == pytest.approx(12.0 - self.CHORD / 2)

    def test_closed_bulged_polyline_bulges_outward(self):
        values = [
            (90, 4),
            (70, 1),
            (10, 12),
            (20, 12),
            (42, 0.4),
            (10, 30),
            (20, 12),
            (42, 0.4),
            (10, 30),
            (20, 30),
            (42, 0.4),
            (10, 12),
            (20, 30),
            (42, 0.4),
        ]
        geometry = read(document([("LWPOLYLINE", "0", values)])).layers[0][1]

        rect = geometry.rect()
        sagitta = 0.4 * 18.0 / 2
        assert rect[0] == pytest.approx(12.0 - sagitta, abs=1e-6)
        assert rect[1] == pytest.approx(12.0 - sagitta, abs=1e-6)
        assert rect[2] == pytest.approx(30.0 + sagitta, abs=1e-6)
        assert rect[3] == pytest.approx(30.0 + sagitta, abs=1e-6)

    def test_centre_matches_the_reference_implementation(self):
        """The centre values here come from ezdxf's bulge_to_arc."""
        values = [
            (90, 2),
            (70, 0),
            (10, 12),
            (20, 12),
            (42, 0.4),
            (10, 30),
            (20, 12),
        ]
        geometry = read(document([("LWPOLYLINE", "0", values)])).layers[0][1]
        arc = geometry.data[1]

        centre_x = 12.0 + arc.center_offset[0]
        centre_y = 12.0 + arc.center_offset[1]
        assert centre_x == pytest.approx(21.0, abs=1e-6)
        assert centre_y == pytest.approx(21.45, abs=1e-6)

    def test_negative_bulge_centre_mirrors(self):
        values = [
            (90, 2),
            (70, 0),
            (10, 12),
            (20, 12),
            (42, -0.4),
            (10, 30),
            (20, 12),
        ]
        geometry = read(document([("LWPOLYLINE", "0", values)])).layers[0][1]
        arc = geometry.data[1]

        assert 12.0 + arc.center_offset[1] == pytest.approx(2.55, abs=1e-6)
