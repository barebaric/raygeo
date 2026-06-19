"""Generate ops example images."""

import math

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType
from tools.plot import plot_ops


def _plot_merged(ax, orig, ops, title):
    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if pos != ep:
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot([], [], color="forestgreen", linewidth=2.5, label="Merged")
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    ax.set_title(title)
    xl = ax.get_xlim()
    yl = ax.get_ylim()
    if abs(yl[1] - yl[0]) < 1:
        pad = max(abs(xl[1] - xl[0]) * 0.1, 5.0)
        ax.set_ylim(-pad, pad)


def _plot(ax, seq, title, travel_d):
    seq.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(seq.len()):
        ct = seq.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = seq.endpoint(i)
            if pos != ep:
                ax.annotate(
                    "",
                    xy=(ep[0], ep[1]),
                    xytext=(pos[0], pos[1]),
                    arrowprops=dict(
                        arrowstyle="->", color="gray", lw=1.5, linestyle=":"
                    ),
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = seq.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="steelblue",
                linewidth=3,
                solid_capstyle="round",
            )
            pos = ep
    ax.plot([], [], color="steelblue", linewidth=3, label="Cut")
    ax.plot([], [], color="gray", linewidth=1.5, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    ax.set_title(f"{title}\nTravel: {travel_d:.1f}", fontsize=12)


def generate_clip_rect():
    """Clip rect."""
    ops = Ops()
    ops.set_power(1.0)
    ops.move_to(10, 10, 0)
    ops.line_to(90, 10, 0)
    ops.line_to(90, 90, 0)
    ops.line_to(10, 90, 0)
    ops.line_to(10, 10, 0)
    ops.move_to(30, 30, 0)
    ops.line_to(70, 30, 0)
    ops.line_to(70, 70, 0)
    ops.line_to(30, 70, 0)
    ops.line_to(30, 30, 0)
    ops.move_to(20, 40, 0)
    ops.line_to(80, 40, 0)
    ops.line_to(50, 80, 0)
    ops.line_to(20, 40, 0)

    clip_rect = (25.0, 25.0, 75.0, 85.0)

    clipped = ops.clip_rect(clip_rect)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))

    def _plot_ops(ax, seq, title):
        seq.preload_state()
        pos = (0.0, 0.0, 0.0)
        for i in range(seq.len()):
            ct = seq.command_type(i)
            if ct == CommandType.MOVE_TO:
                pos = seq.endpoint(i)
                continue
            if ct == CommandType.LINE_TO:
                ep = seq.endpoint(i)
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="steelblue",
                    linewidth=2.5,
                    solid_capstyle="round",
                )
                pos = ep
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3)
        ax.set_title(title, fontsize=13)

    _plot_ops(ax1, ops, "Original paths")
    ax1.add_patch(
        Rectangle(
            (clip_rect[0], clip_rect[1]),
            clip_rect[2] - clip_rect[0],
            clip_rect[3] - clip_rect[1],
            fill=False,
            edgecolor="tomato",
            linewidth=2,
            linestyle="--",
            label="Clip rect",
        )
    )
    ax1.legend(fontsize=10)

    _plot_ops(ax2, clipped, "After clip_rect")
    ax2.add_patch(
        Rectangle(
            (clip_rect[0], clip_rect[1]),
            clip_rect[2] - clip_rect[0],
            clip_rect[3] - clip_rect[1],
            fill=False,
            edgecolor="tomato",
            linewidth=2,
            linestyle="--",
            label="Clip rect",
        )
    )
    ax2.legend(fontsize=10)

    fig.tight_layout()
    return fig


def generate_lead_in_out():
    """Lead-in-out."""
    lead_in = 5.0
    lead_out = 5.0

    ops2 = Ops()
    ops2.set_power(1.0)
    ops2.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops2.move_to(20, 20, 0)
    ops2.line_to(80, 20, 0)
    ops2.line_to(80, 80, 0)
    ops2.line_to(20, 80, 0)
    ops2.line_to(20, 20, 0)
    ops2.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig2 = ops2.copy()
    ops2.apply_lead_in_out(lead_in, lead_out)

    fig2, ax_lead = plt.subplots(figsize=(10, 10))

    orig2.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig2.len()):
        ct = orig2.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig2.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig2.endpoint(i)
            ax_lead.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops2.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops2.len()):
        ct = ops2.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops2.endpoint(i)
            if pos != ep:
                ax_lead.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops2.endpoint(i)
            state = ops2.state(i)
            color = (
                "dodgerblue" if state and state.power < 0.01 else "forestgreen"
            )
            ax_lead.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=color,
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax_lead.plot(
        [], [], color="tomato", linewidth=5, alpha=0.35, label="Original"
    )
    ax_lead.plot(
        [], [], color="forestgreen", linewidth=2.5, label="Cut (power > 0)"
    )
    ax_lead.plot(
        [], [], color="dodgerblue", linewidth=2.5, label="Lead (power = 0)"
    )
    ax_lead.plot(
        [], [], color="gray", linewidth=0.7, linestyle=":", label="Travel"
    )
    ax_lead.set_aspect("equal")
    ax_lead.grid(True, alpha=0.3)
    ax_lead.legend(fontsize=10)

    fig2.tight_layout()
    return fig2


