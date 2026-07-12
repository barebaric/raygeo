"""Visualisation for ops/assembly/profile — adaptive profiling."""

import matplotlib.pyplot as plt

from raygeo.geo.shape.polygon import (
    JoinStyle,
    get_circle_polygon,
    offset_polygon,
)
from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.part import Part
from tools.plot import plot_ops, plot_ops_2d, plot_ops_3d

__docs_target__ = ["raygeo.ops.assembly.profile.md"]


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def generate_profile_outer_rect():
    """profile_outer on a 60×60 rect — combined 3D (left) + 2D (right)."""
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])
    result = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
    )

    fig = plot_ops(result.ops, boundary=boundary)
    ax = fig.axes[1]
    offset_polys = offset_polygon(boundary, 3.0, JoinStyle.Round)
    offset = offset_polys[0]
    ox = [p[0] for p in offset] + [offset[0][0]]
    oy = [p[1] for p in offset] + [offset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="_nolegend_")
    return fig


def generate_profile_outer_circle():
    boundary = get_circle_polygon((0, 0), 30, 64)
    part = Part.from_polygons(boundary, initial=[])
    result = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))

    offset_polys = offset_polygon(boundary, 3.0, JoinStyle.Round)
    offset = offset_polys[0]
    ox = [p[0] for p in offset] + [offset[0][0]]
    oy = [p[1] for p in offset] + [offset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="Offset tool-centre polygon")

    plot_ops_2d(ax, result.ops, boundary=boundary)

    ax.set_title("profile_outer — circle (2D)")
    fig.tight_layout()
    return fig


def generate_profile_outer_concave_pocket():
    boundary = [
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 100.0),
        (60.0, 100.0),
        (60.0, 40.0),
        (0.0, 40.0),
    ]
    part = Part.from_polygons(boundary, initial=[])
    result = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))

    offset_polys = offset_polygon(boundary, 3.0, JoinStyle.Round)
    offset = offset_polys[0]
    ox = [p[0] for p in offset] + [offset[0][0]]
    oy = [p[1] for p in offset] + [offset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="Offset tool-centre polygon")

    plot_ops_2d(ax, result.ops, boundary=boundary)

    ax.set_title("profile_outer — L-shaped pocket (2D, miter join)")
    fig.tight_layout()
    return fig


def generate_profile_outer_rough_then_finish():
    boundary = _rect(0, 0, 60, 60)
    part = Part.from_polygons(boundary, initial=[])

    result_rough = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        stock_to_leave=0.5,
    )

    result_finish = profile_outer(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        stock_to_leave=0.0,
    )

    # Chain the two passes with a travel move between them.
    from raygeo.ops import Ops

    combined = Ops()
    combined.extend(result_rough.ops)
    # The rough pass ends with a lift to safe_z.  The finish pass starts
    # with a plunge.  Insert an explicit travel at safe_z from the rough
    # end position to the finish start position so the gap is visible.
    finish_start = result_finish.ops.endpoint(0)
    combined.move_to(finish_start[0], finish_start[1], 2.0, None)
    combined.extend(result_finish.ops)

    fig, ax = plt.subplots(figsize=(8, 8))

    plot_ops_2d(ax, combined, boundary=boundary)

    ax.set_title("profile_outer — rough + finish (turbo) with travel link")
    fig.tight_layout()
    return fig


def generate_profile_inner_rect_with_square_island_2d():
    boundary = _rect(0, 0, 60, 60)
    island = _rect(15, 0, 10, 10)
    part = Part.from_polygons(boundary, [island], initial=[])
    result = profile_inner(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))

    inset_polys = offset_polygon(boundary, -3.0, JoinStyle.Round)
    inset = inset_polys[0]
    ox = [p[0] for p in inset] + [inset[0][0]]
    oy = [p[1] for p in inset] + [inset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="Inset outer polygon")

    grown_polys = offset_polygon(island, 3.0, JoinStyle.Round)
    grown = grown_polys[0]
    gx = [p[0] for p in grown] + [grown[0][0]]
    gy = [p[1] for p in grown] + [grown[0][1]]
    ax.plot(
        gx, gy, "orange", linestyle="--", linewidth=1.0, label="Grown island"
    )

    plot_ops_2d(ax, result.ops, boundary=boundary, islands=[island])

    ax.set_title("profile_inner — rect with square island (2D)")
    fig.tight_layout()
    return fig


def generate_profile_inner_rect_with_two_islands_2d():
    boundary = _rect(0, 0, 60, 60)
    island1 = _rect(-15, 5, 8, 8)
    island2 = _rect(15, -5, 8, 8)
    part = Part.from_polygons(boundary, [island1, island2], initial=[])
    result = profile_inner(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))

    inset_polys = offset_polygon(boundary, -3.0, JoinStyle.Round)
    inset = inset_polys[0]
    ox = [p[0] for p in inset] + [inset[0][0]]
    oy = [p[1] for p in inset] + [inset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="Inset outer polygon")

    for i, island in enumerate([island1, island2]):
        grown_polys = offset_polygon(island, 3.0, JoinStyle.Round)
        grown = grown_polys[0]
        gx = [p[0] for p in grown] + [grown[0][0]]
        gy = [p[1] for p in grown] + [grown[0][1]]
        label = "Grown island" if i == 0 else ""
        ax.plot(gx, gy, "orange", linestyle="--", linewidth=1.0, label=label)

    plot_ops_2d(ax, result.ops, boundary=boundary, islands=[island1, island2])

    ax.set_title("profile_inner — two islands, nearest-neighbour order (2D)")
    fig.tight_layout()
    return fig


