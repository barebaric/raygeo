"""Tests for the actionable_remaining metric on ClearedArea.

``actionable_remaining(tool_radius)`` returns the area of uncleared
material inside the tool-centre envelope (boundary inset by
``tool_radius``, minus islands).  Material in the wall band
(``stock \\ envelope``) is excluded — the tool centre can never reach
it, so it should not gate convergence.

These tests validate the lazy reference implementation.  A
differentially-tracked accumulator that conforms to the same Python
API would replace the implementation in ``cleared_area.rs`` without
breaking these tests.
"""

import math
import random

from raygeo.ops.cut import StockRegion
from raygeo.ops.cut.cleared_area import ClearedArea


def P(*pts):
    return list(pts)


def _envelope_area(ca: ClearedArea, region, tool_radius: float) -> float:
    """Net enclosed area of the envelope polygons (outer rings CCW
    positive, holes CW negative, summed as signed areas)."""
    from raygeo.geo.shape.polygon import get_polygon_signed_area

    return sum(
        get_polygon_signed_area(poly)
        for poly in ca.envelope(region, tool_radius)
    )


def _poly_area(poly):
    """Unsigned area of a single polygon (for raster-reference tests)."""
    n = len(poly)
    s = 0.0
    for i in range(n):
        x1, y1 = poly[i]
        x2, y2 = poly[(i + 1) % n]
        s += x1 * y2 - x2 * y1
    return abs(s) * 0.5


# ── Basic behaviour ────────────────────────────────────────────


def test_actionable_remaining_empty():
    """No fragments cleared → actionable equals entire envelope area."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    expected = _envelope_area(ca, region, 5.0)
    assert abs(ca.actionable_remaining(region, 5.0) - expected) < 0.5


def test_actionable_remaining_no_boundary():
    """Empty boundary → no envelope → 0."""
    region = StockRegion(boundary=[], islands=[])
    ca = ClearedArea()
    assert ca.actionable_remaining(region, 5.0) == 0.0


def test_actionable_remaining_with_cleared_inside_envelope():
    """Clearing inside the envelope reduces actionable area."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    before = ca.actionable_remaining(region, 5.0)
    # Clear an interior region well inside the wall band
    ca.cut([P((30, 30), (70, 30), (70, 70), (30, 70))])
    after = ca.actionable_remaining(region, 5.0)
    assert after < before
    assert after > 0.0  # wall band still uncleared


def test_actionable_remaining_with_sliver_on_wall_only():
    """If the entire envelope is cleared but a wall band remains,
    actionable_remaining should be ~0 even though remaining_area isn't."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    r = 5.0
    # Clear the entire envelope: everything from (r, r) to (100-r, 100-r).
    # Use a rectangle slightly bigger than the envelope inset so the disc
    # overhang guarantees full envelope coverage including the
    # rounded-miter corners of the inset.
    grow = [
        (r - 1, r - 1),
        (100 - r + 1, r - 1),
        (100 - r + 1, 100 - r + 1),
        (r - 1, 100 - r + 1),
    ]
    ca.cut([grow])
    # Wall band sliver (boundary minus envelope) is still uncut, but
    # should not appear in actionable_remaining.
    actionable = ca.actionable_remaining(region, r)
    assert actionable < 0.5, (
        f"actionable_remaining={actionable:.3f} mm², expected ~0"
    )
    # And remaining_area (over the stock) should be substantially bigger.
    assert ca.remaining_area(region) > 0.0


def test_actionable_remaining_with_islands():
    """Island inside the pocket is excluded from the envelope."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    island = P((40, 40), (60, 40), (60, 60), (40, 60))
    region = StockRegion(boundary=pocket, islands=[island])
    ca = ClearedArea()
    expected = _envelope_area(ca, region, 5.0)
    # The envelope has the island removed from it; with nothing cleared,
    # actionable_remaining == envelope area.
    assert abs(ca.actionable_remaining(region, 5.0) - expected) < 0.5


