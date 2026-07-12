"""Tests for cut_area crescent-area computation."""

import math

from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.crescent import cut_area


def test_cut_area_basic():
    """Stepping forward produces positive crescent area."""
    c1 = (4.0, 5.0)
    c2 = (8.0, 5.0)
    r = 3.0
    total, left = cut_area(c1, c2, r, [], [])
    assert total > 0.0
    # Moving right — left side should contain some area
    assert left > 0.0


def test_cut_area_coincident_is_zero():
    """Identical centres produce zero area."""
    c = (0.0, 0.0)
    r = 3.0
    total, _ = cut_area(c, c, r, [], [])
    assert total == 0.0


def test_cut_area_fragments_reduce():
    """A fragment covering the crescent reduces the area."""
    c1 = (4.0, 5.0)
    c2 = (8.0, 5.0)
    r = 3.0
    total_no_frag, _ = cut_area(c1, c2, r, [], [])
    cleared = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]
    total_frag, _ = cut_area(c1, c2, r, cleared, [])
    assert total_frag < total_no_frag


def test_cut_area_fragments_distant_no_effect():
    """A fragment far from the step has no effect."""
    c1 = (4.0, 5.0)
    c2 = (8.0, 5.0)
    r = 3.0
    total_no_frag, _ = cut_area(c1, c2, r, [], [])
    distant = [[(50.0, 50.0), (52.0, 50.0), (52.0, 52.0), (50.0, 52.0)]]
    total_distant, _ = cut_area(c1, c2, r, distant, [])
    assert abs(total_distant - total_no_frag) < 1e-9


def test_cut_area_valid_area_clips():
    """A tight valid_area reduces the crescent."""
    c1 = (4.0, 5.0)
    c2 = (8.0, 5.0)
    r = 3.0
    total_full, _ = cut_area(c1, c2, r, [], [])
    # Valid area only covers the left half of the crescent
    valid = [
        [
            (c1[0] - r, c1[1] - r),
            (c1[0], c1[1] - r),
            (c1[0], c1[1] + r),
            (c1[0] - r, c1[1] + r),
        ]
    ]
    total_clipped, left_clipped = cut_area(c1, c2, r, [], valid)
    assert total_clipped < total_full


def test_cut_area_returns_tuple():
    """Returns a (total, left) pair of floats."""
    c1 = (0.0, 0.0)
    c2 = (5.0, 0.0)
    r = 3.0
    result = cut_area(c1, c2, r, [], [])
    assert len(result) == 2
    total, left = result
    assert isinstance(total, float)
    assert isinstance(left, float)


def test_cut_area_step_backward():
    """A backward step also produces a crescent."""
    c1 = (8.0, 5.0)
    c2 = (4.0, 5.0)
    r = 3.0
    total, _ = cut_area(c1, c2, r, [], [])
    assert total > 0.0


# ── Analytical helpers ──────────────────────────────────────────────


def _lens_area(d: float, R: float) -> float:
    """Lens area: two circles radius R, centres distance d."""
    if d >= 2.0 * R:
        return 0.0
    if d <= 0.0:
        return math.pi * R * R
    return 2.0 * R * R * math.acos(d / (2.0 * R)) - (d / 2.0) * math.sqrt(
        4.0 * R * R - d * d
    )


def _crescent_area(d: float, R: float) -> float:
    """Crescent: disk(c2) minus overlap with disk(c1) at distance d."""
    return math.pi * R * R - _lens_area(d, R)


# ── Determinism ────────────────────────────────────────────────────


def test_cut_area_determinism_no_fragments():
    """Identical inputs always produce identical output (no fragments)."""
    c1, c2, r = (0.0, 0.0), (4.0, 0.0), 3.0
    first = cut_area(c1, c2, r, [], [])
    for _ in range(100):
        result = cut_area(c1, c2, r, [], [])
        assert result == first, f"Non-deterministic: {result} != {first}"


def test_cut_area_determinism_with_fragments():
    """Identical inputs always produce identical output (with fragments)."""
    c1, c2, r = (0.0, 0.0), (4.0, 0.0), 5.0
    frags = [[(-10, -10), (10, -10), (10, 2), (-10, 2)]]
    first = cut_area(c1, c2, r, frags, [])
    for _ in range(100):
        result = cut_area(c1, c2, r, frags, [])
        assert result == first


def test_cut_area_determinism_via_cleared_area():
    """ClearedArea.cut_area is deterministic across repeated calls."""
    ca = ClearedArea()
    ca.cut([[(-20, -20), (20, -20), (20, 0), (-20, 0)]])
    first = ca.cut_area((0, -1), (4, -1), 5.0)
    for _ in range(50):
        result = ca.cut_area((0, -1), (4, -1), 5.0)
        assert result == first


# ── Continuity ─────────────────────────────────────────────────────