def generate_merge_lines():
    """Merge lines."""
    presets = [
        ("Near-duplicate lines", "nd"),
        ("Adjacent rectangles", "adj"),
    ]

    fig3, axes3 = plt.subplots(1, 2, figsize=(16, 6))

    for idx, (name, key) in enumerate(presets):
        ops3 = Ops()
        ops3.set_power(1.0)
        if key == "nd":
            ops3.move_to(0, 0)
            ops3.line_to(100, 0)
            ops3.move_to(0, 1.5)
            ops3.line_to(100, 1.5)
            ops3.move_to(0, 5)
            ops3.line_to(100, 5)
            tol = 2.0
        else:
            ops3.move_to(0, 0)
            ops3.line_to(100, 0)
            ops3.line_to(100, 100)
            ops3.line_to(0, 100)
            ops3.line_to(0, 0)
            ops3.move_to(100, 0)
            ops3.line_to(200, 0)
            ops3.line_to(200, 100)
            ops3.line_to(100, 100)
            ops3.line_to(100, 0)
            tol = 1.0

        orig3 = ops3.copy()
        ops3.merge_overlapping_lines(tol)
        _plot_merged(axes3[idx], orig3, ops3, f"{name} (tol={tol})")

    fig3.tight_layout()
    return fig3


def generate_optimize_travel():
    """Optimize travel."""
    ops4 = Ops()
    ops4.set_power(1.0)
    ops4.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")

    ops4.move_to(10, 10, 0)
    ops4.line_to(40, 10, 0)
    ops4.move_to(60, 70, 0)
    ops4.line_to(80, 50, 0)
    ops4.move_to(30, 80, 0)
    ops4.line_to(50, 80, 0)
    ops4.line_to(50, 60, 0)
    ops4.move_to(70, 20, 0)
    ops4.line_to(90, 20, 0)
    ops4.line_to(90, 40, 0)

    ops4.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig4 = ops4.copy()
    ops_noflip = ops4.copy()
    ops_flip = ops4.copy()
    ops_noflip.optimize_travel(allow_flip=False)
    ops_flip.optimize_travel(allow_flip=True)

    before_travel = orig4.distance() - orig4.cut_distance()
    travel_noflip = ops_noflip.distance() - ops_noflip.cut_distance()
    travel_flip = ops_flip.distance() - ops_flip.cut_distance()

    fig4, (ax4_1, ax4_2, ax4_3) = plt.subplots(1, 3, figsize=(22, 7))

    _plot(ax4_1, orig4, "Before optimization", before_travel)
    _plot(ax4_2, ops_noflip, "Optimized (no flip)", travel_noflip)
    _plot(ax4_3, ops_flip, "Optimized (with flip)", travel_flip)

    fig4.tight_layout()
    return fig4


def generate_overscan():
    """Overscan."""
    ops5 = Ops()
    ops5.set_power(1.0)
    ops5.ops_section_start(SectionType.RASTER_FILL, "wp1")
    ops5.move_to(10, 10, 0)
    ops5.line_to(90, 10, 0)
    ops5.move_to(10, 20, 0)
    ops5.line_to(90, 20, 0)
    ops5.move_to(10, 30, 0)
    ops5.line_to(90, 30, 0)
    ops5.ops_section_end(SectionType.RASTER_FILL)

    dist = 5.0
    orig5 = ops5.copy()
    ops5.apply_overscan(dist)

    fig5, ax_scan = plt.subplots(figsize=(12, 8))

    orig5.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig5.len()):
        ct = orig5.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig5.endpoint(i)
            continue
        if ct in (CommandType.LINE_TO, CommandType.SCAN_LINE):
            ep = orig5.endpoint(i)
            ax_scan.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops5.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops5.len()):
        ct = ops5.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops5.endpoint(i)
            if pos != ep:
                ax_scan.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct in (CommandType.LINE_TO, CommandType.SCAN_LINE):
            ep = ops5.endpoint(i)
            ax_scan.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax_scan.plot(
        [], [], color="tomato", linewidth=5, alpha=0.35, label="Original"
    )
    ax_scan.plot(
        [], [], color="forestgreen", linewidth=2.5, label="With overscan"
    )
    ax_scan.plot(
        [], [], color="gray", linewidth=0.7, linestyle=":", label="Travel"
    )
    ax_scan.set_aspect("equal")
    ax_scan.grid(True, alpha=0.3)
    ax_scan.legend(fontsize=10)
    ax_scan.set_title(f"Overscan distance: {dist} mm")

    fig5.tight_layout()
    return fig5


