"""Generate visualisations of entry motion assembly."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.ops.assembly.entry import adaptive_entry


def _ops_to_points(ops):
    """Extract (x, y, z, is_travel) for every moving command in *ops*."""
    pts = []
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            ep = ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return pts


def _plot_3d_toolpath(
    ops, ax, title, boundary=None, islands=None, z_plane=None
):
    """Plot 3D toolpath from Ops with boundary/island overlay."""
    pts_list = _ops_to_points(ops)
    if not pts_list:
        fig = ax.figure
        fig.tight_layout()
        return fig
    pts = np.array([(p[0], p[1], p[2]) for p in pts_list])
    ax.plot(pts[:, 0], pts[:, 1], pts[:, 2], "b-", linewidth=0.8, alpha=0.8)

    n = len(pts)
    skip = max(1, n // 200)
    ax.scatter(
        pts[::skip, 0],
        pts[::skip, 1],
        pts[::skip, 2],
        c=pts[::skip, 2],
        cmap="viridis",
        s=4,
        alpha=0.6,
    )

    if boundary is not None and z_plane is not None:
        bnd = np.array(list(boundary) + [boundary[0]])
        ax.plot(
            bnd[:, 0],
            bnd[:, 1],
            zs=z_plane,
            zdir="z",
            color="k",
            linewidth=2,
            alpha=0.5,
        )

    if islands and z_plane is not None:
        for isl in islands:
            isl_arr = np.array(list(isl) + [isl[0]])
            ax.plot(
                isl_arr[:, 0],
                isl_arr[:, 1],
                zs=z_plane,
                zdir="z",
                color="gray",
                linewidth=1.5,
                alpha=0.4,
            )

    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.view_init(elev=30, azim=-45)

    fig = ax.figure
    fig.tight_layout()
    return fig


def generate_entry_multi():
    """Entry multi-island."""
    target_z = -8.0
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [(70, 40), (90, 40), (90, 60), (70, 60)],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]

    ops, _ = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=target_z,
        plunge_pitch=1.0,
    )

    fig1 = plt.figure(figsize=(10, 8))
    ax1 = fig1.add_subplot(111, projection="3d")
    _plot_3d_toolpath(
        ops,
        ax1,
        "Adaptive Entry — Multi-Island Pocket",
        boundary=boundary,
        islands=islands,
        z_plane=target_z,
    )
    return fig1


def generate_entry_lshape():
    """Entry L-shape."""
    target_z3 = -8.0
    lshape = [
        (0, 0),
        (120, 0),
        (120, 40),
        (40, 40),
        (40, 80),
        (0, 80),
    ]

    ops, _ = adaptive_entry(
        pocket_boundary=lshape,
        islands=[],
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=target_z3,
        plunge_pitch=1.0,
    )

    fig3 = plt.figure(figsize=(10, 8))
    ax3 = fig3.add_subplot(111, projection="3d")
    _plot_3d_toolpath(
        ops,
        ax3,
        "Adaptive Entry — L-Shaped Pocket",
        boundary=lshape,
        z_plane=target_z3,
    )
    return fig3


def generate_entry_tight():
    """Entry tight slot."""
    target_z4 = -6.0
    tight_boundary = [(0, 0), (100, 0), (100, 16), (0, 16)]

    ops, _ = adaptive_entry(
        pocket_boundary=tight_boundary,
        islands=[],
        tool_radius=4.0,
        step_over=3.0,
        safe_z=2.0,
        target_z=target_z4,
        plunge_pitch=1.0,
    )

    fig4 = plt.figure(figsize=(10, 6))
    ax4 = fig4.add_subplot(111, projection="3d")
    _plot_3d_toolpath(
        ops,
        ax4,
        "Adaptive Entry — Tight Slot (ZigZag Ramp)",
        boundary=tight_boundary,
        z_plane=target_z4,
    )
    return fig4


__docs_target__ = ["raygeo.ops.assembly.entry.md"]

__images__ = [
    {
        "heading": "adaptive_entry",
        "caption": (
            "Adaptive clearing — Helix → Spiral in a pocket with three islands"
        ),
        "function": generate_entry_multi,
    },
    {
        "heading": "adaptive_entry",
        "caption": (
            "Adaptive clearing — Helix → Spiral in an L-shaped pocket"
        ),
        "function": generate_entry_lshape,
    },
    {
        "heading": "adaptive_entry",
        "caption": "Adaptive clearing — ZigZag Ramp in a tight slot",
        "function": generate_entry_tight,
    },
]
