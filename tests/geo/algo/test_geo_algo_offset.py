from raygeo.geo import Geometry
from raygeo.geo.algo.offset import (
    compute_inset_region,
    concentric_offsets,
    find_deepest_cores,
    offset_contour_group,
)
from raygeo.geo.shape.polygon import JoinStyle


def make_rect(x0, y0, x1, y1):
    g = Geometry()
    g.move_to(x0, y0, 0)
    g.line_to(x1, y0, 0)
    g.line_to(x1, y1, 0)
    g.line_to(x0, y1, 0)
    g.close_path()
    return g


def test_concentric_simple():
    """Basic concentric offsets of a rectangle."""
    g = make_rect(0, 0, 100, 100)
    offsets = concentric_offsets(g, step=5, max_passes=10, min_area=1)
    assert len(offsets) >= 1
    # Each offset should have smaller area
    for i in range(1, len(offsets)):
        area_i = offsets[i].area()
        area_prev = offsets[i - 1].area()
        assert area_i < area_prev, (
            f"pass {i} area {area_i:.1f} >= pass {i - 1} area {area_prev:.1f}"
        )


def test_concentric_count():
    """100x100 rect, step=10 → each offset shrinks edges by 10mm.

    After 4 passes (80², 60², 40², 20² = 400) the 5th would be 0² (collapsed).
    """
    g = make_rect(0, 0, 100, 100)
    offsets = concentric_offsets(g, step=10, max_passes=100, min_area=1)
    assert 3 <= len(offsets) <= 5, f"expected ~4 offsets, got {len(offsets)}"


def test_concentric_max_passes():
    """max_passes should limit the number of passes."""
    g = make_rect(0, 0, 100, 100)
    offsets = concentric_offsets(g, step=5, max_passes=3, min_area=1)
    assert len(offsets) <= 3


def test_concentric_min_area():
    """Should stop early when min_area is reached."""
    g = make_rect(0, 0, 100, 100)
    offsets = concentric_offsets(g, step=5, max_passes=100, min_area=5000)
    # Area drops below 5000 after first pass (8100→6400)
    # Actually 10000 → 8100 (pass 0), 6400 (pass 1) — both above 5000
    # 4900 < 5000 → stop after pass 2
    assert len(offsets) <= 3


def test_concentric_zero_max_passes():
    """max_passes=0 → empty."""
    g = make_rect(0, 0, 100, 100)
    offsets = concentric_offsets(g, step=5, max_passes=0, min_area=1)
    assert offsets == []


def test_concentric_negative_step():
    """step <= 0 → empty (step with negative offset would expand)."""
    g = make_rect(0, 0, 100, 100)
    offsets = concentric_offsets(g, step=0, max_passes=10, min_area=1)
    assert offsets == []


def test_concentric_rectangle_with_hole():
    """Square with a square hole should produce ring-like offsets."""
    outer = Geometry()
    outer.move_to(0, 0, 0)
    outer.line_to(100, 0, 0)
    outer.line_to(100, 100, 0)
    outer.line_to(0, 100, 0)
    outer.close_path()
    # Add hole
    outer.move_to(30, 30, 0)
    outer.line_to(70, 30, 0)
    outer.line_to(70, 70, 0)
    outer.line_to(30, 70, 0)
    outer.close_path()

    offsets = concentric_offsets(outer, step=5, max_passes=10, min_area=1)
    assert len(offsets) >= 1
    # Areas should be decreasing
    for i in range(1, len(offsets)):
        assert offsets[i].area() < offsets[i - 1].area()


def test_concentric_empty_geometry():
    """Empty geometry → empty result."""
    g = Geometry()
    offsets = concentric_offsets(g, step=5, max_passes=10, min_area=1)
    assert offsets == []


