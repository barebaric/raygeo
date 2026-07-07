"""Tests for profile offset machinery used by profiling operations."""

from raygeo.geo.algo.offset import compute_inset_region
from raygeo.geo.shape.polygon import (
    JoinStyle,
    offset_polygon,
)


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _rect_hole(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx - w / 2, cy + h / 2),
        (cx + w / 2, cy + h / 2),
        (cx + w / 2, cy - h / 2),
    ]


def _bbox(poly):
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    return min(xs), min(ys), max(xs), max(ys)


def _assert_bbox_close(poly, expected_w, expected_h, tol=0.05, cx=0.0, cy=0.0):
    x0, y0, x1, y1 = _bbox(poly)
    dw = abs((x1 - x0) - expected_w)
    dh = abs((y1 - y0) - expected_h)
    cx_actual = (x0 + x1) / 2
    cy_actual = (y0 + y1) / 2
    assert dw <= tol, f"width {x1 - x0} \u2260 {expected_w}"
    assert dh <= tol, f"height {y1 - y0} \u2260 {expected_h}"
    assert abs(cx_actual - cx) <= tol, f"center x {cx_actual} \u2260 {cx}"
    assert abs(cy_actual - cy) <= tol, f"center y {cy_actual} \u2260 {cy}"


def test_inset_rect_by_radius():
    """Inset a rectangle by radius 3 gives a 54×54 rect."""
    boundary = _rect(0, 0, 60, 60)
    region, _ = compute_inset_region(boundary, 3, [])
    assert len(region) == 1
    _assert_bbox_close(region[0], 54, 54, tol=0.01)


def test_inset_rect_with_square_island():
    """Islands are grown and subtracted from the inset."""
    boundary = _rect(0, 0, 60, 60)
    island = _rect_hole(0, 0, 10, 10)
    region_with, area_with = compute_inset_region(boundary, 3, [island])
    _, area_without = compute_inset_region(boundary, 3, [])
    assert len(region_with) >= 2
    assert area_with < area_without


def test_grow_rect_by_radius():
    """Growing a rect by radius 3 gives a 66×66 rect."""
    boundary = _rect(0, 0, 60, 60)
    polys = offset_polygon(boundary, 3, JoinStyle.Round)
    assert len(polys) == 1
    _assert_bbox_close(polys[0], 66, 66, tol=0.05)


def test_grow_square_island_by_radius():
    """Growing a 10×10 CW rect by radius 3 gives a ~16×16 shape."""
    island = _rect_hole(0, 0, 10, 10)
    polys = offset_polygon(island, 3, JoinStyle.Round)
    assert len(polys) == 1
    _assert_bbox_close(polys[0], 16, 16, tol=0.05)


def test_inset_with_large_radius_returns_empty():
    """Inset larger than half the extent produces no valid region."""
    region, area = compute_inset_region(_rect(0, 0, 10, 10), 10, [])
    assert len(region) == 0 or area == 0.0


def test_inset_join_style_miter_corners():
    """Miter produces 4 verts on a rect; Round expand adds arc vertices."""
    boundary = _rect(0, 0, 60, 60)
    region, _ = compute_inset_region(boundary, 3, [])
    miter_poly = region[0]
    n_miter = len(miter_poly)
    assert n_miter == 4, f"expected 4 miter vertices, got {n_miter}"
    round_polys = offset_polygon(boundary, 3, JoinStyle.Round)
    n_round = len(round_polys[0])
    assert n_round > 4, f"expected round >4 vertices, got {n_round}"
