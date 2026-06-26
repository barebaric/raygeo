"""Tests for cut_area crescent-area computation."""

from raygeo.ops.cut import cut_area


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
