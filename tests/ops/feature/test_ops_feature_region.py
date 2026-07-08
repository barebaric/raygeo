"""Tests for ops/feature/region — disconnected wide-region detector."""

from raygeo.geo.shape.polygon import get_signed_boundary_distance
from raygeo.ops.feature import region as _region

find_regions = _region.find_regions


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _dumbbell():
    """A dumbbell shape: two 30x30 lobes connected by a 20x5 corridor.

    Left lobe: x=0..30, y=0..30
    Right lobe: x=50..80, y=0..30
    Corridor: x=30..50, y=12.5..17.5
    """
    return [
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 12.5),
        (50.0, 12.5),
        (50.0, 0.0),
        (80.0, 0.0),
        (80.0, 30.0),
        (50.0, 30.0),
        (50.0, 17.5),
        (30.0, 17.5),
        (30.0, 30.0),
        (0.0, 30.0),
    ]


def test_region_rectangle():
    """40x40 rect with tool_radius=3 returns 1 region with r_max > 0."""
    boundary = _rect(-20, -20, 40, 40)
    regions = find_regions(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        tolerance=0.5,
    )
    assert len(regions) == 1, f"expected 1 region, got {len(regions)}"
    _polygon, area, _entry_pt, r_max = regions[0]
    assert r_max > 0, f"expected r_max > 0, got {r_max}"
    assert abs(area - 1600.0) < 1.0, f"area mismatch: {area:.2f} != 1600"


def test_region_dumbbell():
    """Dumbbell with narrow corridor: returns 2 regions, largest first."""
    boundary = _dumbbell()
    regions = find_regions(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        tolerance=0.5,
    )
    assert len(regions) == 2, f"expected 2 regions, got {len(regions)}"
    # Sorted largest first
    _p1, area1, _e1, _r1 = regions[0]
    _p2, area2, _e2, _r2 = regions[1]
    assert area1 >= area2, "regions not sorted by area descending"
    for _poly, area, _ep, r_max in regions:
        assert r_max > 0, f"expected r_max > 0, got {r_max}"
        assert area > 800.0, f"lobe area {area:.2f} < 800"


def test_region_slot_only():
    """Slot narrow enough to be entirely classified as a passage.

    Width 7 mm < 1.5 * 2 * 3 = 9 mm -> narrow.  The polygon difference
    should yield no wide regions.
    """
    boundary = _rect(0, 0, 40, 7)
    regions = find_regions(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        tolerance=0.5,
    )
    assert len(regions) == 0, (
        f"expected 0 regions for narrow slot, got {len(regions)}"
    )


def test_region_island():
    """40x40 rect with central 10x10 island: returns 1 ring region."""
    boundary = _rect(-20, -20, 40, 40)
    island = _rect(-5, -5, 10, 10)
    regions = find_regions(
        boundary=boundary,
        islands=[island],
        tool_radius=3.0,
        tolerance=0.5,
    )
    assert len(regions) == 1, f"expected 1 region, got {len(regions)}"
    _poly, area, _ep, r_max = regions[0]
    assert r_max > 0, f"expected r_max > 0, got {r_max}"
    # Area with island subtracted ~ 1600 - 100 = 1500 (minus erode)
    assert area > 1400.0, f"ring area {area:.2f} too small"
    # r_max should be smaller than the no-island case
    no_island = find_regions(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        tolerance=0.5,
    )
    _np, _na, _ne, no_r_max = no_island[0]
    assert r_max < no_r_max, (
        f"r_max with island {r_max} >= without island {no_r_max}"
    )


def test_region_entry_pt_inside():
    """Entry point of each region lies inside its polygon."""
    boundary = _dumbbell()
    regions = find_regions(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        tolerance=0.5,
    )
    for poly, _area, entry_pt, _r_max in regions:
        dist = get_signed_boundary_distance(entry_pt, [poly])
        assert dist < 0, (
            f"entry_pt {entry_pt} not inside polygon (dist={dist:.3f})"
        )


def test_region_determinism():
    """Same input twice produces identical results."""
    boundary = _dumbbell()
    regions1 = find_regions(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        tolerance=0.5,
    )
    regions2 = find_regions(
        boundary=boundary,
        islands=None,
        tool_radius=3.0,
        tolerance=0.5,
    )
    assert len(regions1) == len(regions2), "different region counts"
    for r1, r2 in zip(regions1, regions2):
        poly1, area1, ep1, rm1 = r1
        poly2, area2, ep2, rm2 = r2
        assert abs(area1 - area2) < 1e-10, f"areas differ: {area1} vs {area2}"
        assert abs(rm1 - rm2) < 1e-10, f"r_max differ: {rm1} vs {rm2}"
        assert abs(ep1[0] - ep2[0]) < 1e-10 and abs(ep1[1] - ep2[1]) < 1e-10, (
            f"entry points differ: {ep1} vs {ep2}"
        )
        assert len(poly1) == len(poly2), "polygon vertex counts differ"
