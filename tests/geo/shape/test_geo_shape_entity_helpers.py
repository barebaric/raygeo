"""Tests for the arc/circle/bezier helpers added for the sketcher
entity delegation (sampling, tangents, signed distance, control
offset constraints)."""

import math

import pytest

from raygeo.geo.shape.arc import (
    get_arc_samples,
    get_arc_sweep,
    get_arc_tangent_at_point,
)
from raygeo.geo.shape.bezier import (
    compute_constrained_control_offset,
    get_bezier_samples,
    get_bezier_tangent,
)
from raygeo.geo.shape.circle import get_circle_signed_distance

CENTER = (10.0, 20.0)
RADIUS = 7.0


def test_arc_samples_count_and_endpoints():
    start_angle = 0.3
    sweep = get_arc_sweep(0.3, 2.1, False)
    samples = get_arc_samples(CENTER, RADIUS, start_angle, sweep, 8)
    assert len(samples) == 9
    first = samples[0]
    assert first == pytest.approx(
        (
            CENTER[0] + RADIUS * math.cos(start_angle),
            CENTER[1] + RADIUS * math.sin(start_angle),
        )
    )
    last = samples[-1]
    assert last == pytest.approx(
        (
            CENTER[0] + RADIUS * math.cos(start_angle + sweep),
            CENTER[1] + RADIUS * math.sin(start_angle + sweep),
        )
    )
    for x, y in samples:
        assert math.hypot(x - CENTER[0], y - CENTER[1]) == pytest.approx(
            RADIUS
        )


def test_arc_samples_zero_segments_is_empty():
    assert get_arc_samples(CENTER, RADIUS, 0.0, 1.0, 0) == []


def test_arc_tangent_is_perpendicular_to_radius():
    angle = 1.1
    point = (
        CENTER[0] + RADIUS * math.cos(angle),
        CENTER[1] + RADIUS * math.sin(angle),
    )
    radius = (math.cos(angle), math.sin(angle))
    for clockwise in (False, True):
        tangent = get_arc_tangent_at_point(CENTER, point, clockwise)
        dot = tangent[0] * radius[0] + tangent[1] * radius[1]
        assert dot == pytest.approx(0.0, abs=1e-12)
        assert math.hypot(*tangent) == pytest.approx(1.0)

    ccw = get_arc_tangent_at_point(CENTER, point, False)
    cw = get_arc_tangent_at_point(CENTER, point, True)
    assert ccw == pytest.approx((-cw[0], -cw[1]))


def test_arc_tangent_chirality_matches_traversal():
    angle = 1.1
    point = (
        CENTER[0] + RADIUS * math.cos(angle),
        CENTER[1] + RADIUS * math.sin(angle),
    )
    slightly_on = (
        CENTER[0] + RADIUS * math.cos(angle + 1e-9),
        CENTER[1] + RADIUS * math.sin(angle + 1e-9),
    )
    forward = (slightly_on[0] - point[0], slightly_on[1] - point[1])
    ccw = get_arc_tangent_at_point(CENTER, point, False)
    assert ccw[0] * forward[0] + ccw[1] * forward[1] > 0.0
    cw = get_arc_tangent_at_point(CENTER, point, True)
    assert cw[0] * forward[0] + cw[1] * forward[1] < 0.0


def test_arc_tangent_degenerate_center_point():
    assert get_arc_tangent_at_point(CENTER, CENTER, False) == (1.0, 0.0)


def test_circle_signed_distance_signs():
    assert get_circle_signed_distance((20.0, 20.0), CENTER, RADIUS) == (
        pytest.approx(3.0)
    )
    assert get_circle_signed_distance((5.0, 20.0), CENTER, RADIUS) == (
        pytest.approx(-2.0)
    )
    on = (CENTER[0] + RADIUS, CENTER[1])
    assert get_circle_signed_distance(on, CENTER, RADIUS) == pytest.approx(0.0)


P0 = (1.0, 2.0)
C1 = (3.0, 5.0)
C2 = (6.0, 4.0)
P3 = (8.0, 7.0)


def test_bezier_samples_endpoints():
    samples = get_bezier_samples(P0, C1, C2, P3, 20)
    assert len(samples) == 21
    assert samples[0] == pytest.approx(P0)
    assert samples[-1] == pytest.approx(P3)


def test_bezier_samples_match_parametric_evaluation():
    samples = get_bezier_samples(P0, C1, C2, P3, 4)
    t = 0.25
    mt = 1.0 - t
    expected = (
        mt**3 * P0[0]
        + 3 * mt**2 * t * C1[0]
        + 3 * mt * t**2 * C2[0]
        + t**3 * P3[0],
        mt**3 * P0[1]
        + 3 * mt**2 * t * C1[1]
        + 3 * mt * t**2 * C2[1]
        + t**3 * P3[1],
    )
    assert samples[1] == pytest.approx(expected)


def test_bezier_tangent_directions_at_endpoints():
    tangent = get_bezier_tangent(P0, C1, C2, P3, 0.0)
    direction = (tangent[0] * C1[0] - tangent[0] * P0[0], 0.0)
    cross = tangent[0] * (C1[1] - P0[1]) - tangent[1] * (C1[0] - P0[0])
    assert cross == pytest.approx(0.0)
    assert direction[0] > 0.0

    end = get_bezier_tangent(P0, C1, C2, P3, 1.0)
    cross = end[0] * (P3[1] - C2[1]) - end[1] * (P3[0] - C2[0])
    assert cross == pytest.approx(0.0)


def test_bezier_samples_zero_segments_is_empty():
    assert get_bezier_samples(P0, C1, C2, P3, 0) == []


def test_constrained_offset_symmetric_mirrors():
    result = compute_constrained_control_offset((2.0, 3.0), None, True)
    assert result == pytest.approx((-2.0, -3.0))


def test_constrained_offset_smooth_keeps_other_length():
    modified = (3.0, 0.0)
    other = (0.0, -2.0)
    result = compute_constrained_control_offset(modified, other, False)
    assert math.hypot(*result) == pytest.approx(2.0)
    assert result[0] < 0.0
    assert result == pytest.approx((-2.0, 0.0))


def test_constrained_offset_degenerate_fallbacks():
    assert compute_constrained_control_offset(
        (2.0, 3.0), None, False
    ) == pytest.approx((-2.0, -3.0))
    assert compute_constrained_control_offset(
        (2.0, 3.0), (0.0, 0.0), False
    ) == pytest.approx((-2.0, -3.0))
    assert compute_constrained_control_offset(
        (0.0, 0.0), (1.0, 1.0), False
    ) == pytest.approx((0.0, 0.0))
