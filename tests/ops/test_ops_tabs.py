"""Tests for Ops tab operations (apply_tab_gaps / apply_tab_power)."""

import math

from raygeo.ops import Ops
from raygeo.ops.types import CommandCategory, CommandType, SectionType

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_rect_ops(x, y, w, h, power=1.0, speed=1000):
    """Build a simple rectangular contour wrapped in a VECTOR_OUTLINE
    section. The rectangle starts at (x, y) and goes clockwise."""
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.set_power(power)
    ops.set_cut_speed(speed)
    ops.move_to(x, y, 0)
    ops.line_to(x + w, y, 0)
    ops.line_to(x + w, y + h, 0)
    ops.line_to(x, y + h, 0)
    ops.close_path()
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    return ops


def make_circle_ops(cx, cy, r, n=64, power=1.0, speed=1000):
    """Build an approximated circle as a VECTOR_OUTLINE section."""
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.set_power(power)
    ops.set_cut_speed(speed)
    ops.move_to(cx + r, cy, 0)
    for i in range(1, n + 1):
        a = 2 * math.pi * i / n
        ops.line_to(cx + r * math.cos(a), cy + r * math.sin(a), 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    return ops


def make_bezier_rect_ops(x, y, w, h, power=1.0, speed=1000):
    """Build a rectangle with Bezier curves (rounded-ish corners)
    to exercise the Bezier-aware tab code path."""
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.set_power(power)
    ops.set_cut_speed(speed)
    d = min(w, h) * 0.1
    ops.move_to(x + d, y, 0)
    # top-right corner as bezier
    ops.bezier_to(
        (x + w - d, y, 0),
        (x + w, y + d, 0),
        (x + w, y + d, 0),
    )
    ops.line_to(x + w, y + h - d, 0)
    # bottom-right corner as bezier
    ops.bezier_to(
        (x + w, y + h - d, 0),
        (x + w - d, y + h, 0),
        (x + w - d, y + h, 0),
    )
    ops.line_to(x + d, y + h, 0)
    # bottom-left corner as bezier
    ops.bezier_to(
        (x + d, y + h, 0),
        (x, y + h - d, 0),
        (x, y + h - d, 0),
    )
    ops.line_to(x, y + d, 0)
    # top-left corner as bezier
    ops.bezier_to(
        (x, y + d, 0),
        (x + d, y, 0),
        (x + d, y, 0),
    )
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    return ops


def count_moving_segments(ops):
    """Count the number of contiguous moving subpaths in ops."""
    count = 0
    in_subpath = False
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            count += 1
            in_subpath = True
        elif ct in (CommandType.LINE_TO, CommandType.BEZIER_TO, CommandType.ARC_TO):
            if not in_subpath:
                count += 1
                in_subpath = True
        else:
            in_subpath = False
    return count


def get_power_values(ops):
    """Extract all SET_POWER values from ops in order."""
    powers = []
    for i in range(ops.len()):
        if ops.command_type(i) == CommandType.SET_POWER:
            info = ops.inspect(i)
            powers.append(info.power)
    return powers


def get_moving_endpoints(ops):
    """Extract the endpoints of all moving commands."""
    pts = []
    for i in range(ops.len()):
        if ops.category(i) == CommandCategory.MOVING:
            pts.append(ops.endpoint(i))
    return pts


# ===================================================================
# Gap-mode tests
# ===================================================================


class TestApplyTabGapsBasic:
    """Basic gap-mode tab tests on line-only paths."""

    def test_empty_ops(self):
        ops = Ops()
        ops.apply_tab_gaps([(5, 5, 2)])
        assert ops.is_empty()

    def test_empty_clips(self):
        ops = make_rect_ops(0, 0, 10, 10)
        orig_len = ops.len()
        ops.apply_tab_gaps([])
        assert ops.len() == orig_len

    def test_single_tab_on_square(self):
        """A single tab in the middle of the top edge should split it."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(5, 0, 2)])
        # The original had 1 subpath (the rectangle). After one gap,
        # we expect at least 2 subpaths (split at the tab).
        segments = list(ops.segment_indices())
        assert len(segments) >= 2

    def test_two_tabs_on_square(self):
        """Two tabs on opposite edges should produce more splits."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps(
            [
                (5, 0, 1),
                (5, 10, 1),
            ]
        )
        segments = list(ops.segment_indices())
        assert len(segments) >= 3

    def test_tab_outside_path_is_noop(self):
        """A clip point far from the path should produce no change."""
        ops = make_rect_ops(0, 0, 10, 10)
        orig_len = ops.len()
        ops.apply_tab_gaps([(100, 100, 2)])
        assert ops.len() == orig_len

    def test_tab_very_small_width(self):
        """A very narrow tab should have minimal effect."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(5, 0, 0.001)])
        # With such a tiny width, the path should be nearly unchanged.
        # There might be a very small gap, but the structure is preserved.
        segments = list(ops.segment_indices())
        assert len(segments) >= 1

    def test_non_vector_section_untouched(self):
        """Tabs should not affect RASTER_FILL sections."""
        ops = Ops()
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.line_to(10, 10, 0)
        ops.line_to(0, 10, 0)
        ops.line_to(0, 0, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
        orig_len = ops.len()
        ops.apply_tab_gaps([(5, 0, 2)])
        assert ops.len() == orig_len

    def test_mixed_sections(self):
        """Tabs should only affect VECTOR_OUTLINE sections,
        leaving RASTER_FILL untouched."""
        ops = Ops()
        # Raster section
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
        # Vector section
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp2")
        ops.set_power(1.0)
        ops.move_to(0, 5, 0)
        ops.line_to(10, 5, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        ops.apply_tab_gaps([(5, 5, 2)])
        # The raster section should still have its original line
        raster_found = False
        for i in range(ops.len()):
            if ops.command_type(i) == CommandType.LINE_TO:
                ep = ops.endpoint(i)
                if abs(ep[1] - 0) < 0.1:
                    raster_found = True
        assert raster_found


class TestApplyTabGapsBezier:
    """Gap-mode tests on paths with Bezier curves."""

    def test_single_tab_on_bezier_path(self):
        """A tab on a path with bezier curves should split correctly."""
        ops = make_bezier_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(5, 0, 2)])
        segments = list(ops.segment_indices())
        assert len(segments) >= 2

    def test_bezier_path_preserves_structure(self):
        """After gap, the resulting ops should still contain valid
        moving commands."""
        ops = make_bezier_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(10, 5, 2)])
        moving_count = sum(
            1 for i in range(ops.len()) if ops.category(i) == CommandCategory.MOVING
        )
        assert moving_count >= 2

    def test_multiple_tabs_bezier(self):
        """Multiple tabs on a bezier path should work correctly."""
        ops = make_bezier_rect_ops(0, 0, 20, 20)
        ops.apply_tab_gaps(
            [
                (10, 0, 2),
                (20, 10, 2),
                (10, 20, 2),
            ]
        )
        segments = list(ops.segment_indices())
        assert len(segments) >= 3


class TestApplyTabGapsCircle:
    """Gap-mode tests on approximated circles (many line segments)."""

    def test_single_tab_on_circle(self):
        ops = make_circle_ops(5, 5, 5)
        ops.apply_tab_gaps([(10, 5, 2)])
        segments = list(ops.segment_indices())
        assert len(segments) >= 2

    def test_two_tabs_on_circle(self):
        ops = make_circle_ops(5, 5, 5)
        ops.apply_tab_gaps(
            [
                (10, 5, 1),
                (0, 5, 1),
            ]
        )
        segments = list(ops.segment_indices())
        assert len(segments) >= 3


# ===================================================================
# Power-mode tests
# ===================================================================


class TestApplyTabPowerBasic:
    """Basic power-mode tab tests on line-only paths."""

    def test_empty_ops(self):
        ops = Ops()
        ops.apply_tab_power([(5, 5, 2)], 0.1, 1.0)
        assert ops.is_empty()

    def test_empty_clips(self):
        ops = make_rect_ops(0, 0, 10, 10)
        orig_len = ops.len()
        ops.apply_tab_power([], 0.1, 1.0)
        assert ops.len() == orig_len

    def test_single_tab_adds_power_commands(self):
        """A single tab should insert SET_POWER commands."""
        ops = make_rect_ops(0, 0, 10, 10)
        orig_power_count = sum(
            1 for i in range(ops.len()) if ops.command_type(i) == CommandType.SET_POWER
        )
        ops.apply_tab_power([(5, 0, 2)], 0.1, 1.0)
        new_power_count = sum(
            1 for i in range(ops.len()) if ops.command_type(i) == CommandType.SET_POWER
        )
        # Should have added at least 1 new power command
        # (enter tab or exit tab — one of them may reuse the existing
        # power state)
        assert new_power_count >= orig_power_count + 1

    def test_power_values_correct(self):
        """The power values in tab regions should match tab_power."""
        ops = make_rect_ops(0, 0, 10, 10)
        tab_power = 0.2
        original_power = 1.0
        ops.apply_tab_power([(5, 0, 2)], tab_power, original_power)
        powers = get_power_values(ops)
        assert tab_power in powers
        assert original_power in powers

    def test_tab_outside_path_is_noop(self):
        ops = make_rect_ops(0, 0, 10, 10)
        orig_len = ops.len()
        ops.apply_tab_power([(100, 100, 2)], 0.1, 1.0)
        assert ops.len() == orig_len

    def test_same_path_length_after_power_tab(self):
        """Power tabs should not change the path length (only insert
        power commands, no gaps)."""
        ops = make_rect_ops(0, 0, 10, 10)
        orig_distance = ops.distance()
        ops.apply_tab_power([(5, 0, 2)], 0.1, 1.0)
        new_distance = ops.distance()
        assert abs(orig_distance - new_distance) < 0.5


class TestApplyTabPowerBezier:
    """Power-mode tests on paths with Bezier curves."""

    def test_single_tab_bezier(self):
        """Power tab on bezier path should insert SET_POWER commands."""
        ops = make_bezier_rect_ops(0, 0, 10, 10)
        orig_power_count = sum(
            1 for i in range(ops.len()) if ops.command_type(i) == CommandType.SET_POWER
        )
        ops.apply_tab_power([(5, 0, 2)], 0.1, 1.0)
        new_power_count = sum(
            1 for i in range(ops.len()) if ops.command_type(i) == CommandType.SET_POWER
        )
        assert new_power_count > orig_power_count

    def test_bezier_power_tab_preserves_geometry(self):
        """Power tabs on bezier paths should preserve path endpoints."""
        ops = make_bezier_rect_ops(0, 0, 10, 10)
        orig_distance = ops.distance()
        ops.apply_tab_power([(10, 5, 2)], 0.1, 1.0)
        new_distance = ops.distance()
        # Path length should be nearly identical
        assert abs(orig_distance - new_distance) / orig_distance < 0.05

    def test_multiple_tabs_bezier_power(self):
        """Multiple power tabs on bezier path."""
        ops = make_bezier_rect_ops(0, 0, 20, 20)
        ops.apply_tab_power(
            [(10, 0, 2), (20, 10, 2)],
            0.15,
            1.0,
        )
        powers = get_power_values(ops)
        assert 0.15 in powers


# ===================================================================
# Edge case tests
# ===================================================================


class TestEdgeCases:
    """Edge cases and boundary conditions."""

    def test_tab_at_start_of_path(self):
        """Tab at the very beginning of the path."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(0, 0, 2)])
        # Should still produce a valid result
        assert ops.len() > 0

    def test_tab_at_corner(self):
        """Tab at a corner of the rectangle."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(10, 10, 2)])
        assert ops.len() > 0

    def test_very_wide_tab(self):
        """A tab wider than the path segment should work."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(5, 0, 20)])
        # Path should be heavily modified but valid
        assert ops.len() > 0

    def test_zero_tab_power(self):
        """Tab power of 0 should be valid (full gap-like effect)."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_power([(5, 0, 2)], 0.0, 1.0)
        powers = get_power_values(ops)
        assert 0.0 in powers

    def test_multiple_sections_multiple_tabs(self):
        """Multiple vector sections with tabs."""
        ops = Ops()
        # First section
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.line_to(10, 10, 0)
        ops.line_to(0, 10, 0)
        ops.close_path()
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        # Second section
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp2")
        ops.set_power(1.0)
        ops.move_to(20, 0, 0)
        ops.line_to(30, 0, 0)
        ops.line_to(30, 10, 0)
        ops.line_to(20, 10, 0)
        ops.close_path()
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
        # Tab on first section
        ops.apply_tab_gaps([(5, 0, 2)])
        segments = list(ops.segment_indices())
        assert len(segments) >= 3

    def test_no_sections_still_works(self):
        """Ops without section markers should not crash."""
        ops = Ops()
        ops.set_power(1.0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.line_to(10, 10, 0)
        ops.line_to(0, 10, 0)
        ops.close_path()
        orig_len = ops.len()
        # Without VECTOR_OUTLINE sections, nothing should change
        ops.apply_tab_gaps([(5, 0, 2)])
        assert ops.len() == orig_len

    def test_repeated_apply(self):
        """Applying tabs multiple times should not crash."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(5, 0, 1)])
        ops.apply_tab_gaps([(5, 10, 1)])
        assert ops.len() > 0


