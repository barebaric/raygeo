import numpy as np

from raygeo.geo import Geometry
from raygeo.svg import (
    geometry_to_svg_path,
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
        geos = parse_svg_path_data(
            "M 0 0 L 10 0 L 10 10 Z", transform=transform
        )
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_with_scale(self):
        geos = parse_svg_path_data(
            "M 0 0 L 10 0 L 10 10 Z", scale_x=2.0, scale_y=2.0
        )
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
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg"><rect x="0" y="0"/></svg>'
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 0

    def test_invalid_xml(self):
        geos = svg_string_to_geometries("not xml at all")
        assert len(geos) == 0

    def test_path_without_d_attribute(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path fill="black"/></svg>'
        )
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


class TestGeometryToSvgPath:
    def test_empty_geometry(self):
        geo = Geometry()
        assert geometry_to_svg_path(geo, 100, 100) == ""

    def test_move_and_line(self):
        geo = Geometry()
        geo.move_to(0.0, 1.0, 0.0)
        geo.line_to(1.0, 0.0, 0.0)
        path = geometry_to_svg_path(geo, 100, 200)
        assert path.startswith("M 0.000 0.000")
        assert "L 100.000 200.000" in path

    def test_y_flip(self):
        geo = Geometry()
        geo.move_to(0.0, 0.0, 0.0)
        geo.line_to(0.0, 1.0, 0.0)
        path = geometry_to_svg_path(geo, 100, 100)
        assert "M 0.000 100.000" in path
        assert "L 0.000 0.000" in path

    def test_bezier(self):
        geo = Geometry()
        geo.move_to(0.0, 0.0, 0.0)
        geo.bezier_to(1.0, 1.0, 0.25, 0.5, 0.75, 0.5)
        path = geometry_to_svg_path(geo, 100, 100)
        assert "C 25.000 50.000 75.000 50.000 100.000 0.000" in path

    def test_arc_cw(self):
        geo = Geometry()
        geo.move_to(0.5, 0.5, 0.0)
        geo.arc_to(1.0, 1.0, i=0.5, j=0.0, clockwise=True)
        path = geometry_to_svg_path(geo, 100, 100)
        assert "A 50.000 50.000 0 0 1 100.000 0.000" in path

    def test_arc_ccw(self):
        geo = Geometry()
        geo.move_to(0.5, 0.5, 0.0)
        geo.arc_to(1.0, 1.0, i=0.5, j=0.0, clockwise=False)
        path = geometry_to_svg_path(geo, 100, 100)
        assert "A 50.000 50.000 0 0 0 100.000 0.000" in path

    def test_close_path_not_in_output(self):
        geo = Geometry()
        geo.move_to(0.0, 0.0, 0.0)
        geo.line_to(1.0, 0.0, 0.0)
        geo.line_to(1.0, 1.0, 0.0)
        geo.close_path()
        path = geometry_to_svg_path(geo, 100, 100)
        assert "Z" not in path

    def test_roundtrip_simple(self):
        geo = Geometry()
        geo.move_to(0.0, 0.0, 0.0)
        geo.line_to(1.0, 1.0, 0.0)
        path = geometry_to_svg_path(geo, 100, 100)
        assert path == "M 0.000 100.000 L 100.000 0.000"


# ---------------------------------------------------------------------------
# Extended SVG path commands (Q, T, S, A)
# ---------------------------------------------------------------------------


class TestQuadraticBezier:
    def test_quadratic_absolute(self):
        geos = parse_svg_path_data("M 0 0 Q 10 20 30 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_quadratic_relative(self):
        geos = parse_svg_path_data("M 0 0 q 10 20 30 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_smooth_quadratic_absolute(self):
        geos = parse_svg_path_data("M 0 0 Q 10 20 30 0 T 60 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_smooth_quadratic_relative(self):
        geos = parse_svg_path_data("M 0 0 q 10 20 30 0 t 30 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_smooth_quadratic_no_prev(self):
        geos = parse_svg_path_data("M 0 0 T 30 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()


class TestSmoothCubic:
    def test_smooth_cubic_absolute(self):
        geos = parse_svg_path_data("M 0 0 C 10 0 20 10 30 0 S 50 10 60 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_smooth_cubic_relative(self):
        geos = parse_svg_path_data("M 0 0 c 10 0 20 10 30 0 s 20 10 30 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_smooth_cubic_no_prev(self):
        geos = parse_svg_path_data("M 0 0 S 20 10 30 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()


class TestArc:
    def test_arc_circular_absolute(self):
        geos = parse_svg_path_data("M 0 0 A 10 10 0 0 1 20 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_arc_circular_relative(self):
        geos = parse_svg_path_data("M 0 0 a 10 10 0 0 1 20 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_arc_elliptical(self):
        geos = parse_svg_path_data("M 0 0 A 20 10 45 0 1 30 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_arc_large_flag(self):
        geos = parse_svg_path_data("M 0 0 A 10 10 0 1 1 20 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_arc_sweep_flag(self):
        geos = parse_svg_path_data("M 0 0 A 10 10 0 0 0 20 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()


# ---------------------------------------------------------------------------
# Extended transforms (scale, rotate, matrix, skewX, skewY)
# ---------------------------------------------------------------------------


class TestExtendedTransform:
    def test_scale(self):
        m = parse_svg_transform("scale(2)")
        assert m.shape == (3, 3)

    def test_scale_xy(self):
        m = parse_svg_transform("scale(2, 3)")
        assert m.shape == (3, 3)

    def test_rotate(self):
        m = parse_svg_transform("rotate(45)")
        assert m.shape == (3, 3)

    def test_rotate_with_center(self):
        m = parse_svg_transform("rotate(45, 10, 20)")
        assert m.shape == (3, 3)

    def test_skew_x(self):
        m = parse_svg_transform("skewX(30)")
        assert m.shape == (3, 3)

    def test_skew_y(self):
        m = parse_svg_transform("skewY(30)")
        assert m.shape == (3, 3)

    def test_matrix(self):
        m = parse_svg_transform("matrix(1, 0, 0, 1, 10, 20)")
        assert m.shape == (3, 3)

    def test_combined_transforms(self):
        m = parse_svg_transform("translate(10, 20) scale(2) rotate(45)")
        assert m.shape == (3, 3)

    def test_combined_with_matrix(self):
        m = parse_svg_transform("translate(5, 10) matrix(1, 0, 0, 1, 3, 4)")
        assert m.shape == (3, 3)


# ---------------------------------------------------------------------------
# Basic shapes (rect, circle, ellipse, line, polyline, polygon)
# ---------------------------------------------------------------------------


class TestSvgBasicShapes:
    def test_rect(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<rect x="0" y="0" width="10" height="20"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_rect_rounded(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<rect x="0" y="0" width="20" height="20" rx="5" ry="5"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_circle(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<circle cx="10" cy="10" r="5"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_ellipse(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<ellipse cx="10" cy="10" rx="8" ry="5"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_line(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<line x1="0" y1="0" x2="10" y2="20"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_polyline(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<polyline points="0,0 10,0 10,10"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_polygon(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<polygon points="0,0 10,0 10,10 0,10"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_mixed_shapes(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<rect x="0" y="0" width="10" height="10"/>'
            '<circle cx="20" cy="20" r="5"/>'
            '<path d="M 30 30 L 40 30 L 40 40 Z"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 3


# ---------------------------------------------------------------------------
# Display / visibility filtering
# ---------------------------------------------------------------------------


class TestSvgVisibility:
    def test_display_none(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 10 10" display="none"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 0

    def test_visibility_hidden(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 10 10" visibility="hidden"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 0

    def test_visible_path_included(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 10 10" visibility="visible"/>'
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 1

    def test_nested_hidden_group(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g display="none">'
            '<path d="M 0 0 L 10 10"/>'
            "</g>"
            "</svg>"
        )
        geos = svg_string_to_geometries(svg)
        assert len(geos) == 0
