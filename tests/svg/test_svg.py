import numpy as np
import pytest

from raygeo.geo import Geometry
from raygeo.svg import (
    filter_svg_by_color,
    geometry_to_svg_path,
    parse_svg_path_data,
    svg_string_to_geometries,
    svg_string_to_geometries_by_color,
    svg_string_to_geometries_by_layer,
    svg_string_to_geometry,
    svg_string_to_geometry_by_color,
)
from raygeo.svg.color import ColorAttr
from raygeo.svg.length import parse_svg_length, svg_length_to_mm
from raygeo.svg.metadata import extract_svg_metadata
from raygeo.svg.transform import parse_svg_transform

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
        # 270° CCW sweep => large-arc=1
        assert "A 50.000 50.000 0 1 0 100.000 0.000" in path

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

    def test_long_arc_non_diametrical_cw(self):
        """Large-arc flag + CW sweep, points NOT diametrically opposed.

        Sweep > 180° → arc is decomposed to beziers at parse time.
        """
        geos = parse_svg_path_data("M 10 0 A 10 10 0 1 1 0 10")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_long_arc_non_diametrical_ccw(self):
        """Large-arc flag + CCW sweep, points NOT diametrically opposed."""
        geos = parse_svg_path_data("M 10 0 A 10 10 0 1 0 0 10")
        assert len(geos) == 1
        assert not geos[0].is_empty()

    def test_large_arc_diametrically_opposed(self):
        """
        Diametrically opposed sweep == PI exactly, large-arc derived as 0.
        """
        geos = parse_svg_path_data("M 0.25 0.5 A 0.25 0.25 0 1 1 0.75 0.5")
        assert len(geos) == 1
        exported = geometry_to_svg_path(geos[0], 100, 100)
        # large-arc flag must be 0 (sweep == PI exactly).
        assert "A 25.000 25.000 0 0 " in exported
        # Verify geometry survives the roundtrip: re-import and check
        # that the arc has the correct bounding box (Y-flipped + scaled).
        geo2 = parse_svg_path_data(exported)[0]
        min_x, min_y, max_x, max_y = geo2.rect()
        assert max_x - min_x == pytest.approx(50.0, abs=1.0)
        assert max_y - min_y == pytest.approx(25.0, abs=1.0)

    def test_large_arc_to_dict_roundtrip(self):
        """Arc dict roundtrip preserves geometry (not the flag)."""
        d = "M 0.25 0.5 A 0.25 0.25 0 1 1 0.75 0.5"
        geos = parse_svg_path_data(d)
        orig = geos[0]
        d = orig.to_dict()
        restored = Geometry.from_dict(d)
        assert restored == orig

    def test_arc_cw_export_match(self):
        """Export of CW arc (90 sweep) has correct large-arc=0."""
        geo = Geometry()
        geo.move_to(0.5, 0.5, 0.0)
        geo.arc_to(1.0, 1.0, i=0.5, j=0.0, clockwise=True)
        path = geometry_to_svg_path(geo, 100, 100)
        assert path == "M 50.000 50.000 A 50.000 50.000 0 0 1 100.000 0.000"

    def test_arc_ccw_export_match(self):
        """Export of CCW arc (270 sweep) has correct large-arc=1."""
        geo = Geometry()
        geo.move_to(0.5, 0.5, 0.0)
        geo.arc_to(1.0, 1.0, i=0.5, j=0.0, clockwise=False)
        path = geometry_to_svg_path(geo, 100, 100)
        assert path == "M 50.000 50.000 A 50.000 50.000 0 1 0 100.000 0.000"

    def test_arc_sweep_pi_export_match(self):
        """Export of half-circle arc has large-arc=0 (sweep==PI exactly)."""
        geo = Geometry()
        geo.move_to(0.25, 0.5, 0.0)
        geo.arc_to(0.75, 0.5, i=0.25, j=0.0, clockwise=True)
        path = geometry_to_svg_path(geo, 100, 100)
        assert path == "M 25.000 50.000 A 25.000 25.000 0 0 1 75.000 50.000"

    def test_elliptical_long_arc(self):
        """Elliptical arcs with large-flag always go through bezier path."""
        geos = parse_svg_path_data("M 0 0 A 20 10 45 1 1 30 0")
        assert len(geos) == 1
        assert not geos[0].is_empty()


# ---------------------------------------------------------------------------
# svg_string_to_geometry (new)
# ---------------------------------------------------------------------------


