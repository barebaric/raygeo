"""Visualisation for ops/feature/ramp — ramp carrier finder."""

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Circle as CirclePatch

from raygeo.ops.feature.ramp import find_ramp_carrier


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _plot_polygon(ax, poly, face, edge, **kwargs):
    xs = [p[0] for p in poly] + [poly[0][0]]
    ys = [p[1] for p in poly] + [poly[0][1]]
    ax.fill(xs, ys, facecolor=face, alpha=kwargs.get("alpha", 0.25))
    ax.plot(xs, ys, "-", color=edge, linewidth=kwargs.get("lw", 1.5))


def _plot_tool(ax, point, radius, color, label=None):
    ax.plot(point[0], point[1], "o", color=color, ms=5, zorder=5)
    ax.add_patch(
        CirclePatch(
            point,
            radius,
            fill=False,
            edgecolor=color,
            linestyle="--",
            linewidth=1.2,
            alpha=0.7,
            zorder=4,
        )
    )
    if label:
        ax.annotate(
            label,
            point,
            textcoords="offset points",
            xytext=(8, -12),
            fontsize=8,
            color=color,
        )


def _plot_dashed_rect(ax, poly, color, **kwargs):
    """Draw a dashed outline of a rectangle."""
    xs = [p[0] for p in poly] + [poly[0][0]]
    ys = [p[1] for p in poly] + [poly[0][1]]
    ax.plot(xs, ys, "--", color=color, linewidth=kwargs.get("lw", 1.2))


