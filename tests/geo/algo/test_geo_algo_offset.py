import pytest

from raygeo.geo import Geometry
from raygeo.geo.algo.offset import (
    concentric_offsets,
    find_deepest_cores,
    offset_contour_group,
)


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
    miter = offset_contour_group(poly, [], 1.0, join_style="miter")
    round_ = offset_contour_group(poly, [], 1.0, join_style="round")
    assert len(round_[0]) > len(miter[0])


def test_offset_contour_group_join_style_square():
    """Square join style should succeed without error."""
    poly = P((0, 0), (10, 0), (5, 10))
    result = offset_contour_group(poly, [], 1.0, join_style="square")
    assert len(result) >= 1


def test_offset_contour_group_invalid_join_style():
    """Invalid join_style should raise ValueError."""
    poly = P((0, 0), (10, 0), (5, 10))
    with pytest.raises(ValueError, match="invalid join_style"):
        offset_contour_group(poly, [], 1.0, join_style="nonexistent")


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
    area = offset_contour_group(boundary, [], -5.0, join_style="round")
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
    area = offset_contour_group(boundary, [], -5.0, join_style="round")
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
    area = offset_contour_group(boundary, [], -5.0, join_style="round")
    if area:
        # If any valid area remains, it should collapse in one step
        cores = find_deepest_cores(area, step_over=100.0)
        # Returns the original area centroid since it collapses immediately
        assert len(cores) == len(area)
