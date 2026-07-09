"""Visualisation for ops/feature/near — plunge point finder."""

import math

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Circle as CirclePatch

from raygeo.ops.feature import near as _near

find_plunge_point = _near.find_plunge_point


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _circle(cx, cy, r, n=64):
    return [
        (
            cx + r * math.cos(2.0 * math.pi * i / n),
            cy + r * math.sin(2.0 * math.pi * i / n),
        )
        for i in range(n)
    ]


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


def _scenario(
    ax, title, boundary, cleared, islands, near, tool_radius, search_radius
):
    _plot_polygon(ax, boundary, "#fafafa", "k", lw=1.8, alpha=0.0)
    for c in cleared:
        _plot_polygon(ax, c, "#cfe8cf", "forestgreen", lw=1.0, alpha=0.35)
    for isl in islands:
        _plot_polygon(ax, isl, "dimgray", "dimgray", lw=1.5, alpha=0.55)

    # requested `near` (will collide)
    ax.plot(near[0], near[1], "x", color="crimson", ms=10, mew=2, zorder=6)
    _plot_tool(ax, near, tool_radius, "crimson")

    result = find_plunge_point(
        near,
        cleared,
        boundary,
        islands if islands else None,
        tool_radius,
        search_radius,
    )

    if result is not None:
        label = f"({result[0]:.1f}, {result[1]:.1f})"
        _plot_tool(ax, result, tool_radius, "navy", label=label)
        ax.annotate(
            "",
            xy=result,
            xytext=near,
            arrowprops=dict(arrowstyle="->", color="navy", lw=1.2, alpha=0.6),
            zorder=3,
        )
        status = f"corrected -> ({result[0]:.2f}, {result[1]:.2f})"
    else:
        status = "no valid point"

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


def _scenario_multi(
    ax, title, boundary, cleared, islands, requests, tool_radius
):
    """Plot several `near` -> plunge corrections on one pocket.

    `requests` is a list of `(near, search_radius)` tuples.
    """
    _plot_polygon(ax, boundary, "#fafafa", "k", lw=1.8, alpha=0.0)
    for c in cleared:
        _plot_polygon(ax, c, "#cfe8cf", "forestgreen", lw=1.0, alpha=0.35)
    for isl in islands:
        _plot_polygon(ax, isl, "dimgray", "dimgray", lw=1.5, alpha=0.55)

    statuses = []
    for idx, (near, search_radius) in enumerate(requests):
        tag = f"#{idx + 1}"
        ax.plot(near[0], near[1], "x", color="crimson", ms=10, mew=2, zorder=6)
        _plot_tool(ax, near, tool_radius, "crimson")
        ax.annotate(
            tag,
            near,
            textcoords="offset points",
            xytext=(7, 7),
            fontsize=8,
            color="crimson",
        )

        result = find_plunge_point(
            near,
            cleared,
            boundary,
            islands if islands else None,
            tool_radius,
            search_radius,
        )

        if result is not None:
            label = f"{tag} ({result[0]:.1f}, {result[1]:.1f})"
            _plot_tool(ax, result, tool_radius, "navy", label=label)
            ax.annotate(
                "",
                xy=result,
                xytext=near,
                arrowprops=dict(
                    arrowstyle="->", color="navy", lw=1.2, alpha=0.6
                ),
                zorder=3,
            )
            statuses.append(f"{tag} -> ({result[0]:.2f}, {result[1]:.2f})")
        else:
            statuses.append(f"{tag} no valid point")

    ax.set_title(title, fontsize=11)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.25)
    ax.text(
        0.5,
        -0.18,
        "  |  ".join(statuses),
        transform=ax.transAxes,
        ha="center",
        fontsize=8,
        color="navy",
    )


def generate_plunge_point_scenarios():
    """Three plunge-point scenarios in one figure.

    - Slot corridor: a tight corridor just wide enough for the tool
      (width ≈ tool diameter + tolerance); the requested point clips
      the wall and is recentred.
    - Narrow passage: a wider corridor in the toroidal range; more
      lateral freedom but the requested point still clips the wall.
    - Wide pocket with island: two requested points are shown — one
      that lands on the central island (full collision), and one that
      sits outside the cleared disk but fully inside the pocket; both
      are corrected back into the cleared area.
    """

    tool_radius = 3.0

    fig, axes = plt.subplots(1, 3, figsize=(16, 5.5))

    # --- Scenario 1: slot corridor (width 7 ≈ D + tol) ---
    boundary_s = _rect(0, 0, 40, 7.0)
    cleared_s = [_rect(0.4, 0.4, 39.2, 6.2)]
    _scenario(
        axes[0],
        "Slot corridor (7 mm wide, barely fits tool)",
        boundary_s,
        cleared_s,
        [],
        near=(2.0, 1.0),
        tool_radius=tool_radius,
        search_radius=10.0,
    )
    axes[0].set_xlim(-6, 46)
    axes[0].set_ylim(-5, 13)

    # --- Scenario 2: narrow passage (width 8.5, toroidal range) ---
    boundary_n = _rect(0, 0, 40, 8.5)
    cleared_n = [_rect(0.5, 0.5, 39, 7.5)]
    _scenario(
        axes[1],
        "Narrow passage (8.5 mm wide, toroidal)",
        boundary_n,
        cleared_n,
        [],
        near=(1.5, 1.2),
        tool_radius=tool_radius,
        search_radius=10.0,
    )
    axes[1].set_xlim(-6, 46)
    axes[1].set_ylim(-5, 13)

    # --- Scenario 3: wide pocket with island, two requested points ---
    boundary_w = _rect(0, 0, 40, 30)
    cleared_w = [_circle(20, 15, 13)]
    island_w = [_rect(18, 13, 4, 4)]
    _scenario_multi(
        axes[2],
        "Wide pocket (island + outside-cleared)",
        boundary_w,
        cleared_w,
        island_w,
        requests=[
            ((20.0, 15.0), 12.0),  # on the island -> collision
            ((3.0, 3.0), 25.0),  # outside cleared disk, inside pocket
        ],
        tool_radius=tool_radius,
    )
    axes[2].set_xlim(-6, 46)
    axes[2].set_ylim(-6, 36)

    # Shared legend
    legend_items = [
        Line2D([0], [0], color="k", lw=1.8, label="boundary"),
        Line2D([0], [0], color="forestgreen", lw=1.0, label="cleared area"),
        Line2D([0], [0], color="dimgray", lw=1.5, label="island"),
        Line2D(
            [0],
            [0],
            color="crimson",
            lw=0,
            marker="x",
            markersize=10,
            mew=2,
            label="requested `near` (collides)",
        ),
        Line2D(
            [0],
            [0],
            color="navy",
            lw=1.2,
            linestyle="--",
            label="corrected plunge (tool disk)",
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
        "find_plunge_point — slot, narrow passage, and collision correction",
        fontsize=13,
        y=1.02,
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.feature.near.md"]
__images__ = [
    {
        "heading": "find_plunge_point",
        "caption": (
            "Three scenarios of find_plunge_point correcting near points:"
            " slot, passage, and pocket with island."
        ),
        "function": generate_plunge_point_scenarios,
    },
]