def generate_ramp_carrier_slots():
    """Three scenarios showing find_ramp_carrier on non-uniform geometry.

    Panel 1 — L-shaped pocket (no islands).
    Panel 2 — Wide rectangle with a central blocking island (the bug
              repro scenario).  The dashed gray rectangle is the dilated
              no-go band; the carrier must pass above or below it.
    Panel 3 — T-shaped pocket with one island in the vertical stem.
    """

    tool_radius = 3.0
    max_ramp_angle_deg = 45.0

    fig, axes = plt.subplots(1, 3, figsize=(16, 5.5))

    # ── Panel 1: L-shaped pocket ──────────────────────────────────────
    ax1 = axes[0]
    L = [(0, 0), (60, 0), (60, 20), (25, 20), (25, 50), (0, 50)]
    _plot_polygon(ax1, L, "#fafafa", "k", lw=1.8, alpha=0.0)

    result = find_ramp_carrier(
        boundary=L,
        islands=None,
        tool_radius=tool_radius,
        max_ramp_angle_deg=max_ramp_angle_deg,
    )
    if result is not None:
        start, end = result
        ax1.plot(
            [start[0], end[0]],
            [start[1], end[1]],
            "-",
            color="navy",
            linewidth=2.5,
            zorder=4,
        )
        _plot_tool(
            ax1,
            start,
            tool_radius,
            "navy",
            label=f"({start[0]:.1f}, {start[1]:.1f})",
        )
        _plot_tool(
            ax1,
            end,
            tool_radius,
            "navy",
            label=f"({end[0]:.1f}, {end[1]:.1f})",
        )

    ax1.set_title("L-shaped pocket", fontsize=11)
    ax1.set_xlabel("X (mm)")
    ax1.set_ylabel("Y (mm)")
    ax1.set_aspect("equal")
    ax1.grid(True, alpha=0.25)
    ax1.set_xlim(-3, 63)
    ax1.set_ylim(-3, 53)

    # ── Panel 2: rectangle with central blocking island ───────────────
    ax2 = axes[1]
    rect = [(0, 0), (50, 0), (50, 40), (0, 40)]
    island = [(20, 17), (30, 17), (30, 23), (20, 23)]
    dilated_no_go = [(17, 14), (33, 14), (33, 26), (17, 26)]

    _plot_polygon(ax2, rect, "#fafafa", "k", lw=1.8, alpha=0.0)
    _plot_polygon(ax2, island, "dimgray", "dimgray", lw=1.5, alpha=0.55)
    _plot_dashed_rect(ax2, dilated_no_go, "gray", lw=1.2)

    result = find_ramp_carrier(
        boundary=rect,
        islands=[island],
        tool_radius=tool_radius,
        max_ramp_angle_deg=max_ramp_angle_deg,
    )
    if result is not None:
        start, end = result
        ax2.plot(
            [start[0], end[0]],
            [start[1], end[1]],
            "-",
            color="navy",
            linewidth=2.5,
            zorder=4,
        )
        _plot_tool(
            ax2,
            start,
            tool_radius,
            "navy",
            label=f"({start[0]:.1f}, {start[1]:.1f})",
        )
        _plot_tool(
            ax2,
            end,
            tool_radius,
            "navy",
            label=f"({end[0]:.1f}, {end[1]:.1f})",
        )

    ax2.set_title("Rectangle with blocking island", fontsize=11)
    ax2.set_xlabel("X (mm)")
    ax2.set_ylabel("Y (mm)")
    ax2.set_aspect("equal")
    ax2.grid(True, alpha=0.25)
    ax2.set_xlim(-3, 53)
    ax2.set_ylim(-3, 43)

    # ── Panel 3: T-shaped pocket with island ──────────────────────────
    ax3 = axes[2]
    T = [
        (0, 0),
        (50, 0),
        (50, 15),
        (35, 15),
        (35, 45),
        (15, 45),
        (15, 15),
        (0, 15),
    ]
    t_island = [(20, 5), (30, 5), (30, 12), (20, 12)]

    _plot_polygon(ax3, T, "#fafafa", "k", lw=1.8, alpha=0.0)
    _plot_polygon(ax3, t_island, "dimgray", "dimgray", lw=1.5, alpha=0.55)

    result = find_ramp_carrier(
        boundary=T,
        islands=[t_island],
        tool_radius=tool_radius,
        max_ramp_angle_deg=max_ramp_angle_deg,
    )
    if result is not None:
        start, end = result
        ax3.plot(
            [start[0], end[0]],
            [start[1], end[1]],
            "-",
            color="navy",
            linewidth=2.5,
            zorder=4,
        )
        _plot_tool(
            ax3,
            start,
            tool_radius,
            "navy",
            label=f"({start[0]:.1f}, {start[1]:.1f})",
        )
        _plot_tool(
            ax3,
            end,
            tool_radius,
            "navy",
            label=f"({end[0]:.1f}, {end[1]:.1f})",
        )

    ax3.set_title("T-shaped pocket with island", fontsize=11)
    ax3.set_xlabel("X (mm)")
    ax3.set_ylabel("Y (mm)")
    ax3.set_aspect("equal")
    ax3.grid(True, alpha=0.25)
    ax3.set_xlim(-3, 53)
    ax3.set_ylim(-3, 48)

    # Shared legend
    legend_items = [
        Line2D([0], [0], color="k", lw=1.8, label="boundary"),
        Line2D([0], [0], color="dimgray", lw=1.5, label="island"),
        Line2D(
            [0],
            [0],
            color="gray",
            lw=1.2,
            linestyle="--",
            label="dilated no-go band",
        ),
        Line2D([0], [0], color="navy", lw=2.5, label="ramp carrier"),
        Line2D(
            [0],
            [0],
            color="navy",
            lw=1.2,
            linestyle="--",
            label="tool disk (radius)",
        ),
    ]
    fig.legend(
        handles=legend_items,
        loc="lower center",
        ncol=5,
        fontsize=9,
        frameon=False,
        bbox_to_anchor=(0.5, -0.02),
    )

    fig.suptitle(
        "find_ramp_carrier — L-shape, rectangle with blocking island, T-shape",
        fontsize=13,
        y=1.02,
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.feature.ramp.md"]
__images__ = [
    {
        "heading": "find_ramp_carrier",
        "caption": (
            "find_ramp_carrier on L-shaped, rectangle with blocking island,"
            " and T-shaped pocket."
        ),
        "function": generate_ramp_carrier_slots,
    },
]


if __name__ == "__main__":
    fig = generate_ramp_carrier_slots()
    fig.savefig("/tmp/ops_feature_ramp.png", dpi=150, bbox_inches="tight")
    print("Saved /tmp/ops_feature_ramp.png")