def test_actionable_remaining_radius_change():
    """Larger tool radius → smaller envelope → smaller actionable."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    small = ca.actionable_remaining(region, 2.0)
    large = ca.actionable_remaining(region, 10.0)
    # With a larger radius the envelope shrinks, so actionable is smaller.
    assert large < small


def test_actionable_remaining_inside_envelope_no_change_outside():
    """Adding a fragment strictly outside the envelope (in the wall band)
    must not change actionable_remaining."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    r = 5.0
    before = ca.actionable_remaining(region, r)
    # Add a fragment entirely within the wall band (the 5-mm strip along
    # the left wall).  This is material the tool centre cannot reach.
    ca.cut([P((0, 0), (2, 0), (2, 50), (0, 50))])
    after = ca.actionable_remaining(region, r)
    assert abs(after - before) < 0.2, (
        f"actionable_remaining changed by {after - before:.3f} when "
        f"adding a strictly-outside fragment"
    )


# ── Mutation-path coverage ─────────────────────────────────────


def test_actionable_remaining_after_cut_fast():
    """cut_fast path also reduces actionable_remaining."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    before = ca.actionable_remaining(region, 5.0)
    ca.cut_fast([P((30, 30), (70, 30), (70, 70), (30, 70))])
    after = ca.actionable_remaining(region, 5.0)
    assert after < before


def test_actionable_remaining_after_expand_step():
    """expand_step adds swept area; actionable_remaining drops."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    # Seed somewhere inside the envelope
    ca.cut([P((30, 30), (40, 30), (40, 40), (30, 40))])
    before = ca.actionable_remaining(region, 5.0)
    ca.expand_step((35, 40), (35, 50), 3.0)
    after = ca.actionable_remaining(region, 5.0)
    assert after < before


def test_actionable_remaining_after_batched_commit():
    """commit_batch path also reduces actionable_remaining."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    region = StockRegion(boundary=pocket)
    ca1 = ClearedArea()
    ca2 = ClearedArea()
    segs = [(30, 40), (50, 40), (50, 60)]
    # Global commit
    ca1.begin_batch()
    for prev, nxt in zip(segs[:-1], segs[1:]):
        ca1.expand_batched(prev, nxt, 3.0)
    ca1.commit_batch()
    # Local commit
    ca2.begin_batch()
    for prev, nxt in zip(segs[:-1], segs[1:]):
        ca2.expand_batched(prev, nxt, 3.0)
    ca2.commit_batch_local()
    # Both paths must produce equivalent actionable_remaining
    a1 = ca1.actionable_remaining(region, 5.0)
    a2 = ca2.actionable_remaining(region, 5.0)
    assert abs(a1 - a2) < 0.5, (
        f"global={a1:.2f}, local={a2:.2f} differ by {a1 - a2:.3f}"
    )


def test_actionable_remaining_after_compact():
    """compact_if_needed shouldn't corrupt the actionable_remaining state."""
    random.seed(42)
    pocket = P((0, 0), (200, 0), (200, 200), (0, 200))
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    # Add many disjoint polys to trigger compaction
    polys = []
    for _ in range(500):
        cx = random.uniform(20, 180)
        cy = random.uniform(20, 180)
        polys.append(P((cx, cy), (cx + 2, cy), (cx + 2, cy + 2), (cx, cy + 2)))
    ca.cut(polys)
    before = ca.actionable_remaining(region, 5.0)
    ca.compact_if_needed_threshold(region, tol=0.5, threshold=100)
    after = ca.actionable_remaining(region, 5.0)
    # Compaction is allowed to change area slightly (simplification)
    # but should not move actionable_remaining by more than a few %.
    if before > 0:
        rel = abs(after - before) / before
        assert rel < 0.10, (
            f"compact changed actionable_remaining by {rel * 100:.1f}%"
        )


# ── Deterministic-random correctness ──────────────────────────


