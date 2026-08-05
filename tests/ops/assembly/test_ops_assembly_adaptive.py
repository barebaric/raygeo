"""Tests for raygeo.ops.assembly.adaptive module."""

import math

from raygeo.geo.shape.polygon import (
    JoinStyle,
    get_polygon_area,
    get_polygons_group_difference,
    get_polygons_group_intersection,
    offset_polygon,
)
from raygeo.ops import Ops
from raygeo.ops.assembly.adaptive import (
    adaptive_clearing,
    target_area_per_distance,
)
from raygeo.ops.part import Part, StockRegion
from raygeo.ops.part.crescent import cut_area
from raygeo.ops.types import CommandType


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _circle(cx, cy, r, n=32):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _valid_tool_area(boundary, islands, radius):
    """Compute the valid tool-centre region (boundary inset
    by radius, islands expanded by radius).

    Returns (polygons, total_area).
    """
    inset = offset_polygon(boundary, -radius, JoinStyle.MITER)
    if not inset:
        return [], 0.0

    if islands:
        island_bufs = []
        for island in islands:
            island_bufs.extend(offset_polygon(island, radius, JoinStyle.MITER))
        region = get_polygons_group_difference(inset, island_bufs)
    else:
        region = inset

    total = sum(get_polygon_area(p) for p in region)
    return region, total


def _remaining_area(ca, region, valid_polys):
    """Sum of remaining (uncut) area within the valid tool-centre region."""
    remaining = ca.remaining(region)
    if not valid_polys:
        return sum(get_polygon_area(p) for p in remaining)
    clipped = get_polygons_group_intersection(remaining, valid_polys)
    return sum(get_polygon_area(p) for p in clipped)


def test_adaptive_clearing_returns_ops():
    """Non-empty pocket returns a valid Ops object."""
    boundary = _rect(0, 0, 60, 60)
    seed = [_circle(0, 0, 5)]
    part = Part.from_polygons(boundary, initial=seed)
    result_clear = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
    )
    assert isinstance(result_clear.ops, Ops)
    assert result_clear.ops.len() > 0
    assert result_clear.ops.cut_distance() > 0


def test_adaptive_clearing_has_move_and_line():
    """Ops contains both travel and cutting commands."""
    boundary = _rect(0, 0, 60, 60)
    seed = [_circle(0, 0, 5)]
    part = Part.from_polygons(boundary, initial=seed)
    result_clear = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
    )
    types = [
        result_clear.ops.command_type(i) for i in range(result_clear.ops.len())
    ]
    assert CommandType.MOVE_TO in types
    assert CommandType.LINE_TO in types


def test_adaptive_clearing_endpoints_inside_pocket():
    """All cut endpoints lie within the pocket boundary."""
    boundary = _rect(25, 25, 50, 50)
    seed = [_circle(25, 25, 5)]
    part = Part.from_polygons(boundary, initial=seed)
    result_clear = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
    )
    for i in range(result_clear.ops.len()):
        if result_clear.ops.is_cutting(i):
            ep = result_clear.ops.endpoint(i)
            assert -2 <= ep[0] <= 52, f"endpoint x={ep[0]} outside pocket"
            assert -2 <= ep[1] <= 52, f"endpoint y={ep[1]} outside pocket"


def test_adaptive_clearing_with_islands():
    """No cut endpoint inside an island polygon."""
    boundary = _rect(25, 25, 50, 50)
    islands = [_rect(25, 25, 10, 10)]
    seed = [_circle(10, 25, 3)]
    part = Part.from_polygons(boundary, islands, initial=seed)
    result_clear = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
    )
    for i in range(result_clear.ops.len()):
        if result_clear.ops.is_cutting(i):
            ep = result_clear.ops.endpoint(i)
            in_island = 20 <= ep[0] <= 30 and 20 <= ep[1] <= 30
            assert not in_island, (
                f"endpoint ({ep[0]:.1f}, {ep[1]:.1f}) in island"
            )


def test_adaptive_clearing_determinism():
    """Same inputs produce identical output."""
    boundary = _rect(0, 0, 60, 60)
    seed = [_circle(0, 0, 5)]
    part = Part.from_polygons(boundary, initial=seed)
    result1_clear = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
    )
    part2 = Part.from_polygons(boundary, initial=seed)
    result2_clear = adaptive_clearing(
        part2,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
    )
    assert result1_clear.ops.dump() == result2_clear.ops.dump()


def test_adaptive_clearing_feed_rate_applied():
    """Cut feed_rate appears on cutting moves from the clearing pass."""
    boundary = _rect(0, 0, 60, 60)
    seed = [_circle(0, 0, 5)]
    part = Part.from_polygons(boundary, initial=seed)
    result_clear = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        cut_feed_rate=1800,
    )
    found_feed = False
    for i in range(result_clear.ops.len()):
        if result_clear.ops.is_cutting(i):
            assert result_clear.ops.state_at(i).feed_rate == 1800
            found_feed = True
            break
    assert found_feed


def test_adaptive_clearing_cut_power_applied():
    """Cut power appears on cutting moves from the clearing pass."""
    boundary = _rect(0, 0, 60, 60)
    seed = [_circle(0, 0, 5)]
    part = Part.from_polygons(boundary, initial=seed)
    result_clear = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        cut_feed_rate=1200,
        cut_power=0.75,
    )
    found_power = False
    for i in range(result_clear.ops.len()):
        if result_clear.ops.is_cutting(i):
            assert result_clear.ops.state_at(i).power == 0.75
            found_power = True
            break
    assert found_power


def test_adaptive_clearing_degenerate_pocket():
    """Degenerate (zero-area) pocket returns empty Ops."""
    boundary = [(0, 0), (1, 0), (1, 1), (0, 1)]
    seed = [_circle(0.5, 0.5, 0.1)]
    part = Part.from_polygons(boundary, initial=seed)
    result_clear = adaptive_clearing(
        part,
        tool_radius=5.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
    )
    assert isinstance(result_clear.ops, Ops)


