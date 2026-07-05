"""Tests for raygeo.ops.assembly.adaptive.routing module."""

import math

import pytest

from raygeo.ops.assembly.adaptive.routing import smooth_route


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


# ── smooth_route ─────────────────────────────────────────────────────


class TestSmoothRoute:
    def test_empty_raw_returns_from(self):
        out = smooth_route((1.0, 2.0, 0.0), [], [], 3.0)
        assert out == [(1.0, 2.0, 0.0)]

    def test_single_waypoint_appended(self):
        """A single raw waypoint is kept as the destination."""
        out = smooth_route((0.0, 0.0, 0.0), [(10.0, 0.0, 0.0)], [], 3.0)
        assert len(out) >= 2
        assert out[0] == (0.0, 0.0, 0.0)
        assert out[-1] == (10.0, 0.0, 0.0)

    def test_preserves_endpoints(self):
        """from_pt is the first point, last raw point the final point."""
        raw = [(5, 0, 0), (10, 0, 0), (10, 10, 0), (10, 20, 0)]
        out = smooth_route((0.0, 0.0, 0.0), raw, [], 3.0)
        assert out[0] == pytest.approx((0.0, 0.0, 0.0))
        assert out[-1] == pytest.approx((10.0, 20.0, 0.0))

    def test_shortens_collinear_without_obstacles(self):
        """With no obstacles the smoothed path is a near-straight line
        from start to end (shortcut phase removes intermediate hops;
        resampling adds density but the total arc length stays short)."""
        raw = [(5, 0, 0), (10, 0, 0), (15, 0, 0), (20, 0, 0)]
        out = smooth_route((0.0, 0.0, 0.0), raw, [], 3.0)
        assert out[0] == pytest.approx((0.0, 0.0, 0.0))
        assert out[-1] == pytest.approx((20.0, 0.0, 0.0))
        # Arc length ≈ straight-line distance (20) within tolerance.
        arc = sum(_dist(a, b) for a, b in zip(out, out[1:]))
        assert arc <= 20.0 + 1.0

    def test_keeps_clearance_from_island(self):
        """A raw path that already skirts the island is not pulled back
        into it by smoothing."""
        island = _rect(15, 10, 8, 8)
        raw = [(5, 10, 0), (15, 20, 0), (25, 10, 0)]
        out = smooth_route((0.0, 10.0, 0.0), raw, [island], clearance=2.0)
        for x, y, _ in out:
            inside = (11 < x < 19) and (6 < y < 14)
            assert not inside, f"point ({x:.2f},{y:.2f}) inside island"

    def test_stays_clear_of_remaining(self):
        """A raw path that skirts the remaining-stock polygon is not
        pulled back into it by smoothing."""
        remaining = _rect(15, 5, 8, 8)
        raw = [(5, 5, 0), (15, 15, 0), (25, 5, 0)]
        out = smooth_route((0.0, 5.0, 0.0), raw, [remaining], clearance=2.0)
        for x, y, _ in out:
            inside = (11 < x < 19) and (1 < y < 9)
            assert not inside, f"point ({x:.2f},{y:.2f}) inside remaining"

    def test_path_is_continuous(self):
        """Successive output points are within a reasonable hop distance."""
        raw = [(0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0)]
        out = smooth_route((-5.0, -5.0, 0.0), raw, [], 3.0)
        for a, b in zip(out, out[1:]):
            assert _dist(a, b) < 50.0