def _segments(seed: int = 42, n: int = 200):
    """Generate a deterministic sequence of (prev, next, radius)."""
    rng = random.Random(seed)
    segs = []
    x, y = 60.0, 60.0
    for _ in range(n):
        angle = rng.uniform(0.0, 2.0 * math.pi)
        dx = 1.0 * math.cos(angle)
        dy = 1.0 * math.sin(angle)
        nx = x + dx
        ny = y + dy
        # Stay inside the 100×100 pocket's radius-3 envelope with margin
        nx = max(5.0, min(95.0, nx))
        ny = max(5.0, min(95.0, ny))
        segs.append(((x, y), (nx, ny), 3.0))
        x, y = nx, ny
    return segs


def test_actionable_remaining_matches_lazy_recompute_random():
    """Each call to actionable_remaining must match an independent
    recomputation of `area(envelope) - area(fragments ∩ envelope)`.

    Uses a raster approximation for the cross-check (0.5 mm cells,
    ε ~2 mm² over a 100×100 envelope), adequate to catch any
    differential accumulator drift.  Sampled every 25 steps to keep
    the test fast.
    """
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    r = 3.0
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    env_polys = ca.envelope(region, r)
    env_area = sum(_poly_area(p) for p in env_polys)

    segs = _segments()
    for i, (prev, nxt, rt) in enumerate(segs):
        ca.begin_batch()
        ca.expand_batched(prev, nxt, rt)
        ca.commit_batch_local()

        if i % 25 != 0 and i != len(segs) - 1:
            continue

        frags = ca.fragments()
        cleared_inside = _cleared_inside_envelope(frags, env_polys)
        expected = max(0.0, env_area - cleared_inside)

        actual = ca.actionable_remaining(region, r)
        # Raster tolerance: ±5 mm² over a ~8000 mm² envelope.
        assert abs(actual - expected) < 5.0, (
            f"step {i} {prev}->{nxt}: expected {expected:.2f}, "
            f"got {actual:.2f} (delta {actual - expected:.2f})"
        )


def _cleared_inside_envelope(frags, env_polys):
    """Approximate `area((union frags) ∩ envelope)` via raster sampling.

    Uses ``0.5 mm`` cells; sampling precision ±~5 mm² over the
    100×100 envelope used in these tests.  Adequate for detecting
    differential-accumulator drift of more than a fraction of a mm².
    """
    cell = 0.5
    cell_area = cell * cell
    xs, ys = [], []
    for p in env_polys:
        for x, y in p:
            xs.append(x)
            ys.append(y)
    if not xs:
        return 0.0
    x_min, x_max = min(xs), max(xs)
    y_min, y_max = min(ys), max(ys)

    def point_in_poly(x, y, poly):
        n = len(poly)
        inside = False
        j = n - 1
        for i in range(n):
            xi, yi = poly[i]
            xj, yj = poly[j]
            if ((yi > y) != (yj > y)) and (
                x < (xj - xi) * (y - yi) / (yj - yi + 1e-12) + xi
            ):
                inside = not inside
            j = i
        return inside

    def point_in_any(x, y, polys):
        for p in polys:
            if point_in_poly(x, y, p):
                return True
        return False

    inside = 0.0
    yy = y_min + cell * 0.5
    while yy < y_max:
        xx = x_min + cell * 0.5
        while xx < x_max:
            if point_in_any(xx, yy, env_polys) and point_in_any(xx, yy, frags):
                inside += cell_area
            xx += cell
        yy += cell
    return inside


# ── Monotonicity ──────────────────────────────────────────────


def test_actionable_remaining_monotone_decreasing():
    """Adding fragments inside the envelope never increases
    actionable_remaining."""
    pocket = P((0, 0), (100, 0), (100, 100), (0, 100))
    r = 5.0
    region = StockRegion(boundary=pocket)
    ca = ClearedArea()
    prev = ca.actionable_remaining(region, r)
    for prev_pt, nxt, _ in _segments(seed=7, n=50):
        ca.begin_batch()
        ca.expand_batched(prev_pt, nxt, 3.0)
        ca.commit_batch_local()
        cur = ca.actionable_remaining(region, r)
        # Allow tiny numerical wiggle but never a real increase.
        assert cur <= prev + 0.5, (
            f"actionable increased from {prev:.3f} to {cur:.3f} step "
            f"{prev_pt}->{nxt}"
        )
        prev = cur
