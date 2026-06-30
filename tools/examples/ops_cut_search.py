"""Visualisation of search_frontier_engagement — backward wall-hugging search.

Square edge = 50, seed circle diameter = 52 (radius 26), centred on
the square and clipped to the valid tool area.
"""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.legend_handler import HandlerPatch
from matplotlib.patches import FancyArrow

from raygeo.geo.algo.offset import compute_inset_region
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.search import (
    ToolPose,
    search_frontier_engagement,
)

_ARROW_HANDLER = HandlerPatch(
    patch_func=lambda **kw: FancyArrow(
        0,
        kw["height"] / 2,
        kw["width"] * 0.7,
        0,
        width=0.25 * kw["height"],
        head_width=0.8 * kw["height"],
        head_length=0.45 * kw["width"],
        color=kw["orig_handle"].get_facecolor(),
    )
)
"""Legend handler that draws arrows as arrow shapes (not rectangles)."""


# ── Shared geometry setup ─────────────────────────────────────────────


def _circle(cx, cy, r, n=64):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _setup():
    half = 25.0
    square = [(-half, -half), (half, -half), (half, half), (-half, half)]

    tool_radius = 3.0
    step_length = 1.0
    advance = 1.5
    min_cut_area = 0.1

    r_seed = 26.0
    seed_raw = _circle(0.0, 0.0, r_seed, 64)

    va, _ = compute_inset_region(square, tool_radius, [])

    ca = ClearedArea(boundary=square, initial=[seed_raw])

    va_right = half - tool_radius
    y_int = math.sqrt(r_seed**2 - va_right**2)
    tool_pos = (va_right, -y_int)
    heading = math.pi / 4

    return (
        square,
        va,
        ca,
        tool_pos,
        heading,
        tool_radius,
        step_length,
        advance,
        min_cut_area,
        r_seed,
    )


# ── Generators ───────────────────────────────────────────────────────


def generate_search_frontier_engagement():
    """search_frontier_engagement — walk forward along the frontier.

    Places a start point on the right edge of the envelope and walks
    forward (CCW) along the frontier to find the next engaged vertex.
    """

    (
        square,
        va,
        ca,
        tool_pos,
        _heading,
        tool_radius,
        step_length,
        advance,
        min_cut_area,
        _,
    ) = _setup()

    fig, ax = _make_axes(square, va, ca, frontier_tol=0.001)

    # ── Right side: walk CCW from the tool position ──
    start_h = math.pi / 2
    fwd_rp = search_frontier_engagement(
        ca,
        start=ToolPose(pos=tool_pos, heading=start_h),
        radius=tool_radius,
        step_length=step_length,
        advance=advance,
        min_cut_area=min_cut_area,
        max_cut_area=float("inf"),
    )

    if fwd_rp:
        seg_xs = [tool_pos[0], fwd_rp.pos[0]]
        seg_ys = [tool_pos[1], fwd_rp.pos[1]]
        ax.plot(
            seg_xs,
            seg_ys,
            "-",
            color="cyan",
            linewidth=2.5,
            label="Path taken",
        )

    ax.plot(
        tool_pos[0],
        tool_pos[1],
        "o",
        color="lime",
        markersize=12,
        zorder=12,
        label="Start",
    )
    ax.arrow(
        tool_pos[0],
        tool_pos[1],
        math.cos(start_h) * 6,
        math.sin(start_h) * 6,
        head_width=1.5,
        head_length=1.5,
        fc="lime",
        ec="lime",
        zorder=13,
        label="Start dir",
    )

    if fwd_rp:
        ax.plot(
            fwd_rp.pos[0],
            fwd_rp.pos[1],
            "o",
            color="darkorange",
            markersize=12,
            zorder=14,
            label="End",
        )
        ddx = math.cos(fwd_rp.heading) * 6
        ddy = math.sin(fwd_rp.heading) * 6
        ax.arrow(
            fwd_rp.pos[0],
            fwd_rp.pos[1],
            ddx,
            ddy,
            head_width=1.5,
            head_length=1.5,
            fc="darkorange",
            ec="darkorange",
            zorder=15,
            label="End dir",
        )

    # ── Left side: walk CW from the left envelope edge ──
    left_pos = (-22.0, 0.0)
    left_h = math.pi / 2
    left_rp = search_frontier_engagement(
        ca,
        start=ToolPose(pos=left_pos, heading=left_h),
        radius=tool_radius,
        step_length=step_length,
        advance=advance,
        min_cut_area=min_cut_area,
        max_cut_area=float("inf"),
    )

    if left_rp:
        seg_xs = [left_pos[0], left_rp.pos[0]]
        seg_ys = [left_pos[1], left_rp.pos[1]]
        ax.plot(
            seg_xs,
            seg_ys,
            "-",
            color="magenta",
            linewidth=2.5,
            label="Left path",
        )

    ax.plot(
        left_pos[0],
        left_pos[1],
        "o",
        color="purple",
        markersize=12,
        zorder=12,
        label="Left start",
    )
    ax.arrow(
        left_pos[0],
        left_pos[1],
        0,
        6,
        head_width=1.5,
        head_length=1.5,
        fc="purple",
        ec="purple",
        zorder=13,
        label="Left start dir",
    )

    if left_rp:
        ax.plot(
            left_rp.pos[0],
            left_rp.pos[1],
            "o",
            color="magenta",
            markersize=12,
            zorder=14,
            label="Left end",
        )
        ddx = math.cos(left_rp.heading) * 6
        ddy = math.sin(left_rp.heading) * 6
        ax.arrow(
            left_rp.pos[0],
            left_rp.pos[1],
            ddx,
            ddy,
            head_width=1.5,
            head_length=1.5,
            fc="magenta",
            ec="magenta",
            zorder=15,
            label="Left end dir",
        )

    ax.set_title("search_frontier_engagement — walk forward/backward")
    ax.legend(
        loc="upper right",
        fontsize=7,
        ncol=2,
        handler_map={FancyArrow: _ARROW_HANDLER},
    )
    fig.tight_layout()
    return fig


