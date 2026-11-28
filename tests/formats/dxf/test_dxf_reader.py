"""Reading DXF files."""

import math

import pytest
from conftest import (
    arc,
    asset,
    block_document,
    circle,
    document,
    ends,
    insert,
    line,
    rectangle,
    types,
)

from raygeo.formats.dxf.reader import read, read_with_inserts


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
        min_x, min_y, max_x, max_y = output.bounds
        assert max_x - min_x == pytest.approx(100.0)
        assert max_y - min_y == pytest.approx(100.0)

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

    def test_block_arc_follows_insert_rotation(self):
        lines = ["0", "SECTION", "2", "BLOCKS"]
        lines += ["0", "BLOCK", "2", "PART", "10", "0", "20", "0", "30", "0"]
        lines += [
            "0",
            "ARC",
            "8",
            "0",
            "10",
            "0",
            "20",
            "0",
            "30",
            "0",
            "40",
            "10",
            "50",
            "0",
            "51",
            "90",
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
            "50",
            "90",
        ]
        lines += ["0", "ENDSEC", "0", "EOF"]
        geometry = read("\n".join(lines) + "\n").layers[0][1]

        assert "Bezier" in types(geometry)
        assert ends(geometry) == [(100.0, 60.0, 0.0), (90.0, 50.0, 0.0)]

    def test_block_circle_keeps_its_shape(self):
        output = read(
            block_document(
                [("PART", (0, 0), [circle(0, 0, 10)])],
                [insert("PART", 100, 50, rotation=45)],
            )
        )
        geometry = output.layers[0][1]

        assert "Bezier" in types(geometry)
        assert geometry.rect() == pytest.approx(
            (90.0, 40.0, 110.0, 60.0), abs=1e-6
        )

    def test_missing_block_is_reported(self):
        output = read(
            document([("INSERT", "0", [(2, "NOPE"), (10, 0), (20, 0)])])
        )

        assert any("NOPE" in issue for issue in output.issues)

    def test_nested_block_insertion_is_expanded(self):
        output = read(
            block_document(
                [
                    ("INNER", (0, 0), [line(0, 0, 10, 0)]),
                    ("OUTER", (0, 0), [insert("INNER", 0, 5)]),
                ],
                [insert("OUTER", 100, 0, layer="LAYER_A")],
            )
        )

        assert [name for name, _ in output.layers] == ["LAYER_A"]
        assert ends(output.layers[0][1]) == [
            (100.0, 5.0, 0.0),
            (110.0, 5.0, 0.0),
        ]
        assert output.issues == []

    def test_nested_block_transforms_accumulate(self):
        output = read(
            block_document(
                [
                    ("INNER", (1, 0), [line(1, 0, 11, 0)]),
                    ("OUTER", (2, 5), [insert("INNER", 2, 10, rotation=90)]),
                ],
                [insert("OUTER", 100, 0, layer="LAYER_A")],
            )
        )

        assert ends(output.layers[0][1]) == [
            (100.0, 5.0, 0.0),
            (100.0, 15.0, 0.0),
        ]

    def test_nested_block_layers_inherit_from_the_outer_insert(self):
        output = read(
            block_document(
                [
                    (
                        "INNER",
                        (0, 0),
                        [
                            line(0, 0, 10, 0),
                            line(0, 1, 10, 1, layer="DETAIL"),
                        ],
                    ),
                    ("OUTER", (0, 0), [insert("INNER", 0, 5)]),
                ],
                [insert("OUTER", 100, 0, layer="LAYER_A")],
            )
        )

        assert [name for name, _ in output.layers] == ["LAYER_A", "DETAIL"]
        assert len(output.layers[0][1].data) == 2
        assert len(output.layers[1][1].data) == 2

    def test_missing_nested_block_is_reported(self):
        output = read(
            block_document(
                [("OUTER", (0, 0), [insert("NOPE", 0, 0)])],
                [insert("OUTER", 0, 0)],
            )
        )

        assert output.is_empty()
        assert any("NOPE" in issue for issue in output.issues)

    def test_circular_block_insertion_is_reported(self):
        output = read(
            block_document(
                [
                    ("A", (0, 0), [insert("B", 0, 0)]),
                    ("B", (0, 0), [insert("A", 0, 0)]),
                ],
                [insert("A", 0, 0)],
            )
        )

        assert output.is_empty()
        assert any("circular" in issue for issue in output.issues)

    def test_block_nesting_depth_is_bounded(self):
        blocks = [
            (f"B{index}", (0, 0), [insert(f"B{index + 1}", 0, 0)])
            for index in range(40)
        ]
        blocks.append(("B40", (0, 0), []))
        output = read(block_document(blocks, [insert("B0", 0, 0)]))

        assert output.is_empty()
        assert any("deeper than" in issue for issue in output.issues)


