import pytest

import raygeo.ops as ops_mod
from raygeo.ops import Ops
from raygeo.ops.transform.link import (
    LinkStrategy,
    find_pass_entry,
    find_pass_exit,
    link_passes,
)


def make_pass(start, end, z=0.0):
    ops = ops_mod.Ops()
    ops.move_to(start[0], start[1], z)
    ops.line_to(end[0], end[1], z)
    return ops


# ── link_passes ──


def test_link_passes_empty():
    result = link_passes([], safe_z=10.0, strategy="retract")
    assert len(result) == 0


def test_link_passes_single():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    result = link_passes([p1], safe_z=10.0, strategy="retract")
    assert len(result) == 2


def test_link_passes_stay_down():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    p2 = make_pass((20.0, 0.0), (30.0, 0.0), 0.0)
    result = link_passes([p1, p2], safe_z=10.0, strategy="stay_down")
    # pass1 + travel MoveTo + pass2 = 5
    assert len(result) == 5
    travel_end = result.endpoint(2)
    assert travel_end[0] == pytest.approx(20.0)
    assert travel_end[2] == pytest.approx(0.0)


def test_link_passes_retract():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    p2 = make_pass((20.0, 0.0), (30.0, 0.0), -5.0)
    result = link_passes([p1, p2], safe_z=10.0, strategy="retract")
    # pass1 (2) + retract + XY + descend (3) + pass2 (2) = 7
    assert len(result) == 7
    # index 2: retract to safe_z
    assert result.endpoint(2)[2] == pytest.approx(10.0)
    # index 3: XY move at safe_z
    assert result.endpoint(3)[0] == pytest.approx(20.0)
    assert result.endpoint(3)[2] == pytest.approx(10.0)
    # index 4: descend to pass2 Z
    assert result.endpoint(4)[2] == pytest.approx(-5.0)


def test_link_passes_three_passes():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    p2 = make_pass((10.0, 10.0), (20.0, 10.0), 0.0)
    p3 = make_pass((20.0, 0.0), (30.0, 0.0), 0.0)
    result = link_passes([p1, p2, p3], safe_z=5.0, strategy="retract")
    assert len(result) == 12


def test_link_passes_retract_same_xy():
    """XY unchanged between passes — no redundant position change."""
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    p2 = make_pass((10.0, 0.0), (20.0, 0.0), 0.0)
    result = link_passes([p1, p2], safe_z=0.0, strategy="retract")
    assert len(result) >= 4


def test_link_passes_invalid_strategy():
    p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0)
    with pytest.raises(ValueError):
        link_passes([p1], safe_z=10.0, strategy="invalid")


def test_link_strategy_constants():
    assert LinkStrategy.RETRACT == "retract"
    assert LinkStrategy.STAY_DOWN == "stay_down"


# ── find_pass_entry / find_pass_exit ──


def test_find_pass_entry_exit_empty():
    ops = ops_mod.Ops()
    assert find_pass_entry(ops) is None
    assert find_pass_exit(ops) is None


def test_find_pass_entry_exit_only_state():
    ops = ops_mod.Ops()
    ops.set_power(0.5)
    assert find_pass_entry(ops) is None
    assert find_pass_exit(ops) is None


def test_find_pass_entry_exit_single_moveto():
    ops = ops_mod.Ops()
    ops.move_to(10.0, 20.0, 5.0)
    entry = find_pass_entry(ops)
    assert entry == pytest.approx((10.0, 20.0, 5.0))
    exit_ = find_pass_exit(ops)
    assert exit_ == pytest.approx((10.0, 20.0, 5.0))


def test_find_pass_entry_exit_single_lineto():
    """No MoveTo — entry falls back to the first moving command."""
    ops = ops_mod.Ops()
    ops.line_to(10.0, 20.0, 5.0)
    entry = find_pass_entry(ops)
    assert entry == pytest.approx((10.0, 20.0, 5.0))
    exit_ = find_pass_exit(ops)
    assert exit_ == pytest.approx((10.0, 20.0, 5.0))


def test_find_pass_entry_exit_full_path():
    ops = ops_mod.Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.line_to(10.0, 10.0, -2.0)
    ops.line_to(0.0, 10.0, -2.0)
    entry = find_pass_entry(ops)
    assert entry == pytest.approx((0.0, 0.0, 0.0))
    exit_ = find_pass_exit(ops)
    assert exit_ == pytest.approx((0.0, 10.0, -2.0))


def test_find_pass_entry_prefers_travel():
    """Entry should return the MoveTo endpoint over a LineTo endpoint."""
    ops = ops_mod.Ops()
    ops.move_to(5.0, 5.0, 0.0)
    ops.line_to(10.0, 10.0, 0.0)
    entry = find_pass_entry(ops)
    assert entry == pytest.approx((5.0, 5.0, 0.0))


def test_find_pass_exit_on_multi_subpath():
    """Exit should return the last moving endpoint across subpaths."""
    ops = ops_mod.Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.move_to(0.0, 10.0, 0.0)
    ops.line_to(10.0, 10.0, 0.0)
    ops.line_to(15.0, 10.0, 0.0)
    exit_ = find_pass_exit(ops)
    assert exit_ == pytest.approx((15.0, 10.0, 0.0))


def test_find_pass_entry_ignores_non_moving():
    """State-only commands should not interfere with entry lookup."""
    ops = ops_mod.Ops()
    ops.set_feed_rate(100)
    ops.set_power(0.8)
    ops.move_to(10.0, 10.0, 0.0)
    ops.line_to(20.0, 10.0, 0.0)
    entry = find_pass_entry(ops)
    assert entry == pytest.approx((10.0, 10.0, 0.0))
    exit_ = find_pass_exit(ops)
    assert exit_ == pytest.approx((20.0, 10.0, 0.0))


# ── smoke tests from original test_ops_assembly ──


def test_assembly_link_passes():
    p1 = Ops.from_polyline([(0.0, 0.0, 0.0), (5.0, 0.0, 0.0)], move_first=True)
    p2 = Ops.from_polyline(
        [(10.0, 5.0, 0.0), (10.0, 10.0, 0.0)], move_first=True
    )
    linked = link_passes(
        [p1, p2], safe_z=10.0, strategy=LinkStrategy.STAY_DOWN
    )
    assert linked.len() >= p1.len() + p2.len()
