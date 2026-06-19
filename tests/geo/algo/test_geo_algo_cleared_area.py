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
