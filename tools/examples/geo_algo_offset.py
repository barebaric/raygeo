"""Generate concentric offset and deepest-core detection example images."""

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

from raygeo.geo import Geometry
from raygeo.geo.algo.offset import (
    compute_inset_region,
    concentric_offsets,
    find_deepest_cores,
    offset_contour_group,
)
from raygeo.geo.shape.polygon import JoinStyle, offset_polygon
from tools.plot import plot_geometry


def _offset_sequence(boundary, step_over):
    """Yield successive inward offsets until collapse."""
    current = [boundary]

    def poly_area(poly):
        n = len(poly)
        sa = 0.0
        for i in range(n):
            j = (i + 1) % n
            sa += poly[i][0] * poly[j][1] - poly[j][0] * poly[i][1]
        return abs(sa) / 2.0

    yield current[0]
    while True:
        next_polys = []
        for poly in current:
            off = offset_contour_group(
                poly, [], -step_over, join_style=JoinStyle.Miter
            )
            next_polys.extend(off)
        next_polys = [
            p for p in next_polys if len(p) >= 3 and poly_area(p) > 1e-9
        ]
        if not next_polys:
            break
        current = next_polys
        for p in current:
            yield p


def generate_concentric():
    """Concentric offsets."""
    g = Geometry()
    g.move_to(10, 10)
    g.line_to(110, 10)
    g.line_to(110, 110)
    g.line_to(10, 110)
    g.close_path()

    offsets = concentric_offsets(g, step=10, max_passes=10, min_area=1)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_geometry(ax, g, color="black", linewidth=2, label="Original")
    colors = plt.cm.plasma(
        [i / max(len(offsets), 1) for i in range(len(offsets))]
    )
    for i, off in enumerate(offsets):
        plot_geometry(
            ax,
            off,
            color=colors[i],
            linewidth=1.5,
            label=f"Offset {i + 1}" if i < 5 else None,
        )

    ax.set_aspect("equal")
    ax.set_xlim(0, 120)
    ax.set_ylim(0, 120)
    ax.set_title("Concentric inward offsets")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=8, loc="upper right")

    fig.tight_layout()
    return fig


