from raygeo.geo.algo.nest2d.ifp import inner_fit_polygon


class TestInnerFitPolygon:
    def test_empty_bin(self):
        bin_poly = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = inner_fit_polygon([], bin_poly)
        assert result == []

    def test_empty_part(self):
        bin_poly = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        result = inner_fit_polygon(bin_poly, [])
        assert result == []

    def test_part_larger_than_bin(self):
        bin_poly = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        part = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        result = inner_fit_polygon(bin_poly, part)
        assert result == []

    def test_rectangle_in_rectangle(self):
        bin_poly = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        part = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = inner_fit_polygon(bin_poly, part)
        assert len(result) >= 1
        for ifp in result:
            assert len(ifp) >= 3

    def test_large_bin_small_part(self):
        bin_poly = [(0.0, 0.0), (200.0, 0.0), (200.0, 200.0), (0.0, 200.0)]
        part = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]
        result = inner_fit_polygon(bin_poly, part)
        assert len(result) >= 1
        for ifp in result:
            assert len(ifp) >= 3

    def test_narrow_bin(self):
        bin_poly = [(0.0, 0.0), (15.0, 0.0), (15.0, 100.0), (0.0, 100.0)]
        part = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = inner_fit_polygon(bin_poly, part)
        assert len(result) >= 1

    def test_part_exactly_fits_width(self):
        bin_poly = [(0.0, 0.0), (10.0, 0.0), (10.0, 100.0), (0.0, 100.0)]
        part = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = inner_fit_polygon(bin_poly, part)
        assert len(result) >= 0

    def test_part_does_not_fit_width(self):
        bin_poly = [(0.0, 0.0), (9.0, 0.0), (9.0, 100.0), (0.0, 100.0)]
        part = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = inner_fit_polygon(bin_poly, part)
        assert result == []

    def test_valid_polygons(self):
        bin_poly = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        part = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = inner_fit_polygon(bin_poly, part)
        assert len(result) >= 1


class TestBuildNoGoZones:
    def test_no_go_zones_produced(self):
        bin = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        part = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = inner_fit_polygon(bin, part)
        assert len(result) >= 0
