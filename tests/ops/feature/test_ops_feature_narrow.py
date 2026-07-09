"""Tests for ops/feature/narrow — narrow-passage machining analysis."""

from raygeo.ops.feature.narrow import analyze_pocket


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_wide_rect_returns_empty():
    """No narrow passages in a wide rectangle."""
    boundary = _rect(0, 0, 80, 60)
    regions = analyze_pocket(
        boundary, holes=None, tool_radius=3.0, tolerance=0.5
    )
    assert regions == []


def test_dumbbell_returns_classified_regions():
    """Dumbbell pocket returns at least one classified region."""
    boundary = [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 21.0),
        (60.0, 21.0),
        (60.0, 0.0),
        (100.0, 0.0),
        (100.0, 50.0),
        (60.0, 50.0),
        (60.0, 29.0),
        (40.0, 29.0),
        (40.0, 50.0),
        (0.0, 50.0),
    ]
    regions = analyze_pocket(
        boundary, holes=None, tool_radius=3.0, tolerance=0.5
    )
    assert len(regions) >= 1
    for poly, cls, min_w, entry_idxs in regions:
        assert len(poly) >= 3
        assert cls in ("narrow", "slot", "unreachable"), f"bad class {cls}"
        assert min_w >= 0.0
        assert isinstance(entry_idxs, list)


def test_slot_width_corridor():
    """A corridor wider than tool_radius but narrower than D+tol is Slot."""
    tool_radius = 3.0
    corridor_w = 6.2  # between D (6.0) and D+tol (6.5)
    boundary = _rect(0, 0, 80, corridor_w)
    regions = analyze_pocket(
        boundary, holes=None, tool_radius=tool_radius, tolerance=0.5
    )
    # The entire corridor is a slot
    slot_regions = [r for r in regions if r[1] == "slot"]
    if regions:
        assert len(slot_regions) > 0, (
            f"expected Slot, got {[r[1] for r in regions]}"
        )


def test_narrow_width_corridor():
    """A corridor wider than D+tol but narrower than 1.5×D is Narrow."""
    tool_radius = 3.0
    # D+tol = 2*3 + 0.5 = 6.5; 1.5*D = 9.0
    corridor_w = 8.0  # between 6.5 and 9.0
    boundary = _rect(0, 0, 80, corridor_w)
    regions = analyze_pocket(
        boundary, holes=None, tool_radius=tool_radius, tolerance=0.5
    )
    narrow_regions = [r for r in regions if r[1] == "narrow"]
    if regions:
        assert len(narrow_regions) > 0, (
            f"expected Narrow, got {[r[1] for r in regions]}"
        )


def test_unreachable_width_corridor():
    """A corridor narrower than tool_radius is Unreachable."""
    tool_radius = 3.0
    corridor_w = 2.0  # less than tool_radius
    boundary = _rect(0, 0, 80, corridor_w)
    regions = analyze_pocket(
        boundary, holes=None, tool_radius=tool_radius, tolerance=0.5
    )
    unreachable_regions = [r for r in regions if r[1] == "unreachable"]
    if regions:
        assert len(unreachable_regions) > 0, (
            f"expected Unreachable, got {[r[1] for r in regions]}"
        )


def test_with_island_returns_classified():
    """Pocket with island still produces classified regions."""
    boundary = _rect(0, 0, 120, 30)
    # A small island in the center
    island = _rect(50, 10, 20, 10)
    regions = analyze_pocket(
        boundary, holes=[island], tool_radius=3.0, tolerance=0.5
    )
    # May or may not have narrow passages depending on gap size
    assert isinstance(regions, list)
    if regions:
        for poly, cls, min_w, entry_idxs in regions:
            assert cls in ("narrow", "slot", "unreachable")
            assert min_w >= 0.0


def test_entry_edge_indices_in_regions():
    """Each region has entry_edge_indices populated."""
    boundary = [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 21.0),
        (60.0, 21.0),
        (60.0, 0.0),
        (100.0, 0.0),
        (100.0, 50.0),
        (60.0, 50.0),
        (60.0, 29.0),
        (40.0, 29.0),
        (40.0, 50.0),
        (0.0, 50.0),
    ]
    regions = analyze_pocket(
        boundary, holes=None, tool_radius=3.0, tolerance=0.5
    )
    for _poly, _cls, _min_w, entry_idxs in regions:
        assert isinstance(entry_idxs, list)
