"""Generate 3D visualisation of HSM adaptive clearing."""

__images__ = [
    {
        "stem": "hsm-entry-multi",
        "caption": (
            "Adaptive clearing — Helix → Spiral in a pocket with three islands"
        ),
        "doc": "raygeo.geo.algo.hsm.md",
        "heading": None,
    },
    {
        "stem": "hsm-entry-lshape",
        "caption": (
            "Adaptive clearing — Helix → Spiral in an L-shaped pocket"
        ),
        "doc": "raygeo.geo.algo.hsm.md",
        "heading": None,
    },
    {
        "stem": "hsm-entry-tight",
        "caption": "Adaptive clearing — ZigZag Ramp in a tight slot",
        "doc": "raygeo.geo.algo.hsm.md",
        "heading": None,
    },
]

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.algo.hsm import adaptive_entry


def _plot_3d_toolpath(
    path,
    ax,
    title,
    filename,
    output_dir,
    images,
    caption,
    boundary=None,
    islands=None,
    z_plane=None,
):
    """Plot a 3D toolpath with boundary/island overlay at *z_plane*."""
    pts = np.array(path)
    ax.plot(pts[:, 0], pts[:, 1], pts[:, 2], "b-", linewidth=0.8, alpha=0.8)

    n = len(path)
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
    p = output_dir / filename
    fig.savefig(p, dpi=150)
    plt.close(fig)
    images.append({"path": filename, "caption": caption})


def generate_examples(output_dir):
    images = []

    # ----------------------------------------------------------------
    # Figure 1: Multi-island pocket (three islands) → Helix + Spiral
    # ----------------------------------------------------------------
    target_z = -8.0
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [(70, 40), (90, 40), (90, 60), (70, 60)],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]

    path, _ = adaptive_entry(
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
        path,
        ax1,
        "Adaptive Entry — Multi-Island Pocket",
        "hsm-entry-multi.png",
        output_dir,
        images,
        "Helix → Spiral in a pocket with three islands.",
        boundary=boundary,
        islands=islands,
        z_plane=target_z,
    )

    # ----------------------------------------------------------------
    # Figure 2: L-shaped pocket (no islands) → Helix + Spiral
    # ----------------------------------------------------------------
    target_z3 = -8.0
    lshape = [
        (0, 0),
        (120, 0),
        (120, 40),
        (40, 40),
        (40, 80),
        (0, 80),
    ]

    path3, _ = adaptive_entry(
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
        path3,
        ax3,
        "Adaptive Entry — L-Shaped Pocket",
        "hsm-entry-lshape.png",
        output_dir,
        images,
        "Helix → Spiral in an L-shaped pocket.",
        boundary=lshape,
        z_plane=target_z3,
    )

    # ----------------------------------------------------------------
    # Figure 3: Tight slot → ZigZag Ramp
    # ----------------------------------------------------------------
    target_z4 = -6.0
    tight_boundary = [(0, 0), (100, 0), (100, 16), (0, 16)]

    tight_path, _ = adaptive_entry(
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
        tight_path,
        ax4,
        "Adaptive Entry — Tight Slot (ZigZag Ramp)",
        "hsm-entry-tight.png",
        output_dir,
        images,
        "ZigZag ramp along the longest axis of a narrow slot.",
        boundary=tight_boundary,
        z_plane=target_z4,
    )

    return {"images": images}
