"""Visualisation for ops/feature/slot_path — slot carrier finder."""

import math

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Circle as CirclePatch

from raygeo.geo.shape.polygon import (
    JoinStyle,
    offset_polygon,
)
from raygeo.ops.feature.slot_path import find_slot_path


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _build_smooth_s(centerline, half_width):
    """Corridor polygon around a planar centerline.

    The outline is traced CCW: left side forward, right side reversed.
    """
    left = []
    right = []
    n = len(centerline)
    for i, (cx, cy) in enumerate(centerline):
        if i < n - 1:
            dx = centerline[i + 1][0] - cx
            dy = centerline[i + 1][1] - cy
        else:
            dx = cx - centerline[i - 1][0]
            dy = cy - centerline[i - 1][1]
        L = math.hypot(dx, dy) or 1.0
        nx, ny = dx / L, dy / L
        px, py = -ny, nx
        left.append((cx + half_width * px, cy + half_width * py))
        right.append((cx - half_width * px, cy - half_width * py))
    return left + list(reversed(right))


def _plot_polygon(ax, poly, face, edge, **kwargs):
    xs = [p[0] for p in poly] + [poly[0][0]]
    ys = [p[1] for p in poly] + [poly[0][1]]
    ax.fill(xs, ys, facecolor=face, alpha=kwargs.get("alpha", 0.25))
    ax.plot(xs, ys, "-", color=edge, linewidth=kwargs.get("lw", 1.5))


def _plot_carrier(ax, carrier, color, label_pts=False):
    xs = [p[0] for p in carrier]
    ys = [p[1] for p in carrier]
    ax.plot(xs, ys, "-", color=color, linewidth=2.5, zorder=4)
    ax.plot(xs[0], ys[0], "o", color=color, ms=7, zorder=5)
    ax.plot(xs[-1], ys[-1], "s", color=color, ms=7, zorder=5)
    if len(carrier) > 2:
        ax.plot(xs[1:-1], ys[1:-1], ".", color=color, ms=4, zorder=5)
    if label_pts and len(carrier) >= 2:
        ax.annotate(
            f"near ({xs[0]:.1f}, {ys[0]:.1f})",
            (xs[0], ys[0]),
            textcoords="offset points",
            xytext=(8, -14),
            fontsize=8,
            color=color,
        )
        ax.annotate(
            f"far ({xs[-1]:.1f}, {ys[-1]:.1f})",
            (xs[-1], ys[-1]),
            textcoords="offset points",
            xytext=(8, -14),
            fontsize=8,
            color=color,
        )


def _plot_entry_edge(ax, slot_polygon, entry_edge_idx, color):
    """Highlight an entry edge of the slot polygon."""
    n = len(slot_polygon)
    p0 = slot_polygon[entry_edge_idx]
    p1 = slot_polygon[(entry_edge_idx + 1) % n]
    ax.plot(
        [p0[0], p1[0]],
        [p0[1], p1[1]],
        "-",
        color=color,
        linewidth=3.5,
        alpha=0.85,
        zorder=3,
    )


def _scenario(
    ax,
    title,
    slot_polygon,
    entry_edges,
    entry_point,
    tool_radius,
    eroded_polygons=None,
):
    """Plot a slot polygon, its entry edge (highlighted), the requested
    entry point, and the carrier returned by `find_slot_path`. The
    eroded slot is drawn as a hatched region so the carrier's fit is
    visible.
    """
    _plot_polygon(ax, slot_polygon, "#fafafa", "k", lw=1.8, alpha=0.0)

    if eroded_polygons is not None:
        for ep in eroded_polygons:
            _plot_polygon(ax, ep, "#dddddd", "gray", lw=1.0, alpha=0.5)
    else:
        xs = [p[0] for p in slot_polygon]
        ys = [p[1] for p in slot_polygon]
        x0, x1 = min(xs), max(xs)
        y0, y1 = min(ys), max(ys)
        eroded = [
            (x0 + tool_radius, y0 + tool_radius),
            (x1 - tool_radius, y0 + tool_radius),
            (x1 - tool_radius, y1 - tool_radius),
            (x0 + tool_radius, y1 - tool_radius),
        ]
        exs = [p[0] for p in eroded] + [eroded[0][0]]
        eys = [p[1] for p in eroded] + [eroded[0][1]]
        ax.plot(exs, eys, ":", color="gray", linewidth=1.2, alpha=0.7)

    _plot_entry_edge(ax, slot_polygon, entry_edges[0], "crimson")

    ax.plot(
        entry_point[0],
        entry_point[1],
        "x",
        color="crimson",
        ms=10,
        mew=2,
        zorder=6,
    )
    ax.annotate(
        "entry_point",
        entry_point,
        textcoords="offset points",
        xytext=(8, 8),
        fontsize=8,
        color="crimson",
    )

    result = find_slot_path(
        slot_polygon=slot_polygon,
        entry_edges=entry_edges,
        entry_point=entry_point,
        tool_radius=tool_radius,
    )

    if result is not None:
        _plot_carrier(ax, result, "navy", label_pts=True)
        for p in result:
            ax.add_patch(
                CirclePatch(
                    p,
                    tool_radius,
                    fill=False,
                    edgecolor="navy",
                    linestyle="--",
                    linewidth=1.0,
                    alpha=0.7,
                    zorder=4,
                )
            )
        total_len = sum(
            math.dist(result[i], result[i + 1]) for i in range(len(result) - 1)
        )
        status = f"carrier: {len(result)} pts, {total_len:.1f} mm path"
    else:
        status = "None (slot too narrow for tool)"

    ax.set_title(title, fontsize=11)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.25)
    ax.text(
        0.5,
        -0.18,
        status,
        transform=ax.transAxes,
        ha="center",
        fontsize=9,
        color="navy" if result is not None else "crimson",
    )