def generate_tab_operations():
    """Tab operations."""
    cx, cy = 10, 10
    w, h = 20, 20

    ops6 = Ops()
    ops6.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops6.set_power(1.0)
    ops6.move_to(cx - w / 2, cy - h / 2, 0)
    ops6.line_to(cx + w / 2, cy - h / 2, 0)
    ops6.line_to(cx + w / 2, cy + h / 2, 0)
    ops6.line_to(cx - w / 2, cy + h / 2, 0)
    ops6.close_path()
    ops6.ops_section_end(SectionType.VECTOR_OUTLINE)

    geo = ops6.to_geometry()
    segments = geo.segments()
    seg_dists = []
    for seg in segments:
        for j in range(1, len(seg)):
            dx = seg[j][0] - seg[j - 1][0]
            dy = seg[j][1] - seg[j - 1][1]
            seg_dists.append(math.sqrt(dx * dx + dy * dy))
    total_dist = sum(seg_dists)

    n_tabs = 2
    tab_width = 2.0
    clips = []
    if total_dist > 0 and n_tabs > 0:
        flat_pts = [p for seg in segments for p in seg]
        for t in range(n_tabs):
            target = total_dist * (t + 1) / (n_tabs + 1)
            accum = 0.0
            for seg_i, sd in enumerate(seg_dists):
                if accum + sd >= target - 1e-9:
                    frac = (target - accum) / sd if sd > 1e-9 else 0.0
                    px = flat_pts[seg_i][0] + frac * (
                        flat_pts[seg_i + 1][0] - flat_pts[seg_i][0]
                    )
                    py = flat_pts[seg_i][1] + frac * (
                        flat_pts[seg_i + 1][1] - flat_pts[seg_i][1]
                    )
                    clips.append((px, py, tab_width))
                    break
                accum += sd

    result_ops = ops6.copy()
    if clips:
        result_ops.apply_tab_gaps(clips)

    fig6, axes6 = plt.subplots(1, 2, figsize=(14, 6))
    axes6[0].set_title("Original")
    plot_ops(axes6[0], ops6, color="steelblue")
    for cx_, cy_, tw_ in clips:
        axes6[0].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
        axes6[0].add_patch(
            mpatches.Circle(
                (cx_, cy_),
                tw_ / 2,
                fill=False,
                color="red",
                linestyle="--",
                linewidth=1,
            )
        )
    axes6[0].set_aspect("equal")
    axes6[0].grid(True, alpha=0.3)

    axes6[1].set_title("After Gap Tabs")
    plot_ops(axes6[1], result_ops, color="steelblue")
    for cx_, cy_, tw_ in clips:
        axes6[1].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
    axes6[1].set_aspect("equal")
    axes6[1].grid(True, alpha=0.3)

    fig6.tight_layout()
    return fig6


__images__ = [
    {
        "heading": "clip_rect",
        "caption": "Ops paths clipped to a rectangle",
        "function": generate_clip_rect,
    },
    {
        "heading": "apply_lead_in_out",
        "caption": "Lead-in and lead-out paths",
        "function": generate_lead_in_out,
    },
    {
        "heading": "merge_overlapping_lines",
        "caption": "Line merging before and after",
        "function": generate_merge_lines,
    },
    {
        "heading": "optimize_travel",
        "caption": "Travel path before and after optimization",
        "function": generate_optimize_travel,
    },
    {
        "heading": "apply_overscan",
        "caption": "Overscan applied to raster lines",
        "function": generate_overscan,
    },
    {
        "heading": "apply_tab_gaps",
        "caption": "Tab operations on a rectangle",
        "function": generate_tab_operations,
    },
]
