"""Tests for self-intersection resolution (Geometry.remove_self_intersections).

Verifies that:

* A bow-tie (self-crossing) contour is rebuilt into a clean outline
  without self-intersections, preserving the filled area.
* Two overlapping contours are merged into a single region.
* Clean geometry passes through unchanged (curves preserved).
* Properly wound holes survive the resolution.
"""

import pytest

from raygeo.geo import Geometry


def _bowtie() -> Geometry:
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 10)
    g.line_to(10, 0)
    g.line_to(0, 10)
    g.line_to(0, 0)
    return g


def _rect(x: float, y: float, w: float, h: float, cw=False) -> Geometry:
    if cw:
        pts = [(x, y), (x, y + h), (x + w, y + h), (x + w, y)]
    else:
        pts = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
    g = Geometry()
    g.move_to(*pts[0])
    for p in pts[1:]:
        g.line_to(*p)
    g.line_to(*pts[0])
    return g


def test_bowtie_detected_as_self_intersecting():
    assert _bowtie().has_self_intersections()


def test_bowtie_resolved():
    resolved = _bowtie().remove_self_intersections()
    assert not resolved.has_self_intersections()
    assert resolved.area() == pytest.approx(50.0, rel=0.01)


def test_overlapping_contours_merged():
    g = Geometry()
    g.extend(_rect(0, 0, 10, 10))
    g.extend(_rect(5, 0, 10, 10))
    resolved = g.remove_self_intersections()
    contours = resolved.split_into_contours()
    assert len(contours) == 1
    assert resolved.area() == pytest.approx(150.0, rel=0.01)


def test_plus_of_two_bars_becomes_single_contour():
    """A plus sign drawn as two crossing rectangles (neither one
    self-intersecting) must merge into one continuous outline."""
    g = Geometry()
    g.extend(_rect(4, -5, 2, 20))
    g.extend(_rect(-5, 4, 20, 2))
    assert len(g.split_into_contours()) == 2
    resolved = g.remove_self_intersections()
    contours = resolved.split_into_contours()
    assert len(contours) == 1
    assert not resolved.has_self_intersections()
    assert resolved.area() == pytest.approx(2 * 20 + 2 * 20 - 4, rel=0.01)


def _cmd_summary(geo: Geometry) -> list:
    summary = []
    for c in geo.data:
        end = getattr(c, "end", None)
        if isinstance(end, tuple):
            summary.append(
                (type(c).__name__, round(end[0], 6), round(end[1], 6))
            )
        elif end is not None:
            summary.append(
                (type(c).__name__, round(end.x, 6), round(end.y, 6))
            )
        else:
            summary.append((type(c).__name__,))
    return summary


def test_clean_geometry_unchanged():
    g = _rect(0, 0, 10, 10)
    original = _cmd_summary(g)
    resolved = g.remove_self_intersections()
    assert _cmd_summary(resolved) == original


def test_clean_curves_preserved():
    g = _rect(0, 0, 20, 20)
    g.bezier_to(10, 2, 5, -5, 15, -5)
    kinds_before = [type(c).__name__ for c in g.data]
    resolved = g.remove_self_intersections()
    kinds_after = [type(c).__name__ for c in resolved.data]
    assert kinds_after == kinds_before
    assert "Bezier" in kinds_after


def test_hole_survives_resolution():
    g = Geometry()
    g.extend(_rect(0, 0, 10, 10))
    g.extend(_rect(9, 0, 10, 10))
    g.extend(_rect(4, 4, 2, 2, cw=True))
    resolved = g.remove_self_intersections()
    kinds = [type(c).__name__ for c in resolved.data]
    assert "Move" in kinds
    assert not resolved.has_self_intersections()
    assert resolved.area() == pytest.approx(190.0 - 4.0, rel=0.01)


def test_open_contours_pass_through():
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 10)
    g.line_to(10, 0)
    resolved = g.remove_self_intersections()
    assert len(resolved.data) == len(g.data)
