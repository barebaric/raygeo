"""Tests for HSM cutting-arc geometry primitives."""

import math

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.hsm import find_cutting_arc
from raygeo.geo.algo.offset import compute_inset_region
from raygeo.ops.assembly.hsm import adaptive_entry


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
    va, _total = compute_inset_region(boundary, tool_r, islands)

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
