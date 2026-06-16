from raygeo.geo.algo.nest2d.gravity import apply_gravity, find_max_slide


class TestFindMaxSlide:
    def test_no_slide_needed(self):
        polys = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]
        sheet = [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]
        sheet_bounds = (0.0, 0.0, 50.0, 50.0)
        result = find_max_slide(polys, [], sheet_bounds, sheet, "y", 0.0)
        assert result == 0.0

    def test_slide_y(self):
        polys = [[(0.0, 10.0), (10.0, 10.0), (10.0, 20.0), (0.0, 20.0)]]
        sheet = [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]
        sheet_bounds = (0.0, 0.0, 50.0, 50.0)
        result = find_max_slide(polys, [], sheet_bounds, sheet, "y", 0.0)
        assert result > 9.0

    def test_slide_x(self):
        polys = [[(10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (10.0, 10.0)]]
        sheet = [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]
        sheet_bounds = (0.0, 0.0, 50.0, 50.0)
        result = find_max_slide(polys, [], sheet_bounds, sheet, "x", 0.0)
        assert result > 9.0

    def test_blocked_by_other(self):
        polys = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]
        other = [[[(0.0, 12.0), (10.0, 12.0), (10.0, 22.0), (0.0, 22.0)]]]
        sheet = [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]
        sheet_bounds = (0.0, 0.0, 50.0, 50.0)
        result = find_max_slide(polys, other, sheet_bounds, sheet, "y", 0.0)
        assert result < 2.0

    def test_zero_spacing(self):
        polys = [[(0.0, 10.0), (10.0, 10.0), (10.0, 20.0), (0.0, 20.0)]]
        sheet = [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]
        sheet_bounds = (0.0, 0.0, 50.0, 50.0)
        result = find_max_slide(polys, [], sheet_bounds, sheet, "y", 0.0)
        assert result > 9.0


class TestApplyGravity:
    def test_single_placement_no_op(self):
        groups = [[[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]]
        sheet = [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]
        result = apply_gravity(groups, sheet, 0.0)
        assert len(result) == 1
        assert result[0] == (0.0, 0.0)

    def test_both_slide(self):
        groups = [
            [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]],
            [[(20.0, 30.0), (30.0, 30.0), (30.0, 40.0), (20.0, 40.0)]],
        ]
        sheet = [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]
        result = apply_gravity(groups, sheet, 0.0)
        assert len(result) == 2
        assert result[0] == (0.0, 0.0)
        assert result[1][0] < 0.0
        assert result[1][1] < 0.0

    def test_empty_groups(self):
        sheet = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = apply_gravity([], sheet, 0.0)
        assert result == []

    def test_already_compact(self):
        groups = [
            [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]],
            [[(0.0, 10.0), (10.0, 10.0), (10.0, 20.0), (0.0, 20.0)]],
        ]
        sheet = [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]
        result = apply_gravity(groups, sheet, 0.0)
        assert result == [(0.0, 0.0), (0.0, 0.0)]
