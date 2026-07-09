"""Generate examples for polyline operations."""

import math

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.shape.line import get_interior_angle
from raygeo.geo.shape.polyline import (
    get_polyline_closest_point,
    split_polyline_at_v_junctions,
    trim_polyline_angular_ends,
    trim_polyline_at,
)


def generate_polyline_closest_point():
    """Show closest point on an open polyline."""
    polyline = [
        (2.0, 12.0),
        (5.0, 14.0),
        (8.0, 10.0),
        (11.0, 13.0),
        (14.0, 10.0),
        (17.0, 14.0),
    ]
    queries = [
        (4.0, 12.5),
        (10.0, 11.0),
        (15.0, 13.0),
    ]

    fig, ax = plt.subplots(figsize=(8, 6))
    arr = np.array(polyline)
    ax.plot(
        arr[:, 0],
        arr[:, 1],
        "-o",
        color="gray",
        lw=2,
        alpha=0.6,
        label="Polyline",
    )

    for q in queries:
        res = get_polyline_closest_point(polyline, q)
        if res is None:
            continue
        ei, t = res
        p1 = np.array(polyline[ei])
        p2 = np.array(polyline[ei + 1])
        cp = p1 + (p2 - p1) * t

        ax.plot(q[0], q[1], "o", color="steelblue", ms=8)
        ax.plot(cp[0], cp[1], "r*", ms=12)
        ax.plot([q[0], cp[0]], [q[1], cp[1]], "-", color="crimson", alpha=0.5)

    ax.plot([], [], "o", color="steelblue", label="Query point")
    ax.plot([], [], "r*", ms=12, label="Closest point on polyline")
    ax.plot([], [], "-", color="crimson", alpha=0.5, label="Distance")

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=11)
    ax.set_title("get_polyline_closest_point — Open polyline", fontsize=13)
    fig.tight_layout()
    return fig


def generate_trim_polyline():
    """Trim a polyline between two points."""
    polyline = [
        (2.0, 12.0),
        (5.0, 14.0),
        (8.0, 10.0),
        (11.0, 13.0),
        (14.0, 10.0),
        (17.0, 14.0),
    ]
    a = (4.0, 13.0)
    b = (15.0, 11.5)

    trimmed = trim_polyline_at(polyline, a, b)

    fig, ax = plt.subplots(figsize=(8, 6))
    arr = np.array(polyline)
    ax.plot(
        arr[:, 0],
        arr[:, 1],
        "-o",
        color="gray",
        lw=1.5,
        alpha=0.5,
        label="Original",
    )
    ax.plot(*arr.T, "o", color="gray", ms=4, alpha=0.5)

    trimmed_arr = np.array(trimmed)
    ax.plot(
        trimmed_arr[:, 0],
        trimmed_arr[:, 1],
        "-o",
        color="#e41a1c",
        lw=2.5,
        label="Trimmed",
    )

    ax.plot(a[0], a[1], "s", color="green", ms=10, label="A")
    ax.plot(b[0], b[1], "s", color="blue", ms=10, label="B")

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=11)
    ax.set_title("trim_polyline_at", fontsize=13)
    fig.tight_layout()
    return fig


