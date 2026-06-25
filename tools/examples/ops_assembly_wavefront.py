"""Generate visualisations of wavefront motion assembly."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import Normalize

from raygeo.ops.area import ClearedArea
from raygeo.ops.assembly.entry import adaptive_entry
from raygeo.ops.assembly.wavefront import adaptive_wavefronts


def _ops_to_points(ops):
    """Extract (x, y, z, is_travel) for every moving command in *ops*."""
    pts = []
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            ep = ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return pts


def _plot_wavefront_2d(ops, boundary, islands, title):
    """Plot wavefront contours from Ops, coloured by iteration."""
    fig, ax = plt.subplots(figsize=(7, 6))
    subpaths = ops.split_into_subpaths()
    n_wf = len(subpaths)
    cmap = plt.colormaps["plasma"]
    for i, sub in enumerate(subpaths):
        color = cmap(i / max(n_wf - 1, 1))
        pts_list = _ops_to_points(sub)
        seg_x, seg_y = [], []
        last_was_travel = False
        for x, y, z, is_travel in pts_list:
            if seg_x and is_travel and not last_was_travel:
                if len(seg_x) >= 2:
                    ax.plot(
                        seg_x, seg_y, color=color, linewidth=0.6, alpha=0.7
                    )
                seg_x, seg_y = [], []
            if not is_travel:
                seg_x.append(x)
                seg_y.append(y)
            last_was_travel = is_travel
        if len(seg_x) >= 2:
            ax.plot(seg_x, seg_y, color=color, linewidth=0.6, alpha=0.7)

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
    return fig


def generate_wavefront_rect():
    """Wavefront rectangular."""
    wf_boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    _, wf_cp = adaptive_entry(
        pocket_boundary=wf_boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    wf_ca = ClearedArea(initial=wf_cp)
    wf_ops = adaptive_wavefronts(
        wf_ca,
        wf_boundary,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    return _plot_wavefront_2d(
        wf_ops,
        wf_boundary,
        None,
        "Adaptive Wavefronts — Rectangular Pocket",
    )


def generate_wavefront_multi():
    """Wavefront multi-island."""
    mi_boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    mi_islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [(70, 40), (90, 40), (90, 60), (70, 60)],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    _, mi_cp = adaptive_entry(
        pocket_boundary=mi_boundary,
        islands=mi_islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    mi_ca = ClearedArea(initial=mi_cp)
    mi_ops = adaptive_wavefronts(
        mi_ca,
        mi_boundary,
        islands=mi_islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    return _plot_wavefront_2d(
        mi_ops,
        mi_boundary,
        mi_islands,
        "Adaptive Wavefronts — Multi-Island Pocket",
    )


def generate_wavefront_yshape():
    """Wavefront Y-shape."""
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
    _, ys_cp = adaptive_entry(
        pocket_boundary=yshape,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ys_ca = ClearedArea(initial=ys_cp)
    ys_ops = adaptive_wavefronts(
        ys_ca,
        yshape,
        tool_radius=3.0,
        step_over=2.0,
        z=-5.0,
        area_tolerance=1.0,
    )
    return _plot_wavefront_2d(
        ys_ops,
        yshape,
        None,
        "Adaptive Wavefronts — Y-Shaped Channel",
    )


__docs_target__ = ["raygeo.ops.assembly.wavefront.md"]

__images__ = [
    {
        "heading": "adaptive_wavefronts",
        "caption": (
            "Adaptive wavefronts expanding outward from the initial cleared"
            " disk (blue) to fill the pocket boundary (black)"
        ),
        "function": generate_wavefront_rect,
    },
    {
        "heading": "adaptive_wavefronts",
        "caption": (
            "Adaptive wavefronts in a pocket with three islands — contours"
            " wrap around each island as they expand"
        ),
        "function": generate_wavefront_multi,
    },
    {
        "heading": "adaptive_wavefronts",
        "caption": (
            "Adaptive wavefronts in a Y-shaped channel — contours split"
            " and propagate along each branch"
        ),
        "function": generate_wavefront_yshape,
    },
]
