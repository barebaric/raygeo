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
    {
        "stem": "hsm-wavefront-rect",
        "caption": (
            "Adaptive wavefronts expanding outward from the initial cleared"
            " disk (blue) to fill the pocket boundary (black)"
        ),
        "doc": "raygeo.geo.algo.hsm.md",
        "heading": "adaptive_wavefronts",
    },
    {
        "stem": "hsm-wavefront-multi",
        "caption": (
            "Adaptive wavefronts in a pocket with three islands — contours"
            " wrap around each island as they expand"
        ),
        "doc": "raygeo.geo.algo.hsm.md",
        "heading": "adaptive_wavefronts",
    },
    {
        "stem": "hsm-wavefront-yshape",
        "caption": (
            "Adaptive wavefronts in a Y-shaped channel — contours split"
            " and propagate along each branch"
        ),
        "doc": "raygeo.geo.algo.hsm.md",
        "heading": "adaptive_wavefronts",
    },
]

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import Normalize

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.hsm import adaptive_entry, adaptive_wavefronts


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


def _plot_wavefront_2d(
    wf_paths, boundary, islands, title, filename, output_dir, images, caption
):
    """Plot wavefront contours as a 2D overlay with colour by iteration."""
    fig, ax = plt.subplots(figsize=(7, 6))
    n_wf = len(wf_paths)
    cmap = plt.colormaps["plasma"]
    for i, wfp in enumerate(wf_paths):
        pts = np.array(wfp)
        if len(pts) == 0:
            continue
        # NaN rows (inserted by the Rust tracer between fragments)
        # naturally break the line in matplotlib.
        color = cmap(i / max(n_wf - 1, 1))
        ax.plot(pts[:, 0], pts[:, 1], color=color, linewidth=0.6, alpha=0.7)

    bnd = np.array(list(boundary) + [boundary[0]])
    ax.plot(bnd[:, 0], bnd[:, 1], "k-", linewidth=2, label="Boundary")
    if islands:
        for isl in islands:
            isl_arr = np.array(list(isl) + [isl[0]])
            ax.fill(
                isl_arr[:, 0],
                isl_arr[:, 1],
                facecolor="#ccc",
                edgecolor="#999",
                linewidth=1.5,
                label="Island" if isl is islands[0] else None,
            )
    ax.set_aspect("equal")
    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)

    sm = plt.cm.ScalarMappable(cmap=cmap, norm=Normalize(0, n_wf - 1))
    sm.set_array([])
    fig.colorbar(sm, ax=ax, label="Iteration")

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
    # Figure 3: Rectangular pocket wavefronts — 2D overlay
    # ----------------------------------------------------------------
    wf_boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    wf_path, wf_cp = adaptive_entry(
        pocket_boundary=wf_boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    wf_ca = ClearedArea(initial=wf_cp)
    wf_paths = adaptive_wavefronts(
        wf_ca,
        wf_boundary,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    _plot_wavefront_2d(
        wf_paths,
        wf_boundary,
        None,
        "Adaptive Wavefronts — Rectangular Pocket",
        "hsm-wavefront-rect.png",
        output_dir,
        images,
        "Wavefront contours expanding from the initial cleared disk"
        " (dark blue) towards the boundary (black).",
    )

    # ----------------------------------------------------------------
    # Figure 4: Tight slot → ZigZag Ramp
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

    # ----------------------------------------------------------------
    # Figure 5: Multi-island wavefronts
    # ----------------------------------------------------------------
    mi_boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    mi_islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [(70, 40), (90, 40), (90, 60), (70, 60)],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    mi_path, mi_cp = adaptive_entry(
        pocket_boundary=mi_boundary,
        islands=mi_islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    mi_ca = ClearedArea(initial=mi_cp)
    mi_paths = adaptive_wavefronts(
        mi_ca,
        mi_boundary,
        islands=mi_islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    _plot_wavefront_2d(
        mi_paths,
        mi_boundary,
        mi_islands,
        "Adaptive Wavefronts — Multi-Island Pocket",
        "hsm-wavefront-multi.png",
        output_dir,
        images,
        "Wavefront contours wrapping around three islands as they"
        " expand outward from the initial cleared disk.",
    )

    # ----------------------------------------------------------------
    # Figure 6: Y-shaped channel wavefronts
    # ----------------------------------------------------------------
    # Y-shaped pocket: vertical stem (bottom) splitting into two
    # diagonal arms (top-left, top-right).
    yshape = [
        (45, 0),
        (75, 0),
        (75, 40),
        (110, 110),
        (80, 110),
        (60, 55),
        (40, 110),
        (10, 110),
        (45, 40),
    ]
    ys_path, ys_cp = adaptive_entry(
        pocket_boundary=yshape,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ys_ca = ClearedArea(initial=ys_cp)
    ys_paths = adaptive_wavefronts(
        ys_ca,
        yshape,
        tool_radius=3.0,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    _plot_wavefront_2d(
        ys_paths,
        yshape,
        None,
        "Adaptive Wavefronts — Y-Shaped Channel",
        "hsm-wavefront-yshape.png",
        output_dir,
        images,
        "Wavefront contours splitting and propagating along each"
        " branch of the Y-shaped pocket.",
    )

    return {"images": images}
