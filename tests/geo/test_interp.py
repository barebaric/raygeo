import pytest

from raygeo.geo.algo.interp import (
    compute_segment_delta,
    compute_t_range,
    project_t_along_segment,
    slice_scanline_data,
    solve_quadratic,
)


class TestComputeSegmentDelta:
    def test_basic(self):
        dx, dy, dz, len_sq = compute_segment_delta((0, 0, 0), (3, 4, 0))
        assert dx == pytest.approx(3.0)
        assert dy == pytest.approx(4.0)
        assert dz == pytest.approx(0.0)
        assert len_sq == pytest.approx(25.0)

    def test_zero_length(self):
        dx, dy, dz, len_sq = compute_segment_delta((1, 2, 3), (1, 2, 3))
        assert len_sq == pytest.approx(0.0)


class TestProjectTAlongSegment:
    def test_midpoint(self):
        d = compute_segment_delta((0, 0, 0), (10, 0, 0))
        t = project_t_along_segment((0, 0, 0), (5, 0, 0), d)
        assert t == pytest.approx(0.5)

    def test_start(self):
        d = compute_segment_delta((0, 0, 0), (10, 0, 0))
        t = project_t_along_segment((0, 0, 0), (0, 0, 0), d)
        assert t == pytest.approx(0.0)

    def test_end(self):
        d = compute_segment_delta((0, 0, 0), (10, 0, 0))
        t = project_t_along_segment((0, 0, 0), (10, 0, 0), d)
        assert t == pytest.approx(1.0)

    def test_beyond_start(self):
        d = compute_segment_delta((5, 0, 0), (10, 0, 0))
        t = project_t_along_segment((5, 0, 0), (0, 0, 0), d)
        assert t == pytest.approx(0.0)

    def test_beyond_end(self):
        d = compute_segment_delta((0, 0, 0), (10, 0, 0))
        t = project_t_along_segment((0, 0, 0), (20, 0, 0), d)
        assert t == pytest.approx(1.0)

    def test_degenerate_segment(self):
        d = compute_segment_delta((5, 5, 5), (5, 5, 5))
        t = project_t_along_segment((5, 5, 5), (10, 10, 10), d)
        assert t == pytest.approx(0.0)


class TestComputeTRange:
    def test_basic(self):
        d = compute_segment_delta((0, 0, 0), (10, 0, 0))
        t_s, t_e = compute_t_range((0, 0, 0), (2, 0, 0), (8, 0, 0), d)
        assert t_s == pytest.approx(0.2)
        assert t_e == pytest.approx(0.8)

    def test_full_range(self):
        d = compute_segment_delta((0, 0, 0), (10, 0, 0))
        t_s, t_e = compute_t_range((0, 0, 0), (0, 0, 0), (10, 0, 0), d)
        assert t_s == pytest.approx(0.0)
        assert t_e == pytest.approx(1.0)

    def test_degenerate_segment(self):
        d = compute_segment_delta((5, 5, 5), (5, 5, 5))
        t_s, t_e = compute_t_range((5, 5, 5), (2, 0, 0), (8, 0, 0), d)
        assert t_s == pytest.approx(0.0)
        assert t_e == pytest.approx(1.0)


class TestSliceScanlineData:
    def test_basic(self):
        data = [10, 20, 30, 40, 50]
        sliced = slice_scanline_data(data, 0.2, 0.6)
        assert sliced == [20, 30]

    def test_full_range(self):
        data = [10, 20, 30, 40, 50]
        sliced = slice_scanline_data(data, 0.0, 1.0)
        assert sliced == data

    def test_empty_range(self):
        data = [10, 20, 30, 40, 50]
        sliced = slice_scanline_data(data, 0.5, 0.5)
        assert sliced == []


class TestSolveQuadratic:
    def test_two_roots(self):
        r1, r2 = solve_quadratic(1.0, -3.0, 2.0)
        assert r1 == pytest.approx(1.0)
        assert r2 == pytest.approx(2.0)

    def test_one_root_linear(self):
        r1, r2 = solve_quadratic(0.0, 2.0, -4.0)
        assert r1 == pytest.approx(2.0)
        assert r2 is None

    def test_no_root_constant(self):
        r1, r2 = solve_quadratic(0.0, 0.0, 1.0)
        assert r1 is None
        assert r2 is None

    def test_negative_discriminant(self):
        r1, r2 = solve_quadratic(1.0, 0.0, 1.0)
        assert r1 is None
        assert r2 is None

    def test_perfect_square(self):
        r1, r2 = solve_quadratic(1.0, -2.0, 1.0)
        assert r1 == pytest.approx(1.0)
        assert r2 == pytest.approx(1.0)
