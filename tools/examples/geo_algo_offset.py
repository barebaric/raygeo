"""Generate concentric offset and deepest-core detection example images."""

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

from raygeo.geo import Geometry
from raygeo.geo.algo.offset import (
    concentric_offsets,
    find_deepest_cores,
    offset_contour_group,
)
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
                poly, [], -step_over, join_style="miter"
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
    area = offset_contour_group(boundary, [], -5.0, join_style="round")
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
    l_area = offset_contour_group(l_shape, [], -5.0, join_style="round")
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
    m_area = offset_contour_group(mb, [isl1, isl2], -5.0, join_style="round")
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
    c_area = offset_contour_group(cb, [cisl], -5.0, join_style="round")
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
            "Deepest-core detection: binary search finds the largest"
            " offset that does NOT collapse the pocket, then returns"
            " the centroid of the largest surviving fragment"
        ),
        "function": generate_deepest_cores,
    },
    {
        "heading": "find_deepest_cores",
        "caption": (
            "Multi-island pocket: the valid tool area splits into"
            " multiple regions; `find_deepest_cores` returns the single"
            " centroid of the largest surviving fragment"
        ),
        "function": generate_deepest_cores_multi,
    },
    {
        "heading": "find_deepest_cores",
        "caption": (
            "Central-island pocket (annular): the island creates a ring"
            " of valid tool area; the deepest core sits at the centre"
            " of the ring"
        ),
        "function": generate_deepest_cores_central,
    },
]
