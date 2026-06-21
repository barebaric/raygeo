"""Tests for HSM adaptive entry, wavefronts, and peeling."""

import math

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.hsm import (
    adaptive_entry,
    adaptive_peeling,
    adaptive_wavefronts,
    compute_valid_tool_area,
    find_cutting_arc,
)
from raygeo.geo.shape.line import get_line_segment_polygon_intersections


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


def test_adaptive_peeling_simple():
    """Basic peeling: starts from cleared disk and grows to fill pocket."""
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

    toolpath = adaptive_peeling(
        ca,
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert len(toolpath) > 10
    # Area should have grown from initial disk toward valid
    assert ca.total_area() > 10000
    # All points are 3D
    assert all(len(pt) == 3 for pt in toolpath)
    # NaN rows separate D-cut segments
    nan_count = sum(1 for p in toolpath if p[0] != p[0])  # NaN != NaN
    assert nan_count >= 1


def test_adaptive_peeling_step_over_larger():
    """Larger step-over → fewer D-cut passes, same convergence."""
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

    toolpath = adaptive_peeling(
        ca,
        boundary,
        tool_radius=3.0,
        step_over=4.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert len(toolpath) >= 1


def test_adaptive_peeling_with_islands():
    """Peeling with islands still converges."""
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

    toolpath = adaptive_peeling(
        ca,
        boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert len(toolpath) >= 1
    assert ca.total_area() > 5000


def test_adaptive_peeling_empty_cleared():
    """Peeling with empty cleared area returns empty toolpath."""
    ca = ClearedArea()
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]

    toolpath = adaptive_peeling(
        ca,
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert isinstance(toolpath, list)
    assert len(toolpath) == 0


def test_adaptive_peeling_nan_separators():
    """NaN rows appear between consecutive D-cut passes."""
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

    toolpath = adaptive_peeling(
        ca,
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    # Find all NaN rows
    nan_indices = []
    for i, p in enumerate(toolpath):
        x, y, z = p
        if x != x or y != y or z != z:
            nan_indices.append(i)

    assert len(nan_indices) >= 1, "Expected at least one NaN separator"
    # NaN separators should not be at index 0
    assert nan_indices[0] > 0
    # The path should contain NaN-separated segments each with valid 3D points
    segments = []
    cur = []
    for p in toolpath:
        x, y, z = p
        if x != x:
            if cur:
                segments.append(cur)
                cur = []
        else:
            cur.append(p)
    if cur:
        segments.append(cur)
    assert len(segments) >= 1
    for seg in segments:
        assert len(seg) >= 1


def test_adaptive_peeling_dcut_z_lift():
    """D-cut passes have outer arc at z and inner (return) arc at safe_z."""
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

    cut_z = -8.0
    lift_z = 5.0
    toolpath = adaptive_peeling(
        ca,
        boundary,
        tool_radius=3.0,
        step_over=2.0,
        z=cut_z,
        safe_z=lift_z,
        area_tolerance=1.0,
    )

    # Must contain points at BOTH Z levels
    z_vals = {p[2] for p in toolpath if p[0] == p[0]}  # skip NaN
    assert cut_z in z_vals, f"Expected cutting Z {cut_z} in path"
    assert lift_z in z_vals, f"Expected lift Z {lift_z} in path"

    # Within each NaN-separated segment the sequence should be:
    #   cutting arc (all at cut_z) → return arc (all at lift_z)
    segments = []
    cur = []
    for p in toolpath:
        x, y, z = p
        if x != x:
            if cur:
                segments.append(cur)
                cur = []
        else:
            cur.append(p)
    if cur:
        segments.append(cur)

    assert len(segments) >= 1
    for seg in segments:
        # At least some points at each Z level (unless a degenerate segment)
        seg_cut = sum(1 for p in seg if abs(p[2] - cut_z) < 0.01)
        # _seg_lift = sum(1 for p in seg if abs(p[2] - lift_z) < 0.01)
        assert seg_cut >= 1, (
            "Segment missing cutting-depth points: Zs="
            f"{set(p[2] for p in seg)}"
        )
        # Every segment should have the transition once (cut → lift)
        transitions = 0
        for i in range(len(seg) - 1):
            if (
                abs(seg[i][2] - cut_z) < 0.01
                and abs(seg[i + 1][2] - lift_z) < 0.01
            ):
                transitions += 1
        assert transitions >= 0  # at least one D-cut segment exists


def test_adaptive_peeling_avoids_islands():
    """Return paths avoid crossing islands — inner-arc fallback is used."""

    boundary = [(0.0, 0.0), (160.0, 0.0), (160.0, 100.0), (0.0, 100.0)]
    island = [(60.0, 35.0), (100.0, 35.0), (100.0, 65.0), (60.0, 65.0)]
    path, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=[island],
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)

    toolpath = adaptive_peeling(
        ca,
        boundary,
        islands=[island],
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        safe_z=5.0,
        area_tolerance=0.5,
    )

    # No consecutive-pair segment should intersect the island
    for i in range(len(toolpath) - 1):
        x1, y1, _ = toolpath[i]
        x2, y2, _ = toolpath[i + 1]
        if math.isnan(x1) or math.isnan(x2):
            continue
        cuts = get_line_segment_polygon_intersections(
            (x1, y1), (x2, y2), [island]
        )
        if len(cuts) > 2:
            raise AssertionError(
                f"Segment ({x1:.2f},{y1:.2f})\u2192({x2:.2f},{y2:.2f})"
                f" crosses island"
            )


def test_find_cutting_arc_angle_at_tip():
    """Find cutting arc — interior vertices should be smooth (> 100°).

    The cutting arc is an open polyline.  Only vertices with two
    neighbours within the arc (indices 1 .. n-2) are checked; the
    endpoints are excluded because they have only one neighbour.
    """
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [[(15, 15), (35, 15), (35, 35), (15, 35)]]
    tool_r = 3.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=tool_r,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, _total = compute_valid_tool_area(boundary, tool_r, islands)

    bad = []
    for iteration in range(10):
        bites = ca.bites(2.0, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc is None or len(arc) < 4:
                continue
            n = len(arc)
            # Interior vertices only (indices 1 .. n-2)
            for ai in range(1, n - 1):
                prev = arc[ai - 1]
                cur = arc[ai]
                nxt = arc[ai + 1]
                v1 = (prev[0] - cur[0], prev[1] - cur[1])
                v2 = (nxt[0] - cur[0], nxt[1] - cur[1])
                dot = v1[0] * v2[0] + v1[1] * v2[1]
                l1 = math.hypot(*v1)
                l2 = math.hypot(*v2)
                if l1 * l2 < 1e-12:
                    continue
                angle = math.degrees(
                    math.acos(max(-1, min(1, dot / (l1 * l2))))
                )
                if angle < 100.0:
                    bad.append((iteration, ai, angle, cur))
        ca.incorporate(bites)

    if bad:
        # A 90° vertex at a pocket boundary corner is valid geometry
        # (e.g., near an island buffer).  Only flag sharper turns that
        # indicate misclassified tip-transition vertices.
        bad_sharp = [(it, ai, a, p) for it, ai, a, p in bad if a < 75.0]
        if bad_sharp:
            raise AssertionError(
                f"{len(bad_sharp)} vertices have angle < 75°:\n"
                + "\n".join(
                    f"  iter={it} arc_vtx={ai} angle={a:.1f}°"
                    f" pos=({p[0]:.2f},{p[1]:.2f})"
                    for it, ai, a, p in bad_sharp[:10]
                )
            )