def generate_trim_polyline_angular_ends():
    """Trim transition vertices from the ends of a subsequence."""
    poly = [
        (0.0, 8.0),
        (0.0, 0.0),
        (2.5, 1.5),
        (5.0, 0.3),
        (7.5, 0.0),
        (10.0, 0.0),
        (12.5, 0.0),
        (15.0, 0.0),
        (17.5, 0.0),
        (20.0, -0.3),
        (22.5, 1.5),
        (25.0, 0.0),
        (25.0, 8.0),
    ]
    n = len(poly)
    idxs = list(range(n))
    threshold_rad = math.radians(25)
    threshold_deg = 25

    cut_start_in = 1
    cut_len_in = n - 3

    (new_start, new_len) = trim_polyline_angular_ends(
        poly,
        cut_start_in,
        cut_len_in,
        threshold_rad,
    )

    cut_before = list(range(cut_start_in, cut_start_in + cut_len_in))
    cut_after = list(range(new_start, new_start + new_len))
    trimmed = [i for i in cut_before if i not in cut_after]

    angles = [
        math.degrees(
            get_interior_angle(poly[(i - 1) % n], poly[i], poly[(i + 1) % n])
        )
        for i in range(n)
    ]

    fig = plt.figure(figsize=(12, 8))
    gs = fig.add_gridspec(1, 2, width_ratios=[1, 1], wspace=0.3)
    ax1 = fig.add_subplot(gs[0])
    ax2 = fig.add_subplot(gs[1])

    def draw_panel(ax, ids, label_drop, title_text):
        arr = np.array(poly)

        closed = np.vstack([arr, arr[0:1]])
        ax.plot(closed[:, 0], closed[:, 1], "-", color="#ccc", lw=1, zorder=1)
        ax.plot(arr[:, 0], arr[:, 1], "o", color="#ccc", ms=4, zorder=2)

        for k in range(len(ids) - 1):
            a = poly[ids[k]]
            b = poly[ids[k + 1]]
            ax.plot(
                [a[0], b[0]],
                [a[1], b[1]],
                "-",
                color="#d62728",
                lw=3.5,
                zorder=3,
            )
        cut_pts = np.array([poly[i] for i in ids])
        ax.plot(
            cut_pts[:, 0],
            cut_pts[:, 1],
            "o",
            color="#d62728",
            ms=8,
            zorder=4,
        )

        for i in label_drop:
            p = poly[i]
            ax.plot(p[0], p[1], "x", color="#888", ms=10, mew=2.5, zorder=5)
            ax.annotate(
                "trimmed",
                xy=(p[0], p[1]),
                xytext=(p[0] + (1.5 if i > 3 else -1.5), p[1] - 1.8),
                fontsize=8,
                color="#666",
                ha="center",
                arrowprops=dict(arrowstyle="->", color="#888", lw=0.8),
            )

        for i in idxs:
            p = poly[i]
            ang = angles[i]
            inside = i in ids
            clr = "#d62728" if inside else "#aaa"
            ox = -2.8 if p[0] < 3 else (2.2 if p[0] > 17 else 0.6)
            oy = 1.6 if p[1] >= 0 else -2.2
            ax.annotate(
                f"{ang:.0f}°",
                xy=p,
                xytext=(p[0] + ox, p[1] + oy),
                fontsize=9,
                color=clr,
                fontweight="bold" if inside else "normal",
                bbox=dict(
                    boxstyle="round,pad=0.15",
                    fc="white" if inside else "none",
                    ec="none",
                    alpha=0.85,
                ),
            )

        if len(ids) >= 3:
            i2, i3 = ids[1], ids[2]
            mid = (
                (poly[i2][0] + poly[i3][0]) / 2,
                (poly[i2][1] + poly[i3][1]) / 2,
            )
            ax.annotate(
                f"Δ {abs(angles[i3] - angles[i2]):.0f}°",
                xy=mid,
                xytext=(mid[0], mid[1] + (2.0 if poly[i2][1] >= 0 else -2.0)),
                fontsize=8,
                color="#d62728",
                ha="center",
                fontweight="bold",
                arrowprops=dict(
                    arrowstyle="->",
                    color="#d62728",
                    lw=1.2,
                    connectionstyle="arc3,rad=0",
                ),
            )
            i_a, i_b = ids[-3], ids[-2]
            mid2 = (
                (poly[i_a][0] + poly[i_b][0]) / 2,
                (poly[i_a][1] + poly[i_b][1]) / 2,
            )
            ax.annotate(
                f"Δ {abs(angles[i_b] - angles[i_a]):.0f}°",
                xy=mid2,
                xytext=(
                    mid2[0],
                    mid2[1] + (2.0 if poly[i_b][1] >= 0 else -2.0),
                ),
                fontsize=8,
                color="#d62728",
                ha="center",
                fontweight="bold",
                arrowprops=dict(
                    arrowstyle="->",
                    color="#d62728",
                    lw=1.2,
                    connectionstyle="arc3,rad=0",
                ),
            )

        ax.set_aspect("equal")
        ax.set_ylim(-6, 12)
        ax.set_xlim(-3, 28)
        ax.grid(True, alpha=0.15, ls=":")
        ax.set_title(title_text, fontsize=12, fontweight="bold")
        ax.set_xlabel("x")
        ax.set_ylabel("y")

    draw_panel(
        ax1,
        cut_before,
        trimmed,
        "Before:  cut = vertices 1…10",
    )
    draw_panel(
        ax2,
        cut_after,
        [],
        "After:  trimmed cut = vertices 2…9",
    )

    ax1.annotate(
        f"threshold = {threshold_deg}°",
        xy=(0.96, 0.96),
        xycoords="axes fraction",
        va="top",
        ha="right",
        fontsize=9,
        bbox=dict(boxstyle="round,pad=0.3", fc="#fff9c4", ec="#f9a825"),
    )
    annot_after = (
        f"cut index {cut_start_in}→{new_start}\n"
        f"cut length {cut_len_in}→{new_len}\n"
        f"dropped vertices: {trimmed}"
    )
    ax2.annotate(
        annot_after,
        xy=(0.96, 0.96),
        xycoords="axes fraction",
        va="top",
        ha="right",
        fontsize=9,
        family="monospace",
        bbox=dict(boxstyle="round,pad=0.3", fc="#e8f5e9", ec="#43a047"),
    )

    fig.suptitle(
        "trim_polyline_angular_ends",
        fontsize=14,
        fontweight="bold",
        y=1.02,
    )
    fig.subplots_adjust(left=0.05, right=0.98, bottom=0.06, top=0.93)
    return fig


