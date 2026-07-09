"""Visualisation for ops/feature/region — wide-region detection."""

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Circle as CirclePatch
from matplotlib.patches import Polygon as PolygonPatch

from raygeo.ops.feature import region as _region

find_regions = _region.find_regions


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _plot_polygon(ax, poly, face, edge, **kwargs):
    xs = [p[0] for p in poly] + [poly[0][0]]
    ys = [p[1] for p in poly] + [poly[0][1]]
    ax.fill(xs, ys, facecolor=face, alpha=kwargs.get("alpha", 0.25))
    ax.plot(xs, ys, "-", color=edge, linewidth=kwargs.get("lw", 1.5))


def generate_find_regions():
    """Triple-dumbbell H-shape: 3 wide lobes connected by 2 narrow corridors.

    The geometry has three wide areas:
    - Left lobe: 20x30 rectangle
    - Centre lobe: 20x30 rectangle
    - Right lobe: 20x30 rectangle
    Connected by corridors 25x5 mm (narrow for tool_radius=3).
    """
    tool_radius = 3.0
    tolerance = 0.5

    # H-shape: three lobes connected by narrow corridors
    # Left lobe: x=-20..0, y=-15..15
    # Centre lobe: x=25..45, y=-15..15
    # Right lobe: x=70..90, y=-15..15
    # Corridor 1: x=0..25, y=-2.5..2.5
    # Corridor 2: x=45..70, y=-2.5..2.5
    pocket = [
        (-20.0, -15.0),
        (0.0, -15.0),
        (0.0, -2.5),
        (25.0, -2.5),
        (25.0, -15.0),
        (45.0, -15.0),
        (45.0, -2.5),
        (70.0, -2.5),
        (70.0, -15.0),
        (90.0, -15.0),
        (90.0, 15.0),
        (70.0, 15.0),
        (70.0, 2.5),
        (45.0, 2.5),
        (45.0, 15.0),
        (25.0, 15.0),
        (25.0, 2.5),
        (0.0, 2.5),
        (0.0, 15.0),
        (-20.0, 15.0),
    ]

    regions = find_regions(
        boundary=pocket,
        islands=None,
        tool_radius=tool_radius,
        tolerance=tolerance,
    )

    fig, ax = plt.subplots(figsize=(10, 5))

    # Draw pocket boundary
    _plot_polygon(ax, pocket, "none", "k", lw=2.0, alpha=0.0)
    xs = [p[0] for p in pocket] + [pocket[0][0]]
    ys = [p[1] for p in pocket] + [pocket[0][1]]
    ax.plot(xs, ys, "-", color="k", linewidth=2.0, label="pocket boundary")

    # Color palette for regions
    colors = ["#4C72B0", "#DD8452", "#55A868", "#C44E52", "#8172B3"]

    for i, r in enumerate(regions):
        poly, _area, entry_pt, r_max = r
        color = colors[i % len(colors)]
        # Fill the wide region
        poly_verts = list(poly)
        patch = PolygonPatch(
            poly_verts,
            facecolor=color,
            edgecolor=color,
            alpha=0.35,
            linewidth=1.5,
            label=f"region {i + 1}" if i < 3 else None,
            zorder=3,
        )
        ax.add_patch(patch)
        # Draw entry point
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

    # Also show the narrow passage outlines
    # (They separate the wide regions)
    corridor1 = [
        (0.0, -2.5),
        (25.0, -2.5),
        (25.0, 2.5),
        (0.0, 2.5),
    ]
    corridor2 = [
        (45.0, -2.5),
        (70.0, -2.5),
        (70.0, 2.5),
        (45.0, 2.5),
    ]
    for corr in [corridor1, corridor2]:
        cx = [p[0] for p in corr] + [corr[0][0]]
        cy = [p[1] for p in corr] + [corr[0][1]]
        ax.fill(
            cx,
            cy,
            facecolor="gray",
            alpha=0.3,
            edgecolor="gray",
            linewidth=1.0,
            linestyle="--",
            label="narrow passage" if corr is corridor1 else None,
        )

    ax.set_title("Disconnected wide regions in an H-shape pocket", fontsize=12)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.25)
    ax.set_xlim(-25, 95)
    ax.set_ylim(-20, 20)

    legend_items = [
        Line2D([0], [0], color="k", lw=2.0, label="pocket boundary"),
        Line2D(
            [0],
            [0],
            color="gray",
            lw=1.0,
            linestyle="--",
            label="narrow passage",
        ),
        Line2D([0], [0], color=colors[0], lw=1.5, label="wide region"),
        Line2D(
            [0],
            [0],
            color=colors[0],
            lw=1.5,
            linestyle="--",
            alpha=0.8,
            label="largest inscribed circle",
        ),
    ]
    fig.legend(
        handles=legend_items,
        loc="lower center",
        ncol=4,
        fontsize=9,
        frameon=False,
        bbox_to_anchor=(0.5, -0.02),
    )

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.feature.region.md"]
__images__ = [
    {
        "heading": "find_regions",
        "caption": (
            "H-shape pocket: wide regions colored, entry points marked,"
            " narrow corridors shaded gray"
        ),
        "function": generate_find_regions,
    },
]

if __name__ == "__main__":
    fig = generate_find_regions()
    fig.savefig("/tmp/ops_feature_region.png", dpi=150, bbox_inches="tight")
    print("Saved /tmp/ops_feature_region.png")