def test_adaptive_clearing_fully_clears_rect():
    """After clearing a plain rectangle, remaining area is below tolerance."""

    boundary = _rect(0, 0, 60, 60)
    tol = 1.0
    valid_polys, valid_total = _valid_tool_area(boundary, [], 3.0)
    assert valid_total > tol, "valid tool area too small for a meaningful test"

    seed = [_circle(0, 0, 5)]
    part = Part.from_polygons(boundary, initial=seed)
    region = StockRegion(boundary=boundary)
    adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        area_tolerance=tol,
    )

    remaining = _remaining_area(part.cleared, region, valid_polys)
    assert remaining < tol, (
        f"expected remaining < {tol}, got {remaining:.2f} mm²"
    )


def test_adaptive_clearing_fully_clears_with_island():
    """After clearing a pocket with an island, remaining
    area is below tolerance.
    """
    boundary = _rect(0, 0, 60, 60)
    islands = [_rect(5, 0, 10, 10)]
    tol = 2
    valid_polys, valid_total = _valid_tool_area(boundary, islands, 3.0)
    assert valid_total > tol, "valid tool area too small for a meaningful test"

    seed = [_circle(-10, 0, 5)]
    part = Part.from_polygons(boundary, islands, initial=seed)
    region = StockRegion(boundary=boundary, islands=islands)
    adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        area_tolerance=tol,
    )

    remaining = _remaining_area(part.cleared, region, valid_polys)
    assert remaining < tol, (
        f"expected remaining < {tol}, got {remaining:.2f} mm²"
    )


# ── target_area_per_distance ───────────────────────────────────────


def _lens_area(d: float, R: float) -> float:
    """Analytical lens (circle-circle intersection) area."""
    if d >= 2.0 * R:
        return 0.0
    if d <= 0.0:
        return math.pi * R * R
    return 2.0 * R * R * math.acos(d / (2.0 * R)) - (d / 2.0) * math.sqrt(
        4.0 * R * R - d * d
    )


def _crescent_area(d: float, R: float) -> float:
    """Analytical crescent: disk(c2) minus overlap with disk(c1)."""
    return math.pi * R * R - _lens_area(d, R)


def _wall_fragment(wall_x: float, span: float) -> list[tuple[float, float]]:
    """Rectangle covering everything to the left of ``wall_x``."""
    return [
        (wall_x - span, -span),
        (wall_x, -span),
        (wall_x, span),
        (wall_x - span, span),
    ]


def test_target_area_pd_vs_cut_area_with_wall():
    """target_area_per_distance matches actual cut_area when a straight
    wall fragment simulates the ideal stepover geometry.

    c1 = (0, 0), c2 = (0, step_length)  — step along +y.
    Wall fragment covers x < (R - advance), leaving only the crescent
    portion to the right as new area.
    """
    R = 5.0
    step_length = 1.0
    span = 3.0 * R
    for advance in [1.0, 2.0, 3.0, 4.0]:
        wall_x = R - advance
        c1 = (0.0, 0.0)
        c2 = (0.0, step_length)
        wall = _wall_fragment(wall_x, span)
        actual_total, _ = cut_area(c1, c2, R, [wall], [])
        actual_apd = actual_total / step_length
        formula = target_area_per_distance(R, advance, step_length)
        rel_err = abs(formula - actual_apd) / max(actual_apd, 1e-9)
        assert rel_err < 0.05, (
            f"advance={advance}: formula={formula:.4f}, "
            f"actual_apd={actual_apd:.4f}, rel_err={rel_err:.2%}"
        )


def test_target_area_pd_full_engagement():
    """When advance = 2R (wall far left), target matches the full
    crescent area divided by step_length."""
    R = 5.0
    step_length = 1.0
    full_crescent = _crescent_area(step_length, R)
    expected_apd = full_crescent / step_length
    formula = target_area_per_distance(R, 2.0 * R, step_length)
    rel_err = abs(formula - expected_apd) / max(expected_apd, 1e-9)
    assert rel_err < 0.01, (
        f"formula={formula:.4f}, expected={expected_apd:.4f}, "
        f"rel_err={rel_err:.2%}"
    )


def test_target_area_pd_zero_advance():
    """When advance = 0 (wall at x = R), no new area."""
    R = 5.0
    step_length = 1.0
    assert abs(target_area_per_distance(R, 0.0, step_length)) < 1e-9


def test_target_area_pd_monotonic_in_advance():
    """Larger advance should produce larger target_area_per_distance."""
    R = 5.0
    step_length = 1.0
    prev = 0.0
    for advance in [0.5, 1.0, 2.0, 4.0, 8.0]:
        val = target_area_per_distance(R, advance, step_length)
        assert val > prev, (
            f"Not monotonic at advance={advance}: "
            f"val={val:.4f}, prev={prev:.4f}"
        )
        prev = val


def test_adaptive_clearing_uses_step_over():
    """adaptive_clearing accepts step_over and produces cut moves."""
    boundary = _rect(0, 0, 80, 80)
    seed = [_rect(35, 35, 10, 10)]
    part = Part.from_polygons(boundary, initial=seed)
    result = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        safe_z=2.0,
    )
    assert result.ops.len() > 0


def test_adaptive_clearing_renamed_fields():
    """adaptive_clearing accepts the renamed fields without error."""
    boundary = _rect(0, 0, 80, 80)
    seed = [_rect(35, 35, 10, 10)]
    part = Part.from_polygons(boundary, initial=seed)
    result = adaptive_clearing(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
    )
    assert hasattr(result, "ops")