# ── Plot helpers ─────────────────────────────────────────────────────


def _make_axes(square, va, ca, frontier_tol=0.5):
    fig, ax = plt.subplots(figsize=(10, 10))
    bnd = np.array(square + [square[0]])
    ax.plot(
        bnd[:, 0],
        bnd[:, 1],
        "k-",
        linewidth=2,
        alpha=0.5,
        label="Square / pocket",
    )

    for poly in va:
        p = np.array(poly + [poly[0]])
        ax.plot(p[:, 0], p[:, 1], "k--", linewidth=1, alpha=0.3)
        ax.fill(p[:, 0], p[:, 1], alpha=0.03, color="green")

    frontier = ca.frontier(frontier_tol)
    for poly in frontier:
        fp = np.array(poly + [poly[0]])
        ax.plot(
            fp[:, 0],
            fp[:, 1],
            "-",
            color="darkblue",
            linewidth=1.5,
            label="Frontier" if poly is frontier[0] else "_",
        )

    remaining = ca.remaining()
    for poly in remaining:
        rp = np.array(poly + [poly[0]])
        ax.fill(
            rp[:, 0],
            rp[:, 1],
            alpha=0.4,
            facecolor="coral",
            label="Uncut" if poly is remaining[0] else "_",
        )

    ax.set_aspect("equal")
    ax.set_xlim(-35, 35)
    ax.set_ylim(-35, 35)
    ax.axhline(0, color="gray", linewidth=0.5)
    ax.axvline(0, color="gray", linewidth=0.5)
    ax.grid(True, alpha=0.3)
    return fig, ax


def _plot_tool(ax, pos, heading):
    ax.plot(
        pos[0],
        pos[1],
        "D",
        color="red",
        markersize=10,
        zorder=10,
        label="_nolegend_",
    )
    hlen = 8.0
    ax.arrow(
        pos[0],
        pos[1],
        math.cos(heading) * hlen,
        math.sin(heading) * hlen,
        head_width=2,
        head_length=2,
        fc="red",
        ec="red",
        zorder=11,
        label="Tool dir",
    )


