"""Tests for area resume module."""

import math

from raygeo.ops.area import ClearedArea, ResumePoint


def _circle(cx, cy, r, n=32):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def test_find_next_resume_basic():
    """Returns a resume point with correct types when one exists."""
    ca = ClearedArea()
    ca.add_cleared_polygons([_circle(50, 40, 15)])

    result = ca.find_next_resume(
        end_pos=(50.0, 55.0),
        radius=3.0,
        step_length=0.6,
        min_cut_area=0.1,
    )
    assert result is not None
    assert len(result.pos) == 2
    assert isinstance(result.heading, float)


def test_find_next_resume_none_empty():
    """Empty cleared area returns None."""
    ca = ClearedArea()

    result = ca.find_next_resume(
        end_pos=(50.0, 55.0),
        radius=3.0,
        step_length=0.6,
        min_cut_area=0.1,
    )
    assert result is None


def test_find_next_resume_returns_resume_point():
    """Returns a ResumePoint with valid attributes when found."""
    ca = ClearedArea()
    ca.add_cleared_polygons([_circle(50, 40, 15)])

    result = ca.find_next_resume(
        end_pos=(50.0, 55.0),
        radius=3.0,
        step_length=0.6,
        min_cut_area=0.1,
    )
    assert isinstance(result, ResumePoint)
    assert isinstance(result.pos, tuple)
    assert len(result.pos) == 2
    assert isinstance(result.heading, float)


def test_find_next_resume_cut_area_filter():
    """Very high min_cut_area returns None (no suitable vertex)."""
    ca = ClearedArea()
    ca.add_cleared_polygons([_circle(50, 40, 15)])

    result = ca.find_next_resume(
        end_pos=(50.0, 55.0),
        radius=3.0,
        step_length=0.6,
        min_cut_area=1e6,
    )
    assert result is None


def test_resume_point_repr():
    """ResumePoint has a readable repr."""
    rp = ResumePoint(pos=(10.0, 20.0), heading=1.5)
    s = repr(rp)
    assert "ResumePoint" in s
    assert "10" in s