def generate_deepest_cores():
    """Deepest cores simple."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    area = offset_contour_group(boundary, [], -5.0, join_style=JoinStyle.Round)
    cores = find_deepest_cores(area, step_over=10.0)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    boundary_arr = list(boundary) + [boundary[0]]
    ax1.plot(*zip(*boundary_arr), "k-", linewidth=2, label="Boundary")
    offsets = list(_offset_sequence(area[0], 10.0))
    for i, off in enumerate(offsets):
        arr = list(off) + [off[0]]
        lbl = f"Offset {i + 1}" if i < 3 else None
        ax1.plot(*zip(*arr), "--", alpha=0.5, label=lbl)
    if cores:
        ax1.plot(cores[0][0], cores[0][1], "r*", markersize=18, label="Core")
    ax1.set_title("Simple Rectangle — Single Core")
    ax1.set_xlabel("X")
    ax1.set_ylabel("Y")
    ax1.set_aspect("equal")
    ax1.legend(fontsize=8)
    ax1.grid(True, alpha=0.3)

    l_shape = [(0, 0), (100, 0), (100, 40), (40, 40), (40, 80), (0, 80)]
    l_area = offset_contour_group(
        l_shape, [], -5.0, join_style=JoinStyle.Round
    )
    l_cores = find_deepest_cores(l_area, step_over=10.0)

    l_arr = list(l_shape) + [l_shape[0]]
    ax2.plot(*zip(*l_arr), "k-", linewidth=2, label="Boundary")
    l_offsets = []
    for poly in l_area:
        l_offsets.extend(list(_offset_sequence(poly, 10.0)))
    for i, off in enumerate(l_offsets[:8]):
        arr = list(off) + [off[0]]
        ax2.plot(*zip(*arr), "--", alpha=0.5)
    for cx, cy in l_cores:
        ax2.plot(cx, cy, "r*", markersize=18)
    ax2.set_title("L-Shaped Pocket — Two Cores")
    ax2.set_xlabel("X")
    ax2.set_ylabel("Y")
    ax2.set_aspect("equal")
    ax2.legend(
        handles=[
            mpatches.Patch(color="k", label="Boundary"),
            Line2D(
                [0],
                [0],
                linestyle="--",
                color="gray",
                alpha=0.5,
                label="Offsets",
            ),
            Line2D(
                [0],
                [0],
                marker="*",
                color="r",
                linestyle="None",
                markersize=12,
                label="Core",
            ),
        ],
        fontsize=8,
    )
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_deepest_cores_multi():
    """Deepest cores multi-island."""
    mb = [(0, 0), (160, 0), (160, 100), (0, 100)]
    isl1 = [(30, 20), (50, 20), (50, 40), (30, 40)]
    isl2 = [(110, 60), (130, 60), (130, 80), (110, 80)]
    m_area = offset_contour_group(
        mb, [isl1, isl2], -5.0, join_style=JoinStyle.Round
    )
    m_cores = find_deepest_cores(m_area, step_over=10.0)

    fig2, (ax3, ax4) = plt.subplots(1, 2, figsize=(14, 6))

    mb_arr = list(mb) + [mb[0]]
    ax3.plot(*zip(*mb_arr), "k-", linewidth=2, label="Boundary")
    for isl in (isl1, isl2):
        arr = list(isl) + [isl[0]]
        ax3.fill(
            *zip(*arr),
            facecolor="#ddd",
            edgecolor="#999",
            linewidth=1.5,
            label="Island" if isl is isl1 else None,
        )
    for poly in m_area:
        arr = list(poly) + [poly[0]]
        ax3.plot(
            *zip(*arr),
            "--",
            color="steelblue",
            alpha=0.7,
            linewidth=1.5,
            label="Valid area" if poly is m_area[0] else None,
        )
    if m_cores:
        ax3.plot(m_cores[0][0], m_cores[0][1], "r*", markersize=18)
    ax3.set_title("Multi-Island Pocket — Valid Tool Area")
    ax3.set_xlabel("X")
    ax3.set_ylabel("Y")
    ax3.set_aspect("equal")
    ax3.legend(fontsize=7, loc="upper right")
    ax3.grid(True, alpha=0.3)

    for poly in m_area:
        seq = list(_offset_sequence(poly, 10.0))
        for i, off in enumerate(seq):
            arr = list(off) + [off[0]]
            ax4.plot(*zip(*arr), "--", alpha=0.4, color="gray", linewidth=1)
    mb_arr2 = list(mb) + [mb[0]]
    ax4.plot(*zip(*mb_arr2), "k-", linewidth=2, label="Boundary")
    for isl in (isl1, isl2):
        arr = list(isl) + [isl[0]]
        ax4.fill(
            *zip(*arr),
            facecolor="#ddd",
            edgecolor="#999",
            linewidth=1.5,
            alpha=0.7,
        )
    if m_cores:
        ax4.plot(m_cores[0][0], m_cores[0][1], "r*", markersize=18)
    ax4.set_title("Multi-Island Pocket — Single Deepest Core")
    ax4.set_xlabel("X")
    ax4.set_ylabel("Y")
    ax4.set_aspect("equal")
    ax4.legend(
        handles=[
            mpatches.Patch(color="k", label="Boundary"),
            mpatches.Patch(facecolor="#ddd", edgecolor="#999", label="Island"),
            Line2D(
                [0],
                [0],
                linestyle="--",
                color="steelblue",
                alpha=0.7,
                label="Valid area",
            ),
            Line2D(
                [0],
                [0],
                marker="*",
                color="r",
                linestyle="None",
                markersize=12,
                label="Core",
            ),
        ],
        fontsize=7,
        loc="upper right",
    )
    ax4.grid(True, alpha=0.3)

    fig2.tight_layout()
    return fig2


def generate_deepest_cores_central():
    """Deepest cores central island."""
    cb = [(0, 0), (100, 0), (100, 100), (0, 100)]
    cisl = [(35, 35), (65, 35), (65, 65), (35, 65)]
    c_area = offset_contour_group(cb, [cisl], -5.0, join_style=JoinStyle.Round)
    c_cores = find_deepest_cores(c_area, step_over=10.0)

    fig3, (ax5, ax6) = plt.subplots(1, 2, figsize=(14, 6))

    cb_arr = list(cb) + [cb[0]]
    ax5.plot(*zip(*cb_arr), "k-", linewidth=2, label="Boundary")
    cisl_arr = list(cisl) + [cisl[0]]
    ax5.fill(
        *zip(*cisl_arr),
        facecolor="#ddd",
        edgecolor="#999",
        linewidth=1.5,
        label="Island",
    )
    for poly in c_area:
        arr = list(poly) + [poly[0]]
        ax5.plot(
            *zip(*arr),
            "--",
            color="steelblue",
            alpha=0.7,
            linewidth=1.5,
            label="Valid area" if poly is c_area[0] else None,
        )
    if c_cores:
        ax5.plot(c_cores[0][0], c_cores[0][1], "r*", markersize=18)
    ax5.set_title("Central Island — Valid Tool Area")
    ax5.set_xlabel("X")
    ax5.set_ylabel("Y")
    ax5.set_aspect("equal")
    ax5.legend(fontsize=8)
    ax5.grid(True, alpha=0.3)

    for poly in c_area:
        seq = list(_offset_sequence(poly, 10.0))
        for i, off in enumerate(seq):
            arr = list(off) + [off[0]]
            ax6.plot(*zip(*arr), "--", alpha=0.4, color="gray", linewidth=1)
    cb_arr2 = list(cb) + [cb[0]]
    ax6.plot(*zip(*cb_arr2), "k-", linewidth=2, label="Boundary")
    cisl_arr2 = list(cisl) + [cisl[0]]
    ax6.fill(
        *zip(*cisl_arr2),
        facecolor="#ddd",
        edgecolor="#999",
        linewidth=1.5,
        alpha=0.7,
    )
    if c_cores:
        ax6.plot(c_cores[0][0], c_cores[0][1], "r*", markersize=18)
    ax6.set_title("Central Island — Single Deepest Core")
    ax6.set_xlabel("X")
    ax6.set_ylabel("Y")
    ax6.set_aspect("equal")
    ax6.legend(
        handles=[
            mpatches.Patch(color="k", label="Boundary"),
            mpatches.Patch(facecolor="#ddd", edgecolor="#999", label="Island"),
            Line2D(
                [0],
                [0],
                linestyle="--",
                color="steelblue",
                alpha=0.7,
                label="Valid area",
            ),
            Line2D(
                [0],
                [0],
                marker="*",
                color="r",
                linestyle="None",
                markersize=12,
                label="Core",
            ),
        ],
        fontsize=8,
    )
    ax6.grid(True, alpha=0.3)

    fig3.tight_layout()
    return fig3


def generate_inset_region():
    """Inset region — boundary shrunk by radius, obstacles subtracted."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Simple inset
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    region, area = compute_inset_region(boundary, 8.0, [])

    bnd_arr = list(boundary) + [boundary[0]]
    ax1.plot(*zip(*bnd_arr), "k-", linewidth=2, label="Boundary")
    for i, poly in enumerate(region):
        arr = list(poly) + [poly[0]]
        ax1.plot(
            *zip(*arr),
            "steelblue",
            linewidth=2.5,
            label=f"Inset (area={area:.0f})" if i == 0 else None,
        )
    ax1.set_title(f"Simple inset (radius=8, area={area:.0f})", fontsize=13)
    ax1.set_aspect("equal")
    ax1.legend(fontsize=10)
    ax1.grid(True, alpha=0.3)
    ax1.set_xlim(-10, 110)
    ax1.set_ylim(-10, 90)

    # Inset with obstacle
    obstacle = [(35, 25), (65, 25), (65, 55), (35, 55)]
    region2, area2 = compute_inset_region(boundary, 8.0, [obstacle])

    bnd_arr2 = list(boundary) + [boundary[0]]
    ax2.plot(*zip(*bnd_arr2), "k-", linewidth=2, label="Boundary")
    obs_arr = list(obstacle) + [obstacle[0]]
    ax2.fill(
        *zip(*obs_arr),
        facecolor="#ddd",
        edgecolor="#999",
        linewidth=1.5,
        label="Obstacle",
    )
    for i, poly in enumerate(region2):
        arr = list(poly) + [poly[0]]
        ax2.plot(
            *zip(*arr),
            "tomato",
            linewidth=2.5,
            label=f"Inset (area={area2:.0f})" if i == 0 else None,
        )
    ax2.set_title(
        f"Inset with obstacle (radius=8, area={area2:.0f})", fontsize=13
    )
    ax2.set_aspect("equal")
    ax2.legend(fontsize=10)
    ax2.grid(True, alpha=0.3)
    ax2.set_xlim(-10, 110)
    ax2.set_ylim(-10, 90)

    fig.tight_layout()
    return fig


