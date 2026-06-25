"""Tests for ClearedArea."""

import math
import random

from raygeo.ops.area import ClearedArea


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


# ── Batch step expansion ──


def test_begin_commit_step_batch_empty():
    ca = ClearedArea()
    ca.begin_step_batch()
    ca.commit_step_batch()
    assert ca.total_area() == 0.0


def test_step_batch_single_segment():
    """A single batched step should match the unbatched expand_step."""
    ca1 = ClearedArea()
    ca1.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    ca1.expand_step((5, 10), (5, 15), 3.0)

    ca2 = ClearedArea()
    ca2.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    ca2.begin_step_batch()
    ca2.expand_step_batched((5, 10), (5, 15), 3.0)
    ca2.commit_step_batch()

    assert abs(ca1.total_area() - ca2.total_area()) < 0.01


def test_step_batch_empty_commit_is_noop():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    area_before = ca.total_area()
    ca.begin_step_batch()
    ca.commit_step_batch()
    assert abs(ca.total_area() - area_before) < 0.01


def test_step_batch_multiple_accumulates():
    """Calling expand_step_batched 3 times then commit should be larger."""
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    ca.begin_step_batch()
    ca.expand_step_batched((5, 10), (5, 12), 3.0)
    ca.expand_step_batched((5, 12), (5, 14), 3.0)
    ca.expand_step_batched((5, 14), (5, 16), 3.0)
    ca.commit_step_batch()
    assert ca.total_area() > 100.0 + 0.01


# ── new / empty / len / is_empty / total_area ──


def test_new_cleared_area_is_empty():
    ca = ClearedArea()
    assert ca.is_empty()
    assert len(ca) == 0
    assert ca.total_area() == 0.0


def test_after_add_is_not_empty():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    assert not ca.is_empty()
    assert len(ca) >= 1
    assert ca.total_area() > 0.0


# ── signed_boundary_distance ──


def test_signed_boundary_distance_inside():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    d = ca.signed_boundary_distance(5, 5)
    assert d < 0  # inside cleared


def test_signed_boundary_distance_outside():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    d = ca.signed_boundary_distance(50, 50)
    assert d > 0  # outside cleared


def test_signed_boundary_distance_on_boundary():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    d = ca.signed_boundary_distance(0, 5)
    assert abs(d) < 1e-6


# ── remaining ──


def test_remaining_empty_cleared():
    ca = ClearedArea()
    pocket = [(0, 0), (100, 0), (100, 100), (0, 100)]
    r = ca.remaining([pocket])
    assert len(r) == 1


def test_remaining_partial():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(10, 10), (90, 10), (90, 90), (10, 90)]])
    pocket = [(0, 0), (100, 0), (100, 100), (0, 100)]
    r = ca.remaining([pocket])
    assert len(r) >= 1


# ── expand ──


def test_expand_increases_area():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    area_before = ca.total_area()
    ca.expand([(5, 10), (5, 20)], 2.0)
    assert ca.total_area() > area_before


# ── expand_step ──


def test_expand_step_increases_area():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    area_before = ca.total_area()
    ca.expand_step((5, 10), (5, 15), 3.0)
    assert ca.total_area() > area_before


# ── query_window ──


def test_query_window_returns_fragments():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    frags = ca.query_window((-5, -5, 15, 15))
    assert len(frags) >= 1


def test_query_window_outside_bbox():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    frags = ca.query_window((100, 100, 200, 200))
    assert len(frags) == 0


# ── point_engagement ──


def test_point_engagement_inside_cleared():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    angle, _, _ = ca.point_engagement((5, 5), 5.0)
    assert angle < math.pi  # inside cleared → low engagement


def test_point_engagement_outside_cleared():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    angle, _, _ = ca.point_engagement((50, 50), 5.0)
    assert angle > math.pi  # far outside → high engagement


# ── path_engagement ──


def test_path_engagement_empty():
    ca = ClearedArea()
    assert ca.path_engagement([], 5.0) == []


def test_path_engagement_returns_results():
    ca = ClearedArea()
    ca.add_cleared_polygons([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    results = ca.path_engagement([(5, 5), (50, 50)], 5.0)
    assert len(results) == 2
    for angle, area, depth in results:
        assert isinstance(angle, float)
        assert isinstance(area, float)
        assert isinstance(depth, float)


# ── Local update strategy ──────────────────────────────────────


def _segments():
    """Generate a semi-random sequence of (prev, next, radius) for testing."""
    segs = []
    for i in range(200):
        a = i * 0.7
        segs.append(
            (
                (50 + a * math.cos(a * 0.3), 50 + a * math.sin(a * 0.3)),
                (
                    50 + (a + 1.0) * math.cos((a + 1.0) * 0.3),
                    50 + (a + 1.0) * math.sin((a + 1.0) * 0.3),
                ),
                3.0,
            )
        )
    return segs


def test_local_equivalence():
    """Local and Global strategies produce identical fragments."""
    segs = _segments()

    ca_global = ClearedArea()
    for prev, next, r in segs:
        ca_global.begin_step_batch()
        ca_global.expand_step_batched(prev, next, r)
        ca_global.commit_step_batch()

    ca_local = ClearedArea()
    ca_local.set_update_strategy("local")
    for prev, next, r in segs:
        ca_local.begin_step_batch()
        ca_local.expand_step_batched(prev, next, r)
        ca_local.commit_step_batch()

    # Same total area (±0.1 %)
    ag = ca_global.total_area()
    al = ca_local.total_area()
    assert abs(al - ag) / max(ag, 1.0) < 0.001, (
        f"Global={ag:.1f} Local={al:.1f}"
    )


def test_compact():
    """compact_if_needed reduces vertex count with minimal area change."""
    random.seed(42)
    ca = ClearedArea()

    # Add some small polygons, then compact with a low threshold
    polys = []
    for _ in range(500):
        cx = random.uniform(0, 200)
        cy = random.uniform(0, 200)
        polys.append(
            P(
                (cx, cy),
                (cx + 2, cy),
                (cx + 2, cy + 2),
                (cx + 0.5, cy + 2),
            )
        )
    ca.add_cleared_polygons(polys)

    area_before = ca.total_area()
    v_before = sum(len(p) for p in ca.fragments())

    ca.compact_if_needed_threshold(tol=0.5, threshold=100)

    v_after = sum(len(p) for p in ca.fragments())
    area_after = ca.total_area()

    # Vertex count should drop
    assert v_after < v_before
    # Area change should be small
    if area_before > 0:
        assert abs(area_after - area_before) / area_before < 0.05
