"""Tests for cut search module."""

import math

import pytest

from raygeo.ops.assembly.adaptive import target_area_per_distance
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.search import (
    ToolPose,
    search_frontier_engagement,
    search_reengagement,
)



def _circle(cx, cy, r, n=32):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


class TestSearchFrontierEngagement:
    def test_returns_tool_pose(self):
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            0.1,
            float("inf"),
        )
        assert isinstance(result, ToolPose)

    def test_none_for_empty(self):
        ca = ClearedArea(boundary=[])
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            0.1,
            float("inf"),
        )
        assert result is None

    def test_none_when_min_too_high(self):
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            1e6,
            float("inf"),
        )
        assert result is None

    def test_skips_closest_vertex(self):
        """Result differs from the closest frontier vertex (not start=end)."""
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            0.1,
            float("inf"),
        )
        assert result is not None
        # Result should be at least one vertex away.
        d = math.hypot(result.pos[0] - 50.0, result.pos[1] - 55.0)
        assert d > 0.1

    def test_position_offset_inward_by_radius_minus_advance(self):
        """The returned position must be offset inward from the
        frontier by ``radius - advance``, not sitting on the
        boundary.
        """
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        R = 3.0
        advance = 1.5
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            R,
            0.6,
            advance,
            0.1,
            float("inf"),
        )
        assert result is not None
        # The frontier is the circle of radius 15 centred at (50,40).
        # The tool must be inside this circle (inward offset) by
        # R - advance = 1.5mm.
        dist = math.hypot(result.pos[0] - 50.0, result.pos[1] - 40.0)
        expected = 15.0 - (R - advance)
        assert abs(dist - expected) < 1.0, (
            f"Tool centre at dist {dist:.3f} from circle centre, "
            f"expected ~{expected:.3f} (frontier_radius - inward_offset)"
        )


class TestSearchReengagement:
    def test_returns_tool_pose(self):
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        result = search_reengagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            0.1,
        )
        assert isinstance(result, ToolPose)

    def test_none_for_empty(self):
        ca = ClearedArea(boundary=[])
        result = search_reengagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            0.1,
        )
        assert result is None

    def test_forward_and_backward_return_valid(self):
        """Both forward and backward return valid results."""
        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])
        fwd = search_frontier_engagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            0.1,
            float("inf"),
        )
        bwd = search_reengagement(
            ca,
            ToolPose(pos=(50.0, 55.0), heading=0.0),
            3.0,
            0.6,
            1.5,
            0.1,
        )
        assert fwd is not None and bwd is not None


class TestToolPose:
    def test_repr(self):
        rp = ToolPose(pos=(10.0, 20.0), heading=1.5)
        s = repr(rp)
        assert "ToolPose" in s
        assert "10" in s


