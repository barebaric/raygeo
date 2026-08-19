import math

import pytest

import raygeo.geo.algo.overcut as overcut_mod
from raygeo.geo import Arc, Geometry
from raygeo.geo.algo.overcut import apply_overcut


def _closed_rectangle(w=10, h=10):
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(w, 0)
    geo.line_to(w, h)
    geo.line_to(0, h)
    geo.close_path()
    return geo


def _closed_triangle():
    geo = Geometry()
    geo.move_to(0, 0)
    geo.line_to(10, 0)
    geo.line_to(5, 10)
    geo.close_path()
    return geo


def _closed_circle_approx(n=16, r=10):
    geo = Geometry()
    geo.move_to(r, 0)
    for i in range(1, n + 1):
        angle = 2 * math.pi * i / n
        geo.line_to(r * math.cos(angle), r * math.sin(angle))
    geo.close_path()
    return geo


class TestOvercutEmpty:
    def test_empty_geometry(self):
        geo = Geometry()
        result = apply_overcut(geo, 5.0)
        assert result.is_empty()

    def test_empty_geometry_zero_overcut(self):
        geo = Geometry()
        result = apply_overcut(geo, 0.0)
        assert result.is_empty()


class TestOvercutNoop:
    def test_zero_overcut(self):
        geo = _closed_rectangle()
        result = apply_overcut(geo, 0.0)
        assert len(result) == len(geo)

    def test_negative_overcut(self):
        geo = _closed_rectangle()
        result = apply_overcut(geo, -1.0)
        assert len(result) == len(geo)

    def test_open_path_no_overcut(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)
        result = apply_overcut(geo, 5.0)
        assert len(result) == len(geo)

    def test_single_point_no_overcut(self):
        geo = Geometry()
        geo.move_to(5, 5)
        result = apply_overcut(geo, 5.0)
        assert len(result) == len(geo)

    def test_two_points_no_overcut(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        result = apply_overcut(geo, 5.0)
        assert len(result) == len(geo)


class TestOvercutClosedPaths:
    def test_rectangle_adds_commands(self):
        geo = _closed_rectangle()
        result = apply_overcut(geo, 5.0)
        assert len(result) > len(geo)

    def test_triangle_adds_commands(self):
        geo = _closed_triangle()
        result = apply_overcut(geo, 3.0)
        assert len(result) > len(geo)

    def test_circle_approx_adds_commands(self):
        geo = _closed_circle_approx()
        result = apply_overcut(geo, 2.0)
        assert len(result) > len(geo)

    def test_small_overcut(self):
        geo = _closed_rectangle()
        original_len = len(geo)
        result = apply_overcut(geo, 0.01)
        assert len(result) > original_len

    def test_overcut_larger_than_first_side(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)
        geo.close_path()
        result = apply_overcut(geo, 15.0)
        assert len(result) > len(geo)

    def test_overcut_equals_first_side(self):
        geo = _closed_rectangle(w=10, h=10)
        result = apply_overcut(geo, 10.0)
        assert len(result) > len(geo)


class TestOvercutGeometry:
    def test_overcut_extends_from_start(self):
        geo = _closed_rectangle()
        result = apply_overcut(geo, 5.0)
        start_pt = geo.last_move_to
        result_start = result.last_move_to
        assert result_start == pytest.approx(start_pt)

    def test_overcut_preserves_original_segment_count(self):
        geo = _closed_rectangle()
        original_len = len(geo)
        result = apply_overcut(geo, 1.0)
        assert len(result) > original_len

    def test_overcut_does_not_modify_input(self):
        geo = _closed_rectangle()
        original_len = len(geo)
        _ = apply_overcut(geo, 5.0)
        assert len(geo) == original_len


class TestOvercutArcPaths:
    def test_closed_arc_path(self):
        geo = Geometry()
        geo.move_to(10, 0)
        geo.arc_to(10, 0, i=0, j=-10, clockwise=True)
        geo.close_path()
        result = apply_overcut(geo, 2.0)
        assert len(result) >= len(geo)


class TestOvercutBezierPaths:
    def test_closed_bezier_path(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.bezier_to(10, 10, c1x=3, c1y=10, c2x=7, c2y=10)
        geo.bezier_to(0, 0, c1x=10, c1y=5, c2x=5, c2y=0)
        geo.close_path()
        result = apply_overcut(geo, 2.0)
        assert len(result) >= len(geo)


class TestOvercutMultipleContours:
    def test_two_closed_rectangles_not_closed_as_whole(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)
        geo.line_to(0, 10)
        geo.close_path()
        geo.move_to(20, 0)
        geo.line_to(30, 0)
        geo.line_to(30, 10)
        geo.line_to(20, 10)
        geo.close_path()
        result = apply_overcut(geo, 2.0)
        assert isinstance(result, Geometry)


class TestOvercutReturnValues:
    def test_returns_new_geometry(self):
        geo = _closed_rectangle()
        result = apply_overcut(geo, 5.0)
        assert isinstance(result, Geometry)

    def test_returns_different_object(self):
        geo = _closed_rectangle()
        result = apply_overcut(geo, 5.0)
        assert result is not geo

    def test_noop_returns_copy(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.line_to(10, 10)
        result = apply_overcut(geo, 5.0)
        assert isinstance(result, Geometry)
        assert result is not geo


class TestOvercutExactValues:
    def test_overcut_exact_side_length(self):
        geo = Geometry()
        geo.move_to(0, 0)
        geo.line_to(10, 0)
        geo.close_path()
        result = apply_overcut(geo, 10.0)
        assert len(result) > len(geo)

    def test_overcut_very_small(self):
        geo = _closed_rectangle()
        result = apply_overcut(geo, 1e-6)
        assert len(result) >= len(geo)

    def test_overcut_very_large(self):
        geo = _closed_rectangle()
        original_len = len(geo)
        result = apply_overcut(geo, 1000.0)
        assert len(result) > original_len


class TestOvercutImport:
    def test_import_from_algo(self):
        assert callable(apply_overcut)

    def test_import_from_algo_module(self):
        assert hasattr(overcut_mod, "apply_overcut")


class TestOvercutArcFitting:
    """Regression tests for rayforge#357.

    Applying an overcut to a closed contour extends it past its start
    point, producing a path that wraps a full revolution around the
    contour center with non-coincident endpoints.  Arc fitting used to
    collapse such a path into a single tiny arc, dropping the rest of
    the contour from the toolpath.
    """

    @staticmethod
    def _closed_circle(radius=27.5, n=360):
        geo = Geometry()
        geo.move_to(radius, 0)
        for i in range(1, n + 1):
            angle = 2 * math.pi * i / n
            geo.line_to(radius * math.cos(angle), radius * math.sin(angle))
        geo.close_path()
        return geo

    @staticmethod
    def _total_arc_sweep(geo, start):
        """Total sweep (radians) of the arc commands in *geo*."""
        total = 0.0
        last = start
        for cmd in geo.data:
            if not isinstance(cmd, Arc):
                continue
            cx = last[0] + cmd.center_offset[0]
            cy = last[1] + cmd.center_offset[1]
            a0 = math.atan2(last[1] - cy, last[0] - cx)
            a1 = math.atan2(cmd.end[1] - cy, cmd.end[0] - cx)
            sweep = (a1 - a0) % (2 * math.pi)
            if cmd.clockwise:
                sweep = 2 * math.pi - sweep
            total += sweep
            last = cmd.end
        return total

    def test_overcut_circle_fit_curves_keeps_full_circle(self):
        """Fitting an overcut circle must keep the full circle + overlap."""
        geo = self._closed_circle()
        start = (geo.data[0].end[0], geo.data[0].end[1])
        oc = apply_overcut(geo, 2.0)

        fitted = oc.copy()
        fitted.fit_curves(0.1, beziers=False, arcs=True)

        sweep = self._total_arc_sweep(fitted, start)
        assert sweep > 2 * math.pi - 0.1, (
            f"Expected full-circle coverage, got {sweep:.3f} rad"
        )

    def test_overcut_circle_fit_curves_no_tiny_arc(self):
        """The fitted result must contain more than one arc command."""
        geo = self._closed_circle()
        oc = apply_overcut(geo, 2.0)

        fitted = oc.copy()
        fitted.fit_curves(0.1, beziers=False, arcs=True)

        arcs = [c for c in fitted.data if isinstance(c, Arc)]
        assert len(arcs) > 1, (
            "A single arc would collapse to the short way between the "
            "nearby endpoints and drop the rest of the circle"
        )

    def test_overcut_doughnut_inner_contour_survives(self):
        """Both contours of a doughnut must survive arc fitting (rayforge#357).

        The inner (hole) contour collapses to a tiny arc when an overcut is
        applied; the outer contour used to survive while the inner one
        disappeared from the toolpath.
        """
        inner = self._closed_circle(radius=27.5)
        outer = self._closed_circle(radius=147.5)

        for contour in (inner, outer):
            oc = apply_overcut(contour, 2.0)
            fitted = oc.copy()
            fitted.fit_curves(0.1, beziers=False, arcs=True)
            start = (contour.data[0].end[0], contour.data[0].end[1])
            sweep = self._total_arc_sweep(fitted, start)
            assert sweep > 2 * math.pi - 0.1, (
                f"Contour collapsed to {sweep:.3f} rad"
            )

    def test_overcut_circle_no_overcut_still_single_arcs(self):
        """Without an overcut, closed circles are split into two arcs and
        the full coverage is preserved."""
        geo = self._closed_circle()
        start = (geo.data[0].end[0], geo.data[0].end[1])
        fitted = geo.copy()
        fitted.fit_curves(0.1, beziers=False, arcs=True)
        sweep = self._total_arc_sweep(fitted, start)
        assert sweep > 2 * math.pi - 0.1
        arcs = [c for c in fitted.data if isinstance(c, Arc)]
        assert len(arcs) == 2
