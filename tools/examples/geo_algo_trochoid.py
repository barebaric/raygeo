"""Generate trochoid example images."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.lines import Line2D
from mpl_toolkits.mplot3d.art3d import Line3DCollection

from raygeo.geo.algo.trochoid import (
    get_trochoid_along_3d,
    get_trochoid_along_3d_ramped,
)


def generate_straight():
    # Straight carrier — compare low vs high engagement angle
    carrier = [(0, 0), (80, 0)]

    fig, axes = plt.subplots(1, 2, figsize=(14, 5))

    for ax, label, eng_deg in zip(
        axes, ["Engagement 60°", "Engagement 120°"], [60, 120]
    ):
        pts = get_trochoid_along_3d(
            carrier,
            diameter=10,
            engagement_angle_deg=eng_deg,
            step_over_ratio=0.2,
            z=0,
        )
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        ax.plot(xs, ys, "steelblue", linewidth=2, label=label)
        ax.scatter(xs, ys, c=range(len(pts)), cmap="viridis", s=4, alpha=0.5)
        ax.plot(
            [p[0] for p in carrier],
            [p[1] for p in carrier],
            "r--",
            linewidth=1,
            label="Carrier",
        )
        ax.set_aspect("equal")
        ax.set_xlim(-10, 100)
        ax.set_ylim(-15, 15)
        ax.set_title(label)
        ax.set_xlabel("X")
        ax.set_ylabel("Y")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)

    fig.tight_layout()
    return fig


def generate_l_shaped():
    carrier_l = [(0, 0), (50, 0), (50, 50)]
    pts = get_trochoid_along_3d(
        carrier_l,
        diameter=10,
        engagement_angle_deg=90,
        step_over_ratio=0.2,
        z=0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    ax.plot(xs, ys, "steelblue", linewidth=2, label="Trochoid path")
    ax.scatter(xs, ys, c=range(len(pts)), cmap="viridis", s=4, alpha=0.5)
    cx = [p[0] for p in carrier_l]
    cy = [p[1] for p in carrier_l]
    ax.plot(cx, cy, "r--", linewidth=2, label="Carrier")
    ax.plot(cx, cy, "ro", markersize=6)
    ax.set_aspect("equal")
    ax.set_xlim(-10, 70)
    ax.set_ylim(-10, 60)
    ax.set_title("Trochoidal path on L-shaped carrier")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    return fig


def generate_ramped_3d():
    """3D ramped trochoid along a straight carrier.

    Z descends linearly with arc-length from ``z_start = 4`` at the carrier
    start to ``z_end = -2`` at the carrier end. The trochoid loops
    themselves are unchanged from ``get_trochoid_along_3d`` — only Z varies
    along the path. Colour encodes cumulative arc-length from start (blue)
    to end (red).
    """
    carrier = [(0, 0), (80, 0)]
    pts = get_trochoid_along_3d_ramped(
        carrier,
        diameter=10,
        z_start=4.0,
        z_end=-2.0,
        engagement_angle_deg=90.0,
        step_over_ratio=0.2,
        min_loop_radius=0.5,
    )

    fig = plt.figure(figsize=(11, 7))
    ax = fig.add_subplot(111, projection="3d")

    if len(pts) >= 2:
        xs = np.array([p[0] for p in pts])
        ys = np.array([p[1] for p in pts])
        zs = np.array([p[2] for p in pts])

        # Colour by cumulative arc-length (turbo: blue=start, red=end).
        d_deltas = np.sqrt(
            np.diff(xs) ** 2 + np.diff(ys) ** 2 + np.diff(zs) ** 2
        )
        cum = np.concatenate(([0.0], np.cumsum(d_deltas)))
        total = cum[-1] if cum[-1] > 0 else 1.0
        colors = plt.cm.turbo(cum / total)

        pts_arr = np.column_stack([xs, ys, zs])
        segs = np.stack([pts_arr[:-1], pts_arr[1:]], axis=1)
        lc = Line3DCollection(
            segs, colors=colors[:-1], linewidth=1.0, alpha=1.0
        )
        ax.add_collection3d(lc)

        # Carrier projected on the z_start plane (visual reference).
        cx = [p[0] for p in carrier]
        cy = [p[1] for p in carrier]
        cz = [4.0, 4.0]
        ax.plot(cx, cy, cz, "k--", linewidth=1.5, alpha=0.5)

    ax.set_title("get_trochoid_along_3d_ramped — Z descends with arc-length")
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_zlabel("Z (mm)")
    ax.view_init(elev=25, azim=-55)

    # Honest XY aspect.
    xl, xr = ax.get_xlim()
    yl, yr = ax.get_ylim()
    half = max(xr - xl, yr - yl) * 0.5
    xmid = (xl + xr) * 0.5
    ymid = (yl + yr) * 0.5
    ax.set_xlim(xmid - half, xmid + half)
    ax.set_ylim(ymid - half, ymid + half)

    legend_items = [
        Line2D(
            [0], [0], color="k", linestyle="--", label="carrier at z_start"
        ),
    ]
    ax.legend(handles=legend_items, loc="upper right", fontsize=9)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.trochoid.md"]
__images__ = [
    {
        "heading": "get_trochoid_along_3d",
        "caption": (
            "Trochoidal toolpath along a straight carrier —"
            " shallow vs steep engagement"
        ),
        "function": generate_straight,
    },
    {
        "heading": "get_trochoid_along_3d",
        "caption": "Trochoidal toolpath around an L-shaped corner",
        "function": generate_l_shaped,
    },
    {
        "heading": "get_trochoid_along_3d_ramped",
        "caption": (
            "3D ramped trochoid along a long carrier with Z descending"
            " from start to end."
        ),
        "function": generate_ramped_3d,
    },
]