class TestSearchFrontierEngagementMaxBound:
    """The search must never return a point whose probe area exceeds
    ``max_cut_area``, even at sharp corners where geometrically the
    disk would cover far more material than the target engagement.
    """

    def _probe_area(self, ca, pos, heading, step_length, R):
        """Compute cut_area for one step from pos along heading."""
        probe = (
            pos[0] + math.cos(heading) * step_length,
            pos[1] + math.sin(heading) * step_length,
        )
        return ca.cut_area(pos, probe, R)

    def test_square_corner_does_not_exceed_max(self):
        """A 90° corner has ~5× the target engagement.  The search
        must skip it and find a straight-edge vertex instead.
        """
        R = 5.0
        advance = 2.0
        step_length = 1.0
        target_apd = target_area_per_distance(R, advance, step_length)
        max_cut_area = target_apd * step_length * 1.5

        # Subdivide the square edges so the search has non-corner
        # vertices to find.  Each edge gets 10 segments.
        s = 50.0
        n = 10
        poly = []
        for i in range(n):
            t = i / n
            poly.append((-s + 2 * s * t, -s))  # bottom edge
        for i in range(n):
            t = i / n
            poly.append((s, -s + 2 * s * t))  # right edge
        for i in range(n):
            t = i / n
            poly.append((s - 2 * s * t, s))  # top edge
        for i in range(n):
            t = i / n
            poly.append((-s, s - 2 * s * t))  # left edge

        ca = ClearedArea(boundary=[])
        ca.cut([poly])

        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(45.0, -49.0), heading=0.0),
            R,
            step_length,
            advance,
            0.01,
            max_cut_area,
        )
        assert result is not None, (
            "Search returned None — all vertices exceeded max_cut_area"
        )
        area = self._probe_area(ca, result.pos, result.heading, step_length, R)
        assert area <= max_cut_area, (
            f"Probe area {area:.4f} exceeds max_cut_area "
            f"{max_cut_area:.4f} at pos={result.pos}"
        )

    def test_sharp_corner_does_not_exceed_max(self):
        """An acute-angle corner (30°) has even more material.  The
        search must still respect the upper bound.
        """
        R = 5.0
        advance = 2.0
        step_length = 1.0
        target_apd = target_area_per_distance(R, advance, step_length)
        max_cut_area = target_apd * step_length * 1.5

        # Build a cleared polygon with a sharp 30° wedge pointing right.
        # The tip of the wedge is at (60, 0).  The two edges go back
        # to (0, ±tan(15°)*60) ≈ (0, ±16.1).
        half_angle = math.radians(15)
        tip = (60.0, 0.0)
        top = (0.0, 60.0 * math.tan(half_angle))
        bot = (0.0, -60.0 * math.tan(half_angle))
        # Wound CCW: bot → tip → top → back
        wedge = [bot, tip, top, (0.0, 0.0)]
        ca = ClearedArea(boundary=[])
        ca.cut([wedge])

        # Start near the tip, heading up (along the top edge).
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(55.0, 1.0), heading=math.pi / 2),
            R,
            step_length,
            advance,
            0.01,
            max_cut_area,
        )
        if result is not None:
            area = self._probe_area(
                ca, result.pos, result.heading, step_length, R
            )
            assert area <= max_cut_area, (
                f"Probe area {area:.4f} exceeds max_cut_area "
                f"{max_cut_area:.4f} at pos={result.pos} "
                f"heading={result.heading:.4f}"
            )

    def test_circle_never_exceeds_max(self):
        """A smooth circle has uniform engagement ~target.  All
        returned points should be well within the bound.
        """
        R = 5.0
        advance = 2.0
        step_length = 1.0
        target_apd = target_area_per_distance(R, advance, step_length)
        max_cut_area = target_apd * step_length * 1.5

        ca = ClearedArea(boundary=[])
        ca.cut([_circle(50, 40, 15)])

        # Try several start positions around the circle.
        for angle_deg in [0, 45, 90, 135, 180, 270]:
            angle = math.radians(angle_deg)
            start = (50 + 15 * math.cos(angle), 40 + 15 * math.sin(angle))
            heading = angle + math.pi / 2  # tangent
            result = search_frontier_engagement(
                ca,
                ToolPose(pos=start, heading=heading),
                R,
                step_length,
                advance,
                0.01,
                max_cut_area,
            )
            if result is None:
                continue
            area = self._probe_area(
                ca, result.pos, result.heading, step_length, R
            )
            assert area <= max_cut_area, (
                f"Probe area {area:.4f} exceeds max_cut_area "
                f"{max_cut_area:.4f} at angle={angle_deg}° "
                f"pos={result.pos}"
            )

    @pytest.mark.parametrize("corner_angle_deg", [30, 60, 90, 120, 150])
    def test_various_corner_angles_respect_max(self, corner_angle_deg):
        """Parametric test: corners from 30° to 150° must all
        respect the max_cut_area bound.
        """
        R = 5.0
        advance = 2.0
        step_length = 1.0
        target_apd = target_area_per_distance(R, advance, step_length)
        max_cut_area = target_apd * step_length * 1.5

        half = math.radians(corner_angle_deg / 2)
        # Build a polygon with a corner at (50, 0) pointing right.
        # Subdivide edges so non-corner vertices exist.
        tip = (50.0, 0.0)
        top_y = 50.0 * math.tan(half)
        bot_y = -top_y
        left_x = -50.0

        n = 10
        poly = []
        # bot → tip (bottom diagonal edge)
        for i in range(n):
            t = i / n
            poly.append((left_x + (tip[0] - left_x) * t, bot_y * (1 - t)))
        # tip → top (top diagonal edge)
        for i in range(n):
            t = i / n
            poly.append((tip[0] + (left_x - tip[0]) * t, tip[1] + top_y * t))
        # top → bot (left edge)
        for i in range(n):
            t = i / n
            poly.append((left_x, top_y + (bot_y - top_y) * t))

        ca = ClearedArea(boundary=[])
        ca.cut([poly])

        # Start near the tip, heading up.
        result = search_frontier_engagement(
            ca,
            ToolPose(pos=(45.0, 1.0), heading=math.pi / 2),
            R,
            step_length,
            advance,
            0.01,
            max_cut_area,
        )
        if result is None:
            # Try heading down (opposite direction along the frontier).
            result = search_frontier_engagement(
                ca,
                ToolPose(pos=(45.0, -1.0), heading=-math.pi / 2),
                R,
                step_length,
                advance,
                0.01,
                max_cut_area,
            )
        if result is None:
            pytest.skip(
                f"No engagement found for corner_angle={corner_angle_deg}°"
            )
        area = self._probe_area(ca, result.pos, result.heading, step_length, R)
        assert area <= max_cut_area, (
            f"corner={corner_angle_deg}°: probe area {area:.4f} exceeds "
            f"max_cut_area {max_cut_area:.4f} at pos={result.pos}"
        )


