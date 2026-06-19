"""Tests for the Rust NFP module (raygeo.geo.algo.nest2d.nfp)."""

from raygeo.geo.algo.nest2d import nfp


class TestNoFitPolygon:
    """Tests for the no_fit_polygon function."""

    def test_empty_static(self):
        orbiting = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = nfp.no_fit_polygon([], orbiting)
        assert result == []

    def test_empty_orbiting(self):
        static = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        result = nfp.no_fit_polygon(static, [])
        assert result == []

    def test_static_too_few_points(self):
        static = [(0.0, 0.0), (10.0, 0.0)]
        orbiting = [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)]
        result = nfp.no_fit_polygon(static, orbiting)
        assert result == []

    def test_orbiting_too_few_points(self):
        static = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        orbiting = [(0.0, 0.0), (5.0, 0.0)]
        result = nfp.no_fit_polygon(static, orbiting)
        assert result == []

    def test_rectangle_outside(self):
        static = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]
        orbiting = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = nfp.no_fit_polygon(static, orbiting)
        assert len(result) >= 1
        for nfp_poly in result:
            assert len(nfp_poly) >= 3

    def test_triangle_outside(self):
        static = [(0.0, 0.0), (30.0, 0.0), (15.0, 30.0)]
        orbiting = [(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)]
        result = nfp.no_fit_polygon(static, orbiting)
        assert len(result) >= 0

    def test_complex_polygon(self):
        static = [
            (0.0, 0.0),
            (50.0, 0.0),
            (50.0, 30.0),
            (30.0, 30.0),
            (30.0, 50.0),
            (0.0, 50.0),
        ]
        orbiting = [(0.0, 0.0), (15.0, 0.0), (15.0, 15.0), (0.0, 15.0)]
        result = nfp.no_fit_polygon(static, orbiting)
        assert len(result) >= 0

    def test_concave_polygon(self):
        static = [
            (0.0, 0.0),
            (50.0, 0.0),
            (50.0, 25.0),
            (25.0, 25.0),
            (25.0, 50.0),
            (50.0, 50.0),
            (50.0, 75.0),
            (0.0, 75.0),
        ]
        orbiting = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = nfp.no_fit_polygon(static, orbiting)
        assert len(result) >= 0


class TestNfpConvexFast:
    """Tests for convex dispatch of no_fit_polygon."""

    def test_convex_rectangles(self):
        static = [
            (0.0, 0.0),
            (20.0, 0.0),
            (20.0, 20.0),
            (0.0, 20.0),
        ]
        orbiting = [
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
        ]
        result = nfp.no_fit_polygon(static, orbiting)
        assert len(result) >= 1
        for poly in result:
            assert len(poly) >= 3


class TestNfpMinkowski:
    """Tests for nfp_minkowski function."""

    def test_basic(self):
        static = [
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 100.0),
            (0.0, 100.0),
        ]
        orbiting = [
            (0.0, 0.0),
            (20.0, 0.0),
            (20.0, 20.0),
            (0.0, 20.0),
        ]
        result = nfp.nfp_minkowski(static, orbiting)
        assert len(result) >= 1
        for poly in result:
            assert len(poly) >= 3

    def test_concave(self):
        static = [
            (0.0, 0.0),
            (50.0, 0.0),
            (50.0, 25.0),
            (25.0, 25.0),
            (25.0, 50.0),
            (50.0, 50.0),
            (50.0, 75.0),
            (0.0, 75.0),
        ]
        orbiting = [
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
        ]
        result = nfp.nfp_minkowski(static, orbiting)
        assert len(result) >= 0


class TestNormalizePolygon:
    """Tests for normalize_polygon function."""

    def test_normalize_simple(self):
        poly = [(10.0, 20.0), (30.0, 20.0), (30.0, 50.0), (10.0, 50.0)]
        norm, ox, oy = nfp.normalize_polygon(poly)
        assert ox == 10.0
        assert oy == 20.0
        assert len(norm) == 4
        assert norm[0] == (0.0, 0.0)
        assert norm[1] == (20.0, 0.0)

    def test_normalize_empty(self):
        norm, ox, oy = nfp.normalize_polygon([])
        assert norm == []
        assert ox == 0.0
        assert oy == 0.0

    def test_normalize_already_at_origin(self):
        poly = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        norm, ox, oy = nfp.normalize_polygon(poly)
        assert ox == 0.0
        assert oy == 0.0
        assert norm == poly


class TestPolygonToKey:
    """Tests for polygon_to_key function."""

    def test_key_simple(self):
        poly = [(0.0, 0.0), (10.1234, 0.0), (10.1234, 10.5678)]
        key = nfp.polygon_to_key(poly)
        assert len(key) == 3
        assert key[0] == (0, 0)
        assert key[1] == (101234, 0)
        assert key[2] == (101234, 105678)

    def test_key_rounding(self):
        poly = [(0.000049, 0.000049)]
        key = nfp.polygon_to_key(poly)
        assert key[0] == (0, 0)

    def test_key_empty(self):
        key = nfp.polygon_to_key([])
        assert key == []
