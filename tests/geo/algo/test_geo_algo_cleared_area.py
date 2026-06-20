"""Tests for ClearedArea."""

from raygeo.geo.algo.cleared_area import ClearedArea


def P(*pts):
    return list(pts)


def test_add_cleared_polygons_empty():
    ca = ClearedArea()
    ca.add_cleared_polygons([])
    assert ca.total_area() == 0.0


def test_add_cleared_polygons_basic():
    ca = ClearedArea()
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.add_cleared_polygons([poly])
    assert ca.total_area() > 0.0


def test_add_cleared_polygons_union():
    ca = ClearedArea()
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
    ca.add_cleared_polygons([poly1, poly2])
    total = ca.total_area()
    assert total > 0.0
    # Union of overlapping squares < sum of individual areas
    assert total < 200.0


def test_add_cleared_polygons_remaining():
    """After registering cleared polygons, remaining subtracts them."""
    ca = ClearedArea()
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    cleared = P((10, 10), (90, 10), (90, 90), (10, 90))
    ca.add_cleared_polygons([cleared])
    remaining = ca.remaining([pocket])
    assert len(remaining) >= 1


# --- incorporate ---


def test_incorporate_empty():
    ca = ClearedArea()
    result = ca.incorporate([])
    assert result == []
    assert ca.total_area() == 0.0


def test_incorporate_returns_new_only():
    ca = ClearedArea()
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    result = ca.incorporate([poly])
    assert len(result) == 1
    assert ca.total_area() > 0.0


def test_incorporate_overlapping_returns_only_new():
    """Input overlapping existing cleared area returns only the new portion."""
    ca = ClearedArea()
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.add_cleared_polygons([poly])
    # Same poly again — no new area
    result = ca.incorporate([poly])
    assert result == []
    area_before = ca.total_area()
    # Slightly larger poly — only the outer ring is new
    larger = P((-2, -2), (12, -2), (12, 12), (-2, 12))
    result2 = ca.incorporate([larger])
    assert len(result2) >= 1
    assert ca.total_area() > area_before


def test_incorporate_disjoint_fast_path():
    ca = ClearedArea()
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.add_cleared_polygons([poly1])
    # Disjoint poly — should take the fast append path
    poly2 = P((100, 100), (110, 100), (110, 110), (100, 110))
    result = ca.incorporate([poly2])
    assert len(result) == 1
    assert ca.total_area() > 100.0


# --- frontier ---


def test_frontier_empty():
    ca = ClearedArea()
    f = ca.frontier(0.01)
    assert f == []


def test_frontier_merges_overlapping():
    ca = ClearedArea()
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
    ca.add_cleared_polygons([poly1, poly2])
    f = ca.frontier(0.01)
    # Frontier merges overlapping polygons — fewer fragments expected
    assert len(f) >= 1


def test_frontier_simplifies():
    ca = ClearedArea()
    # A poly with collinear points that should be simplified
    poly = P((0, 0), (5, 0), (10, 0), (10, 10), (0, 10))
    ca.add_cleared_polygons([poly])
    f = ca.frontier(0.5)
    assert len(f) == 1
    # Should have removed the collinear (5, 0) vertex
    assert len(f[0]) <= 4


# --- bites ---


def test_bites_empty_cleared():
    """No frontier to expand from — bites returns empty."""
    ca = ClearedArea()
    valid = [P((-50, -50), (50, -50), (50, 50), (-50, 50))]
    b = ca.bites(5.0, valid, 0.01)
    assert b == []


def test_bites_returns_positive_bites():
    ca = ClearedArea()
    ca.add_cleared_polygons([P((0, 0), (10, 0), (10, 10), (0, 10))])
    valid = [P((-50, -50), (50, -50), (50, 50), (-50, 50))]
    b = ca.bites(5.0, valid, 0.01)
    # Expanding a 10x10 by 5 gives material to remove
    assert len(b) >= 1


def test_bites_clipped_to_valid_area():
    ca = ClearedArea()
    ca.add_cleared_polygons([P((0, 0), (10, 0), (10, 10), (0, 10))])
    # Valid area is exactly the same as cleared — nothing to bite
    valid = [P((0, 0), (10, 0), (10, 10), (0, 10))]
    b = ca.bites(5.0, valid, 0.01)
    assert b == []
