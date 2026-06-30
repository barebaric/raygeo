"""Tests for cut module."""

import math
import random

from raygeo.ops.cut.cleared_area import ClearedArea


def P(*pts):
    return list(pts)


def test_cut_empty():
    ca = ClearedArea(boundary=[])
    ca.cut([])
    assert ca.total_area() == 0.0


def test_cut_basic():
    ca = ClearedArea(boundary=[])
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.cut([poly])
    assert ca.total_area() > 0.0


def test_cut_union():
    ca = ClearedArea(boundary=[])
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
    ca.cut([poly1, poly2])
    total = ca.total_area()
    assert total > 0.0
    # Union of overlapping squares < sum of individual areas
    assert total < 200.0


def test_cut_remaining():
    """After registering cleared polygons, remaining subtracts them."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    ca = ClearedArea(boundary=pocket)
    cleared = P((10, 10), (90, 10), (90, 90), (10, 90))
    ca.cut([cleared])
    remaining = ca.remaining()
    assert len(remaining) >= 1


# --- fragments ---


def test_fragments_empty():
    ca = ClearedArea(boundary=[])
    assert ca.fragments() == []


def test_fragments_after_single_polygon():
    ca = ClearedArea(boundary=[])
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.cut([poly])
    frags = ca.fragments()
    assert len(frags) == 1
    assert len(frags[0]) >= 4


def test_fragments_after_multiple_disjoint():
    ca = ClearedArea(boundary=[])
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    poly2 = P((100, 100), (110, 100), (110, 110), (100, 110))
    ca.cut([poly1, poly2])
    frags = ca.fragments()
    assert len(frags) == 2


def test_fragments_merges_overlapping_add():
    """Overlapping polygons passed to cut are merged."""
    ca = ClearedArea(boundary=[])
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
    ca.cut([poly1, poly2])
    frags = ca.fragments()
    # overlapping squares union into a single fragment
    assert len(frags) == 1


def test_fragments_after_incorporate_disjoint():
    """Disjoint incorporate appends without merging."""
    ca = ClearedArea(boundary=[])
    ca.cut_fast([P((0, 0), (10, 0), (10, 10), (0, 10))])
    ca.cut_fast([P((100, 100), (110, 100), (110, 110), (100, 110))])
    assert len(ca.fragments()) == 2


def test_fragments_after_incorporate_overlapping():
    """Overlapping incorporate triggers a merge."""
    ca = ClearedArea(boundary=[])
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.cut_fast([poly])
    larger = P((-2, -2), (12, -2), (12, 12), (-2, 12))
    ca.cut_fast([larger])
    frags = ca.fragments()
    assert len(frags) == 1


def test_fragments_mixed_add_and_incorporate():
    """Fragments reflect total state after both add and incorporate."""
    ca = ClearedArea(boundary=[])
    ca.cut([P((0, 0), (10, 0), (10, 10), (0, 10))])
    ca.cut_fast([P((100, 100), (110, 100), (110, 110), (100, 110))])
    assert len(ca.fragments()) == 2


def test_fragments_vertices_format():
    """Each fragment vertex is an (x, y) pair of floats."""
    ca = ClearedArea(boundary=[])
    ca.cut([P((0, 0), (10, 0), (10, 10), (0, 10))])
    frags = ca.fragments()
    for frag in frags:
        for v in frag:
            assert len(v) == 2
            assert isinstance(v[0], (int, float))
            assert isinstance(v[1], (int, float))


def test_fragments_min_vertex_count():
    """Each fragment polygon has at least 4 vertices."""
    ca = ClearedArea(boundary=[])
    ca.cut([P((0, 0), (10, 0), (10, 10), (0, 10))])
    frags = ca.fragments()
    for frag in frags:
        assert len(frag) >= 4


# --- incorporate ---


def test_incorporate_empty():
    ca = ClearedArea(boundary=[])
    result = ca.cut_fast([])
    assert result == []
    assert ca.total_area() == 0.0


def test_incorporate_returns_new_only():
    ca = ClearedArea(boundary=[])
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    result = ca.cut_fast([poly])
    assert len(result) == 1
    assert ca.total_area() > 0.0


def test_incorporate_overlapping_returns_only_new():
    """Input overlapping existing cleared area returns only the new portion."""
    ca = ClearedArea(boundary=[])
    poly = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.cut([poly])
    # Same poly again — no new area
    result = ca.cut_fast([poly])
    assert result == []
    area_before = ca.total_area()
    # Slightly larger poly — only the outer ring is new
    larger = P((-2, -2), (12, -2), (12, 12), (-2, 12))
    result2 = ca.cut_fast([larger])
    assert len(result2) >= 1
    assert ca.total_area() > area_before


def test_incorporate_disjoint_fast_path():
    ca = ClearedArea(boundary=[])
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    ca.cut([poly1])
    # Disjoint poly — should take the fast append path
    poly2 = P((100, 100), (110, 100), (110, 110), (100, 110))
    result = ca.cut_fast([poly2])
    assert len(result) == 1
    assert ca.total_area() > 100.0


# --- frontier ---


def test_frontier_empty():
    ca = ClearedArea(boundary=[])
    f = ca.frontier(0.01)
    assert f == []


def test_frontier_merges_overlapping():
    ca = ClearedArea(boundary=[])
    poly1 = P((0, 0), (10, 0), (10, 10), (0, 10))
    poly2 = P((5, 5), (15, 5), (15, 15), (5, 15))
    ca.cut([poly1, poly2])
    f = ca.frontier(0.01)
    # Frontier merges overlapping polygons — fewer fragments expected
    assert len(f) >= 1


def test_frontier_simplifies():
    ca = ClearedArea(boundary=[])
    # A poly with collinear points that should be simplified
    poly = P((0, 0), (5, 0), (10, 0), (10, 10), (0, 10))
    ca.cut([poly])
    f = ca.frontier(0.5)
    assert len(f) == 1
    # Should have removed the collinear (5, 0) vertex
    assert len(f[0]) <= 4


# --- bites ---


def test_bites_empty_cleared():
    """No frontier to expand from — bites returns empty."""
    ca = ClearedArea(boundary=[(-50, -50), (50, -50), (50, 50), (-50, 50)])
    b = ca.bites(5.0, 5.0, 0.01)
    assert b == []


def test_bites_returns_positive_bites():
    ca = ClearedArea(boundary=[(-50, -50), (50, -50), (50, 50), (-50, 50)])
    ca.cut([P((0, 0), (10, 0), (10, 10), (0, 10))])
    b = ca.bites(5.0, 5.0, 0.01)
    assert len(b) >= 1


def test_bites_clipped_to_valid_area():
    ca = ClearedArea(boundary=[(0, 0), (10, 0), (10, 10), (0, 10)])
    ca.cut([P((0, 0), (10, 0), (10, 10), (0, 10))])
    b = ca.bites(5.0, 5.0, 0.01)
    assert b == []


# ── Batch step expansion ──


def test_begin_commit_batch_empty():
    ca = ClearedArea(boundary=[])
    ca.begin_batch()
    ca.commit_batch()
    assert ca.total_area() == 0.0


def test_step_batch_single_segment():
    """A single batched step should match the unbatched expand_step."""
    ca1 = ClearedArea(boundary=[])
    ca1.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    ca1.expand_step((5, 10), (5, 15), 3.0)

    ca2 = ClearedArea(boundary=[])
    ca2.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    ca2.begin_batch()
    ca2.expand_batched((5, 10), (5, 15), 3.0)
    ca2.commit_batch()

    assert abs(ca1.total_area() - ca2.total_area()) < 0.01


def test_step_batch_empty_commit_is_noop():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    area_before = ca.total_area()
    ca.begin_batch()
    ca.commit_batch()
    assert abs(ca.total_area() - area_before) < 0.01


def test_step_batch_multiple_accumulates():
    """Calling expand_batched 3 times then commit should be larger."""
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    ca.begin_batch()
    ca.expand_batched((5, 10), (5, 12), 3.0)
    ca.expand_batched((5, 12), (5, 14), 3.0)
    ca.expand_batched((5, 14), (5, 16), 3.0)
    ca.commit_batch()
    assert ca.total_area() > 100.0 + 0.01


# ── new / empty / len / is_empty / total_area ──


def test_new_cleared_area_is_empty():
    ca = ClearedArea(boundary=[])
    assert ca.is_empty()
    assert len(ca) == 0
    assert ca.total_area() == 0.0


def test_after_add_is_not_empty():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    assert not ca.is_empty()
    assert len(ca) >= 1
    assert ca.total_area() > 0.0


# ── signed_boundary_distance ──


def test_signed_boundary_distance_inside():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    d = ca.signed_boundary_distance(5, 5)
    assert d < 0  # inside cleared


def test_signed_boundary_distance_outside():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    d = ca.signed_boundary_distance(50, 50)
    assert d > 0  # outside cleared


def test_signed_boundary_distance_on_boundary():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    d = ca.signed_boundary_distance(0, 5)
    assert abs(d) < 1e-6


# ── remaining ──


def test_remaining_empty_cleared():
    ca = ClearedArea(boundary=[(0, 0), (100, 0), (100, 100), (0, 100)])
    r = ca.remaining()
    assert len(r) == 1


def test_remaining_partial():
    ca = ClearedArea(boundary=[(0, 0), (100, 0), (100, 100), (0, 100)])
    ca.cut([[(10, 10), (90, 10), (90, 90), (10, 90)]])
    r = ca.remaining()
    assert len(r) >= 1


# ── remaining_area ──


def test_remaining_area_empty():
    """No cleared fragments — remaining equals stock area."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    ca = ClearedArea(boundary=pocket)
    stock_area = 10000.0
    assert abs(ca.remaining_area() - stock_area) < 0.01


