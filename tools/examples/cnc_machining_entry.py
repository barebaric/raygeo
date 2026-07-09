"""Generate visualisations of entry workplan planning."""

import matplotlib.pyplot as plt
from matplotlib.patches import Circle as CirclePatch
from matplotlib.patches import Polygon as PolygonPatch

from raygeo.cnc.machining.entry import build_entry_workplan
from raygeo.ops.feature import region as _region

find_regions = _region.find_regions


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _hshape():
    return [
        (0.0, 0.0),
        (15.0, 0.0),
        (15.0, 30.0),
        (30.0, 30.0),
        (30.0, 0.0),
        (45.0, 0.0),
        (45.0, 45.0),
        (30.0, 45.0),
        (30.0, 16.0),
        (15.0, 16.0),
        (15.0, 45.0),
        (0.0, 45.0),
    ]


def _cup():
    """Inverted-U: 40x8 bottom bar + 8-wide vertical post (x=16..24)."""
    return [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 8.0),
        (24.0, 8.0),
        (24.0, 30.0),
        (16.0, 30.0),
        (16.0, 8.0),
        (0.0, 8.0),
    ]


def _draw_polygon(ax, poly, face, edge, **kwargs):
    xs = [p[0] for p in poly] + [poly[0][0]]
    ys = [p[1] for p in poly] + [poly[0][1]]
    ax.fill(xs, ys, facecolor=face, alpha=kwargs.get("alpha", 0.25))
    ax.plot(xs, ys, "-", color=edge, linewidth=kwargs.get("lw", 1.5))


def _draw_regions(ax, regions, colors):
    for i, r in enumerate(regions):
        poly, _area, entry_pt, r_max = r
        color = colors[i % len(colors)]
        patch = PolygonPatch(
            poly,
            facecolor=color,
            edgecolor=color,
            alpha=0.35,
            linewidth=1.5,
            zorder=3,
        )
        ax.add_patch(patch)
        cx, cy = entry_pt
        ax.plot(cx, cy, "o", color=color, ms=6, zorder=5)
        ax.add_patch(
            CirclePatch(
                (cx, cy),
                r_max,
                fill=False,
                edgecolor=color,
                linestyle="--",
                linewidth=1.5,
                alpha=0.8,
                zorder=4,
            )
        )


def _draw_carrier(ax, carrier, color):
    cx = [p[0] for p in carrier]
    cy = [p[1] for p in carrier]
    ax.plot(cx, cy, "-", color=color, linewidth=2.5, zorder=5)
    ax.plot(cx, cy, "o", color=color, ms=7, zorder=6)


def _annotate_workplan(ax, workplan, palette):
    """Annotate each step kind and draw geometric primitives."""
    for step in workplan:
        kind = step["kind"]
        color = palette[0]
        if kind in ("HelixPlunge", "FlatSpiral"):
            cx, cy = step["center"]
            if kind == "HelixPlunge":
                ax.add_patch(
                    CirclePatch(
                        (cx, cy),
                        step["helix_r"],
                        fill=False,
                        edgecolor=color,
                        linestyle=":",
                        linewidth=2.0,
                        alpha=0.9,
                        zorder=4,
                    )
                )
            else:
                spir_r = step["end_radius"]
                ax.add_patch(
                    CirclePatch(
                        (cx, cy),
                        spir_r,
                        fill=False,
                        edgecolor=color,
                        linestyle="--",
                        linewidth=1.5,
                        alpha=0.7,
                        zorder=4,
                    )
                )
            label_xy = (cx, cy)
            label = (
                "HelixPlunge"
                if kind == "HelixPlunge"
                else f"FlatSpiral (r={step['end_radius']:.1f})"
            )
        elif kind == "RampEntry":
            sx, sy = step["start"]
            ex, ey = step["end"]
            ax.plot(
                [sx, ex],
                [sy, ey],
                "-",
                color=color,
                linewidth=2.5,
                zorder=5,
            )
            ax.plot([sx, ex], [sy, ey], "o", color=color, ms=7, zorder=6)
            label_xy = ((sx + ex) / 2, (sy + ey) / 2)
            label = "RampEntry"
        elif kind == "ToroidalClear":
            carrier = step["carrier"]
            _draw_carrier(ax, carrier, color)
            label_xy = (
                (carrier[0][0] + carrier[-1][0]) / 2,
                (carrier[0][1] + carrier[-1][1]) / 2,
            )
            label = "ToroidalClear"
        else:
            continue

        ax.annotate(
            label,
            label_xy,
            xytext=(0, 18),
            textcoords="offset points",
            ha="center",
            va="bottom",
            fontsize=8,
            bbox=dict(
                boxstyle="round,pad=0.3",
                fc="lightyellow",
                ec="gray",
                alpha=0.85,
            ),
            arrowprops=dict(
                arrowstyle="->",
                color="gray",
                lw=0.8,
            ),
        )