def generate_inset_region_multi_obstacle():
    """Inset region with multiple obstacles — area splits into pieces."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    obs1 = [(20, 20), (40, 20), (40, 40), (20, 40)]
    obs2 = [(120, 60), (140, 60), (140, 80), (120, 80)]
    region, area = compute_inset_region(boundary, 6.0, [obs1, obs2])

    fig, ax = plt.subplots(figsize=(8, 6))

    bnd_arr = list(boundary) + [boundary[0]]
    ax.plot(*zip(*bnd_arr), "k-", linewidth=2, label="Boundary")
    for j, (obs, clr) in enumerate([(obs1, "#ddd"), (obs2, "#ddd")]):
        arr = list(obs) + [obs[0]]
        ax.fill(
            *zip(*arr),
            facecolor=clr,
            edgecolor="#999",
            linewidth=1.5,
            label="Obstacle" if j == 0 else None,
        )
    colors = plt.cm.plasma(
        [i / max(len(region), 1) for i in range(len(region))]
    )
    for i, poly in enumerate(region):
        arr = list(poly) + [poly[0]]
        ax.plot(
            *zip(*arr), color=colors[i], linewidth=2.5, label=f"Region {i + 1}"
        )
    ax.set_title(f"Multi-obstacle inset (total area={area:.0f})", fontsize=13)
    ax.set_aspect("equal")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    ax.set_xlim(-10, 170)
    ax.set_ylim(-10, 110)

    fig.tight_layout()
    return fig


# ── Mixed convex/concave joints + L-shaped island ─────────────────


def generate_inset_region_joints():
    """Boundary with mixed convex/concave joints + L-shaped island — show
    how ``compute_inset_region`` (Round for boundary, Round for islands)
    handles each corner type.

    Convex boundary corners → Round produces a smooth arc.
    Concave boundary corners → offset edges diverge → Round arcs bridge
    the gap correctly (was a bevel under the old Miter join).
    """

    r = 6.0

    # Notched 120×120 rectangle (CCW).  5 convex + 3 concave corners.
    boundary = [
        (0, 0),
        (120, 0),
        (120, 40),
        (80, 40),
        (80, 80),
        (120, 80),
        (120, 120),
        (0, 120),
    ]
    convex = [(0, 0), (120, 0), (120, 80), (120, 120), (0, 120)]
    concave = [(120, 40), (80, 40), (80, 80)]

    island = [(20, 20), (45, 20), (45, 45), (70, 45), (70, 70), (20, 70)]

    # Actual Rust output
    region, area = compute_inset_region(boundary, r, [island])
    island_buffed = offset_polygon(island, r, JoinStyle.Round)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))

    bnd_arr = list(boundary) + [boundary[0]]
    isl_arr = list(island) + [island[0]]

    # ── LEFT: Round inset behaviour ──
    ax1.plot(*zip(*bnd_arr), "k-", linewidth=2, label="Boundary")
    ax1.plot(
        *zip(*convex),
        "o",
        color="green",
        markersize=9,
        zorder=5,
        label="Convex corner",
    )
    ax1.plot(
        *zip(*concave),
        "o",
        color="red",
        markersize=9,
        zorder=5,
        label="Concave corner",
    )
    ax1.fill(
        *zip(*isl_arr),
        facecolor="#ddd",
        edgecolor="#999",
        linewidth=1.5,
        alpha=0.5,
        label="L-shaped island",
    )
    for i, poly in enumerate(region):
        arr = list(poly) + [poly[0]]
        lbl = "Inset region (Round)" if i == 0 else None
        ax1.plot(*zip(*arr), color="steelblue", linewidth=2.5, label=lbl)
        ax1.fill(*zip(*arr), color="steelblue", alpha=0.08)
    for pt in convex:
        ax1.annotate(
            "Arc (Round)",
            pt,
            xytext=(0, 10),
            textcoords="offset points",
            ha="center",
            fontsize=7,
            color="green",
            fontweight="bold",
        )
    for pt in concave:
        ax1.annotate(
            "Arc bridges\ndiverging edges",
            pt,
            xytext=(0, -16),
            textcoords="offset points",
            ha="center",
            fontsize=7,
            color="red",
            fontweight="bold",
        )

    ax1.set_title("compute_inset_region  (JoinStyle.Round)", fontsize=12)
    ax1.set_aspect("equal")
    ax1.legend(fontsize=8, loc="upper right")
    ax1.grid(True, alpha=0.3)
    ax1.set_xlim(-10, 130)
    ax1.set_ylim(-10, 130)
    ax1.set_xlabel("X (mm)")
    ax1.set_ylabel("Y (mm)")

    # ── RIGHT: Full result ──
    ax2.plot(*zip(*bnd_arr), "k-", linewidth=2, label="Boundary")
    ax2.fill(
        *zip(*isl_arr),
        facecolor="#ddd",
        edgecolor="#999",
        linewidth=1.5,
        label="L-shaped island",
    )
    for i, poly in enumerate(island_buffed):
        arr = list(poly) + [poly[0]]
        lbl = "Island +r (Round)" if i == 0 else None
        ax2.plot(*zip(*arr), "--", color="darkorange", linewidth=2, label=lbl)
    for i, poly in enumerate(region):
        arr = list(poly) + [poly[0]]
        lbl = f"Inset region  (area={area:.0f})" if i == 0 else None
        ax2.plot(*zip(*arr), color="crimson", linewidth=3, label=lbl)
        ax2.fill(*zip(*arr), color="crimson", alpha=0.07)
    ax2.set_title(
        f"L-shaped island + inset region  (r={r}, area={area:.0f})",
        fontsize=12,
    )
    ax2.set_aspect("equal")
    ax2.legend(fontsize=8, loc="upper right")
    ax2.grid(True, alpha=0.3)
    ax2.set_xlim(-10, 130)
    ax2.set_ylim(-10, 130)
    ax2.set_xlabel("X (mm)")
    ax2.set_ylabel("Y (mm)")

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.offset.md"]
__images__ = [
    {
        "heading": "concentric_offsets",
        "caption": (
            "Concentric inward offsets for adaptive clearing / pocketing"
        ),
        "function": generate_concentric,
    },
    {
        "heading": "find_deepest_cores",
        "caption": (
            "Deepest-core detection: finds max clearance in valid tool "
            "area — best helical-entry for the pocket"
        ),
        "function": generate_deepest_cores,
    },
    {
        "heading": "find_deepest_cores",
        "caption": (
            "Multi-island pocket: clockwise contours are islands; "
            "core is deepest point in largest valid region."
        ),
        "function": generate_deepest_cores_multi,
    },
    {
        "heading": "find_deepest_cores",
        "caption": (
            "Central-island (annular): ring of valid tool area; "
            "deepest core is max clearance, never in island."
        ),
        "function": generate_deepest_cores_central,
    },
    {
        "heading": "compute_inset_region",
        "caption": (
            "Inset region: boundary shrunk, obstacles removed. "
            "Left: simple. Right: with central obstacle."
        ),
        "function": generate_inset_region,
    },
    {
        "heading": "compute_inset_region",
        "caption": (
            "Multi-obstacle inset: the region splits into multiple"
            " disconnected polygons."
        ),
        "function": generate_inset_region_multi_obstacle,
    },
    {
        "heading": "compute_inset_region",
        "caption": (
            "Mixed convex/concave boundary + L-shaped island:"
            " verifies Round inset arcs at all joint types."
        ),
        "function": generate_inset_region_joints,
    },
]
