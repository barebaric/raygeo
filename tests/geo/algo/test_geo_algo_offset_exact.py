"""Tests for the exact line-and-arc offsetting of closed contours."""

import math

import pytest

from raygeo.geo import Geometry
from raygeo.geo.algo.offset_exact import offset_exact


def rect(x0, y0, x1, y1, z=0.0, ccw=True):
    """A closed rectangle; counter-clockwise unless ``ccw`` is false."""
    g = Geometry()
    pts = (
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        if ccw
        else [(x0, y1), (x1, y1), (x1, y0), (x0, y0)]
    )
    g.move_to(*pts[0], z)
    for p in pts[1:]:
        g.line_to(*p, z)
    g.close_path()
    return g


def circle(r, cx=0.0, cy=0.0, z=0.0):
    """A circle as two half arcs, counter-clockwise."""
    g = Geometry()
    g.move_to(cx + r, cy, z)
    g.arc_to(cx - r, cy, -r, 0.0, False, z)
    g.arc_to(cx + r, cy, r, 0.0, False, z)
    return g


def arcs_of(loop):
    return [c for c in loop.data if type(c).__name__ == "Arc"]


def lines_of(loop):
    return [c for c in loop.data if type(c).__name__ == "Line"]


def arc_radius(arc):
    return math.hypot(arc.center_offset[0], arc.center_offset[1])


def bbox(loop):
    ends = [c.end for c in loop.data if type(c).__name__ != "Move"]
    xs = [e[0] for e in ends]
    ys = [e[1] for e in ends]
    return min(xs), min(ys), max(xs), max(ys)


def test_circle_grows_by_an_exact_arc():
    """A grown circle stays a circle: its arcs keep the exact new radius."""
    for grown in (1.0, 3.5):
        loops = offset_exact(circle(10.0), grown, 0.1)
        assert len(loops) == 1, f"expected one loop, got {len(loops)}"
        arcs = arcs_of(loops[0])
        assert len(arcs) == 2, f"expected the two halves as arcs, got {arcs}"
        for arc in arcs:
            assert arc_radius(arc) == pytest.approx(10.0 + grown, abs=1e-9)


def test_circle_shrinks_by_an_exact_arc():
    loops = offset_exact(circle(10.0), -2.0, 0.1)
    assert len(loops) == 1
    for arc in arcs_of(loops[0]):
        assert arc_radius(arc) == pytest.approx(8.0, abs=1e-9)


def test_rectangle_grows_with_round_corners():
    """A grown rectangle is four straight walls joined by roll arcs of the
    offset's radius, and reaches exactly one offset further on every side."""
    loops = offset_exact(rect(0, 0, 40, 30), 5.0, 0.1)
    assert len(loops) == 1
    assert len(lines_of(loops[0])) == 4
    arcs = arcs_of(loops[0])
    assert len(arcs) == 4
    for arc in arcs:
        assert arc_radius(arc) == pytest.approx(5.0, abs=1e-9)
    assert bbox(loops[0]) == pytest.approx((-5.0, -5.0, 45.0, 35.0), abs=1e-9)


def test_rectangle_shrunk_past_half_its_width_vanishes():
    assert offset_exact(rect(0, 0, 20, 20), -15.0, 0.1) == []


def test_hole_shrinks_when_the_region_grows():
    """A clockwise inner loop bounds an island: growing the region
    shrinks it."""
    region = rect(0, 0, 100, 100)
    region.extend(rect(40, 40, 60, 60, ccw=False))
    loops = offset_exact(region, 5.0, 0.1)
    assert len(loops) == 2, (
        f"expected outer and island loops, got {len(loops)}"
    )
    boxes = sorted(bbox(lp) for lp in loops)
    assert boxes[0] == pytest.approx((-5.0, -5.0, 105.0, 105.0), abs=1e-9)
    assert boxes[1] == pytest.approx((45.0, 45.0, 55.0, 55.0), abs=1e-9)


def test_two_disjoint_solids_offset_separately():
    region = rect(0, 0, 10, 10)
    region.extend(rect(20, 20, 30, 30))
    loops = offset_exact(region, 1.0, 0.1)
    assert len(loops) == 2, f"expected two loops, got {len(loops)}"


def test_slot_grows_shut_as_one_loop():
    """Growing a U-shaped boundary until its slot closes trims the offset to
    one simple loop instead of a self-overlapping one."""
    u = Geometry()
    pts = [
        (0, 0),
        (30, 0),
        (30, 20),
        (18, 20),
        (18, 6),
        (12, 6),
        (12, 20),
        (0, 20),
    ]
    u.move_to(*pts[0], 0)
    for p in pts[1:]:
        u.line_to(*p, 0)
    u.close_path()
    loops = offset_exact(u, 5.0, 0.1)
    assert len(loops) == 1, (
        f"expected the slot to close into one loop, got {len(loops)}"
    )


def test_l_shape_shrinks_into_itself():
    """Shrinking keeps walls exact: the L's legs move in by the offset,
    its convex corners roll and its concave corner is trimmed."""
    l_shape = Geometry()
    pts = [(0, 0), (30, 0), (30, 10), (10, 10), (10, 30), (0, 30)]
    l_shape.move_to(*pts[0], 0)
    for p in pts[1:]:
        l_shape.line_to(*p, 0)
    l_shape.close_path()
    loops = offset_exact(l_shape, -2.0, 0.1)
    assert len(loops) == 1
    assert bbox(loops[0]) == pytest.approx((2.0, 2.0, 28.0, 28.0), abs=1e-9)


def test_spline_boundaries_follow_line_chains_within_tolerance():
    """A bezier boundary has no exact offset: it is followed as a line
    chain, mitered while that stays within tolerance."""
    b = Geometry()
    b.move_to(0, 0, 0)
    b.line_to(40, 0, 0)
    b.bezier_to(40, 30, 50, 0, 50, 30)
    b.line_to(0, 30, 0)
    b.close_path()
    loops = offset_exact(b, 3.0, 0.05)
    assert len(loops) == 1
    kinds = [
        type(c).__name__ for c in loops[0].data if type(c).__name__ != "Move"
    ]
    assert len(kinds) < 1200, f"unterminated chain: {len(kinds)} segments"
    # Only the two spline-to-wall corners are sharp enough to roll.
    assert len([k for k in kinds if k == "Arc"]) == 2
    xs = [c.end[0] for c in loops[0].data if type(c).__name__ != "Move"]
    assert max(xs) > 50.0, "the spline bulge did not grow outwards"


def test_z_is_carried_through():
    loops = offset_exact(rect(0, 0, 40, 30, z=7.0), 2.0, 0.1)
    assert len(loops) == 1
    for cmd in loops[0].data:
        assert cmd.end[2] == pytest.approx(7.0, abs=1e-12)


def test_open_subpaths_are_ignored():
    g = Geometry()
    g.move_to(0, 0, 0)
    g.line_to(10, 0, 0)
    g.line_to(10, 10, 0)
    assert offset_exact(g, 2.0, 0.1) == []


def test_degenerate_input_returns_empty():
    assert offset_exact(Geometry(), 1.0, 0.1) == []
    assert offset_exact(circle(10.0), float("nan"), 0.1) == []
    assert offset_exact(circle(10.0), 1.0, 0.0) == []