def generate_split_v_junctions():
    n = 25

    def semi_arc(x0, y0, x1, y1, n):
        pts = []
        cx = (x0 + x1) / 2
        r = abs(x1 - x0) / 2
        amp = r * 0.2
        for i in range(n):
            t = i / (n - 1)
            a = math.pi * t
            x = cx - r * math.cos(a)
            y = y0 + amp * math.sin(a)
            pts.append((x, round(y, 6)))
        return pts

    hill1 = semi_arc(10, 30, 40, 30, n)
    hill2 = semi_arc(40, 30, 70, 30, n)
    hill3 = semi_arc(70, 30, 100, 30, n)

    polyline = hill1 + hill2[1:] + hill3[1:]
    vj1_idx = len(hill1)
    vj2_idx = len(hill1) + len(hill2) - 1

    angle_thresh = math.radians(25)
    segments = split_polyline_at_v_junctions(polyline, angle_thresh)

    fig, axes = plt.subplots(1, 2, figsize=(12, 5))

    xs = [p[0] for p in polyline]
    ys = [p[1] for p in polyline]
    axes[0].plot(xs, ys, "-", color="steelblue", linewidth=2, alpha=0.7)
    axes[0].plot(xs, ys, "o", color="steelblue", markersize=2)
    for idx, label in [(vj1_idx, "V₁"), (vj2_idx, "V₂")]:
        axes[0].plot(
            xs[idx], ys[idx], "v", color="red", markersize=12, zorder=5
        )
        axes[0].annotate(
            label,
            (xs[idx], ys[idx]),
            xytext=(0, -18),
            textcoords="offset points",
            ha="center",
            fontsize=11,
            fontweight="bold",
            color="red",
        )
    axes[0].set_title(
        f"Original polyline — {len(polyline)} pts, 2 V-junctions (▼)"
    )
    axes[0].set_aspect("equal")
    axes[0].set_xlim(5, 105)
    axes[0].set_ylim(25, 50)
    axes[0].grid(True, alpha=0.3)

    cmap = plt.get_cmap("tab10")
    for si, seg in enumerate(segments):
        xs = [p[0] for p in seg]
        ys = [p[1] for p in seg]
        axes[1].plot(
            xs,
            ys,
            "o-",
            color=cmap(si % 10),
            linewidth=2.5,
            markersize=4,
            label=f"Segment {si + 1}",
        )
    axes[1].set_title(f"After split — {len(segments)} segments")
    axes[1].set_aspect("equal")
    axes[1].set_xlim(5, 105)
    axes[1].set_ylim(25, 50)
    axes[1].grid(True, alpha=0.3)
    axes[1].legend(fontsize=9)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.shape.polyline.md"]
__images__ = [
    {
        "heading": "get_polyline_closest_point",
        "caption": (
            "``get_polyline_closest_point`` finds the closest point on an open"
            " polyline to a query point"
        ),
        "function": generate_polyline_closest_point,
    },
    {
        "heading": "trim_polyline_at",
        "caption": "``trim_polyline_at`` trims a polyline between two points",
        "function": generate_trim_polyline,
    },
    {
        "heading": "trim_polyline_angular_ends",
        "caption": (
            "``trim_polyline_angular_ends`` removes transition vertices"
            " from subseq ends at sharp angle jumps"
        ),
        "function": generate_trim_polyline_angular_ends,
    },
    {
        "heading": "split_polyline_at_v_junctions",
        "caption": (
            "Three semi-arcs form two V-junctions; splits and trims each"
            " segment's angular ends"
        ),
        "function": generate_split_v_junctions,
    },
]
