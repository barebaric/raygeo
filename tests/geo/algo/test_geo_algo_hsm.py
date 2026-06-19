"""Tests for HSM adaptive entry and wavefronts."""

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.hsm import adaptive_entry, adaptive_wavefronts


def test_adaptive_entry_wide_area_returns_path():
    """Wide pocket with no islands returns a non-empty 3D toolpath."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    path, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=1.0,
    )
    assert len(path) > 10
    assert all(len(p) == 3 for p in path)
    # First point is at safe_z (helix start)
    assert abs(path[0][2] - 2.0) < 0.01
    # Points descend and end at target_z
    assert abs(path[-1][2] - (-8.0)) < 0.01


def test_adaptive_entry_wide_returns_cleared_polygons():
    """Wide pocket returns at least one cleared polygon (the tool disk)."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    path, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=1.0,
    )
    assert len(cleared) >= 1
    # The cleared polygon should be a circle (at least 3 points)
    for poly in cleared:
        assert len(poly) >= 3


def test_adaptive_entry_wide_cleared_polygon_validates():
    """Cleared polygon from wide branch can be used as ClearedArea initial."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    path, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cleared)
    assert ca.total_area() > 0
    remaining = ca.remaining([boundary])
    assert len(remaining) >= 1


def test_adaptive_entry_tight_slot_returns_path():
    """Narrow slot (r_max < 1.5*tool_radius) returns a zigzag ramp path."""
    boundary = [(0, 0), (100, 0), (100, 16), (0, 16)]
    path, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=4.0,
        step_over=3.0,
        safe_z=2.0,
        target_z=-6.0,
        plunge_pitch=1.0,
    )
    assert len(path) > 2
    # Points descend from safe_z to target_z
    assert abs(path[0][2] - 2.0) < 0.01
    assert abs(path[-1][2] - (-6.0)) < 0.01


def test_adaptive_entry_tight_with_islands():
    """Tight slot with small islands still generates a path."""
    boundary = [(0, 0), (80, 0), (80, 20), (0, 20)]
    islands = [[(30, 5), (40, 5), (40, 15), (30, 15)]]
    path, cleared = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=4.0,
        step_over=3.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    assert len(path) > 2


def test_adaptive_entry_degenerate_pocket():
    """Degenerate pocket falls back to centroid and returns empty-ish path."""
    # Too small boundary — find_largest_circle will return None
    boundary = [(0, 0), (1, 0), (1, 1), (0, 1)]
    path, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=5.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    # Path may be empty since tool is larger than pocket
    assert isinstance(path, list)


def test_adaptive_entry_step_over_ratio():
    """Larger step_over produces fewer spiral points (fewer revolutions)."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    path1, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=2.0,
    )
    path2, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=4.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=2.0,
    )
    # Smaller step_over = more revolutions = more points
    assert len(path1) > len(path2)


def test_adaptive_entry_same_z_no_path():
    """safe_z == target_z produces no vertical path."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    path, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=0.0,
        target_z=0.0,
        plunge_pitch=1.0,
    )
    # No descent needed, but the spiral at Z=0 may still be generated
    if path:
        assert all(p[2] == 0.0 for p in path)


# ── adaptive_wavefronts ──────────────────────────────────────────


def test_adaptive_wavefronts_simple():
    """Basic wavefronts: starts from cleared disk and grows to fill pocket."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    path, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)

    paths = adaptive_wavefronts(
        ca,
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert len(paths) >= 1
    # Area should have grown from initial disk (~7500) toward valid (~14476)
    assert ca.total_area() > 10000
    # Each iteration's path should have 3D points
    for p in paths:
        assert all(len(pt) == 3 for pt in p)


def test_adaptive_wavefronts_step_over_larger():
    """Larger step-over → fewer iterations."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    path, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=4.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)

    paths = adaptive_wavefronts(
        ca,
        boundary,
        tool_radius=3.0,
        step_over=4.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert len(paths) >= 1


def test_adaptive_wavefronts_with_islands():
    """Wavefronts with islands still converge."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    path, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)

    paths = adaptive_wavefronts(
        ca,
        boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert len(paths) >= 1
    assert ca.total_area() > 5000


def test_adaptive_wavefronts_empty_cleared():
    """Wavefronts with empty cleared area returns empty toolpaths."""
    ca = ClearedArea()
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]

    paths = adaptive_wavefronts(
        ca,
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    # No initial cleared area → no iterations
    assert isinstance(paths, list)
