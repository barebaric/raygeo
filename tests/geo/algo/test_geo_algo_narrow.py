"""Tests for geo/algo/narrow — narrow-passage detection."""

import pytest

from raygeo.geo.algo.narrow import find_narrow_passages


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_narrow_wide_rect_returns_empty():
    """Open rectangle: no narrow passages."""
    boundary = _rect(0, 0, 80, 60)
    regions = find_narrow_passages(boundary, holes=None, max_width=10.0)
    assert regions == []


def test_narrow_dumbbell_finds_channel():
    """Dumbbell: the connecting channel is detected as a narrow passage."""
    boundary = [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 21.0),
        (60.0, 21.0),
        (60.0, 0.0),
        (100.0, 0.0),
        (100.0, 50.0),
        (60.0, 50.0),
        (60.0, 29.0),
        (40.0, 29.0),
        (40.0, 50.0),
        (0.0, 50.0),
    ]
    regions = find_narrow_passages(boundary, holes=None, max_width=10.0)
    assert len(regions) >= 1
    for poly in regions:
        assert len(poly) >= 3


def test_narrow_tight_slot_detected():
    """A 4mm-wide corridor is a narrow passage for max_width=10."""
    boundary = _rect(0, 0, 120, 4)
    regions = find_narrow_passages(boundary, holes=None, max_width=10.0)
    assert len(regions) >= 1


def test_narrow_threshold_controls_result():
    """A 12mm-wide corridor (clearance ~6) is not narrow for max_width=10
    (threshold 5) but IS narrow for max_width=20 (threshold 10)."""
    boundary = _rect(0, 0, 120, 12)
    regions_small = find_narrow_passages(boundary, max_width=10.0)
    regions_large = find_narrow_passages(boundary, max_width=20.0)
    assert len(regions_small) == 0
    assert len(regions_large) >= 1


def test_narrow_polygon_has_no_nan():
    """No NaN coordinates in the passage polygons."""
    boundary = [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 21.0),
        (60.0, 21.0),
        (60.0, 0.0),
        (100.0, 0.0),
        (100.0, 50.0),
        (60.0, 50.0),
        (60.0, 29.0),
        (40.0, 29.0),
        (40.0, 50.0),
        (0.0, 50.0),
    ]
    regions = find_narrow_passages(boundary, holes=None, max_width=10.0)
    for poly in regions:
        for x, y in poly:
            assert x == x, "NaN x in polygon"
            assert y == y, "NaN y in polygon"


def test_narrow_invalid_polygon_raises():
    """Fewer than 3 vertices raises RuntimeError."""
    with pytest.raises(RuntimeError):
        find_narrow_passages([(0, 0), (1, 1)], max_width=10.0)


def test_narrow_invalid_max_width_raises():
    """Non-positive max_width raises RuntimeError."""
    with pytest.raises(RuntimeError):
        find_narrow_passages(_rect(0, 0, 40, 40), max_width=0.0)
    with pytest.raises(RuntimeError):
        find_narrow_passages(_rect(0, 0, 40, 40), max_width=-1.0)
