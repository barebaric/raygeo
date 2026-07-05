"""Generate visualisations of entry motion assembly."""

import math

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


def _draw_3d_boundary(ax, boundary, islands, z_plane):
    """Draw boundary and islands on the 3D z-plane."""
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


def _plot_3d_toolpath(
    ops,
    ax,
    title,
    boundary=None,
    islands=None,
    z_plane=None,
):
    """Plot 3D toolpath: travel=dotted gray, cutting=rainbow by arc-length."""
    pts_list = _ops_to_points(ops)
    if not pts_list:
        fig = ax.figure
        fig.tight_layout()
        return fig

    segments = []
    cur = []
    for p in pts_list:
        x, y, z, is_travel = p
        if is_travel:
            if len(cur) > 1:
                segments.append(cur)
            cur = []
        else:
            cur.append((x, y, z))
    if len(cur) > 1:
        segments.append(cur)

    segs_3d = []
    cum_dists = []
    cum = 0.0
    prev = None
    for seg in segments:
        for p in seg:
            if prev is not None:
                segs_3d.append([prev, p])
                cum += math.sqrt(
                    (p[0] - prev[0]) ** 2
                    + (p[1] - prev[1]) ** 2
                    + (p[2] - prev[2]) ** 2
                )
                cum_dists.append(cum)
            prev = p
        prev = None
    total = cum if cum > 0 else 1.0
    if segs_3d:
        from mpl_toolkits.mplot3d.art3d import Line3DCollection

        lc3d = Line3DCollection(
            segs_3d,
            colors=plt.cm.turbo([d / total for d in cum_dists]),
            linewidth=0.8,
            alpha=1.0,
        )
        ax.add_collection3d(lc3d)

    prev = None
    for p in pts_list:
        x, y, z, is_travel = p
        if is_travel:
            if prev is not None:
                ax.plot(
                    [prev[0], x],
                    [prev[1], y],
                    [prev[2], z],
                    linestyle="--",
                    linewidth=1.0,
                    color="dimgray",
                    alpha=0.8,
                )
            prev = (x, y, z)
        else:
            prev = (x, y, z)

    _draw_3d_boundary(ax, boundary, islands, z_plane)

    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.view_init(elev=30, azim=-45)

    xl, xr = ax.get_xlim()
    yl, yr = ax.get_ylim()
    zl, zr = ax.get_zlim()
    half = max(xr - xl, yr - yl, zr - zl) * 0.5
    xm = (xl + xr) * 0.5
    ym = (yl + yr) * 0.5
    zm = (zl + zr) * 0.5
    ax.set_xlim(xm - half, xm + half)
    ax.set_ylim(ym - half, ym + half)
    ax.set_zlim(zm - half, zm + half)
    ax.set_box_aspect(None)

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

    result = adaptive_entry(
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
        result.ops,
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

    result = adaptive_entry(
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
        result.ops,
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

    result = adaptive_entry(
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
        result.ops,
        ax4,
        "Adaptive Entry — Tight Slot (ZigZag Ramp)",
        boundary=tight_boundary,
        z_plane=target_z4,
    )
    return fig4


def _rect(cx, cy, w, h):
    """CCW rectangle centred at (cx, cy)."""
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


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
