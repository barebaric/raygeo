import pytest

from raygeo.geo.algo.interp import (
    barycentric_interpolate,
    compute_segment_delta_3d,
    compute_t_range,
    project_t_along_segment,
    slice_scanline_data,
    solve_quadratic,
)


class TestComputeSegmentDelta:
    def test_basic(self):
        dx, dy, dz, len_sq = compute_segment_delta_3d((0, 0, 0), (3, 4, 0))
        assert dx == pytest.approx(3.0)
        assert dy == pytest.approx(4.0)
        assert dz == pytest.approx(0.0)
        assert len_sq == pytest.approx(25.0)

    def test_zero_length(self):
        dx, dy, dz, len_sq = compute_segment_delta_3d((1, 2, 3), (1, 2, 3))
        assert len_sq == pytest.approx(0.0)


class TestProjectTAlongSegment:
    def test_midpoint(self):
        d = compute_segment_delta_3d((0, 0, 0), (10, 0, 0))
        t = project_t_along_segment((0, 0, 0), (5, 0, 0), d)
        assert t == pytest.approx(0.5)

    def test_start(self):
        d = compute_segment_delta_3d((0, 0, 0), (10, 0, 0))
        t = project_t_along_segment((0, 0, 0), (0, 0, 0), d)
        assert t == pytest.approx(0.0)

    def test_end(self):
        d = compute_segment_delta_3d((0, 0, 0), (10, 0, 0))
        t = project_t_along_segment((0, 0, 0), (10, 0, 0), d)
        assert t == pytest.approx(1.0)

    def test_beyond_start(self):
        d = compute_segment_delta_3d((5, 0, 0), (10, 0, 0))
        t = project_t_along_segment((5, 0, 0), (0, 0, 0), d)
        assert t == pytest.approx(0.0)

    def test_beyond_end(self):
        d = compute_segment_delta_3d((0, 0, 0), (10, 0, 0))
        t = project_t_along_segment((0, 0, 0), (20, 0, 0), d)
        assert t == pytest.approx(1.0)

    def test_degenerate_segment(self):
        d = compute_segment_delta_3d((5, 5, 5), (5, 5, 5))
        t = project_t_along_segment((5, 5, 5), (10, 10, 10), d)
        assert t == pytest.approx(0.0)


class TestComputeTRange:
    def test_basic(self):
        d = compute_segment_delta_3d((0, 0, 0), (10, 0, 0))
        t_s, t_e = compute_t_range((0, 0, 0), (2, 0, 0), (8, 0, 0), d)
        assert t_s == pytest.approx(0.2)
        assert t_e == pytest.approx(0.8)

    def test_full_range(self):
        d = compute_segment_delta_3d((0, 0, 0), (10, 0, 0))
        t_s, t_e = compute_t_range((0, 0, 0), (0, 0, 0), (10, 0, 0), d)
        assert t_s == pytest.approx(0.0)
        assert t_e == pytest.approx(1.0)

    def test_degenerate_segment(self):
        d = compute_segment_delta_3d((5, 5, 5), (5, 5, 5))
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


class TestBarycentricInterpolate:
    def test_vertex_a(self):
        v = barycentric_interpolate(
            (0.0, 0.0), (0, 0), (1, 0), (0, 1), 10.0, 20.0, 30.0
        )
        assert v == pytest.approx(10.0)

    def test_vertex_b(self):
        v = barycentric_interpolate(
            (1.0, 0.0), (0, 0), (1, 0), (0, 1), 10.0, 20.0, 30.0
        )
        assert v == pytest.approx(20.0)

    def test_vertex_c(self):
        v = barycentric_interpolate(
            (0.0, 1.0), (0, 0), (1, 0), (0, 1), 10.0, 20.0, 30.0
        )
        assert v == pytest.approx(30.0)

    def test_centroid(self):
        v = barycentric_interpolate(
            (1.0 / 3.0, 1.0 / 3.0),
            (0, 0),
            (1, 0),
            (0, 1),
            10.0,
            20.0,
            30.0,
        )
        assert v == pytest.approx(20.0)

    def test_midpoint_ab(self):
        v = barycentric_interpolate(
            (0.5, 0.0), (0, 0), (1, 0), (0, 1), 10.0, 20.0, 30.0
        )
        assert v == pytest.approx(15.0)

    def test_linear_field_x(self):
        va = (0, 0)
        vb = (2, 0)
        vc = (0, 2)
        ua, ub, uc = 0.0, 1.0, 0.0
        v = barycentric_interpolate(
            (0.5, 0.0),
            va,
            vb,
            vc,
            ua,
            ub,
            uc,
        )
        assert v == pytest.approx(0.25)

    def test_linear_field_y(self):
        va = (0, 0)
        vb = (1, 0)
        vc = (0, 3)
        ua, ub, uc = 0.0, 0.0, 1.0
        v = barycentric_interpolate(
            (0.0, 1.5),
            va,
            vb,
            vc,
            ua,
            ub,
            uc,
        )
        assert v == pytest.approx(0.5)

    def test_outside_point_clamped(self):
        v = barycentric_interpolate(
            (10.0, 10.0), (0, 0), (1, 0), (0, 1), 10.0, 20.0, 30.0
        )
        assert 10.0 <= v <= 30.0

    def test_negative_point_clamped(self):
        v = barycentric_interpolate(
            (-1.0, -1.0), (0, 0), (1, 0), (0, 1), 10.0, 20.0, 30.0
        )
        assert 10.0 <= v <= 30.0

    def test_collapsed_triangle(self):
        v = barycentric_interpolate(
            (0.5, 0.5), (0, 0), (0, 0), (1, 1), 10.0, 20.0, 30.0
        )
        assert v == pytest.approx(20.0)

    def test_large_coordinates(self):
        v = barycentric_interpolate(
            (5000.0, 5000.0),
            (0, 0),
            (10000, 0),
            (0, 10000),
            0.0,
            100.0,
            200.0,
        )
        assert v == pytest.approx(150.0)

    def test_non_unit_triangle(self):
        v = barycentric_interpolate(
            (2.5, 2.5), (2, 2), (4, 2), (2, 4), 10.0, 20.0, 30.0
        )
        assert v == pytest.approx(17.5)

    def test_scalar_field_identity(self):
        """For a field u=x+y on triangle (0,0),(1,0),(0,1),
        at (0.2, 0.3) we expect 0.5."""
        va, vb, vc = (0, 0), (1, 0), (0, 1)
        x, y = 0.2, 0.3
        ua = va[0] + va[1]
        ub = vb[0] + vb[1]
        uc = vc[0] + vc[1]
        v = barycentric_interpolate((x, y), va, vb, vc, ua, ub, uc)
        assert v == pytest.approx(x + y)