def generate_slot_path_scenarios():
    """Four slot scenarios showing find_slot_path.

    - Horizontal slot (40x7 mm): the carrier is the long-axis centred
      segment through the eroded slot.
    - Vertical slot (7x40 mm): the long axis flips to y; carrier is
      vertical.
    - Too-narrow slot (30x5 mm) with tool_radius=3: the eroded region is
      empty, so the function returns None.
    - Sinusoidal S-slot (6 mm corridor, r=2): the disk-probe snake walk
      follows the S-curve from bottom to top without crossing empty
      space or running outside the eroded region.
    """
    tool_radius = 3.0
    fig, axes = plt.subplots(2, 2, figsize=(13, 11))
    axes = axes.flatten()

    _scenario(
        axes[0],
        "Horizontal slot (40x7 mm)",
        _rect(0, 0, 40, 7),
        entry_edges=[3],
        entry_point=(0, 3.5),
        tool_radius=tool_radius,
    )
    axes[0].set_xlim(-7, 47)
    axes[0].set_ylim(-6, 14)

    _scenario(
        axes[1],
        "Vertical slot (7x40 mm)",
        _rect(0, 0, 7, 40),
        entry_edges=[0],
        entry_point=(0, 0),
        tool_radius=tool_radius,
    )
    axes[1].set_xlim(-6, 14)
    axes[1].set_ylim(-7, 47)

    _scenario(
        axes[2],
        "Too-narrow slot (30x5 mm, r=3)",
        _rect(0, 0, 30, 5),
        entry_edges=[3],
        entry_point=(0, 2.5),
        tool_radius=tool_radius,
    )
    axes[2].set_xlim(-7, 37)
    axes[2].set_ylim(-6, 14)

    s_radius = 2.0
    centerline = [
        (10 + 7 * math.sin(2 * math.pi * y / 30), y) for y in range(0, 45)
    ]
    s_slot = _build_smooth_s(centerline, half_width=3.0)
    eroded_s = offset_polygon(s_slot, -s_radius, JoinStyle.Miter)
    closing_edge = len(s_slot) - 1
    _scenario(
        axes[3],
        "Sinusoidal S-slot (6 mm corridor, r=2)",
        s_slot,
        entry_edges=[closing_edge],
        entry_point=centerline[0],
        tool_radius=s_radius,
        eroded_polygons=eroded_s,
    )
    axes[3].set_xlim(-6, 26)
    axes[3].set_ylim(-3, 47)

    legend_items = [
        Line2D([0], [0], color="k", lw=1.8, label="slot polygon"),
        Line2D(
            [0],
            [0],
            color="gray",
            lw=1.2,
            linestyle=":",
            label="eroded region",
        ),
        Line2D(
            [0],
            [0],
            color="crimson",
            lw=3.5,
            label="entry edge",
        ),
        Line2D(
            [0],
            [0],
            color="crimson",
            lw=0,
            marker="x",
            markersize=10,
            mew=2,
            label="entry_point",
        ),
        Line2D(
            [0],
            [0],
            color="navy",
            lw=2.5,
            label="slot carrier (near \u25cf / far \u25a0)",
        ),
        Line2D(
            [0],
            [0],
            color="navy",
            lw=1.0,
            linestyle="--",
            label="tool disk",
        ),
    ]
    fig.legend(
        handles=legend_items,
        loc="lower center",
        ncol=3,
        fontsize=9,
        frameon=False,
        bbox_to_anchor=(0.5, -0.02),
    )

    fig.suptitle(
        "find_slot_path - disk-probe snake walk: horizontal, vertical,"
        " too-narrow, and sinusoidal S-curve slots",
        fontsize=13,
        y=1.02,
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.feature.slot_path.md"]
__images__ = [
    {
        "heading": "find_slot_path",
        "caption": (
            "find_slot_path on multiple slots: horizontal, vertical,"
            " too-narrow (None), and sinusoidal S-slot."
        ),
        "function": generate_slot_path_scenarios,
    },
]


if __name__ == "__main__":
    fig = generate_slot_path_scenarios()
    fig.savefig("/tmp/ops_feature_slot_path.png", dpi=150, bbox_inches="tight")
    print("Saved /tmp/ops_feature_slot_path.png")
