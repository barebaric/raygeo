"""Tests for ClearedArea."""

import math

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


# --- fragments ---


def test_fragments_empty():
    ca = ClearedArea()
    assert ca.fragments() == []


def test_fragments_after_single_polygon():
    ca = ClearedArea()
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.add_cleared_polygons([poly])
    frags = ca.fragments()
    assert len(frags) == 1
    assert len(frags[0]) >= 4


def test_fragments_after_multiple_disjoint():
    ca = ClearedArea()
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    poly2 = P((100, 100), (110, 100), (110, 110), (100, 110))
    ca.add_cleared_polygons([poly1, poly2])
    frags = ca.fragments()
    assert len(frags) == 2


def test_fragments_merges_overlapping_add():
    """Overlapping polygons passed to add_cleared_polygons are merged."""
    ca = ClearedArea()
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
    ca.add_cleared_polygons([poly1, poly2])
    frags = ca.fragments()
    # overlapping squares union into a single fragment
    assert len(frags) == 1


def test_fragments_after_incorporate_disjoint():
    """Disjoint incorporate appends without merging."""
    ca = ClearedArea()
    ca.incorporate([P((0, 0), (10, 0), (10, 10), (0, 10))])
    ca.incorporate([P((100, 100), (110, 100), (110, 110), (100, 110))])
    assert len(ca.fragments()) == 2


def test_fragments_after_incorporate_overlapping():
    """Overlapping incorporate triggers a merge."""
    ca = ClearedArea()
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.incorporate([poly])
    larger = P((-2, -2), (12, -2), (12, 12), (-2, 12))
    ca.incorporate([larger])
    frags = ca.fragments()
    assert len(frags) == 1


def test_fragments_mixed_add_and_incorporate():
    """Fragments reflect total state after both add and incorporate."""
    ca = ClearedArea()
    ca.add_cleared_polygons([P((0, 0), (10, 0), (10, 10), (0, 10))])
    ca.incorporate([P((100, 100), (110, 100), (110, 110), (100, 110))])
    assert len(ca.fragments()) == 2


def test_fragments_vertices_format():
    """Each fragment vertex is an (x, y) pair of floats."""
    ca = ClearedArea()
    ca.add_cleared_polygons([P((0, 0), (10, 0), (10, 10), (0, 10))])
    frags = ca.fragments()
    for frag in frags:
        for v in frag:
            assert len(v) == 2
            assert isinstance(v[0], (int, float))
            assert isinstance(v[1], (int, float))


def test_fragments_min_vertex_count():
    """Each fragment polygon has at least 4 vertices."""
    ca = ClearedArea()
    ca.add_cleared_polygons([P((0, 0), (10, 0), (10, 10), (0, 10))])
    frags = ca.fragments()
    for frag in frags:
        assert len(frag) >= 4


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


# --- bite_in_direction ---


def test_bite_in_direction_empty_cleared():
    """No cleared area — returns all bites (no filtering)."""
    ca = ClearedArea()
    valid = [P((-50, -50), (50, -50), (50, 50), (-50, 50))]
    b = ca.bite_in_direction(5.0, valid, 0.01, (0, 0), 0.5)
    assert b == []


def test_bite_in_direction_filters_some():
    """Direction filter removes bites pointing away from target."""
    ca = ClearedArea()
    ca.add_cleared_polygons([P((0, 0), (10, 0), (10, 10), (0, 10))])
    valid = [P((-50, -50), (50, -50), (50, 50), (-50, 50))]

    all_bites = ca.bites(5.0, valid, 0.01)
    dir_bites = ca.bite_in_direction(5.0, valid, 0.01, (100, 0), 0.8)

    assert len(dir_bites) <= len(all_bites)


def test_bite_in_direction_wide_angle_returns_all():
    """max_angle >= π returns all bites (no filtering)."""
    ca = ClearedArea()
    ca.add_cleared_polygons([P((0, 0), (10, 0), (10, 10), (0, 10))])
    valid = [P((-50, -50), (50, -50), (50, 50), (-50, 50))]

    all_bites = ca.bites(5.0, valid, 0.01)
    dir_bites = ca.bite_in_direction(5.0, valid, 0.01, (100, 0), math.pi)

    assert len(dir_bites) == len(all_bites)


def test_bite_in_direction_narrow_angle():
    """Very narrow angle may return zero bites."""
    ca = ClearedArea()
    ca.add_cleared_polygons([P((0, 0), (10, 0), (10, 10), (0, 10))])
    valid = [P((-50, -50), (50, -50), (50, 50), (-50, 50))]

    dir_bites = ca.bite_in_direction(5.0, valid, 0.01, (100, 0), 0.01)
    assert isinstance(dir_bites, list)


# --- remaining_in_inset ---


def test_remaining_in_inset_empty_cleared():
    """Empty stored fragments returns the full inset region."""
    ca = ClearedArea()
    boundary = P((0, 0), (20, 0), (20, 20), (0, 20))
    result = ca.remaining_in_inset(boundary, [], 2.0)
    assert len(result) >= 1


def test_remaining_in_inset_with_stored_interior():
    """A central stored polygon leaves an uncovered ring around it."""
    ca = ClearedArea()
    stored = P((8, 8), (12, 8), (12, 12), (8, 12))
    ca.add_cleared_polygons([stored])
    boundary = P((0, 0), (20, 0), (20, 20), (0, 20))
    result = ca.remaining_in_inset(boundary, [], 2.0)
    assert len(result) >= 1


def test_remaining_in_inset_fully_covered():
    """When the entire inset region is covered, result has near-zero area."""
    ca = ClearedArea()
    boundary = P((0, 0), (20, 0), (20, 20), (0, 20))
    covered = P((-1, -1), (21, -1), (21, 21), (-1, 21))
    ca.add_cleared_polygons([covered])
    result = ca.remaining_in_inset(boundary, [], 2.0)
    total_area = 0.0
    for poly in result:
        n = len(poly)
        for i in range(n):
            x1, y1 = poly[i]
            x2, y2 = poly[(i + 1) % n]
            total_area += x1 * y2 - x2 * y1
    total_area = abs(total_area) / 2.0
    assert total_area < 5.0


def test_remaining_in_inset_includes_obstacles():
    """Obstacle polygons are included in the result."""
    ca = ClearedArea()
    boundary = P((0, 0), (30, 0), (30, 30), (0, 30))
    obstacles = [P((10, 10), (20, 10), (20, 20), (10, 20))]
    result = ca.remaining_in_inset(boundary, obstacles, 3.0)
    assert len(result) >= 1
    total_area = 0.0
    for poly in result:
        n = len(poly)
        for i in range(n):
            x1, y1 = poly[i]
            x2, y2 = poly[(i + 1) % n]
            total_area += x1 * y2 - x2 * y1
    total_area = abs(total_area) / 2.0
    assert total_area > 10.0
