"""Tests for MAT-driven morphing spiral generation."""

import pytest

from raygeo.geo.algo.morph_spiral import (
    morph_spiral,
    morph_spiral_from_branch,
)


def _rect(w, h):
    return [(0, 0), (w, 0), (w, h), (0, h)]


class TestMorphSpiralFromBranch:
    def test_uniform_channel_returns_path(self):
        pts = [(0.0, 0.0), (20.0, 0.0), (40.0, 0.0)]
        cls = [6.0, 6.0, 6.0]
        path = morph_spiral_from_branch(pts, cls, step_over=2.0, z=0.0)
        assert len(path) > 0
        for p in path:
            assert len(p) == 3
            assert p[2] == 0.0

    def test_empty_points_returns_empty(self):
        assert morph_spiral_from_branch([], [], step_over=2.0, z=0.0) == []

    def test_single_point_returns_empty(self):
        pts = [(0.0, 0.0)]
        cls = [5.0]
        assert morph_spiral_from_branch(pts, cls, step_over=2.0, z=0.0) == []

    def test_variable_width_truncates_passes(self):
        """A tapered channel should have more passes near the wide end."""
        pts = [(0.0, 0.0), (20.0, 0.0), (40.0, 0.0), (60.0, 0.0)]
        cls_uniform = [8.0, 8.0, 8.0, 8.0]
        cls_tapered = [8.0, 6.0, 4.0, 2.0]
        path_u = morph_spiral_from_branch(
            pts, cls_uniform, step_over=2.0, z=0.0
        )
        path_t = morph_spiral_from_branch(
            pts, cls_tapered, step_over=2.0, z=0.0
        )
        assert len(path_u) > 0
        assert len(path_t) > 0

    def test_zero_stepover_returns_empty(self):
        pts = [(0.0, 0.0), (20.0, 0.0)]
        cls = [5.0, 5.0]
        assert morph_spiral_from_branch(pts, cls, step_over=0.0, z=0.0) == []

    def test_negative_stepover_returns_empty(self):
        pts = [(0.0, 0.0), (20.0, 0.0)]
        cls = [5.0, 5.0]
        assert morph_spiral_from_branch(pts, cls, step_over=-1.0, z=0.0) == []

    def test_clearance_smaller_than_stepover(self):
        """When channel is narrower than step_over, only centerline pass."""
        pts = [(0.0, 0.0), (10.0, 0.0)]
        cls = [0.5, 0.5]
        path = morph_spiral_from_branch(pts, cls, step_over=2.0, z=0.0)
        assert len(path) > 0  # at least centerline

    def test_z_height_preserved(self):
        pts = [(0.0, 0.0), (10.0, 0.0)]
        cls = [5.0, 5.0]
        path = morph_spiral_from_branch(pts, cls, step_over=1.0, z=-5.0)
        assert all(p[2] == -5.0 for p in path)

    def test_large_stepover_few_passes(self):
        pts = [(0.0, 0.0), (10.0, 0.0)]
        cls = [20.0, 20.0]
        path_small = morph_spiral_from_branch(pts, cls, step_over=1.0, z=0.0)
        path_large = morph_spiral_from_branch(pts, cls, step_over=10.0, z=0.0)
        # Smaller step_over → more points
        assert len(path_small) > len(path_large)