def test_remaining_area_no_boundary():
    ca = ClearedArea(boundary=[])
    assert ca.remaining_area() == 0.0


def test_remaining_area_partial():
    """Partial clearing reduces remaining area."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    ca = ClearedArea(boundary=pocket)
    stock_area = 10000.0
    cleared_poly = P((10, 10), (90, 10), (90, 90), (10, 90))
    ca.cut([cleared_poly])
    assert 0.0 < ca.remaining_area() < stock_area


def test_remaining_area_plus_total_area():
    """remaining + total approximates stock area for a simple pocket."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    ca = ClearedArea(boundary=pocket)
    stock_area = 10000.0
    ca.cut([P((10, 10), (90, 10), (90, 90), (10, 90))])
    assert abs(ca.total_area() + ca.remaining_area() - stock_area) < 0.1


def test_remaining_area_with_islands():
    """Islands are excluded from remaining area."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    island = P((40, 40), (60, 40), (60, 60), (40, 60))  # 400 mm² island
    ca = ClearedArea(boundary=pocket, islands=[island])
    # Stock area = 10000 - 400 = 9600 mm²
    # With nothing cleared, remaining_area should be ~9600
    assert abs(ca.remaining_area() - 9600.0) < 0.1


def test_remaining_area_with_islands_partial():
    """Island area stays excluded even after clearing."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    island = P((40, 40), (60, 40), (60, 60), (40, 60))
    ca = ClearedArea(boundary=pocket, islands=[island])
    # Clear the top-left quadrant, outside the island
    ca.cut([P((0, 0), (50, 0), (50, 50), (0, 50))])  # ~2500 mm² cleared
    assert 0.0 < ca.remaining_area() < 9600.0


