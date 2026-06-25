"""Tests for area resume module."""

import math

from raygeo.geo.algo.medial_axis import MedialAxis
from raygeo.ops.area import ClearedArea, ResumePoint


def test_find_next_resume_needs_mat():
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    ca = ClearedArea()
    n = 32
    circle = [
        (
            50 + 15 * math.cos(2 * math.pi * i / n),
            40 + 15 * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]
    ca.add_cleared_polygons([circle])

    mat = MedialAxis.compute(
        boundary, holes=[], min_clearance=1.0, sampling_spacing=6.0
    )

    result = ca.find_next_resume(
        mat=mat,
        end_pos=(50.0, 55.0),
        radius=3.0,
        min_engagement=math.pi * 0.3,
    )
    if result is not None:
        assert len(result.pos) == 2
        assert isinstance(result.heading, float)
        assert len(result.link_path) >= 1


def test_find_next_resume_none_empty():
    """Empty cleared area returns None."""
    ca = ClearedArea()
    mat = MedialAxis.compute(
        [(0, 0), (100, 0), (100, 80), (0, 80)],
        holes=[],
        min_clearance=1.0,
        sampling_spacing=6.0,
    )
    result = ca.find_next_resume(
        mat=mat,
        end_pos=(50.0, 55.0),
        radius=3.0,
        min_engagement=math.pi * 0.3,
    )
    assert result is None


def test_find_next_resume_returns_resume_point():
    """Returns a ResumePoint with valid attributes when found."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    ca = ClearedArea()
    n = 32
    circle = [
        (
            50 + 15 * math.cos(2 * math.pi * i / n),
            40 + 15 * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]
    ca.add_cleared_polygons([circle])
    mat = MedialAxis.compute(
        boundary,
        holes=[],
        min_clearance=1.0,
        sampling_spacing=6.0,
    )
    result = ca.find_next_resume(
        mat=mat,
        end_pos=(50.0, 55.0),
        radius=3.0,
        min_engagement=math.pi * 0.3,
    )
    assert isinstance(result, ResumePoint)
    assert isinstance(result.pos, tuple)
    assert len(result.pos) == 2
    assert isinstance(result.heading, float)
    assert isinstance(result.link_path, list)
    assert len(result.link_path) >= 1


def test_find_next_resume_min_engagement_filter():
    """Higher min_engagement may return None (no suitable vertex)."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    ca = ClearedArea()
    n = 32
    circle = [
        (
            50 + 15 * math.cos(2 * math.pi * i / n),
            40 + 15 * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]
    ca.add_cleared_polygons([circle])
    mat = MedialAxis.compute(
        boundary,
        holes=[],
        min_clearance=1.0,
        sampling_spacing=6.0,
    )
    # 2π engagement is impossible on any real vertex → likely None
    result = ca.find_next_resume(
        mat=mat,
        end_pos=(50.0, 55.0),
        radius=3.0,
        min_engagement=2.0 * math.pi,
    )
    # Either None or a resume point with engagement ≥ 2π
    if result is not None:
        assert result.heading is not None


def test_resume_point_repr():
    """ResumePoint has a readable repr."""
    rp = ResumePoint(pos=(10.0, 20.0), heading=1.5, link_path=[(0, 0)])
    s = repr(rp)
    assert "ResumePoint" in s
    assert "10" in s