# ── Reengagement offset ─────────────────────────────────────────────


def test_reengagement_position_offset_inward():
    """``search_frontier_engagement`` must offset the tool position
    inward by ``R - advance`` from the frontier, not return a point
    directly on the boundary.

    With a circular cleared area of radius 15 centered at (50, 40):
      - frontier radius = 15
      - expected tool distance from centre = 15 - (R - advance)
      - e.g. R=5, advance=2 → expected dist ≈ 12
    """
    R = 5.0
    advance = 2.0
    step_length = 1.0

    ca = ClearedArea(boundary=[])
    n = 32
    circle = [
        (
            50.0 + 15.0 * math.cos(2 * math.pi * i / n),
            40.0 + 15.0 * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]
    ca.cut([circle])

    result = search_frontier_engagement(
        ca,
        ToolPose(pos=(50.0, 55.0), heading=0.0),
        R,
        step_length,
        advance,
        0.1,
        float("inf"),
    )
    assert result is not None
    dist = math.hypot(result.pos[0] - 50.0, result.pos[1] - 40.0)
    expected = 15.0 - (R - advance)
    assert abs(dist - expected) < 1.0, (
        f"Tool centre at dist {dist:.3f} from circle centre, "
        f"expected ~{expected:.3f}"
    )


def test_reengagement_first_step_has_correct_engagement():
    """``search_reengagement`` returns a position offset inward from
    the frontier by ``R - advance``, and the cut area at that
    position is at least ``min_cut_area``.
    """
    R = 5.0
    advance = 2.0
    step_length = 1.0
    min_cut_area = target_area_per_distance(R, advance, step_length) * step_length

    ca = ClearedArea(boundary=[])
    n = 32
    circle = [
        (
            50.0 + 15.0 * math.cos(2 * math.pi * i / n),
            40.0 + 15.0 * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]
    ca.cut([circle])

    result = search_reengagement(
        ca,
        ToolPose(pos=(50.0, 55.0), heading=0.0),
        R,
        step_length,
        advance,
        min_cut_area,
    )
    assert result is not None

    dist = math.hypot(result.pos[0] - 50.0, result.pos[1] - 40.0)
    expected_dist = 15.0 - (R - advance)
    assert abs(dist - expected_dist) < 1.0, (
        f"Tool centre at dist {dist:.3f} from circle centre, "
        f"expected ~{expected_dist:.3f}"
    )

    probe = (
        result.pos[0] + math.cos(result.heading) * step_length,
        result.pos[1] + math.sin(result.heading) * step_length,
    )
    actual_area = ca.cut_area(result.pos, probe, R)
    assert actual_area >= min_cut_area, (
        f"actual_area={actual_area:.4f} < min_cut_area={min_cut_area:.4f}"
    )
