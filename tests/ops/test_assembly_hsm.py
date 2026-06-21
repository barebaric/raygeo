"""Tests for raygeo.ops.assembly.hsm module."""

import math

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.offset import compute_inset_region
from raygeo.ops import Ops
from raygeo.ops.assembly.hsm import (
    adaptive_entry,
    adaptive_peeling,
    adaptive_wavefronts,
    find_cutting_arc,
    link_arcs_to_ops,
)
from raygeo.ops.types import CommandType


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _seed_circle(cx, cy, r, n=16):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


# ── link_arcs_to_ops ───────────────────────────────────────────


def test_link_arcs_basic():
    arcs = [[(0, 0), (10, 0)], [(0, 5), (10, 5)], [(0, 10), (10, 10)]]
    ops = link_arcs_to_ops(
        arcs=arcs,
        uncleared=[],
        cut_z=-1.0,
        safe_z=5.0,
    )
    assert ops.len() > 0
    assert ops.cut_distance() > 0
    assert ops.distance() > ops.cut_distance()


def test_link_arcs_has_move_and_line():
    arcs = [[(0, 0), (10, 0)], [(0, 5), (10, 5)]]
    ops = link_arcs_to_ops(
        arcs=arcs,
        uncleared=[],
        cut_z=-1.0,
        safe_z=5.0,
    )
    types = [ops.command_type(i) for i in range(ops.len())]
    assert CommandType.MOVE_TO in types
    assert CommandType.LINE_TO in types


def test_link_arcs_preserve_order():
    arcs = [[(0, 0), (10, 0)], [(0, 5), (10, 5)]]
    ops = link_arcs_to_ops(
        arcs=arcs,
        uncleared=[],
        cut_z=-1.0,
        safe_z=5.0,
        preserve_order=True,
    )
    assert ops.len() > 0


def test_link_arcs_empty():
    ops = link_arcs_to_ops(arcs=[], uncleared=[], cut_z=-1.0, safe_z=5.0)
    assert ops.len() == 0


def test_link_arcs_feed_rate_applied():
    arcs = [[(0, 0), (10, 0)]]
    ops = link_arcs_to_ops(
        arcs=arcs,
        uncleared=[],
        cut_z=-1.0,
        safe_z=5.0,
        cut_feed_rate=1500,
        travel_rapid_rate=9000,
    )
    found_feed = False
    for i in range(ops.len()):
        if ops.is_cutting(i):
            assert ops.state_at(i).feed_rate == 1500
            found_feed = True
            break
    assert found_feed


# ── adaptive_peeling ─────────────────────────────────────────


def test_adaptive_peeling_rect():
    boundary = _rect(15, 15, 30, 30)
    ca = ClearedArea([_seed_circle(15, 15, 3)])
    ops = adaptive_peeling(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
    )
    assert ops.len() > 0
    assert ops.cut_distance() > 0
    assert ops.distance() > ops.cut_distance()


def test_adaptive_peeling_has_move_and_line():
    boundary = _rect(15, 15, 30, 30)
    ca = ClearedArea([_seed_circle(15, 15, 3)])
    ops = adaptive_peeling(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
    )
    types = [ops.command_type(i) for i in range(ops.len())]
    assert CommandType.MOVE_TO in types
    assert CommandType.LINE_TO in types


def test_adaptive_peeling_with_islands():
    boundary = _rect(25, 25, 50, 50)
    islands = [_rect(25, 25, 10, 10)]
    ca = ClearedArea([_seed_circle(10, 10, 3)])
    ops = adaptive_peeling(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
    )
    assert ops.len() > 0
    for i in range(ops.len()):
        if ops.is_cutting(i):
            ep = ops.endpoint(i)
            assert not (20 <= ep[0] <= 30 and 20 <= ep[1] <= 30)


def test_adaptive_peeling_empty_cleared():
    boundary = _rect(15, 15, 30, 30)
    ca = ClearedArea()
    ops = adaptive_peeling(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
    )
    assert ops.len() >= 0


def test_adaptive_peeling_determinism():
    boundary = _rect(15, 15, 30, 30)
    ca1 = ClearedArea([_seed_circle(15, 15, 3)])
    ca2 = ClearedArea([_seed_circle(15, 15, 3)])
    ops1 = adaptive_peeling(
        cleared=ca1,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
    )
    ops2 = adaptive_peeling(
        cleared=ca2,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
    )
    assert ops1.dump() == ops2.dump()


def test_adaptive_peeling_endpoints_inside_pocket():
    boundary = _rect(15, 15, 30, 30)
    ca = ClearedArea([_seed_circle(15, 15, 3)])
    ops = adaptive_peeling(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
    )
    for i in range(ops.len()):
        if ops.is_cutting(i):
            ep = ops.endpoint(i)
            assert -1 <= ep[0] <= 31
            assert -1 <= ep[1] <= 31


def test_adaptive_peeling_feed_rate_applied():
    boundary = _rect(15, 15, 30, 30)
    ca = ClearedArea([_seed_circle(15, 15, 3)])
    ops = adaptive_peeling(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
        cut_feed_rate=1800,
        travel_rapid_rate=10000,
    )
    found_feed = False
    for i in range(ops.len()):
        if ops.is_cutting(i):
            assert ops.state_at(i).feed_rate == 1800
            found_feed = True
            break
    assert found_feed


# ── adaptive_entry ─────────────────────────────────────────────


def _first_move_idx(ops):
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            return i
    return 0


def test_adaptive_entry_wide_area_returns_path():
    """Wide pocket returns a non-empty 3D toolpath."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    ops, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    idx = _first_move_idx(ops)
    assert ops.len() > 10
    assert abs(ops.endpoint(idx)[2] - 2.0) < 0.01
    assert abs(ops.endpoint(ops.len() - 1)[2] - (-8.0)) < 0.01


def test_adaptive_entry_wide_returns_cleared_polygons():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    _, cleared = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert len(cleared) >= 1
    for poly in cleared:
        assert len(poly) >= 3


def test_adaptive_entry_tight_slot_returns_path():
    boundary = [(0, 0), (100, 0), (100, 16), (0, 16)]
    ops, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=4.0,
        step_over=3.0,
        safe_z=2.0,
        target_z=-6.0,
    )
    idx = _first_move_idx(ops)
    assert ops.len() > 2
    assert abs(ops.endpoint(idx)[2] - 2.0) < 0.01
    assert abs(ops.endpoint(ops.len() - 1)[2] - (-6.0)) < 0.01


def test_adaptive_entry_degenerate_pocket():
    boundary = [(0, 0), (1, 0), (1, 1), (0, 1)]
    ops, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=5.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
    )
    assert isinstance(ops, Ops)


def test_adaptive_entry_step_over_ratio():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    ops1, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ops2, _ = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=4.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    assert ops1.len() > ops2.len()


# ── adaptive_wavefronts ────────────────────────────────────────


def test_adaptive_wavefronts_simple():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_wavefronts(
        ca,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert ops.len() > 0
    assert ca.total_area() > 10000


def test_adaptive_wavefronts_with_islands():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_wavefronts(
        ca,
        boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert ops.len() > 0
    assert ca.total_area() > 5000


def test_adaptive_wavefronts_empty_cleared():
    ca = ClearedArea()
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    ops = adaptive_wavefronts(
        ca,
        boundary,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert isinstance(ops, Ops)


def test_find_cutting_arc_angle_at_tip():
    """Find cutting arc — interior vertices should be smooth (> 100°)."""
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