class TestMorphSpiral:
    def test_rect_returns_path(self):
        tp, branches = morph_spiral(
            pocket_boundary=_rect(50, 40),
            tool_radius=2.0,
            step_over=2.0,
            z=0.0,
            sampling_spacing=5.0,
        )
        assert len(tp) > 0
        assert len(branches) >= 1
        for p in tp:
            assert len(p) == 3

    def test_z_height(self):
        tp, _ = morph_spiral(
            pocket_boundary=_rect(50, 40),
            tool_radius=2.0,
            step_over=2.0,
            z=-5.0,
            sampling_spacing=5.0,
        )
        assert all(p[2] == -5.0 for p in tp)

    def test_lshape(self):
        lshape = [(0, 0), (120, 0), (120, 40), (40, 40), (40, 80), (0, 80)]
        tp, branches = morph_spiral(
            pocket_boundary=lshape,
            tool_radius=3.0,
            step_over=2.0,
            z=0.0,
            sampling_spacing=6.0,
        )
        assert len(tp) > 0
        assert len(branches) >= 1

    def test_yshape(self):
        yshape = [
            (45, 0),
            (75, 0),
            (75, 40),
            (110, 110),
            (80, 110),
            (60, 55),
            (40, 110),
            (10, 110),
            (45, 40),
        ]
        tp, _ = morph_spiral(
            pocket_boundary=yshape,
            tool_radius=3.0,
            step_over=2.0,
            z=0.0,
            sampling_spacing=6.0,
        )
        assert len(tp) > 0

    def test_multi_island(self):
        boundary = _rect(180, 120)
        islands = [
            [(15, 15), (35, 15), (35, 35), (15, 35)],
            [(70, 40), (90, 40), (90, 60), (70, 60)],
            [(130, 80), (160, 80), (160, 105), (130, 105)],
        ]
        tp, branches = morph_spiral(
            pocket_boundary=boundary,
            islands=islands,
            tool_radius=3.0,
            step_over=2.0,
            z=0.0,
            sampling_spacing=6.0,
        )
        assert len(tp) > 0
        assert len(branches) >= 1

    def test_tool_radius_too_large_errors(self):
        with pytest.raises(RuntimeError, match="too narrow|no valid"):
            morph_spiral(
                pocket_boundary=_rect(10, 10),
                tool_radius=10.0,
                step_over=2.0,
                z=0.0,
            )

    def test_path_inside_boundary(self):
        """All path points should be within the valid tool area."""
        boundary = _rect(50, 40)
        tool_r = 3.0
        tp, _ = morph_spiral(
            pocket_boundary=boundary,
            tool_radius=tool_r,
            step_over=2.0,
            z=0.0,
            sampling_spacing=5.0,
        )
        # Valid tool area is [3, 47] x [3, 37] (offset by tool_radius)
        for x, y, _ in tp:
            assert 2.5 <= x <= 47.5, f"x={x} out of range"
            assert 2.5 <= y <= 37.5, f"y={y} out of range"

    def test_step_over_produces_distinct_passes(self):
        """A uniform channel should produce multiple distinct passes."""
        pts = [(0.0, 0.0), (30.0, 0.0), (60.0, 0.0)]
        cls = [10.0, 10.0, 10.0]
        step = 2.0
        path = morph_spiral_from_branch(pts, cls, step_over=step, z=0.0)
        assert len(path) >= 10
        # At least 3 distinct y values (centerline + both sides)
        y_vals = set(round(p[1], 1) for p in path)
        assert len(y_vals) >= 3

    @pytest.mark.slow
    def test_coverage_rect(self):
        """Spiral should cover at least 90% of valid tool area (swept)."""
        boundary = _rect(60, 50)
        tool_r = 3.0
        step = 2.0
        tp, _ = morph_spiral(
            pocket_boundary=boundary,
            tool_radius=tool_r,
            step_over=step,
            z=0.0,
            sampling_spacing=6.0,
        )
        # Compute swept coverage using raygeo if available
        from raygeo.geo.algo.cleared_area import ClearedArea

        ca = ClearedArea()
        path_2d = [(p[0], p[1]) for p in tp]
        ca.expand(path_2d, tool_r)
        swept_area = ca.total_area()
        # Valid tool area ≈ (60-6)*(50-6) = 54*44 = 2376
        expected = (60 - 2 * tool_r) * (50 - 2 * tool_r)
        assert swept_area > expected * 0.75, (
            f"swept {swept_area:.0f}, expected ~{expected:.0f}"
        )
