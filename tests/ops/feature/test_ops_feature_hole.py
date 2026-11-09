"""Tests for circular hole detection (ops.feature.hole)."""

import math

import pytest

from raygeo.ops.feature.hole import find_holes


def _circle(cx, cy, r, n=96):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_detects_circular_islands():
    holes = find_holes([_circle(10, 20, 2.5), _circle(50, 20, 4.0)], 0.1)
    assert len(holes) == 2
    (x, y, d), (x2, y2, d2) = holes
    assert (x, y) == pytest.approx((10.0, 20.0), abs=1e-3)
    assert d == pytest.approx(5.0, abs=5e-3)
    assert (x2, y2) == pytest.approx((50.0, 20.0), abs=1e-3)
    assert d2 == pytest.approx(8.0, abs=5e-3)


def test_rejects_non_circular_islands():
    assert find_holes([_rect(0, 0, 10, 5)], 0.1) == []
    triangle = [(0, 0), (10, 0), (5, 8)]
    assert find_holes([triangle], 0.1) == []


def test_order_follows_input():
    holes = find_holes([_circle(50, 0, 1.0), _circle(0, 0, 1.0)], 0.1)
    assert holes[0][0] > holes[1][0]


def test_tolerance_gates_coarse_circles():
    # A 12-segment circle approximates r=2.5 with a chord error of
    # ~0.085 mm: passes at 0.1, rejected at 0.01.
    coarse = _circle(0, 0, 2.5, n=12)
    assert len(find_holes([coarse], 0.1)) == 1
    assert find_holes([coarse], 0.01) == []


def test_empty_and_degenerate_inputs():
    assert find_holes([], 0.1) == []
    assert find_holes([[(0, 0), (1, 1)]], 0.1) == []