def generate_search_frontier_engagement_multi():
    """Resume-point travel in a multi-island pocket."""
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [
            (
                80 + 10 * math.cos(2 * math.pi * i / 32),
                50 + 10 * math.sin(2 * math.pi * i / 32),
            )
            for i in range(32)
        ],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    tool_radius = 3.0

    centre = (90.0, 60.0)
    seed = _circle(120, 30, 40, n=64)
    ca = ClearedArea(boundary=boundary, islands=islands, initial=[seed])

    centre = (90.0, 60.0)

    frontier = ca.frontier(0.5)
    end_positions = []
    for poly in frontier:
        if len(poly) < 4:
            continue
        for idx in (0, len(poly) // 3, 2 * len(poly) // 3):
            fv = poly[idx]
            ep = (
                fv[0] + (centre[0] - fv[0]) * 0.4,
                fv[1] + (centre[1] - fv[1]) * 0.4,
            )
            end_positions.append(ep)
        break

    resume_results = []
    for ep in end_positions:
        r = search_frontier_engagement(
            ca,
            start=ToolPose(pos=ep, heading=0.0),
            radius=tool_radius,
            step_length=0.6,
            advance=1.5,
            min_cut_area=0.1,
            max_cut_area=float("inf"),
        )
        resume_results.append(r)

    fig, ax = plt.subplots(figsize=(8, 6))

    bnd = np.array(boundary + [boundary[0]])
    ax.plot(bnd[:, 0], bnd[:, 1], "k-", linewidth=1.5, label="Pocket")

    ca_env = ca.envelope(tool_radius)
    for i, env in enumerate(ca_env):
        ea = np.array(env + [env[0]])
        ax.plot(
            ea[:, 0],
            ea[:, 1],
            ":",
            color="gray",
            linewidth=1,
            label="Envelope" if i == 0 else "",
        )

    for isl in islands:
        ia = np.array(isl + [isl[0]])
        ax.fill(
            ia[:, 0],
            ia[:, 1],
            facecolor="#ddd",
            edgecolor="#999",
            linewidth=1,
        )

    for i, poly in enumerate(frontier):
        fa = np.array(poly + [poly[0]])
        ax.fill(
            fa[:, 0],
            fa[:, 1],
            color="#2ca02c",
            alpha=0.15,
            label="Cleared area" if i == 0 else "",
        )

    colors = ["#d62728", "#1f77b4", "#ff7f0e"]
    for j, (ep, rr) in enumerate(zip(end_positions, resume_results)):
        ax.plot(ep[0], ep[1], "v", color=colors[j], markersize=10, zorder=5)
        ax.annotate(
            f"End {j + 1}",
            ep,
            xytext=(3, 6),
            textcoords="offset points",
            fontsize=7,
            color=colors[j],
        )

        if rr is not None:
            ax.plot(
                rr.pos[0],
                rr.pos[1],
                "*",
                color=colors[j],
                markersize=14,
                zorder=6,
            )
            dx = math.cos(rr.heading) * 10
            dy = math.sin(rr.heading) * 10
            ax.annotate(
                "",
                xy=(rr.pos[0] + dx, rr.pos[1] + dy),
                xytext=(rr.pos[0], rr.pos[1]),
                arrowprops=dict(
                    arrowstyle="->", color=colors[j], lw=1.5, zorder=7
                ),
            )

    ax.set_aspect("equal")
    ax.set_xlim(-5, 185)
    ax.set_ylim(-5, 125)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("search_frontier_engagement: Multi-Island Pocket")
    ax.legend(fontsize=7, loc="upper right")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.cut.search.md"]
__images__ = [
    {
        "heading": "search_frontier_engagement",
        "caption": (
            "Walk forward from the engagement point to find "
            "the next frontier match."
        ),
        "function": generate_search_frontier_engagement,
    },
    {
        "heading": "search_frontier_engagement",
        "caption": (
            "Multi-island pocket — end positions (triangles) yield "
            "resume positions (stars) with outward headings."
        ),
        "function": generate_search_frontier_engagement_multi,
    },
]
