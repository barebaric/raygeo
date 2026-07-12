"""Tests for raygeo.ops.assembly.profile module (inner profiling)."""

from typing import Any

from raygeo.geo.shape.polygon import (
    JoinStyle,
    get_polygon_area,
    get_polygons_closest_point,
    offset_polygon,
)
from raygeo.ops import Ops
from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.cut import Part


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _kwargs(initial, boundary, **over: Any) -> dict[str, Any]:
    islands = over.pop("islands", None)
    part = Part.from_polygons(boundary, islands or [], initial=initial)
    kw = dict(
        part=part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        stock_to_leave=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        start_pos=None,
        cut_direction="ccw",
        engagement_area_threshold=0.0,
        engagement_angle_threshold=3.141592653589793,
    )
    kw.update(over)
    return kw


def test_profile_inner_smoke_returns_ops():
    """profile_inner returns an AssemblyResult with Ops."""
    boundary = _rect(0, 0, 60, 60)
    result = profile_inner(**_kwargs([], boundary))
    assert isinstance(result.ops, Ops)
    assert result.ops.len() > 0


def test_profile_inner_adds_cut_distance_for_island():
    """Adding an accessible island increases total cut distance."""
    boundary = _rect(0, 0, 60, 60)
    island = _rect(15, 0, 10, 10)

    result_no_island = profile_inner(**_kwargs([], boundary))

    result_with_island = profile_inner(
        **_kwargs([], boundary, islands=[island])
    )

    cd_no = result_no_island.ops.cut_distance()
    cd_yes = result_with_island.ops.cut_distance()
    assert cd_yes > cd_no, (
        f"cut distance should increase with island: {cd_yes} <= {cd_no}"
    )


def test_profile_inner_adds_more_distance_for_two_islands():
    """Two accessible islands add more cut distance than one."""
    boundary = _rect(0, 0, 60, 60)
    island1 = _rect(15, 0, 8, 8)
    island2 = _rect(-15, 0, 8, 8)

    result_one = profile_inner(**_kwargs([], boundary, islands=[island1]))

    result_two = profile_inner(
        **_kwargs([], boundary, islands=[island1, island2])
    )

    cd1 = result_one.ops.cut_distance()
    cd2 = result_two.ops.cut_distance()
    assert cd2 > cd1, (
        f"two islands should add more cut distance: {cd2} <= {cd1}"
    )


def test_profile_inner_island_paths_on_grown_polygon():
    """Island-walk cut vertices lie on the grown island polygon."""
    boundary = _rect(0, 0, 60, 60)
    island = _rect(15, 0, 10, 10)
    result = profile_inner(**_kwargs([], boundary, islands=[island]))
    ops = result.ops
    radius = 3.0
    offset_dist = radius
    grown_polys = offset_polygon(island, offset_dist, JoinStyle.Round)
    grown = grown_polys[0]
    gxs = [p[0] for p in grown]
    gys = [p[1] for p in grown]
    gx_min, gx_max = min(gxs), max(gxs)
    gy_min, gy_max = min(gys), max(gys)

    island_cut_vertices = []
    for i in range(ops.len()):
        if not ops.is_cutting(i):
            continue
        x, y, _ = ops.endpoint(i)
        if gx_min - 2.0 <= x <= gx_max + 2.0:
            if gy_min - 2.0 <= y <= gy_max + 2.0:
                island_cut_vertices.append((x, y))

    assert len(island_cut_vertices) > 0, "no cut vertices near island"
    for x, y in island_cut_vertices:
        result_cp = get_polygons_closest_point(grown_polys, x, y)
        assert result_cp is not None
        _, _, _, dist_sq = result_cp
        drift = dist_sq**0.5
        assert drift <= 1.5, (
            f"island vertex ({x:.2f}, {y:.2f}) drifts {drift:.2f}mm"
        )


