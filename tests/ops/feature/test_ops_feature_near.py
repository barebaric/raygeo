"""Tests for ops/feature/near — plunge point finder."""

import math

from raygeo.geo.shape.polygon import get_signed_boundary_distance
from raygeo.ops.feature.near import find_plunge_point


def _circle(cx, cy, r, n=32):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_near_empty_cleared_returns_none():
    """No cleared polygons returns None."""
    boundary = _rect(-20, -20, 40, 40)
    result = find_plunge_point(
        near=(0, 0),
        cleared_polygons=[],
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        search_radius=10.0,
    )
    assert result is None


def test_near_finds_point_in_cleared_disk():
    """Single cleared disk — returns a point whose tool disk fits inside it."""
    cleared = [_circle(0, 0, 5.0)]
    boundary = _rect(-20, -20, 40, 40)
    result = find_plunge_point(
        near=(0, 0),
        cleared_polygons=cleared,
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        search_radius=10.0,
    )
    assert result is not None
    x, y = result
    d = math.sqrt(x * x + y * y)
    assert d <= 5.0, f"expected point inside cleared disk, got distance {d}"
    # Full tool disk must fit inside the cleared area.
    sd = get_signed_boundary_distance((x, y), cleared)
    assert sd <= -3.0, (
        f"tool disk must fit inside cleared area, signed dist {sd} > -3.0"
    )


def test_near_avoids_island():
    """Island blocks the center — tool disk must not overlap the island."""
    cleared = [_circle(0, 0, 10.0)]
    boundary = _rect(-20, -20, 40, 40)
    island = _rect(-2, -2, 4, 4)
    result = find_plunge_point(
        near=(0, 0),
        cleared_polygons=cleared,
        boundary=boundary,
        islands=[island],
        tool_radius=3.0,
        search_radius=10.0,
    )
    assert result is not None
    x, y = result
    # Tool disk must not overlap the island (centre outside island and
    # at least tool_radius away from its boundary).
    sd_island = get_signed_boundary_distance((x, y), [island])
    assert sd_island >= 3.0, (
        f"tool disk overlaps island, signed dist {sd_island} < 3.0"
    )
    # And it must still fit inside the cleared area.
    sd_cleared = get_signed_boundary_distance((x, y), cleared)
    assert sd_cleared <= -3.0, (
        f"tool disk must fit inside cleared area, signed dist {sd_cleared}"
    )


def test_near_disk_fits_in_narrow_corridor():
    """In a narrow corridor the full tool disk must fit inside the cleared
    area (regression: a previous version only checked the centre)."""
    tool_radius = 3.0
    boundary = _rect(0, 0, 40, 8.0)
    cleared = [_rect(0.5, 0.5, 39, 7.0)]
    result = find_plunge_point(
        near=(1.5, 1.2),
        cleared_polygons=cleared,
        boundary=boundary,
        islands=None,
        tool_radius=tool_radius,
        search_radius=10.0,
    )
    assert result is not None
    sd = get_signed_boundary_distance(result, cleared)
    assert sd <= -tool_radius, (
        f"tool disk must fit inside cleared corridor, signed dist {sd}"
    )


def test_near_returns_closest_valid_point():
    """The returned point is the closest valid placement to `near`.

    Regression: an earlier version returned the first valid candidate
    found while sweeping rings at fixed angles, which could be far from
    the closest legal position.  Here `near` is just outside a cleared
    disk; the result must be within one ring step of the disk boundary.
    """
    tool_radius = 3.0
    boundary = _rect(-20, -20, 40, 40)
    cleared = [_circle(0, 0, 6.0)]
    near = (9.0, 0.0)  # 3 mm outside the cleared disk of radius 6
    result = find_plunge_point(
        near=near,
        cleared_polygons=cleared,
        boundary=boundary,
        islands=None,
        tool_radius=tool_radius,
        search_radius=10.0,
    )
    assert result is not None
    # The closest legal centre is on the line from origin to `near`,
    # at cleared_radius - tool_radius = 6 - 3 = 3 mm from the origin.
    expected = (3.0, 0.0)
    d = math.dist(result, expected)
    # Allow one ring step of slack (tool_radius / 2 = 1.5).
    assert d <= tool_radius * 0.5, (
        f"expected point near {expected}, got {result} (dist {d:.2f})"
    )