def generate_entry_workplan():
    """3-panel: Helix+Spiral, ToroidalClear, RampEntry."""
    tool_radius = 3.0
    step_over = 2.0
    safe_z = 2.0
    target_z = -5.0

    colors = ["#4C72B0", "#DD8452", "#55A868", "#C44E52", "#8172B3"]

    # --- Panel 1: wide rectangle -> HelixPlunge + FlatSpiral ---
    rect_boundary = _rect(0, 0, 40, 40)
    rect_regions = find_regions(
        boundary=rect_boundary,
        tool_radius=tool_radius,
        tolerance=0.5,
    )
    rect_workplan = build_entry_workplan(
        pocket_boundary=rect_boundary,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=safe_z,
        target_z=target_z,
    )

    # --- Panel 2: H-shape -> 2x ToroidalClear (regions) ---
    h_boundary = _hshape()
    h_regions = find_regions(
        boundary=h_boundary,
        tool_radius=tool_radius,
        tolerance=0.5,
    )
    h_workplan = build_entry_workplan(
        pocket_boundary=h_boundary,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=safe_z,
        target_z=target_z,
    )

    # --- Panel 3: cup -> RampEntry (region-based, no carrier) ---
    cup_boundary = _cup()
    cup_regions = find_regions(
        boundary=cup_boundary,
        tool_radius=tool_radius,
        tolerance=0.5,
    )
    cup_workplan = build_entry_workplan(
        pocket_boundary=cup_boundary,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=safe_z,
        target_z=target_z,
    )

    fig, axes = plt.subplots(1, 3, figsize=(18, 6))
    ax1, ax2, ax3 = axes

    # -- Panel 1: wide rectangle --
    _draw_polygon(ax1, rect_boundary, "none", "k", lw=2)
    _draw_regions(ax1, rect_regions, colors)
    _annotate_workplan(ax1, rect_workplan, ["#4C72B0"])
    ax1.set_title("Wide Rectangle\nHelixPlunge + FlatSpiral", fontsize=10)
    ax1.set_xlabel("X (mm)")
    ax1.set_ylabel("Y (mm)")
    ax1.set_aspect("equal")
    ax1.grid(True, alpha=0.25)

    # -- Panel 2: H-shape --
    _draw_polygon(ax2, h_boundary, "none", "k", lw=2)
    _draw_regions(ax2, h_regions, colors)
    _annotate_workplan(ax2, h_workplan, ["#DD8452"])
    ax2.set_title("H-Shape\n2x ToroidalClear (per region)", fontsize=10)
    ax2.set_xlabel("X (mm)")
    ax2.set_ylabel("Y (mm)")
    ax2.set_aspect("equal")
    ax2.grid(True, alpha=0.25)

    # -- Panel 3: cup -> RampEntry --
    _draw_polygon(ax3, cup_boundary, "none", "k", lw=2)
    _draw_regions(ax3, cup_regions, colors)
    _annotate_workplan(ax3, cup_workplan, ["#55A868"])
    ax3.set_title("Cup Shape\nRampEntry (no carrier)", fontsize=10)
    ax3.set_xlabel("X (mm)")
    ax3.set_ylabel("Y (mm)")
    ax3.set_aspect("equal")
    ax3.grid(True, alpha=0.25)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.cnc.machining.entry.md"]

__images__ = [
    {
        "heading": "build_entry_workplan",
        "caption": (
            "Entry workplan for 3 shapes: rectangle (Helix+FlatSpiral),"
            " H-shape (ToroidalClear), cup (RampEntry)."
        ),
        "function": generate_entry_workplan,
    },
]

if __name__ == "__main__":
    fig = generate_entry_workplan()
    fig.savefig("/tmp/cnc_machining_entry.png", dpi=150, bbox_inches="tight")
    print("Saved /tmp/cnc_machining_entry.png")
