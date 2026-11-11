"""Tests for the ellipse shape queries."""

import math

import pytest

from raygeo.geo.shape.ellipse import (
    get_ellipse_area,
    get_ellipse_point_at,
    get_ellipse_samples,
    is_point_inside_ellipse,
    is_point_on_ellipse,
)

CENTER = (10.0, 20.0)
RX = 30.0
RY = 12.0
ROTATION = math.pi / 6


def test_point_at_endpoints_matches_parametric_form():
    t0 = get_ellipse_point_at(CENTER, RX, RY, ROTATION, 0.0)
    expected = (
        CENTER[0] + RX * math.cos(ROTATION),
        CENTER[1] + RX * math.sin(ROTATION),
    )
    assert t0 == pytest.approx(expected)

    t_half = get_ellipse_point_at(CENTER, RX, RY, ROTATION, math.pi / 2)
    expected = (
        CENTER[0] - RY * math.sin(ROTATION),
        CENTER[1] + RY * math.cos(ROTATION),
    )
    assert t_half == pytest.approx(expected)


def test_axis_aligned_point_at_matches_unrotated_form():
    point = get_ellipse_point_at((0.0, 0.0), RX, RY, 0.0, math.pi)
    assert point == pytest.approx((-RX, 0.0))


def test_samples_are_closed_and_on_curve():
    samples = get_ellipse_samples(CENTER, RX, RY, ROTATION, 16)
    assert len(samples) == 17
    assert samples[0] == pytest.approx(samples[-1])
    for point in samples:
        assert is_point_on_ellipse(point, CENTER, RX, RY, ROTATION, 1e-6)


def test_point_on_ellipse_tolerances():
    point = get_ellipse_point_at(CENTER, RX, RY, ROTATION, 1.0)
    assert is_point_on_ellipse(point, CENTER, RX, RY, ROTATION, 1e-6)
    assert not is_point_on_ellipse(point, CENTER, RX, RY, ROTATION, 0.0)

    outside = (
        point[0] + 4.0 * math.cos(ROTATION),
        point[1] + 4.0 * math.sin(ROTATION),
    )
    assert not is_point_on_ellipse(outside, CENTER, RX, RY, ROTATION, 0.5)
    assert is_point_on_ellipse(outside, CENTER, RX, RY, ROTATION, 6.0)


def test_point_on_ellipse_degenerate():
    assert not is_point_on_ellipse(CENTER, CENTER, 0.0, RY, 0.0, 1e-6)


def test_point_inside_ellipse():
    assert is_point_inside_ellipse(CENTER, CENTER, RX, RY, ROTATION)
    inside = get_ellipse_point_at(CENTER, RX * 0.5, RY * 0.5, ROTATION, 1.0)
    assert is_point_inside_ellipse(inside, CENTER, RX, RY, ROTATION)
    outside = get_ellipse_point_at(CENTER, RX * 2.0, RY * 2.0, ROTATION, 1.0)
    assert not is_point_inside_ellipse(outside, CENTER, RX, RY, ROTATION)
    assert not is_point_inside_ellipse(CENTER, CENTER, 0.0, RY, 0.0)


def test_ellipse_area():
    assert get_ellipse_area(RX, RY) == pytest.approx(math.pi * RX * RY)