def test_cut_area_continuity_no_fragments():
    """Small perturbation of c2 produces small change in area."""
    c1 = (0.0, 0.0)
    r = 5.0
    base = cut_area(c1, (4.0, 0.0), r, [], [])[0]
    for eps in [0.001, 0.01, 0.1]:
        for direction in [(eps, 0), (-eps, 0), (0, eps), (0, -eps)]:
            c2 = (4.0 + direction[0], 0.0 + direction[1])
            val = cut_area(c1, c2, r, [], [])[0]
            delta = abs(val - base)
            assert delta < eps * 50.0, (
                f"Discontinuity at eps={eps}: delta={delta:.4f} "
                f"(base={base:.4f}, val={val:.4f})"
            )


def test_cut_area_continuity_near_fragment_edge():
    """Area changes smoothly as c2 crosses a fragment boundary."""
    c1 = (0.0, 0.0)
    r = 5.0
    frags = [[(-20, -20), (20, -20), (20, 0), (-20, 0)]]
    results = []
    for i in range(-20, 21):
        dy = i * 0.01
        c2 = (4.0, dy)
        total, _ = cut_area(c1, c2, r, frags, [])
        results.append(total)
    for i in range(1, len(results)):
        delta = abs(results[i] - results[i - 1])
        assert delta < 1.0, (
            f"Discontinuity at step {i} (dy={i * 0.01:.2f}): delta={delta:.4f}"
        )


# ── Concave fragment false-zero (Bug 1) ────────────────────────────


def test_cut_area_concave_fragment_not_enclosing():
    """A concave fragment whose AABB contains the disk but does not
    actually enclose it must NOT cause a false zero.

    L-shape polygon:
        (0,0) → (20,0) → (20,10) → (10,10) → (10,20) → (0,20)

    Disk at (8, 8) r=4:
      - Centre (8,8) is inside the L-shape (bottom rectangle).
      - Disk AABB (4,4)–(12,12) is inside the L-shape AABB (0,0)–(20,20).
      - BUT the top-right corner of the disk (near 12,12) protrudes
        outside the L-shape (x>10 and y>10 is exterior).
    """
    l_shape = [
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0, 10.0),
        (10.0, 10.0),
        (10.0, 20.0),
        (0.0, 20.0),
    ]
    c1 = (4.0, 8.0)
    c2 = (8.0, 8.0)
    r = 4.0
    total, _ = cut_area(c1, c2, r, [l_shape], [])
    assert total > 0.0, (
        "Concave fragment falsely reported as enclosing disk "
        "(does_polygon_enclose_circle bug)"
    )


# ── Analytical crescent comparison ─────────────────────────────────


def test_cut_area_analytical_crescent():
    """With no fragments, cut_area matches the analytical crescent formula."""
    R = 5.0
    for d in [1.0, 2.0, 3.0, 5.0, 8.0, 9.5]:
        c1 = (0.0, 0.0)
        c2 = (d, 0.0)
        total, _ = cut_area(c1, c2, R, [], [])
        expected = _crescent_area(d, R)
        rel_err = abs(total - expected) / max(expected, 1e-9)
        assert rel_err < 0.01, (
            f"d={d}: cut_area={total:.4f}, expected={expected:.4f}, "
            f"rel_err={rel_err:.4f}"
        )


def test_cut_area_analytical_crescent_various_radii():
    """Crescent formula holds across different radii."""
    for R in [1.0, 2.5, 5.0, 10.0]:
        d = R * 0.5
        total, _ = cut_area((0, 0), (d, 0), R, [], [])
        expected = _crescent_area(d, R)
        rel_err = abs(total - expected) / max(expected, 1e-9)
        assert rel_err < 0.01, (
            f"R={R}: cut_area={total:.4f}, expected={expected:.4f}"
        )


# ── Correct short-circuit: fragment fully encloses disk ────────────


def test_cut_area_fragment_fully_encloses():
    """A large convex fragment covering the c2 disk yields near-zero area."""
    c1 = (4.0, 5.0)
    c2 = (8.0, 5.0)
    r = 3.0
    huge_frag = [[(-100, -100), (100, -100), (100, 100), (-100, 100)]]
    total, _ = cut_area(c1, c2, r, huge_frag, [])
    assert total < 0.01, (
        f"Expected ~0 area (disk fully enclosed), got {total:.4f}"
    )


# ── Batch invisibility ─────────────────────────────────────────────


def test_cut_area_batch_invisibility():
    """cut_area must not see uncommitted batch expansions."""
    ca = ClearedArea()
    ca.cut([[(-20, -20), (20, -20), (20, 0), (-20, 0)]])
    c1, c2, r = (0.0, -1.0), (0.0, 3.0), 5.0

    area_before = ca.cut_area(c1, c2, r)

    ca.begin_batch()
    ca.expand_batched(c1, c2, r)
    area_during_batch = ca.cut_area(c1, c2, r)
    assert abs(area_during_batch - area_before) < 1e-9, (
        f"cut_area changed during batch: before={area_before:.4f}, "
        f"during={area_during_batch:.4f}"
    )

    ca.commit_batch()
    area_after_commit = ca.cut_area(c1, c2, r)
    assert area_after_commit < area_before, (
        f"cut_area should decrease after commit: before={area_before:.4f}, "
        f"after={area_after_commit:.4f}"
    )