def generate_profile_inner_concave_with_island_3d():
    pocket = [
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 100.0),
        (60.0, 100.0),
        (60.0, 40.0),
        (0.0, 40.0),
    ]
    island = _rect(30, 65, 12, 12)
    part = Part.from_polygons(pocket, [island], initial=[])
    result = profile_inner(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
    )

    fig = plt.figure(figsize=(8, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, result.ops, boundary=pocket)
    fig.tight_layout()
    return fig


def generate_profile_inner_narrow_channel_skips_island():
    boundary = _rect(0, 0, 60, 60)
    accessible = _rect(0, 15, 10, 10)
    blocked = _rect(24, 0, 10, 10)
    part = Part.from_polygons(boundary, [accessible, blocked], initial=[])
    result = profile_inner(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))

    aix = [p[0] for p in accessible] + [accessible[0][0]]
    aiy = [p[1] for p in accessible] + [accessible[0][1]]
    ax.fill(aix, aiy, color="lightgray", alpha=0.6, label="Accessible island")

    bix = [p[0] for p in blocked] + [blocked[0][0]]
    biy = [p[1] for p in blocked] + [blocked[0][1]]
    ax.fill(
        bix,
        biy,
        color="lightcoral",
        alpha=0.4,
        label="Blocked island (skipped)",
    )

    inset_polys = offset_polygon(boundary, -3.0, JoinStyle.Round)
    inset = inset_polys[0]
    ox = [p[0] for p in inset] + [inset[0][0]]
    oy = [p[1] for p in inset] + [inset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="Inset outer polygon")

    grown_polys = offset_polygon(accessible, 3.0, JoinStyle.Round)
    grown = grown_polys[0]
    gx = [p[0] for p in grown] + [grown[0][0]]
    gy = [p[1] for p in grown] + [grown[0][1]]
    ax.plot(
        gx,
        gy,
        "orange",
        linestyle="--",
        linewidth=1.0,
        label="Grown accessible island",
    )

    plot_ops_2d(ax, result.ops, boundary=boundary)

    ax.set_title("profile_inner — narrow channel skips blocked island (2D)")
    fig.tight_layout()
    return fig


def generate_profile_inner_rough_then_finish():
    boundary = _rect(0, 0, 60, 60)
    island = _rect(15, 0, 10, 10)
    part = Part.from_polygons(boundary, [island], initial=[])

    result_rough = profile_inner(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        stock_to_leave=0.5,
    )

    result_finish = profile_inner(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        stock_to_leave=0.0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))

    from raygeo.ops import Ops

    combined = Ops()
    combined.extend(result_rough.ops)
    finish_start = result_finish.ops.endpoint(0)
    combined.move_to(finish_start[0], finish_start[1], 2.0, None)
    combined.extend(result_finish.ops)

    plot_ops_2d(ax, combined, boundary=boundary, islands=[island])

    ax.set_title("profile_inner — rough + finish (turbo) with travel link")
    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": "profile_outer",
        "caption": (
            "profile_outer on a rect pocket — 3D (left) and 2D"
            " top-down with offset tool-centre polygon (right)."
        ),
        "function": generate_profile_outer_rect,
    },
    {
        "heading": "profile_outer",
        "caption": (
            "profile_outer on a circular boundary — smooth walk around "
            "the offset circle."
        ),
        "function": generate_profile_outer_circle,
    },
    {
        "heading": "profile_outer",
        "caption": (
            "profile_outer on an L-shaped pocket with miter join at "
            "the concave corner."
        ),
        "function": generate_profile_outer_concave_pocket,
    },
    {
        "heading": "profile_outer",
        "caption": (
            "Two-pass profiling: rough (with stock, orange) + "
            "finish (red) on the same ClearedArea."
        ),
        "function": generate_profile_outer_rough_then_finish,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "profile_inner on a square pocket with island — "
            "2D: boundary, island, offset walks, cuts (turbo)."
        ),
        "function": generate_profile_inner_rect_with_square_island_2d,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "profile_inner with two accessible islands — "
            "nearest-neighbour order via turbo gradient."
        ),
        "function": generate_profile_inner_rect_with_two_islands_2d,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "profile_inner on an L-shaped pocket with island — "
            "3D: cut path at cut_z, rapids at safe_z."
        ),
        "function": generate_profile_inner_concave_with_island_3d,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "profile_inner skips an island when the channel between "
            "island and wall is too narrow."
        ),
        "function": generate_profile_inner_narrow_channel_skips_island,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "Two-pass inner profiling: rough (orange) + "
            "finish (red) on same ClearedArea."
        ),
        "function": generate_profile_inner_rough_then_finish,
    },
]
