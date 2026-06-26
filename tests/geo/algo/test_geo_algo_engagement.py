"""Tests for engagement module."""

import math

from raygeo.geo.algo.engagement import (
    angular_engagement,
    compute_engagement,
    point_engagement,
)


def _square(
    x: float, y: float, w: float, h: float
) -> list[tuple[float, float]]:
    """Axis-aligned rectangle as a polygon."""
    return [
        (x, y),
        (x + w, y),
        (x + w, y + h),
        (x, y + h),
    ]


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


# ── point_engagement ──────────────────────────────────────────────


def test_point_engagement_inside_cleared():
    """Inside cleared material yields engagement < π."""
    square = _square(0, 0, 10, 10)
    angle, _, _ = point_engagement((5, 5), 5.0, [square])
    assert angle < math.pi


def test_point_engagement_outside_cleared():
    """Far from cleared material yields engagement ≈ 2π."""
    square = _square(0, 0, 10, 10)
    angle, _, _ = point_engagement((50, 50), 5.0, [square])
    assert angle > math.pi


def test_point_engagement_on_boundary():
    """On the boundary engagement ≈ π."""
    square = _square(0, 0, 10, 10)
    angle, _, _ = point_engagement((0, 5), 5.0, [square])
    assert abs(angle - math.pi) < 0.1


# ── angular_engagement ────────────────────────────────────────────


def test_angular_engagement_empty():
    """No fragments returns 2π (full engagement)."""
    e = angular_engagement((0, 0), 5.0, [])
    assert abs(e - 2.0 * math.pi) < 1e-12


def test_angular_engagement_inside_cleared():
    """Disk inside cleared area returns near-zero engagement."""
    square = _square(0, 0, 100, 100)
    e = angular_engagement((50, 50), 5.0, [square])
    assert e < 0.1


def test_angular_engagement_outside_cleared():
    """Disk far from cleared returns ≈ 2π."""
    square = _square(0, 0, 10, 10)
    e = angular_engagement((50, 50), 5.0, [square])
    assert abs(e - 2.0 * math.pi) < 0.5  # approximation tolerance
