import numpy as np
from raygeo.svg import (
    parse_svg_path_data,
    parse_svg_transform,
    svg_string_to_geometries,
)

# ---------------------------------------------------------------------------
# parse_svg_transform
# ---------------------------------------------------------------------------


class TestParseSvgTransform:
    def test_empty_string(self):
        m = parse_svg_transform("")
        assert m.shape == (3, 3)
        np.testing.assert_array_equal(m, np.eye(3))

    def test_translate_both_args(self):
        m = parse_svg_transform("translate(10.5, 20.0)")
        expected = np.eye(3)
        expected[0, 2] = 10.5
        expected[1, 2] = 20.0
        np.testing.assert_array_almost_equal(m, expected)

    def test_translate_single_arg(self):
        m = parse_svg_transform("translate(5.0)")
        expected = np.eye(3)
        expected[0, 2] = 5.0
        np.testing.assert_array_almost_equal(m, expected)

    def test_translate_negative(self):
        m = parse_svg_transform("translate(-3.5, -7.2)")
        expected = np.eye(3)
        expected[0, 2] = -3.5
        expected[1, 2] = -7.2
        np.testing.assert_array_almost_equal(m, expected)


# ---------------------------------------------------------------------------
# parse_svg_path_data
# ---------------------------------------------------------------------------


class TestParseSvgPathData:
    def test_moveto_lineto_closepath(self):
        geos = parse_svg_path_data("M 0 0 L 10 0 L 10 10 Z")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_relative_commands(self):
        geos = parse_svg_path_data("m 2 2 l 10 0 l 0 10 z")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_hv_commands(self):
        geos = parse_svg_path_data("M 0 0 H 10 V 10 H 0 Z")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_curveto(self):
        geos = parse_svg_path_data("M 0 0 C 10 0 10 10 0 10 Z")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_relative_curveto(self):
        geos = parse_svg_path_data("M 0 0 c 10 0 10 10 0 10 Z")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_multiple_subpaths(self):
        geos = parse_svg_path_data("M 0 0 L 5 5 M 10 10 L 15 15")
        assert len(geos) == 2

    def test_with_transform(self):
        transform = parse_svg_transform("translate(5, 3)")
        geos = parse_svg_path_data("M 0 0 L 10 0 L 10 10 Z", transform=transform)
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_with_scale(self):
        geos = parse_svg_path_data("M 0 0 L 10 0 L 10 10 Z", scale_x=2.0, scale_y=2.0)
        assert len(geos) == 1

    def test_implicit_lineto_after_moveto(self):
        geos = parse_svg_path_data("M 0 0 10 0 10 10")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_empty_path(self):
        geos = parse_svg_path_data("")
        assert len(geos) == 0


# ---------------------------------------------------------------------------
# svg_string_to_geometries
# ---------------------------------------------------------------------------


class TestSvgStringToGeometries:
    def test_basic_svg(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 10 0 L 10 10 Z"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_multiple_paths(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5"/>'
            '<path d="M 10 10 L 15 15"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 2

    def test_nested_groups_with_transform(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g transform="translate(5, 3)">'
            '<path d="M 0 0 L 10 0 L 10 10 Z"/>'
            "</g>"
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_deeply_nested(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            "<g>"
            "<g>"
            '<path d="M 0 0 L 10 10"/>'
            "</g>"
            "</g>"
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_no_paths(self):
        svg = '<svg xmlns="http://www.w3.org/2000/svg"><rect x="0" y="0"/></svg>'
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 0

    def test_invalid_xml(self):
        geos = svg_string_to_geometries("not xml at all")
        assert len(geos) == 0

    def test_path_without_d_attribute(self):
        svg = '<svg xmlns="http://www.w3.org/2000/svg"><path fill="black"/></svg>'
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 0

    def test_with_scale(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 10 0 L 10 10 Z"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg, scale_x=2.0, scale_y=3.0)
        assert len(geos) == 1

    def test_real_vtracer_output(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'width="100" height="100" viewBox="0 0 100 100">'
            '<path d="M 2 2 L 98 2 L 98 98 L 2 98 Z" fill="black"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1
        assert not geos[0].is_empty()