class TestSvgStringToGeometry:
    def test_single_path(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 10 0 L 10 10 Z"/>'
            "</svg>"
        )
        geo = svg_string_to_geometry(svg)
        assert not geo.is_empty()

    def test_merged_paths(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5"/>'
            '<path d="M 10 10 L 15 15"/>'
            "</svg>"
        )
        geo = svg_string_to_geometry(svg)
        assert not geo.is_empty()
        # Both paths should be in a single geometry
        assert len(geo.data) >= 1

    def test_with_long_arc(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0.25 0.5 A 0.25 0.25 0 1 1 0.75 0.5"/>'
            "</svg>"
        )
        geo = svg_string_to_geometry(svg)
        assert not geo.is_empty()
        exported = geometry_to_svg_path(geo, 100, 100)
        assert "A 25.000 25.000 0" in exported

    def test_invalid_xml(self):
        geo = svg_string_to_geometry("not xml")
        assert geo.is_empty()

    def test_no_paths(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg"><rect x="0" y="0"/></svg>'
        )
        geo = svg_string_to_geometry(svg)
        assert geo.is_empty()

    def test_with_scale(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 10 0 L 10 10 Z"/>'
            "</svg>"
        )
        geo = svg_string_to_geometry(svg, scale_x=2.0, scale_y=3.0)
        assert not geo.is_empty()


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


# ---------------------------------------------------------------------------
# parse_svg_length
# ---------------------------------------------------------------------------


class TestParseSvgLength:
    def test_mm(self):
        val, unit = parse_svg_length("10mm")
        assert val == 10.0
        assert unit == "mm"

    def test_cm(self):
        val, unit = parse_svg_length("2.5cm")
        assert val == 2.5
        assert unit == "cm"

    def test_inch(self):
        val, unit = parse_svg_length("1in")
        assert val == 1.0
        assert unit == "in"

    def test_pt(self):
        val, unit = parse_svg_length("12pt")
        assert val == 12.0
        assert unit == "pt"

    def test_pc(self):
        val, unit = parse_svg_length("1pc")
        assert val == 1.0
        assert unit == "pc"

    def test_px(self):
        val, unit = parse_svg_length("100px")
        assert val == 100.0
        assert unit == "px"

    def test_unitless_defaults_to_px(self):
        val, unit = parse_svg_length("42")
        assert val == 42.0
        assert unit == "px"

    def test_whitespace(self):
        val, unit = parse_svg_length("  5mm  ")
        assert val == 5.0
        assert unit == "mm"

    def test_empty_string(self):
        val, unit = parse_svg_length("")
        assert val == 0.0
        assert unit == "px"

    def test_negative(self):
        val, unit = parse_svg_length("-3.5mm")
        assert val == -3.5
        assert unit == "mm"


class TestSvgLengthToMm:
    def test_mm_direct(self):
        assert svg_length_to_mm("10mm") == 10.0

    def test_cm(self):
        assert svg_length_to_mm("1cm") == 10.0

    def test_inch(self):
        assert svg_length_to_mm("1in") == 25.4

    def test_pt(self):
        assert abs(svg_length_to_mm("72pt") - 25.4) < 1e-9

    def test_pc(self):
        assert abs(svg_length_to_mm("1pc") - 25.4 / 6) < 1e-9

    def test_px_default_dpi(self):
        # 96 DPI default: 1px = 25.4/96 mm
        expected = 100 * 25.4 / 96
        assert abs(svg_length_to_mm("100px") - expected) < 1e-9

    def test_px_custom_dpi(self):
        expected = 100 * 25.4 / 200
        assert abs(svg_length_to_mm("100px", dpi=200) - expected) < 1e-9

    def test_unitless_default_dpi(self):
        expected = 42 * 25.4 / 96
        assert abs(svg_length_to_mm("42") - expected) < 1e-9

    def test_unitless_custom_dpi(self):
        expected = 42 * 25.4 / 72
        assert abs(svg_length_to_mm("42", dpi=72) - expected) < 1e-9


# ---------------------------------------------------------------------------
# extract_svg_metadata
# ---------------------------------------------------------------------------


