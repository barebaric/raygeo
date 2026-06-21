"""Tests for the Polylabel (pole of inaccessibility) algorithm."""

from raygeo.geo.algo.offset import offset_contour_group
from raygeo.geo.algo.polylabel import polylabel
from raygeo.geo.shape.polygon import JoinStyle, get_polygon_signed_area


def test_rectangle_centre():
    """Pole of a rectangle is its centre."""
    p = polylabel(
        [(0, 0), (100, 0), (100, 80), (0, 80)],
        holes=[],
        precision=0.1,
    )
    assert p is not None
    cx, cy = p
    assert abs(cx - 50.0) < 0.5
    assert abs(cy - 40.0) < 0.5


def test_l_shape():
    """Pole of an L-shape lies in the overlap region."""
    p = polylabel(
        [(0, 0), (100, 0), (100, 40), (40, 40), (40, 80), (0, 80)],
        holes=[],
        precision=0.5,
    )
    assert p is not None
    cx, cy = p
    assert 20.0 < cx < 35.0
    assert 20.0 < cy < 35.0


def test_triangle():
    """Pole of a triangle is near its incenter."""
    p = polylabel(
        [(0, 0), (100, 0), (50, 100)],
        holes=[],
        precision=0.5,
    )
    assert p is not None
    cx, cy = p
    assert abs(cx - 50.0) < 2.0
    assert 30.0 < cy < 32.0


def test_empty_polygon():
    """Empty polygon → None."""
    assert polylabel([], holes=[], precision=0.1) is None


def test_degenerate_polygon():
    """Degenerate polygon (single point) → None."""
    assert polylabel([(0, 0)], holes=[], precision=0.1) is None


def test_precision_improves_accuracy():
    """Higher precision (smaller value) gives a more accurate result."""
    poly = [(0, 0), (100, 0), (100, 100), (0, 100)]
    coarse = polylabel(poly, holes=[], precision=10.0)
    fine = polylabel(poly, holes=[], precision=0.1)
    assert coarse is not None and fine is not None
    err_coarse = max(abs(coarse[0] - 50.0), abs(coarse[1] - 50.0))
    err_fine = max(abs(fine[0] - 50.0), abs(fine[1] - 50.0))
    assert err_coarse >= err_fine, (
        f"coarse error {err_coarse:.4f} < fine error {err_fine:.4f}"
    )


def test_central_island():
    """Central-island pocket — pole in the ring, not in the island."""
    cb = [(0, 0), (100, 0), (100, 100), (0, 100)]
    cisl = [(35, 35), (65, 35), (65, 65), (35, 65)]
    area = offset_contour_group(cb, [cisl], -5.0, join_style=JoinStyle.Round)

    shell = None
    holes = []
    for p in area:
        if get_polygon_signed_area(p) >= 0:
            shell = p
        else:
            holes.append(p)

    assert shell is not None
    pole = polylabel(shell, holes=holes, precision=0.5)
    assert pole is not None
    cx, cy = pole
    assert not (35.0 < cx < 65.0 and 35.0 < cy < 65.0), (
        f"pole {pole} is inside the island"
    )


def test_multi_island_pocket():
    """Multi-island pocket returns pole in the largest region."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    isl1 = [(30, 20), (50, 20), (50, 40), (30, 40)]
    isl2 = [(110, 60), (130, 60), (130, 80), (110, 80)]
    area = offset_contour_group(
        boundary, [isl1, isl2], -5.0, join_style=JoinStyle.Round
    )

    assert area
    largest = max(area, key=get_polygon_signed_area)
    p = polylabel(largest, holes=[], precision=0.5)
    assert p is not None
    cx, cy = p
    assert 55.0 < cx < 105.0
    assert 5.0 < cy < 95.0