def test_profile_inner_outer_walk_on_inset_polygon():
    """Outer-walk cut vertices are within the inset polygon bounds."""
    boundary = _rect(0, 0, 60, 60)
    result = profile_inner(**_kwargs([], boundary))
    ops = result.ops
    radius = 3.0
    inset_half = 30.0 - radius
    outer_vertices = []
    for i in range(ops.len()):
        if not ops.is_cutting(i):
            continue
        x, y, _ = ops.endpoint(i)
        outer_vertices.append((x, y))
    assert len(outer_vertices) > 0
    for x, y in outer_vertices:
        lo = -inset_half - 1.0
        hi = inset_half + 1.0
        assert lo <= x <= hi, f"x={x:.2f} outside [{lo:.2f}, {hi:.2f}]"
        assert lo <= y <= hi, f"y={y:.2f} outside [{lo:.2f}, {hi:.2f}]"


def test_profile_inner_mutates_cleared_area():
    """profile_inner adds swept area to ClearedArea."""
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])
    region = part.stock_region
    remaining_before = part.cleared.remaining(region)
    before_area = sum(get_polygon_area(p) for p in remaining_before)

    profile_inner(
        part=part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        stock_to_leave=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        start_pos=None,
        cut_direction="ccw",
        engagement_area_threshold=0.0,
        engagement_angle_threshold=3.141592653589793,
    )

    remaining_after = part.cleared.remaining(region)
    after_area = sum(get_polygon_area(p) for p in remaining_after)
    assert after_area < before_area, (
        f"remaining area did not decrease: {before_area} -> {after_area}"
    )
    assert part.cleared.total_area() > 0


def test_profile_inner_then_outer_chained():
    """profile_inner then profile_outer on same ClearedArea produces cuts."""
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])

    result_inner = profile_inner(
        part=part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        stock_to_leave=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        start_pos=None,
        cut_direction="ccw",
        engagement_area_threshold=0.0,
        engagement_angle_threshold=3.141592653589793,
    )
    assert result_inner.ops.cut_distance() > 0

    result_outer = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        stock_to_leave=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        start_pos=None,
        cut_direction="ccw",
    )
    assert result_outer.ops.cut_distance() > 0


def test_profile_inner_skips_blocked_island():
    """profile_inner skips island with channel narrower than 2*radius."""
    boundary = _rect(0, 0, 60, 60)
    island_far = _rect(-15, 0, 8, 8)
    island_close = _rect(24, 0, 8, 8)

    result_far = profile_inner(**_kwargs([], boundary, islands=[island_far]))

    result_both = profile_inner(
        **_kwargs([], boundary, islands=[island_far, island_close])
    )

    # If close island is blocked, cut distance should be similar.
    diff = abs(result_both.ops.cut_distance() - result_far.ops.cut_distance())
    close_perimeter = 2 * (8 + 2 * 3.0) * 2
    assert diff < close_perimeter * 0.5, (
        f"close island should be blocked; diff={diff:.1f}mm >= "
        f"{close_perimeter * 0.5:.1f}mm"
    )


def test_profile_inner_walks_accessible_island():
    """profile_inner walks an accessible island, adding significant dist."""
    boundary = _rect(0, 0, 60, 60)
    island = _rect(-15, 0, 8, 8)

    result_no = profile_inner(**_kwargs([], boundary))

    result_yes = profile_inner(**_kwargs([], boundary, islands=[island]))

    grown_perimeter_est = 4 * (8 + 2 * 3.0)
    added = result_yes.ops.cut_distance() - result_no.ops.cut_distance()
    assert added > grown_perimeter_est * 0.3, (
        f"added cut distance {added:.1f}mm should be substantial "
        f"(grown perimeter ~{grown_perimeter_est}mm)"
    )


def test_profile_inner_renamed_fields():
    """profile_inner accepts the renamed tool_radius/target_z fields."""
    boundary = _rect(0, 0, 80, 80)
    part = Part.from_polygons(
        boundary,
        initial=[[(35, 35), (45, 35), (45, 45), (35, 45)]],
    )
    result = profile_inner(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        stock_to_leave=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
    )
    assert result.ops.len() > 0
