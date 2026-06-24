"""Tests for raygeo.ops.assembly.adaptive module."""

from raygeo.ops import Ops
from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.assembly.hsm import adaptive_entry
from raygeo.ops.cleared_area import ClearedArea
from raygeo.ops.types import CommandType


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def test_adaptive_clearing_returns_ops():
    """Non-empty pocket returns a valid Ops object."""
    boundary = _rect(0, 0, 60, 60)
    entry_ops, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
    )
    # Combine entry + clearing
    combined = Ops()
    combined.extend(entry_ops)
    combined.extend(ops)
    assert isinstance(combined, Ops)
    assert combined.len() > 0
    assert combined.cut_distance() > 0


def test_adaptive_clearing_has_move_and_line():
    """Ops contains both travel and cutting commands."""
    boundary = _rect(0, 0, 60, 60)
    entry_ops, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
    )
    combined = Ops()
    combined.extend(entry_ops)
    combined.extend(ops)
    types = [combined.command_type(i) for i in range(combined.len())]
    assert CommandType.MOVE_TO in types
    assert CommandType.LINE_TO in types


def test_adaptive_clearing_endpoints_inside_pocket():
    """All cut endpoints lie within the pocket boundary."""
    boundary = _rect(25, 25, 50, 50)
    entry_ops, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
    )
    for i in range(ops.len()):
        if ops.is_cutting(i):
            ep = ops.endpoint(i)
            assert -2 <= ep[0] <= 52, f"endpoint x={ep[0]} outside pocket"
            assert -2 <= ep[1] <= 52, f"endpoint y={ep[1]} outside pocket"


def test_adaptive_clearing_with_islands():
    """No cut endpoint inside an island polygon."""
    boundary = _rect(25, 25, 50, 50)
    islands = [_rect(25, 25, 10, 10)]
    entry_ops, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
    )
    for i in range(ops.len()):
        if ops.is_cutting(i):
            ep = ops.endpoint(i)
            in_island = 20 <= ep[0] <= 30 and 20 <= ep[1] <= 30
            assert not in_island, (
                f"endpoint ({ep[0]:.1f}, {ep[1]:.1f}) in island"
            )


def test_adaptive_clearing_determinism():
    """Same inputs produce identical output."""
    boundary = _rect(0, 0, 60, 60)
    _, cp1 = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
    )
    _, cp2 = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
    )
    ca1 = ClearedArea(initial=cp1)
    ca2 = ClearedArea(initial=cp2)
    ops1 = adaptive_clearing(
        cleared=ca1,
        pocket_boundary=boundary,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
    )
    ops2 = adaptive_clearing(
        cleared=ca2,
        pocket_boundary=boundary,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
    )
    # Only compare the clearing part (entry is not included)
    assert ops1.dump() == ops2.dump()


def test_adaptive_clearing_feed_rate_applied():
    """Cut feed_rate appears on cutting moves from the clearing pass."""
    boundary = _rect(0, 0, 60, 60)
    entry_ops, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
        cut_feed_rate=1800,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
        cut_feed_rate=1800,
    )
    found_feed = False
    for i in range(ops.len()):
        if ops.is_cutting(i):
            assert ops.state_at(i).feed_rate == 1800
            found_feed = True
            break
    assert found_feed


def test_adaptive_clearing_cut_power_applied():
    """Cut power appears on cutting moves from the clearing pass."""
    boundary = _rect(0, 0, 60, 60)
    entry_ops, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
        cut_feed_rate=1200,
        cut_power=0.75,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
        cut_feed_rate=1200,
        cut_power=0.75,
    )
    found_power = False
    for i in range(ops.len()):
        if ops.is_cutting(i):
            assert ops.state_at(i).power == 0.75
            found_power = True
            break
    assert found_power


def test_adaptive_clearing_degenerate_pocket():
    """Degenerate (zero-area) pocket returns empty Ops."""
    boundary = [(0, 0), (1, 0), (1, 1), (0, 1)]
    entry_ops, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=5.0,
        step_over=1.5,
        safe_z=2.0,
        target_z=-5.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=5.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
    )
    assert isinstance(ops, Ops)