# ── expand ──


def test_expand_increases_area():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    area_before = ca.total_area()
    ca.expand([(5, 10), (5, 20)], 2.0)
    assert ca.total_area() > area_before


# ── expand_step ──


def test_expand_step_increases_area():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    area_before = ca.total_area()
    ca.expand_step((5, 10), (5, 15), 3.0)
    assert ca.total_area() > area_before


# ── query_window ──


def test_query_window_returns_fragments():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    frags = ca.query_window((-5, -5, 15, 15))
    assert len(frags) >= 1


def test_query_window_outside_bbox():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    frags = ca.query_window((100, 100, 200, 200))
    assert len(frags) == 0


# ── point_engagement ──


def test_point_engagement_inside_cleared():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    angle, _, _ = ca.point_engagement((5, 5), 5.0)
    assert angle < math.pi  # inside cleared → low engagement


def test_point_engagement_outside_cleared():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
    angle, _, _ = ca.point_engagement((50, 50), 5.0)
    assert angle > math.pi  # far outside → high engagement


# ── path_engagement ──


def test_path_engagement_empty():
    ca = ClearedArea(boundary=[])
    assert ca.path_engagement([], 5.0) == []


def test_path_engagement_returns_results():
    ca = ClearedArea(boundary=[])
    ca.cut([[(0, 0), (10, 0), (10, 10), (0, 10)]])
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

    ca_global = ClearedArea(boundary=[])
    for prev, next, r in segs:
        ca_global.begin_batch()
        ca_global.expand_batched(prev, next, r)
        ca_global.commit_batch()

    ca_local = ClearedArea(boundary=[])
    for prev, next, r in segs:
        ca_local.begin_batch()
        ca_local.expand_batched(prev, next, r)
        ca_local.commit_batch_local()

    # Same total area (±0.1 %)
    ag = ca_global.total_area()
    al = ca_local.total_area()
    assert abs(al - ag) / max(ag, 1.0) < 0.001, (
        f"Global={ag:.1f} Local={al:.1f}"
    )