# ===================================================================
# Structural / invariant tests
# ===================================================================


class TestStructuralInvariants:
    """Verify structural properties of the output."""

    def test_gap_preserves_section_markers(self):
        """Section markers should be preserved after gap tabs."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_gaps([(5, 0, 2)])
        section_starts = sum(
            1
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.OPS_SECTION_START
        )
        section_ends = sum(
            1
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.OPS_SECTION_END
        )
        assert section_starts == 1
        assert section_ends == 1

    def test_power_preserves_section_markers(self):
        """Section markers should be preserved after power tabs."""
        ops = make_rect_ops(0, 0, 10, 10)
        ops.apply_tab_power([(5, 0, 2)], 0.1, 1.0)
        section_starts = sum(
            1
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.OPS_SECTION_START
        )
        section_ends = sum(
            1
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.OPS_SECTION_END
        )
        assert section_starts == 1
        assert section_ends == 1

    def test_gap_endpoint_preserved(self):
        """After gap tabs, the original endpoint should be preserved
        (the path should navigate back to it)."""
        ops = make_rect_ops(0, 0, 10, 10)
        orig_endpoint = ops.endpoint(ops.len() - 1)
        ops.apply_tab_gaps([(5, 0, 1)])
        # Find the last moving endpoint
        last_ep = None
        for i in range(ops.len() - 1, -1, -1):
            if ops.category(i) == CommandCategory.MOVING:
                last_ep = ops.endpoint(i)
                break
        assert last_ep is not None
        # The path should end at or near the original endpoint
        assert abs(last_ep[0] - orig_endpoint[0]) < 1e-3
        assert abs(last_ep[1] - orig_endpoint[1]) < 1e-3

    def test_gap_reduces_cut_distance(self):
        """Gap tabs should reduce the total cut distance."""
        ops = make_rect_ops(0, 0, 10, 10)
        orig_cut = ops.cut_distance()
        ops.apply_tab_gaps([(5, 0, 3)])
        new_cut = ops.cut_distance()
        assert new_cut < orig_cut
