"""Tests for engagement module."""

import math

from raygeo.geo.algo.engagement import compute_engagement


def test_zero_engagement_deep_inside():
    """d = -R → fully inside cleared → 0 engagement."""
    angle, area, depth = compute_engagement(-5.0, 5.0)
    assert abs(angle) < 1e-12
    assert abs(area) < 1e-12
    assert abs(depth) < 1e-12


def test_half_engagement_on_boundary():
    """d = 0 → on boundary → π radians."""
    angle, area, depth = compute_engagement(0.0, 5.0)
    assert abs(angle - math.pi) < 1e-12
    assert abs(depth - 5.0) < 1e-12


def test_full_engagement_far_into_uncut():
    """d = R → R units outside cleared → 2π."""
    angle, area, depth = compute_engagement(5.0, 5.0)
    assert abs(angle - 2 * math.pi) < 1e-12
    assert abs(depth) < 1e-12


def test_full_engagement_very_far():
    """d >> R → fully in uncut → 2π."""
    angle, area, depth = compute_engagement(50.0, 5.0)
    assert abs(angle - 2 * math.pi) < 1e-12


def test_monotonic():
    """Engagement should be non-decreasing from -R to +R."""
    r = 5.0
    prev = -1.0
    for i in range(0, 21):
        d = -r + i * r * 0.1
        angle, _, _ = compute_engagement(d, r)
        assert angle + 1e-12 >= prev, f"not monotonic at d={d:.2f}"
        prev = angle