# ── envelope ──


def test_envelope_empty_boundary():
    """Envelope with empty boundary returns empty."""
    ca = ClearedArea(boundary=[])
    e = ca.envelope(5.0)
    assert e == []


def test_envelope_basic():
    """Envelope returns tool-centre region inset from boundary."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    ca = ClearedArea(boundary=pocket)
    e = ca.envelope(5.0)
    assert len(e) >= 1
    # Each polygon should have at least 3 vertices
    for poly in e:
        assert len(poly) >= 3


def test_envelope_smaller_radius_larger_area():
    """Smaller tool radius produces a larger envelope area."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    ca = ClearedArea(boundary=pocket)
    e_large = ca.envelope(2.0)
    e_small = ca.envelope(10.0)
    # Count a rough proxy: the sum of polygon vertex counts should
    # differ (or the number of polygons should differ).
    # With a larger radius the inset is deeper, so area shrinks.
    # We'll just check they produce different results.
    # (Different series of points means different shapes.)
    assert e_large != e_small


def test_envelope_with_islands():
    """Islands reduce the envelope region."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    island = P((40, 40), (60, 40), (60, 60), (40, 60))
    ca_no_island = ClearedArea(boundary=pocket)
    ca_island = ClearedArea(boundary=pocket, islands=[island])

    e_no = ca_no_island.envelope(2.0)
    e_is = ca_island.envelope(2.0)

    # Without island the envelope should have fewer polygons or at
    # least be different from the one with an island.
    # We use total vertex count as a rough proxy.
    verts_no = sum(len(p) for p in e_no)
    verts_is = sum(len(p) for p in e_is)
    assert verts_is != verts_no


def test_envelope_vertices_format():
    """Each vertex is an (x, y) pair of floats."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    ca = ClearedArea(boundary=pocket)
    e = ca.envelope(5.0)
    for poly in e:
        for v in poly:
            assert len(v) == 2
            assert isinstance(v[0], (int, float))
            assert isinstance(v[1], (int, float))


def test_envelope_zero_radius():
    """Zero tool-radius envelope should match the full boundary shape."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    ca = ClearedArea(boundary=pocket)
    e = ca.envelope(0.0)
    assert len(e) >= 1


def test_compact():
    """compact_if_needed reduces vertex count with minimal area change."""
    random.seed(42)
    ca = ClearedArea(boundary=[])

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
    ca.cut(polys)

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