def test_concentric_z_preserved():
    """Z height from first point should be preserved in offsets."""
    g = Geometry()
    g.move_to(0, 0, -5)
    g.line_to(100, 0, -5)
    g.line_to(100, 100, -5)
    g.line_to(0, 100, -5)
    g.close_path()

    offsets = concentric_offsets(g, step=5, max_passes=5, min_area=1)
    for off in offsets:
        cmd = off.data[0]
        assert cmd.end[2] == -5, f"expected z=-5, got {cmd.end[2]}"


def P(*pts):
    """Shorthand: list of (x, y) tuples."""
    return list(pts)


def test_offset_contour_group_basic():
    """Offset a solid without holes."""
    poly = P((0, 0), (10, 0), (5, 10))
    result = offset_contour_group(poly, [], 1.0)
    assert len(result) >= 1


def test_offset_contour_group_with_hole():
    """Offset a solid with a hole."""
    outer = P((0, 0), (100, 0), (100, 100), (0, 100))
    hole = P((30, 30), (70, 30), (70, 70), (30, 70))
    result = offset_contour_group(outer, [hole], 5.0)
    assert len(result) >= 1


def test_offset_contour_group_shrink():
    """Negative offset (shrink) works."""
    poly = P((0, 0), (10, 0), (5, 10))
    result = offset_contour_group(poly, [], -0.5)
    assert len(result) >= 1


def test_offset_contour_group_join_style_round():
    """Round join style produces distinct geometry from miter."""
    poly = P((0, 0), (10, 0), (5, 10))
    miter = offset_contour_group(poly, [], 1.0, join_style=JoinStyle.MITER)
    round_ = offset_contour_group(poly, [], 1.0, join_style=JoinStyle.ROUND)
    assert len(round_[0]) > len(miter[0])


def test_offset_contour_group_join_style_square():
    """Square join style should succeed without error."""
    poly = P((0, 0), (10, 0), (5, 10))
    result = offset_contour_group(poly, [], 1.0, join_style=JoinStyle.SQUARE)
    assert len(result) >= 1


# --- find_deepest_cores ---


def rect_poly(w, h):
    return [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]


def poly_area(poly):
    n = len(poly)
    sa = 0.0
    for i in range(n):
        j = (i + 1) % n
        sa += poly[i][0] * poly[j][1] - poly[j][0] * poly[i][1]
    return abs(sa) / 2.0


def test_find_deepest_cores_simple_rect():
    """Find the centre of a rectangle."""
    boundary = rect_poly(100.0, 80.0)
    area = offset_contour_group(boundary, [], -5.0, join_style=JoinStyle.ROUND)
    cores = find_deepest_cores(area, step_over=10.0)
    assert len(cores) > 0
    cx, cy = cores[0]
    assert abs(cx - 50.0) < 1.0
    assert abs(cy - 40.0) < 1.0


def test_find_deepest_cores_empty_input():
    """Empty input → empty result."""
    assert find_deepest_cores([], step_over=10.0) == []


def test_find_deepest_cores_zero_stepover():
    """step_over ≤ 0 → empty result."""
    boundary = rect_poly(100.0, 80.0)
    area = offset_contour_group(boundary, [], -5.0, join_style=JoinStyle.ROUND)
    assert find_deepest_cores(area, step_over=0.0) == []


def test_find_deepest_cores_two_regions():
    """Dumbbell shape: largest region's centroid is returned."""
    left = rect_poly(40.0, 80.0)
    right = [(x + 60, y) for (x, y) in rect_poly(40.0, 80.0)]
    area = [left, right]
    cores = find_deepest_cores(area, step_over=10.0)
    assert len(cores) == 1, f"expected 1 core, got {len(cores)}: {cores}"
    cx, cy = cores[0]
    # Both regions are same size, so either centre is valid
    assert (abs(cx - 20.0) < 5.0 and abs(cy - 40.0) < 5.0) or (
        abs(cx - 80.0) < 5.0 and abs(cy - 40.0) < 5.0
    ), f"core ({cx:.1f},{cy:.1f}) not near either lobe centre"