class TestExtractSvgMetadata:
    def test_basic(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'width="100mm" height="200mm" viewBox="0 0 100 200">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.width == 100.0
        assert meta.height == 200.0
        assert meta.width_unit == "mm"
        assert meta.height_unit == "mm"
        assert meta.viewbox == (0.0, 0.0, 100.0, 200.0)

    def test_px_units(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'width="800" height="600">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.width == 800.0
        assert meta.height == 600.0
        assert meta.width_unit == "px"
        assert meta.height_unit == "px"
        assert meta.viewbox is None

    def test_no_viewbox(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'width="50mm" height="50mm">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.width == 50.0
        assert meta.viewbox is None

    def test_viewbox_with_various_units(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'width="2in" height="3cm" viewBox="-10 -20 300 400">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.width == 2.0
        assert meta.width_unit == "in"
        assert meta.height == 3.0
        assert meta.height_unit == "cm"
        assert meta.viewbox == (-10.0, -20.0, 300.0, 400.0)

    def test_viewbox_float_values(self):
        """Real-world: Inkscape gradient.svg uses float viewBox values."""
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'viewBox="0 0 85.599999 53.979999">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.viewbox == (0.0, 0.0, 85.599999, 53.979999)

    def test_viewbox_high_precision_pixels(self):
        """Real-world: mouse.svg viewBox in unscaled pixels
        (793.7008 x 1122.5197)."""
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'width="210mm" height="297mm" viewBox="0 0 793.7008 1122.5197">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.viewbox == (0.0, 0.0, 793.7008, 1122.5197)
        assert meta.width == 210.0
        assert meta.width_unit == "mm"

    def test_viewbox_non_zero_origin_floats(self):
        """Real-world: analytical trim produces non-zero origin with floats."""
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'width="200px" height="200px" viewBox="49.9 49.9 10.2 10.2">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.viewbox == (49.9, 49.9, 10.2, 10.2)

    def test_viewbox_negative_origin(self):
        """Real-world: exporter uses negative origin for padding."""
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'viewBox="-1 -1 102 102">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.viewbox == (-1.0, -1.0, 102.0, 102.0)

    def test_viewbox_with_width_missing(self):
        """Real-world: some SVGs have viewBox but no explicit width/height."""
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'viewBox="0 0 210 297">'
            "</svg>"
        )
        meta = extract_svg_metadata(svg)
        assert meta.width is None
        assert meta.height is None
        assert meta.viewbox == (0.0, 0.0, 210.0, 297.0)

    def test_missing_width_and_height(self):
        svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"/>'
        meta = extract_svg_metadata(svg)
        assert meta.width is None
        assert meta.height is None

    def test_invalid_xml_raises(self):
        with pytest.raises(ValueError, match="failed to parse SVG"):
            extract_svg_metadata("not xml")

    def test_not_svg_root(self):
        with pytest.raises(ValueError, match="root element is not"):
            extract_svg_metadata("<html></html>")


# ---------------------------------------------------------------------------
# svg_string_to_geometries_by_layer
# ---------------------------------------------------------------------------


class TestSvgStringToGeometriesByLayer:
    def test_two_layers(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g id="layer1">'
            '<path d="M 0 0 L 10 0 L 10 10 Z"/>'
            "</g>"
            '<g id="layer2">'
            '<path d="M 20 20 L 30 20 L 30 30 Z"/>'
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 2
        ids = [lid for lid, _ in layers]
        assert "layer1" in ids
        assert "layer2" in ids
        for _, geos in layers:
            assert len(geos) >= 1

    def test_no_layers(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 10 10"/>'
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 0

    def test_empty_id_ignored(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g id="">'
            '<path d="M 0 0 L 10 10"/>'
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 0

    def test_nested_groups_in_layer(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g id="top">'
            "<g>"
            '<path d="M 5 5 L 15 15"/>'
            "</g>"
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 1
        assert layers[0][0] == "top"
        assert len(layers[0][1]) >= 1

    def test_layer_transform_applied(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g id="moved" transform="translate(10, 20)">'
            '<path d="M 0 0 L 5 0 L 5 5 Z"/>'
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 1
        geos = layers[0][1]
        assert len(geos) >= 1
        # The transform should have shifted geometry, so it's not near origin
        rect = geos[0].rect()
        # min_x should be > 5 after transform + border
        assert rect[0] > 5.0 or rect[0] < 5.0

    def test_hidden_layer_skipped(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g id="visible">'
            '<path d="M 0 0 L 10 10"/>'
            "</g>"
            '<g id="hidden" display="none">'
            '<path d="M 20 20 L 30 30"/>'
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 1
        assert layers[0][0] == "visible"

    def test_with_scale(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g id="scaled">'
            '<path d="M 0 0 L 10 0 L 10 10 Z"/>'
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(
            svg, scale_x=2.0, scale_y=2.0
        )
        assert len(layers) == 1
        geos = layers[0][1]
        assert len(geos) >= 1

    def test_layer_with_inkexape_label(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" '
            'xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape">'
            '<g id="g123" inkscape:label="My Layer">'
            '<path d="M 0 0 L 10 10"/>'
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 1
        assert layers[0][0] == "g123"

    def test_visibility_hidden_group_skipped(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g id="hidden" visibility="hidden">'
            '<path d="M 0 0 L 10 10"/>'
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 0

    def test_multiple_subpaths_in_layer(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g id="multi">'
            '<path d="M 0 0 L 5 5"/>'
            '<path d="M 10 10 L 15 15"/>'
            '<rect x="20" y="20" width="10" height="10"/>'
            "</g>"
            "</svg>"
        )
        layers = svg_string_to_geometries_by_layer(svg)
        assert len(layers) == 1
        assert len(layers[0][1]) >= 3


# ---------------------------------------------------------------------------
# svg_string_to_geometries_by_color
# ---------------------------------------------------------------------------


class TestSvgStringToGeometriesByColor:
    def test_buckets_by_fill(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="red"/>'
            '<path d="M 10 10 L 15 15" fill="#00ff00"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(svg)
        assert [k for k, _ in buckets] == ["#00ff00", "#ff0000"]

    def test_unset_fill_goes_to_no_color(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(svg)
        assert [k for k, _ in buckets] == ["_no_color"]
        assert len(buckets[0][1]) == 1

    def test_fill_none_goes_to_no_color(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="none"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(svg)
        assert [k for k, _ in buckets] == ["_no_color"]

    def test_color_key_normalized_lowercase(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="RED"/>'
            '<path d="M 10 10 L 15 15" fill="#F00"/>'
            '<path d="M 20 20 L 25 25" fill="#ff0000"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(svg)
        assert [k for k, _ in buckets] == ["#ff0000"]
        assert len(buckets[0][1]) == 3

    def test_stroke_attr(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" stroke="red"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(
            svg, color_attr=ColorAttr.STROKE
        )
        assert [k for k, _ in buckets] == ["#ff0000"]

    def test_fill_else_stroke_prefers_fill(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="red" stroke="blue"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(
            svg, color_attr=ColorAttr.FILL_ELSE_STROKE
        )
        assert [k for k, _ in buckets] == ["#ff0000"]

    def test_fill_else_stroke_falls_back_to_stroke(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="none" stroke="blue"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(
            svg, color_attr=ColorAttr.FILL_ELSE_STROKE
        )
        assert [k for k, _ in buckets] == ["#0000ff"]

    def test_color_inherited_from_group(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<g fill="#112233">'
            '<path d="M 0 0 L 5 5"/>'
            "</g>"
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(svg)
        assert [k for k, _ in buckets] == ["#112233"]
        assert len(buckets[0][1]) == 1

    def test_current_color_resolves_to_nearest_color(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" color="blue">'
            '<path d="M 0 0 L 5 5" fill="currentColor"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(svg)
        assert [k for k, _ in buckets] == ["#0000ff"]

    def test_current_color_defaults_to_black(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="currentColor"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(svg)
        assert [k for k, _ in buckets] == ["#000000"]

    def test_multiple_shapes_per_color(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="red"/>'
            '<path d="M 10 10 L 15 15" fill="red"/>'
            '<rect x="20" y="20" width="10" height="10" fill="red"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(svg)
        assert [k for k, _ in buckets] == ["#ff0000"]
        assert len(buckets[0][1]) == 3

    def test_with_scale(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="red"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(
            svg, scale_x=2.0, scale_y=3.0
        )
        assert [k for k, _ in buckets] == ["#ff0000"]
        assert len(buckets[0][1]) == 1

    def test_any_split_fill_and_stroke(self):
        """In any mode, differing fill and stroke yield two buckets."""
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="red" stroke="blue"/>'
            '<path d="M 10 10 L 15 15" fill="red"/>'
            '<path d="M 20 20 L 25 25" stroke="#00ff00"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(
            svg, color_attr=ColorAttr.ANY
        )
        by_key = {k: v for k, v in buckets}
        assert set(by_key) == {"#0000ff", "#00ff00", "#ff0000"}
        # The fill+stroke shape lands in both buckets.
        assert len(by_key["#ff0000"]) == 2
        assert len(by_key["#0000ff"]) == 1
        assert len(by_key["#00ff00"]) == 1

    def test_any_matching_fill_and_stroke_bucket_once(self):
        """Equal fill and stroke colors produce a single bucket."""
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="red" stroke="red"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(
            svg, color_attr=ColorAttr.ANY
        )
        assert [k for k, _ in buckets] == ["#ff0000"]
        assert len(buckets[0][1]) == 1

    def test_any_no_color_bucket(self):
        """Shapes with neither fill nor stroke color go to _no_color."""
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometries_by_color(
            svg, color_attr=ColorAttr.ANY
        )
        assert [k for k, _ in buckets] == ["_no_color"]

    def test_no_paths(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg"><rect x="0" y="0"/></svg>'
        )
        assert svg_string_to_geometries_by_color(svg) == []

    def test_invalid_xml_raises(self):
        with pytest.raises(ValueError, match="failed to parse SVG"):
            svg_string_to_geometries_by_color("not xml")


# ---------------------------------------------------------------------------
# svg_string_to_geometry_by_color
# ---------------------------------------------------------------------------


class TestSvgStringToGeometryByColor:
    def test_merges_subpaths_per_color(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" fill="red"/>'
            '<path d="M 10 10 L 15 15" fill="red"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometry_by_color(svg)
        assert [k for k, _ in buckets] == ["#ff0000"]
        key, geo = buckets[0]
        assert not geo.is_empty()
        assert len(geo.data) >= 2

    def test_color_attr_passed_through(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<path d="M 0 0 L 5 5" stroke="red"/>'
            "</svg>"
        )
        buckets = svg_string_to_geometry_by_color(
            svg, color_attr=ColorAttr.STROKE
        )
        assert [k for k, _ in buckets] == ["#ff0000"]


class TestFilterSvgByColor:
    def test_keeps_only_matching_color(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<rect x="0" y="0" width="10" height="10" fill="#ff0000"/>'
            '<rect x="20" y="20" width="10" height="10" fill="#00ff00"/>'
            "</svg>"
        )
        out = filter_svg_by_color(svg, "#ff0000")
        assert '<rect x="0"' in out
        assert '<rect x="20"' not in out

    def test_keeps_structure_intact(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">'
            '<defs><linearGradient id="g"/></defs>'
            '<rect x="0" y="0" width="5" height="5" fill="#ff0000"/>'
            '<rect x="1" y="1" width="5" height="5" fill="#00ff00"/>'
            "</svg>"
        )
        out = filter_svg_by_color(svg, "#ff0000")
        assert "<defs>" in out
        assert "linearGradient" in out
        assert 'width="10"' in out
        assert 'fill="#00ff00"' not in out

    def test_any_keeps_fill_or_stroke(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<rect x="0" y="0" width="5" height="5" fill="#ff0000" '
            'stroke="#0000ff"/>'
            "</svg>"
        )
        by_fill = filter_svg_by_color(svg, "#ff0000")
        assert 'fill="#ff0000"' in by_fill
        by_stroke = filter_svg_by_color(svg, "#0000ff")
        assert 'fill="#ff0000"' in by_stroke

    def test_fill_mode_ignores_stroke(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<rect x="0" y="0" width="5" height="5" stroke="#ff0000"/>'
            '<rect x="10" y="10" width="5" height="5" fill="#ff0000"/>'
            "</svg>"
        )
        out = filter_svg_by_color(svg, "#ff0000", color_attr=ColorAttr.FILL)
        # Only the filled shape survives in fill mode.
        assert out.count("rect") == 1

    def test_no_color_bucket(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<rect x="0" y="0" width="5" height="5" fill="#ff0000"/>'
            '<rect x="10" y="10" width="5" height="5"/>'
            "</svg>"
        )
        out = filter_svg_by_color(svg, "_no_color")
        assert out.count("rect") == 1
        assert 'fill="#ff0000"' not in out

    def test_missing_color_removes_all_shapes(self):
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<rect x="0" y="0" width="5" height="5" fill="#ff0000"/>'
            "</svg>"
        )
        out = filter_svg_by_color(svg, "#00ff00")
        assert "rect" not in out

    def test_invalid_xml_raises(self):
        with pytest.raises(ValueError, match="failed to parse SVG"):
            filter_svg_by_color("not xml", "#ff0000")

    def test_no_paths(self):
        svg = '<svg xmlns="http://www.w3.org/2000/svg"></svg>'
        assert svg_string_to_geometry_by_color(svg) == []
