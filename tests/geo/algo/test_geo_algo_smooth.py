import math

import pytest

from raygeo.geo.algo.smooth import (
    compute_gaussian_kernel,
    shortcut_path,
    smooth_circularly,
    smooth_path,
    smooth_polyline,
    smooth_sub_segment,
)
from raygeo.geo.types import Point3D


class TestComputeGaussianKernel:
    """Tests for compute_gaussian_kernel function."""

    def test_zero_amount_returns_unit_kernel(self):
        """Zero amount should return a kernel with a single 1.0 value."""
        kernel, sigma = compute_gaussian_kernel(0)
        assert kernel == [1.0]
        assert sigma == 0.0

    def test_low_amount_produces_small_sigma(self):
        """Low smoothing amount should produce small sigma."""
        kernel, sigma = compute_gaussian_kernel(10)
        assert sigma > 0
        assert sigma < 1.0
        assert len(kernel) > 1

    def test_high_amount_produces_larger_kernel(self):
        """Higher smoothing should produce larger kernel."""
        kernel_low, _ = compute_gaussian_kernel(20)
        kernel_high, _ = compute_gaussian_kernel(80)
        assert len(kernel_high) > len(kernel_low)

    def test_kernel_is_normalized(self):
        """Kernel values should sum to approximately 1.0."""
        for amount in [10, 50, 100]:
            kernel, _ = compute_gaussian_kernel(amount)
            assert abs(sum(kernel) - 1.0) < 1e-10

    def test_kernel_is_symmetric(self):
        """Gaussian kernel should be symmetric around center."""
        kernel, _ = compute_gaussian_kernel(50)
        n = len(kernel)
        for i in range(n // 2):
            assert abs(kernel[i] - kernel[n - 1 - i]) < 1e-10


class TestSmoothSubSegment:
    """Tests for smooth_sub_segment function."""

    def test_preserves_endpoints(self):
        """Endpoints should be preserved exactly."""
        points: list[Point3D] = [(0, 0, 0), (5, 5, 0), (10, 0, 0)]
        kernel, _ = compute_gaussian_kernel(50)
        smoothed = smooth_sub_segment(points, kernel)
        assert smoothed[0] == points[0]
        assert smoothed[-1] == points[-1]

    def test_returns_same_for_short_input(self):
        """Should return input unchanged for less than 3 points."""
        points: list[Point3D] = [(0, 0, 0), (10, 0, 0)]
        kernel, _ = compute_gaussian_kernel(50)
        smoothed = smooth_sub_segment(points, kernel)
        assert smoothed == points

    def test_preserves_z_coordinate(self):
        """Z coordinate should be preserved from original point."""
        points: list[Point3D] = [(0, 0, 5), (5, 5, 5), (10, 0, 5)]
        kernel, _ = compute_gaussian_kernel(50)
        smoothed = smooth_sub_segment(points, kernel)
        for i, point in enumerate(smoothed):
            assert abs(point[2] - 5.0) < 1e-10


class TestSmoothCircularly:
    """Tests for smooth_circularly function."""

    def test_closes_path(self):
        """Result should have first point appended at end."""
        points: list[Point3D] = [(0, 0, 0), (10, 0, 0), (5, 10, 0)]
        kernel, _ = compute_gaussian_kernel(50)
        smoothed = smooth_circularly(points, kernel)
        assert smoothed[0] == smoothed[-1]

    def test_returns_same_for_short_input(self):
        """Should return input unchanged for less than 3 points."""
        points: list[Point3D] = [(0, 0, 0), (10, 0, 0)]
        kernel, _ = compute_gaussian_kernel(50)
        smoothed = smooth_circularly(points, kernel)
        assert smoothed == points

    def test_preserves_z_coordinate(self):
        """Z coordinate should be preserved from original point."""
        points: list[Point3D] = [(0, 0, 3), (10, 0, 3), (5, 10, 3)]
        kernel, _ = compute_gaussian_kernel(50)
        smoothed = smooth_circularly(points, kernel)
        for point in smoothed:
            assert abs(point[2] - 3.0) < 1e-10


class TestSmoothPolyline:
    """Tests for smooth_polyline function."""

    def test_zero_amount_returns_input(self):
        """Zero amount should return input unchanged."""
        points: list[Point3D] = [(0, 0, 0), (10, 0, 0), (20, 10, 0)]
        result = smooth_polyline(points, 0, 45)
        assert result == points

    def test_short_input_returns_unchanged(self):
        """Less than 3 points should return input unchanged."""
        points: list[Point3D] = [(0, 0, 0), (10, 0, 0)]
        result = smooth_polyline(points, 50, 45)
        assert result == points

    def test_preserves_open_path_endpoints(self):
        """Open paths should have exact endpoints preserved."""
        points: list[Point3D] = [
            (0, 0, 0),
            (10, 0, 0),
            (20, 10, 0),
            (30, 0, 0),
            (40, 0, 0),
        ]
        result = smooth_polyline(points, 50, 45, is_closed=False)
        assert result[0] == points[0]
        assert result[-1] == points[-1]

    def test_closed_path_detection(self):
        """Should auto-detect closed path when start == end."""
        points: list[Point3D] = [
            (0, 0, 0),
            (10, 0, 0),
            (10, 10, 0),
            (0, 10, 0),
            (0, 0, 0),
        ]
        result = smooth_polyline(points, 30, 120, is_closed=None)
        assert result[0] == result[-1]

    def test_sharp_corner_preserved(self):
        """Sharp corners below threshold should be preserved."""
        points: list[Point3D] = [
            (0, 50, 0),
            (50, 0, 0),
            (100, 50, 0),
        ]
        result = smooth_polyline(points, 30, 95, is_closed=False)
        corner_point = (50, 0, 0)
        closest = min(result, key=lambda p: math.dist(p, corner_point))
        dist = math.dist(closest, corner_point)
        assert dist < 1.0, "Sharp corner should be preserved"

    def test_dull_corner_smoothed(self):
        """Dull corners above threshold should be smoothed."""
        points: list[Point3D] = [
            (0, 50, 0),
            (50, 0, 0),
            (100, 50, 0),
            (150, 50, 0),
        ]
        result = smooth_polyline(points, 40, 95, is_closed=False)
        dull_corner = (100, 50, 0)
        closest = min(result, key=lambda p: math.dist(p, dull_corner))
        dist = math.dist(closest, dull_corner)
        assert dist > 0.1, "Dull corner should be smoothed"

    def test_closed_loop_smoothing(self):
        """Closed loops should be smoothed circularly."""
        points: list[Point3D] = [
            (0, 0, 0),
            (10, 0, 0),
            (10, 10, 0),
            (0, 10, 0),
        ]
        result = smooth_polyline(points, 50, 45, is_closed=True)
        assert len(result) >= 3
        assert result[0] == result[-1]

    def test_z_preserved_through_smoothing(self):
        """Z coordinates should be preserved during smoothing."""
        points: list[Point3D] = [
            (0, 0, 7),
            (10, 0, 7),
            (20, 10, 7),
            (30, 0, 7),
            (40, 0, 7),
        ]
        result = smooth_polyline(points, 50, 45, is_closed=False)
        for point in result:
            assert abs(point[2] - 7.0) < 1e-10


class TestSmoothPath:
    """Tests for smooth_path (constrained shortcut + relaxation)."""

    def test_short_path_unchanged(self):
        """Paths with <= 2 points should be returned as-is."""
        result = smooth_path([(0, 0), (10, 10)], [], 1.0)
        assert result == [(0, 0), (10, 10)]

    def test_single_point(self):
        """Single point should be returned as-is."""
        result = smooth_path([(5, 5)], [], 1.0)
        assert result == [(5, 5)]

    def test_endpoints_preserved(self):
        """First and last points must always be preserved."""
        obstacle = [(45, 45), (55, 45), (55, 55), (45, 55)]
        path = [(0, 50), (30, 50), (50, 80), (70, 50), (100, 50)]
        result = smooth_path(path, [obstacle], 3.0, 50)
        assert result[0] == pytest.approx(path[0])
        assert result[-1] == pytest.approx(path[-1])

    def test_no_obstacles_collapses_to_direct(self):
        """With no obstacles, shortcut should collapse to a direct line."""
        path = [(0, 0), (25, 10), (50, 20), (75, 10), (100, 0)]
        result = smooth_path(path, [], 1.0, 50)
        assert len(result) == 2
        assert result[0] == pytest.approx(path[0])
        assert result[-1] == pytest.approx(path[-1])

    def test_obstacle_avoidance(self):
        """Smoothed path must maintain clearance from obstacles."""
        obstacle: list[tuple[float, float]] = [
            (45.0, 45.0),
            (55.0, 45.0),
            (55.0, 55.0),
            (45.0, 55.0),
        ]
        path = [(0, 50), (30, 50), (50, 80), (70, 50), (100, 50)]
        clearance = 3.0
        result = smooth_path(path, [obstacle], clearance, 50)
        from raygeo.geo.shape.polygon import (
            does_path_sweep_intersect_polygon,
        )

        assert not does_path_sweep_intersect_polygon(
            result, clearance, [obstacle]
        )

    def test_clearance_zero_shortcut_only(self):
        """With clearance=0 and smoothing=0, shortcut should still work."""
        obstacle = [(45, 45), (55, 45), (55, 55), (45, 55)]
        path = [(0, 50), (30, 50), (50, 80), (70, 50), (100, 50)]
        result = smooth_path(path, [obstacle], 0.0, 0)
        assert len(result) >= 2
        assert result[0] == pytest.approx(path[0])
        assert result[-1] == pytest.approx(path[-1])

    def test_smoothing_amount_zero_shortcut_only(self):
        """smoothing_amount=0 should apply shortcut phase only."""
        path = [(0, 0), (25, 10), (50, 0), (75, 10), (100, 0)]
        result = smooth_path(path, [], 1.0, 0)
        assert len(result) == 2

    def test_shortcut_reduces_redundant_waypoints(self):
        """Collinear intermediate points should be removed by shortcut."""
        path = [(0, 0), (10, 0), (20, 0), (30, 0), (40, 0)]
        result = smooth_path(path, [], 0.0, 50)
        assert len(result) <= 2

    def test_result_is_shorter_or_equal(self):
        """Smoothed path length should not exceed original."""
        obstacle = [(45, 30), (55, 30), (55, 60), (45, 60)]
        path = [(0, 50), (30, 50), (50, 80), (70, 50), (100, 50)]
        result = smooth_path(path, [obstacle], 3.0, 50)

        orig_len = sum(
            math.dist(path[i], path[i + 1]) for i in range(len(path) - 1)
        )
        result_len = sum(
            math.dist(result[i], result[i + 1]) for i in range(len(result) - 1)
        )
        assert result_len <= orig_len + 1e-6

    def test_multiple_obstacles(self):
        """Multiple obstacles should all be avoided."""
        obs1: list[tuple[float, float]] = [
            (30.0, 40.0),
            (35.0, 40.0),
            (35.0, 60.0),
            (30.0, 60.0),
        ]
        obs2: list[tuple[float, float]] = [
            (65.0, 40.0),
            (70.0, 40.0),
            (70.0, 60.0),
            (65.0, 60.0),
        ]
        path = [(0, 50), (20, 50), (50, 80), (80, 50), (100, 50)]
        clearance = 2.0
        result = smooth_path(path, [obs1, obs2], clearance, 50)
        from raygeo.geo.shape.polygon import (
            does_path_sweep_intersect_polygon,
        )

        assert not does_path_sweep_intersect_polygon(
            result, clearance, [obs1, obs2]
        )


class TestShortcutPath:
    """Tests for shortcut_path (iterative waypoint removal)."""

    def test_shortcut_straight_line(self):
        """Three collinear points → middle point removed."""
        result = shortcut_path([(0, 0), (50, 0), (100, 0)], [], 1.0)
        assert result == [(0.0, 0.0), (100.0, 0.0)]

    def test_shortcut_around_obstacle(self):
        """Waypoint that avoids an obstacle is preserved."""
        obstacle = [(30, -10), (30, 10), (70, 10), (70, -10)]
        path = [(0, 0), (50, 0), (100, 0)]
        result = shortcut_path(path, [obstacle], 1.0)
        # (0,0) → (100,0) crosses obstacle, so (50,0) is preserved
        assert result == [(0.0, 0.0), (50.0, 0.0), (100.0, 0.0)]

    def test_shortcut_no_obstacles_long_path(self):
        """All intermediate points removable with no obstacles."""
        path = [(0, 0), (10, 0), (20, 0), (30, 0), (40, 0), (50, 0)]
        result = shortcut_path(path, [], 1.0)
        assert result == [(0.0, 0.0), (50.0, 0.0)]

    def test_shortcut_zigzag_clear(self):
        """Zigzag with no obstacles → collapsed to endpoints."""
        path = [(0, 0), (25, 10), (50, 0), (75, 10), (100, 0)]
        result = shortcut_path(path, [], 1.0)
        assert result == [(0.0, 0.0), (100.0, 0.0)]

    def test_shortcut_two_points(self):
        """Two points → unchanged."""
        result = shortcut_path([(0, 0), (100, 0)], [], 1.0)
        assert result == [(0.0, 0.0), (100.0, 0.0)]

    def test_shortcut_one_point(self):
        """Single point → unchanged."""
        result = shortcut_path([(42, 42)], [], 1.0)
        assert result == [(42.0, 42.0)]

    def test_preserves_clearance_from_multiple_obstacles(self):
        """Multiple obstacles prevent removal of critical waypoints."""
        obs1 = [(10, -10), (10, 10), (30, 10), (30, -10)]
        obs2 = [(70, -10), (70, 10), (90, 10), (90, -10)]
        path = [(0, 0), (20, 0), (50, 0), (80, 0), (100, 0)]
        result = shortcut_path(path, [obs1, obs2], 1.0)
        # All direct connections skip over at least one obstacle,
        # so no interior point can be removed.
        assert len(result) == 5