def test_find_deepest_cores_single_point_for_small_pocket():
    """A pocket smaller than step_over still returns its centroid."""
    # 10x10 rect, tool offset 5 → 0x0 (collapses), so valid area is just
    # whatever offset_contour_group returns for -5
    boundary = [(0, 0), (10, 0), (10, 10), (0, 10)]
    area = offset_contour_group(boundary, [], -5.0, join_style=JoinStyle.ROUND)
    if area:
        # If any valid area remains, it should collapse in one step
        cores = find_deepest_cores(area, step_over=100.0)
        # Returns the original area centroid since it collapses immediately
        assert len(cores) == len(area)


# ── compute_inset_region ──────────────────────────────────────────


def test_compute_inset_region_simple():
    """Boundary inset by radius produces a smaller region."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    region, area = compute_inset_region(boundary, 5.0, [])
    assert len(region) >= 1
    assert area > 0
    orig_area = poly_area(boundary)
    assert area < orig_area, (
        f"inset area {area:.1f} should be < {orig_area:.1f}"
    )


def test_compute_inset_region_with_obstacle():
    """An obstacle changes the region — at least one polygon returned."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    obstacle = [(40, 30), (60, 30), (60, 50), (40, 50)]
    region_w_obs, area_w_obs = compute_inset_region(boundary, 5.0, [obstacle])
    assert len(region_w_obs) >= 1
    assert area_w_obs > 0


def test_compute_inset_region_large_radius_collapses():
    """A radius larger than half the boundary extent collapses the region."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    region, area = compute_inset_region(boundary, 200.0, [])
    assert area == 0.0


def test_compute_inset_region_zero_radius():
    """Zero radius returns the original boundary."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    region, area = compute_inset_region(boundary, 0.0, [])
    assert len(region) >= 1
    orig = poly_area(boundary)
    assert abs(area - orig) < 1.0


def test_compute_inset_region_multiple_obstacles():
    """Multiple obstacles produce valid regions."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    obs1 = [(20, 20), (40, 20), (40, 40), (20, 40)]
    obs2 = [(120, 60), (140, 60), (140, 80), (120, 80)]
    region, area = compute_inset_region(boundary, 5.0, [obs1, obs2])
    assert len(region) >= 1
    assert area > 0


def test_compute_inset_region_empty_boundary():
    """Empty boundary produces zero area."""
    region, area = compute_inset_region([], 5.0, [])
    assert area == 0.0


def test_compute_inset_region_negative_radius():
    """Negative radius (expansion) works — area increases."""
    boundary = [(0, 0), (10, 0), (10, 10), (0, 10)]
    region, area = compute_inset_region(boundary, -2.0, [])
    assert len(region) >= 1
    orig = poly_area(boundary)
    assert area > orig, f"expanded area {area:.1f} should be > {orig:.1f}"


def test_compute_inset_region_rejects_collapsed():
    """Radius larger than half the minimum bbox extent returns empty."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    region, area = compute_inset_region(boundary, 200.0, [])
    assert area == 0.0


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


def test_inset_with_large_radius_returns_empty():
    """Inset larger than half the extent produces no valid region."""
    region, area = compute_inset_region(_rect(0, 0, 10, 10), 10, [])
    assert len(region) == 0 or area == 0.0


def test_inset_join_style_round_corners():
    """Inset now uses Round — expect arc vertices at concave corners."""
    # Notched rectangle has 3 concave (270°) corners
    boundary = [
        (0, 0),
        (60, 0),
        (60, 20),
        (40, 20),
        (40, 40),
        (60, 40),
        (60, 60),
        (0, 60),
    ]
    region, _ = compute_inset_region(boundary, 3, [])
    poly = region[0]
    n = len(poly)
    assert n > 20, f"expected Round >20 vertices for notched rect, got {n}"
