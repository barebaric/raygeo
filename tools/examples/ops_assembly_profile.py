"""Visualisation for ops/assembly/profile — adaptive profiling."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.collections import LineCollection

from raygeo.geo.shape.polygon import (
    JoinStyle,
    get_circle_polygon,
    offset_polygon,
)
from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.cut.cleared_area import ClearedArea

__docs_target__ = ["raygeo.ops.assembly.profile.md"]


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _ops_to_points(ops):
    out = []
    for i in range(ops.len()):
        if ops.is_cutting(i) or ops.is_travel(i):
            ep = ops.endpoint(i)
            out.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return out


def _plot_2d_toolpath(ops, ax):
    pts = _ops_to_points(ops)
    if not pts:
        return

    segments = []
    cur = []
    for p in pts:
        x, y, z, is_travel = p
        if is_travel:
            if len(cur) > 1:
                segments.append(cur)
            # The next cut segment starts from where the travel ended,
            # so seed `cur` with the travel endpoint.
            cur = [(x, y)]
        else:
            cur.append((x, y))
    if len(cur) > 1:
        segments.append(cur)

    segs_list = []
    cum_dists = []
    cum = 0.0
    prev = None
    for seg in segments:
        for p in seg:
            if prev is not None:
                segs_list.append([prev, p])
                cum += math.hypot(p[0] - prev[0], p[1] - prev[1])
                cum_dists.append(cum)
            prev = p
        prev = None
    total = cum if cum > 0 else 1.0
    if segs_list:
        ax.add_collection(
            LineCollection(
                segs_list,
                colors=plt.cm.turbo([d / total for d in cum_dists]),
                linewidth=0.8,
                alpha=1.0,
            )
        )

    prev = None
    for p in pts:
        x, y, z, is_travel = p
        if is_travel:
            if prev is not None:
                ax.plot(
                    [prev[0], x],
                    [prev[1], y],
                    linestyle="--",
                    linewidth=0.5,
                    color="dimgray",
                    alpha=0.8,
                )
            prev = (x, y)
        else:
            prev = (x, y)


def _draw_3d_boundary(ax, boundary, z_plane):
    if boundary is not None and z_plane is not None:
        bnd = np.array(list(boundary) + [boundary[0]])
        ax.plot(
            bnd[:, 0],
            bnd[:, 1],
            zs=z_plane,
            zdir="z",
            color="k",
            linewidth=2,
            alpha=0.5,
        )


def _plot_3d_toolpath(
    ops,
    ax,
    title,
    boundary=None,
    z_plane=None,
):
    pts_list = _ops_to_points(ops)
    if not pts_list:
        fig = ax.figure
        fig.tight_layout()
        return fig

    segments = []
    cur = []
    for p in pts_list:
        x, y, z, is_travel = p
        if is_travel:
            if len(cur) > 1:
                segments.append(cur)
            cur = [(x, y, z)]
        else:
            cur.append((x, y, z))
    if len(cur) > 1:
        segments.append(cur)

    segs_3d = []
    cum_dists = []
    cum = 0.0
    prev = None
    for seg in segments:
        for p in seg:
            if prev is not None:
                segs_3d.append([prev, p])
                d = math.sqrt(
                    (p[0] - prev[0]) ** 2
                    + (p[1] - prev[1]) ** 2
                    + (p[2] - prev[2]) ** 2
                )
                cum += d
                cum_dists.append(cum)
            prev = p
        prev = None
    total = cum if cum > 0 else 1.0
    if segs_3d:
        from mpl_toolkits.mplot3d.art3d import Line3DCollection

        lc3d = Line3DCollection(
            segs_3d,
            colors=plt.cm.turbo([d / total for d in cum_dists]),
            linewidth=0.8,
            alpha=1.0,
        )
        ax.add_collection3d(lc3d)

    prev = None
    for p in pts_list:
        x, y, z, is_travel = p
        if is_travel:
            if prev is not None:
                ax.plot(
                    [prev[0], x],
                    [prev[1], y],
                    [prev[2], z],
                    linestyle="--",
                    linewidth=0.5,
                    color="dimgray",
                    alpha=0.8,
                )
            prev = (x, y, z)
        else:
            prev = (x, y, z)

    _draw_3d_boundary(ax, boundary, z_plane)
    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.view_init(elev=30, azim=-45)

    xl, xr = ax.get_xlim()
    yl, yr = ax.get_ylim()
    zl, zr = ax.get_zlim()
    half = max(xr - xl, yr - yl, zr - zl) * 0.5
    xm = (xl + xr) * 0.5
    ym = (yl + yr) * 0.5
    zm = (zl + zr) * 0.5
    ax.set_xlim(xm - half, xm + half)
    ax.set_ylim(ym - half, ym + half)
    ax.set_zlim(zm - half, zm + half)


def generate_profile_outer_rect_2d():
    boundary = _rect(0, 0, 60, 60)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_outer(
        cleared=ca,
        boundary=boundary,
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
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Source boundary")

    offset_polys = offset_polygon(boundary, 3.0, JoinStyle.Round)
    offset = offset_polys[0]
    ox = [p[0] for p in offset] + [offset[0][0]]
    oy = [p[1] for p in offset] + [offset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="Offset tool-centre polygon")

    _plot_2d_toolpath(result.ops, ax)

    ax.plot(
        [],
        [],
        linestyle="--",
        linewidth=0.5,
        color="dimgray",
        alpha=0.8,
        label="Travel",
    )
    ax.set_title("profile_outer — 60×60 rect (2D)")
    handles, labels = ax.get_legend_handles_labels()
    seen = set()
    unique = []
    for h, lbl in zip(handles, labels):
        if lbl not in seen:
            unique.append((h, lbl))
            seen.add(lbl)
    ax.legend(*zip(*unique), loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    fig.tight_layout()
    return fig


def generate_profile_outer_rect_3d():
    boundary = _rect(0, 0, 60, 60)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_outer(
        cleared=ca,
        boundary=boundary,
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
    _plot_3d_toolpath(
        result.ops,
        ax,
        "profile_outer — 60×60 rect (3D)",
        boundary=boundary,
        z_plane=-5.0,
    )
    fig.tight_layout()
    return fig


def generate_profile_outer_circle():
    boundary = get_circle_polygon((0, 0), 30, 64)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_outer(
        cleared=ca,
        boundary=boundary,
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
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Source boundary")

    offset_polys = offset_polygon(boundary, 3.0, JoinStyle.Round)
    offset = offset_polys[0]
    ox = [p[0] for p in offset] + [offset[0][0]]
    oy = [p[1] for p in offset] + [offset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="Offset tool-centre polygon")

    _plot_2d_toolpath(result.ops, ax)

    ax.plot(
        [],
        [],
        linestyle="--",
        linewidth=0.5,
        color="dimgray",
        alpha=0.8,
        label="Travel",
    )
    ax.set_title("profile_outer — circle (2D)")
    handles, labels = ax.get_legend_handles_labels()
    seen = set()
    unique = []
    for h, lbl in zip(handles, labels):
        if lbl not in seen:
            unique.append((h, lbl))
            seen.add(lbl)
    ax.legend(*zip(*unique), loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
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
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_outer(
        cleared=ca,
        boundary=boundary,
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
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Source boundary")

    offset_polys = offset_polygon(boundary, 3.0, JoinStyle.Round)
    offset = offset_polys[0]
    ox = [p[0] for p in offset] + [offset[0][0]]
    oy = [p[1] for p in offset] + [offset[0][1]]
    ax.plot(ox, oy, "b--", linewidth=1.0, label="Offset tool-centre polygon")

    _plot_2d_toolpath(result.ops, ax)

    ax.plot(
        [],
        [],
        linestyle="--",
        linewidth=0.5,
        color="dimgray",
        alpha=0.8,
        label="Travel",
    )
    ax.set_title("profile_outer — L-shaped pocket (2D, miter join)")
    handles, labels = ax.get_legend_handles_labels()
    seen = set()
    unique = []
    for h, lbl in zip(handles, labels):
        if lbl not in seen:
            unique.append((h, lbl))
            seen.add(lbl)
    ax.legend(*zip(*unique), loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    fig.tight_layout()
    return fig


def generate_profile_outer_rough_then_finish():
    boundary = _rect(0, 0, 60, 60)
    ca = ClearedArea(boundary=boundary, initial=[])

    result_rough = profile_outer(
        cleared=ca,
        boundary=boundary,
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
        cleared=ca,
        boundary=boundary,
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
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Boundary")

    _plot_2d_toolpath(combined, ax)

    ax.set_title("profile_outer — rough + finish (turbo) with travel link")
    ax.plot(
        [],
        [],
        linestyle="--",
        linewidth=0.5,
        color="dimgray",
        alpha=0.8,
        label="Travel",
    )
    ax.legend(loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    fig.tight_layout()
    return fig


def generate_profile_inner_rect_with_square_island_2d():
    boundary = _rect(0, 0, 60, 60)
    island = _rect(15, 0, 10, 10)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_inner(
        cleared=ca,
        boundary=boundary,
        islands=[island],
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
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Source boundary")

    ix = [p[0] for p in island] + [island[0][0]]
    iy = [p[1] for p in island] + [island[0][1]]
    ax.fill(ix, iy, color="lightgray", alpha=0.6, label="Source island")

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

    _plot_2d_toolpath(result.ops, ax)

    ax.plot(
        [],
        [],
        linestyle="--",
        linewidth=0.5,
        color="dimgray",
        alpha=0.8,
        label="Travel",
    )
    ax.set_title("profile_inner — rect with square island (2D)")
    handles, labels = ax.get_legend_handles_labels()
    seen = set()
    unique = []
    for h, lbl in zip(handles, labels):
        if lbl not in seen:
            unique.append((h, lbl))
            seen.add(lbl)
    ax.legend(*zip(*unique), loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    fig.tight_layout()
    return fig


def generate_profile_inner_rect_with_two_islands_2d():
    boundary = _rect(0, 0, 60, 60)
    island1 = _rect(-15, 5, 8, 8)
    island2 = _rect(15, -5, 8, 8)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_inner(
        cleared=ca,
        boundary=boundary,
        islands=[island1, island2],
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
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Source boundary")

    for island in [island1, island2]:
        ix = [p[0] for p in island] + [island[0][0]]
        iy = [p[1] for p in island] + [island[0][1]]
        ax.fill(
            ix,
            iy,
            color="lightgray",
            alpha=0.6,
            label="Source island" if island is island1 else "",
        )

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

    _plot_2d_toolpath(result.ops, ax)

    ax.plot(
        [],
        [],
        linestyle="--",
        linewidth=0.5,
        color="dimgray",
        alpha=0.8,
        label="Travel",
    )
    ax.set_title("profile_inner — two islands, nearest-neighbour order (2D)")
    handles, labels = ax.get_legend_handles_labels()
    seen = set()
    unique = []
    for h, lbl in zip(handles, labels):
        if lbl not in seen:
            unique.append((h, lbl))
            seen.add(lbl)
    ax.legend(*zip(*unique), loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
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
    ca = ClearedArea(boundary=pocket, initial=[])
    result = profile_inner(
        cleared=ca,
        boundary=pocket,
        islands=[island],
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
    _plot_3d_toolpath(
        result.ops,
        ax,
        "profile_inner — L-pocket with island (3D)",
        boundary=pocket,
        z_plane=-5.0,
    )
    fig.tight_layout()
    return fig


def generate_profile_inner_narrow_channel_skips_island():
    boundary = _rect(0, 0, 60, 60)
    accessible = _rect(0, 15, 10, 10)
    blocked = _rect(24, 0, 10, 10)
    ca = ClearedArea(boundary=boundary, initial=[])
    result = profile_inner(
        cleared=ca,
        boundary=boundary,
        islands=[accessible, blocked],
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
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Source boundary")

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

    _plot_2d_toolpath(result.ops, ax)

    ax.plot(
        [],
        [],
        linestyle="--",
        linewidth=0.5,
        color="dimgray",
        alpha=0.8,
        label="Travel",
    )
    ax.set_title("profile_inner — narrow channel skips blocked island (2D)")
    handles, labels = ax.get_legend_handles_labels()
    seen = set()
    unique = []
    for h, lbl in zip(handles, labels):
        if lbl not in seen:
            unique.append((h, lbl))
            seen.add(lbl)
    ax.legend(*zip(*unique), loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    fig.tight_layout()
    return fig


def generate_profile_inner_rough_then_finish():
    boundary = _rect(0, 0, 60, 60)
    island = _rect(15, 0, 10, 10)
    ca = ClearedArea(boundary=boundary, initial=[])

    result_rough = profile_inner(
        cleared=ca,
        boundary=boundary,
        islands=[island],
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
        cleared=ca,
        boundary=boundary,
        islands=[island],
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
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Boundary")

    ix = [p[0] for p in island] + [island[0][0]]
    iy = [p[1] for p in island] + [island[0][1]]
    ax.fill(ix, iy, color="lightgray", alpha=0.6, label="Island")

    from raygeo.ops import Ops

    combined = Ops()
    combined.extend(result_rough.ops)
    finish_start = result_finish.ops.endpoint(0)
    combined.move_to(finish_start[0], finish_start[1], 2.0, None)
    combined.extend(result_finish.ops)

    _plot_2d_toolpath(combined, ax)

    ax.set_title("profile_inner — rough + finish (turbo) with travel link")
    ax.plot(
        [],
        [],
        linestyle="--",
        linewidth=0.5,
        color="dimgray",
        alpha=0.8,
        label="Travel",
    )
    ax.legend(loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": "profile_outer",
        "caption": (
            "profile_outer on a 60×60 rectangular pocket — 2D top-down "
            "view. Black: source boundary. Blue dashed: offset tool-centre "
            "polygon. Turbo gradient: cut moves. Gray dashes: travel."
        ),
        "function": generate_profile_outer_rect_2d,
    },
    {
        "heading": "profile_outer",
        "caption": (
            "profile_outer on a 60×60 rectangular pocket — 3D view "
            "showing cut path at cut_z and rapids at safe_z."
        ),
        "function": generate_profile_outer_rect_3d,
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
            "Two-pass profiling: rough pass with stock_to_leave=0.5 "
            "(orange) followed by finish pass with stock_to_leave=0.0 "
            "(red) on the same ClearedArea."
        ),
        "function": generate_profile_outer_rough_then_finish,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "profile_inner on a 60×60 pocket with a square island — 2D "
            "top-down. Black: boundary. Gray: island. Blue dashed: inset "
            "outer walk. Orange dashed: grown island walk. Turbo: cuts."
        ),
        "function": generate_profile_inner_rect_with_square_island_2d,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "profile_inner with two accessible islands — nearest-neighbour "
            "ordering visible via the turbo gradient."
        ),
        "function": generate_profile_inner_rect_with_two_islands_2d,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "profile_inner on an L-shaped pocket with an island — 3D view "
            "showing cut path at cut_z and rapids at safe_z."
        ),
        "function": generate_profile_inner_concave_with_island_3d,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "profile_inner skips an island when the channel between "
            "island and wall is narrower than 2×tool_radius."
        ),
        "function": generate_profile_inner_narrow_channel_skips_island,
    },
    {
        "heading": "profile_inner",
        "caption": (
            "Two-pass inner profiling: rough with stock_to_leave=0.5 "
            "(orange) + finish with stock_to_leave=0.0 (red) on the same "
            "ClearedArea."
        ),
        "function": generate_profile_inner_rough_then_finish,
    },
]