class TestInsertStructure:
    """``read_with_inserts`` exposes blocks without changing flattening."""

    def test_flat_result_matches_read(self):
        text = block_document(
            [("PART", (0, 0), [line(0, 0, 10, 0)])],
            [line(0, 5, 5, 5), insert("PART", 100, 0, layer="CUT")],
        )

        flat = read(text)
        structure = read_with_inserts(text)

        assert [
            (name, ends(geo)) for name, geo in structure.output.layers
        ] == [(name, ends(geo)) for name, geo in flat.layers]
        assert structure.output.bounds == flat.bounds

    def test_one_entry_per_insert(self):
        text = block_document(
            [("PART", (0, 0), [line(0, 0, 10, 0)])],
            [
                insert("PART", 0, 0, layer="CUT"),
                insert("PART", 20, 0, layer="CUT"),
            ],
        )

        structure = read_with_inserts(text)

        assert len(structure.inserts) == 2
        assert [entry.block_name for entry in structure.inserts] == [
            "PART",
            "PART",
        ]
        assert ends(structure.inserts[0].layers[0][1]) == [
            (0.0, 0.0, 0.0),
            (10.0, 0.0, 0.0),
        ]
        assert ends(structure.inserts[1].layers[0][1]) == [
            (20.0, 0.0, 0.0),
            (30.0, 0.0, 0.0),
        ]

    def test_entry_layer_inherits_insert_layer(self):
        text = block_document(
            [
                (
                    "PART",
                    (0, 0),
                    [
                        line(0, 0, 10, 0),
                        line(0, 1, 10, 1, layer="DETAIL"),
                    ],
                )
            ],
            [insert("PART", 100, 50, layer="CUT")],
        )

        entry = read_with_inserts(text).inserts[0]

        assert entry.layer == "CUT"
        assert [name for name, _ in entry.layers] == ["CUT", "DETAIL"]
        assert ends(entry.layers[0][1]) == [
            (100.0, 50.0, 0.0),
            (110.0, 50.0, 0.0),
        ]
        assert ends(entry.layers[1][1]) == [
            (100.0, 51.0, 0.0),
            (110.0, 51.0, 0.0),
        ]

    def test_empty_insert_layer_is_reported_as_default(self):
        text = block_document(
            [("PART", (0, 0), [line(0, 0, 10, 0)])],
            [insert("PART", 0, 0, layer="")],
        )

        entry = read_with_inserts(text).inserts[0]

        assert entry.layer == "0"
        assert [name for name, _ in entry.layers] == ["0"]

    def test_transform_maps_block_coordinates_to_world(self):
        text = block_document(
            [("PART", (0, 0), [line(0, 0, 10, 0)])],
            [
                insert(
                    "PART",
                    100,
                    50,
                    layer="CUT",
                    rotation=90,
                    x_scale=2,
                    y_scale=2,
                )
            ],
        )

        transform = read_with_inserts(text).inserts[0].transform

        expected = [
            [0.0, -2.0, 0.0, 100.0],
            [2.0, 0.0, 0.0, 50.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
        for row, expected_row in zip(transform, expected):
            assert row == pytest.approx(expected_row)

    def test_standalone_layers_hold_only_content_outside_blocks(self):
        text = block_document(
            [("PART", (0, 0), [line(0, 0, 10, 0)])],
            [
                line(0, 5, 5, 5, layer="CUT"),
                insert("PART", 100, 0, layer="CUT"),
            ],
        )

        structure = read_with_inserts(text)

        assert [name for name, _ in structure.standalone_layers] == ["CUT"]
        assert ends(structure.standalone_layers[0][1]) == [
            (0.0, 5.0, 0.0),
            (5.0, 5.0, 0.0),
        ]
        assert ends(structure.inserts[0].layers[0][1]) == [
            (100.0, 0.0, 0.0),
            (110.0, 0.0, 0.0),
        ]

    def test_nested_insert_belongs_to_the_outer_entry(self):
        text = block_document(
            [
                ("INNER", (0, 0), [line(0, 0, 10, 0)]),
                ("OUTER", (0, 0), [insert("INNER", 0, 5)]),
            ],
            [insert("OUTER", 100, 0, layer="LAYER_A")],
        )

        structure = read_with_inserts(text)

        assert [entry.block_name for entry in structure.inserts] == ["OUTER"]
        assert ends(structure.inserts[0].layers[0][1]) == [
            (100.0, 5.0, 0.0),
            (110.0, 5.0, 0.0),
        ]

    def test_missing_block_is_still_reported(self):
        structure = read_with_inserts(
            document([("INSERT", "0", [(2, "NOPE"), (10, 0), (20, 0)])])
        )

        assert len(structure.inserts) == 1
        assert structure.inserts[0].block_name == "NOPE"
        assert structure.inserts[0].layers == []
        assert any("NOPE" in issue for issue in structure.output.issues)

    def test_drawing_without_inserts_has_no_entries(self):
        structure = read_with_inserts(document([line(0, 0, 10, 0)]))

        assert structure.inserts == []
        assert [name for name, _ in structure.standalone_layers] == ["0"]


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


class TestSplineSpans:
    """Exporters write splines whose interior knots already carry full
    multiplicity, with a control-point count that does not match the
    knot vector. The trailing clamped knots must not be treated as span
    boundaries; doing so appends degenerate control points, which render
    as zig-zag geometry."""

    def _spline(self, control, knots, degree=3):
        values = [
            (70, 8),
            (71, degree),
            (72, len(knots)),
            (73, len(control)),
            (74, 0),
        ]
        values += [(40, knot) for knot in knots]
        for x, y in control:
            values += [(10, x), (20, y), (30, 0.0)]
        return read(document([("SPLINE", "0", values)])).layers[0][1]

    # 8 control points with a 12-knot clamped vector whose interior knot
    # has full multiplicity: the shape of a spline exported as bezier
    # spans. The count is one point more than the spans need, which is
    # the inconsistency real exporters produce.
    CONTROL = [
        (0.0, 0.0),
        (10.0, 30.0),
        (10.0, 30.0),
        (20.0, 30.0),
        (20.0, 30.0),
        (30.0, 0.0),
        (40.0, -30.0),
        (50.0, -30.0),
    ]
    KNOTS = [0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0]

    def _curves(self, geometry):
        return [c for c in geometry.data if type(c).__name__ == "Bezier"]

    def test_produces_curves_without_degenerate_runs(self):
        curves = self._curves(self._spline(self.CONTROL, self.KNOTS))

        assert curves, "the spline must produce bezier segments"
        degenerate = sum(
            1
            for a, b in zip(curves, curves[1:])
            if a.end == b.end and a.control1 == b.control1
        )
        assert degenerate <= len(curves) / 2

    def test_segments_advance_along_the_curve(self):
        curves = self._curves(self._spline(self.CONTROL, self.KNOTS))

        xs = [c.end[0] for c in curves]
        assert xs == sorted(xs), "segments must advance along the curve"

    # A smoothly exported spline: nine control points with a clamped knot
    # vector whose five interior knots each have multiplicity one. Every
    # span is cubic, so the curve must split into six exact bezier
    # segments. Dropping interior knots used to make the insertion pass
    # fall back to sampled fitting, which emitted control points far
    # outside the control polygon.
    CONTROL_SMOOTH = [
        (499.5215204169139, 273.4515044987481),
        (499.9932659807569, 273.4891789611759),
        (500.9485143844753, 273.5654668435713),
        (502.3897908343616, 274.1036289032998),
        (504.9409087635065, 275.8074123164537),
        (508.4954315557188, 280.672808730019),
        (512.9564574831799, 291.5334438699177),
        (513.9438942021492, 305.3620876943978),
        (514.5215204169139, 313.4515044987481),
    ]
    KNOTS_SMOOTH = [
        0.0,
        0.0,
        0.0,
        0.0,
        0.08105394958607,
        0.1641280001160383,
        0.2800278960888437,
        0.4436051444910396,
        0.674522682691375,
        1.0,
        1.0,
        1.0,
        1.0,
    ]

    def test_simple_interior_knots_split_into_exact_spans(self):
        geometry = self._spline(self.CONTROL_SMOOTH, self.KNOTS_SMOOTH)
        curves = self._curves(geometry)

        assert len(curves) == 6
        assert curves[-1].end[:2] == pytest.approx(self.CONTROL_SMOOTH[-1])

    def test_simple_interior_knots_stay_inside_control_polygon(self):
        geometry = self._spline(self.CONTROL_SMOOTH, self.KNOTS_SMOOTH)

        min_x = min(x for x, _ in self.CONTROL_SMOOTH)
        max_x = max(x for x, _ in self.CONTROL_SMOOTH)
        min_y = min(y for _, y in self.CONTROL_SMOOTH)
        max_y = max(y for _, y in self.CONTROL_SMOOTH)
        left, bottom, right, top = geometry.rect()
        assert left >= min_x - 1e-6
        assert right <= max_x + 1e-6
        assert bottom >= min_y - 1e-6
        assert top <= max_y + 1e-6


class TestEllipse:
    """An ELLIPSE entity must become bezier segments, not a polygon:
    flattening it to straight lines renders visible corners on every
    curved edge."""

    def _ellipse(self):
        values = [
            (10, 0),
            (20, 0),
            (30, 0),
            (11, 40),
            (21, 0),
            (31, 0),
            (40, 0.5),
            (41, 0.0),
            (42, 6.283185307179586),
        ]
        return read(document([("ELLIPSE", "0", values)])).layers[0][1]

    def test_ellipse_produces_curves_not_lines(self):
        geometry = self._ellipse()

        kinds = {type(c).__name__ for c in geometry.data}
        assert "Bezier" in kinds
        assert "Line" not in kinds

    def test_ellipse_is_closed(self):
        geometry = self._ellipse()

        assert geometry.is_closed(1e-6)

    def test_ellipse_extent_matches_radii(self):
        geometry = self._ellipse()

        min_x, min_y, max_x, max_y = geometry.rect()
        assert max_x == pytest.approx(40.0, abs=1e-6)
        assert min_x == pytest.approx(-40.0, abs=1e-6)
        assert max_y == pytest.approx(20.0, abs=1e-6)
        assert min_y == pytest.approx(-20.0, abs=1e-6)
