"""Generate 3D visualisation of HSM adaptive clearing."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import Normalize

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.hsm import (
    adaptive_entry,
    adaptive_peeling,
    adaptive_wavefronts,
    fillet_arc_ends,
    find_cutting_arc,
    find_safe_sweep_end,
    link_filleted_arcs,
)
from raygeo.geo.algo.medial_axis import compute_medial_axis
from raygeo.geo.algo.offset import compute_inset_region
from raygeo.geo.shape.polygon import get_polygons_group_difference


def _plot_3d_toolpath(
    path, ax, title, boundary=None, islands=None, z_plane=None
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
    return fig


def _plot_wavefront_2d(wf_paths, boundary, islands, title):
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
        boundary=lshape,
        z_plane=target_z3,
    )
    return fig3


def generate_entry_tight():
    """Entry tight slot."""
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
        boundary=tight_boundary,
        z_plane=target_z4,
    )
    return fig4


def generate_wavefront_rect():
    """Wavefront rectangular."""
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
    return _plot_wavefront_2d(
        wf_paths,
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
    return _plot_wavefront_2d(
        mi_paths,
        mi_boundary,
        mi_islands,
        "Adaptive Wavefronts — Multi-Island Pocket",
    )


def _plot_peeling_2d(toolpath, boundary, islands, title):
    """Plot peeling linked toolpath — cutting arcs solid, travel dashed."""
    fig, ax = plt.subplots(figsize=(7, 6))
    pts = np.array(toolpath)
    if len(pts):
        seg_x, seg_y, z_last = [], [], 0.0
        for p in pts:
            if seg_x and abs(p[2] - z_last) > 0.1:
                if len(seg_x) >= 2:
                    color = "#2ca02c" if z_last > 0.1 else "#e41a1c"
                    ls = "--" if z_last > 0.1 else "-"
                    ax.plot(
                        seg_x, seg_y, color=color, linewidth=0.7, linestyle=ls
                    )
                seg_x, seg_y = [], []
            if not math.isnan(p[0]):
                if not seg_x:
                    z_last = p[2]
                seg_x.append(p[0])
                seg_y.append(p[1])
        if len(seg_x) >= 2:
            color = "#2ca02c" if z_last > 0.1 else "#e41a1c"
            ls = "--" if z_last > 0.1 else "-"
            ax.plot(seg_x, seg_y, color=color, linewidth=0.7, linestyle=ls)

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
    ax.legend(loc="upper right", fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def _plot_peeling_3d(toolpath, boundary, islands, title, cut_z, lift_z):
    """Plot peeling D-cut passes in 3-D with Z-lift colouring."""
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")

    pts = np.array(toolpath)
    # Split on NaN rows for per-segment rendering
    segments = []
    cur = []
    for p in pts:
        if np.any(np.isnan(p)):
            if cur:
                segments.append(np.array(cur))
                cur = []
        else:
            cur.append(p)
    if cur:
        segments.append(np.array(cur))

    for seg in segments:
        if len(seg) < 2:
            continue
        # Colour by Z: cutting arc vs lift arc
        zs = seg[:, 2]
        ax.plot(
            seg[:, 0],
            seg[:, 1],
            zs,
            color="#1f77b4",
            linewidth=0.7,
            alpha=0.7,
        )
        ax.scatter(
            seg[:, 0],
            seg[:, 1],
            zs,
            c=zs,
            cmap="coolwarm",
            s=4,
            alpha=0.6,
            norm=Normalize(cut_z, lift_z),
        )

    bnd = np.array(list(boundary) + [boundary[0]])
    ax.plot(
        bnd[:, 0],
        bnd[:, 1],
        zs=cut_z,
        zdir="z",
        color="k",
        linewidth=2,
        alpha=0.4,
    )

    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.view_init(elev=30, azim=-45)

    sm = plt.cm.ScalarMappable(cmap="coolwarm", norm=Normalize(cut_z, lift_z))
    sm.set_array([])
    fig.colorbar(sm, ax=ax, label="Z", shrink=0.6)
    fig.tight_layout()
    return fig


def generate_peeling_rect_2d():
    """Peeling rectangular pocket (2D projection)."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    path, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    tp = adaptive_peeling(
        ca,
        boundary,
        step_over=2.0,
        z=-5.0,
        safe_z=5.0,
        area_tolerance=1.0,
    )
    return _plot_peeling_2d(
        tp,
        boundary,
        None,
        "Adaptive Peeling (2-D) — Rectangular Pocket",
    )


def generate_peeling_rect_3d():
    """Peeling rectangular pocket (3D with Z lift)."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    cut_z = -5.0
    lift_z = 5.0
    path, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=cut_z,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    tp = adaptive_peeling(
        ca,
        boundary,
        step_over=2.0,
        z=cut_z,
        safe_z=lift_z,
        area_tolerance=1.0,
    )
    return _plot_peeling_3d(
        tp,
        boundary,
        None,
        "Adaptive Peeling (3-D) — Rectangular Pocket\n"
        "Cutting arc at depth (blue), return arc at lift (red)",
        cut_z,
        lift_z,
    )


def generate_peeling_multi():
    """Peeling multi-island pocket."""
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [
            (
                80 + 10 * math.cos(2 * math.pi * i / 32),
                50 + 10 * math.sin(2 * math.pi * i / 32),
            )
            for i in range(32)
        ],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    path, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    tp = adaptive_peeling(
        ca,
        boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        z=-5.0,
        safe_z=5.0,
        area_tolerance=1.0,
    )
    return _plot_peeling_2d(
        tp,
        boundary,
        islands,
        "Adaptive Peeling — Multi-Island Pocket",
    )


def generate_find_cutting_arc():
    """Show cutting arcs from ten iterations of peeling."""
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [
            (
                80 + 10 * math.cos(2 * math.pi * i / 32),
                50 + 10 * math.sin(2 * math.pi * i / 32),
            )
            for i in range(32)
        ],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, islands)

    # Run several iterations, collecting all cutting arcs
    all_arcs = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc:
                all_arcs.append(arc)
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    # pocket boundary
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    # islands
    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(
            ix,
            iy,
            facecolor="lightgray",
            edgecolor="gray",
            hatch="///",
            linewidth=1,
        )

    # colour arcs by iteration (first → dark, later → pale)
    n = len(all_arcs)
    for idx, arc in enumerate(all_arcs):
        t = idx / max(n - 1, 1)
        r = 0.9 - 0.6 * t
        g = 0.2 + 0.5 * t
        color = (r, g, 0.2)
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color=color,
            linewidth=2.0,
            alpha=0.85,
        )

    ax.set_title(f"Cutting arcs from {n} passes")
    fig.tight_layout()
    return fig


def generate_fillet_arc_ends():
    """Show cutting arcs with filleted ends flowing into the frontier."""
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [
            (
                80 + 10 * math.cos(2 * math.pi * i / 32),
                50 + 10 * math.sin(2 * math.pi * i / 32),
            )
            for i in range(32)
        ],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, islands)

    all_arcs = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc and len(arc) >= 3:
                safe = find_safe_sweep_end(arc, boundary, islands, tool_radius)
                if safe:
                    fa = fillet_arc_ends(arc, boundary, islands, tool_radius)
                    all_arcs.append((arc, safe, fa))
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    # pocket boundary
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(
            ix,
            iy,
            facecolor="lightgray",
            edgecolor="gray",
            hatch="///",
            linewidth=1,
        )

    raw_color = "#1f77b4"
    fillet_color = "#e41a1c"
    cross_color = "#2ca02c"
    for idx, (arc, safe, fa) in enumerate(all_arcs):
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color=raw_color,
            linewidth=1.5,
            alpha=0.4,
        )
        ax.plot(
            [p[0] for p in fa],
            [p[1] for p in fa],
            color=fillet_color,
            linewidth=2.5,
            alpha=0.9,
            label="Trimmed" if idx == 0 else "",
        )
        if safe is not None:
            enter, exit_pt = safe
            ax.plot(
                enter[0],
                enter[1],
                "o",
                color=cross_color,
                markersize=3,
                label="Enter" if idx == 0 else "",
            )
            ax.plot(
                exit_pt[0],
                exit_pt[1],
                "s",
                color=cross_color,
                markersize=3,
                label="Exit" if idx == 0 else "",
            )

    ax.set_title("Cutting arcs trimmed at 2×tool_radius from frontier")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


def generate_find_cutting_arc_simple():
    """Show cutting arcs without islands."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, [])

    all_arcs = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc:
                all_arcs.append(arc)
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    n = len(all_arcs)
    for idx, arc in enumerate(all_arcs):
        t = idx / max(n - 1, 1)
        r = 0.9 - 0.6 * t
        g = 0.2 + 0.5 * t
        color = (r, g, 0.2)
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color=color,
            linewidth=2.0,
            alpha=0.85,
        )

    ax.set_title(f"Cutting arcs from {n} passes (no islands)")
    fig.tight_layout()
    return fig


def generate_fillet_arc_ends_simple():
    """Show filleted cutting arcs without islands."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = []
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, [])

    all_arcs = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc and len(arc) >= 3:
                safe = find_safe_sweep_end(arc, boundary, islands, tool_radius)
                if safe:
                    fa = fillet_arc_ends(arc, boundary, islands, tool_radius)
                    all_arcs.append((arc, safe, fa))
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    raw_color = "#1f77b4"
    fillet_color = "#e41a1c"
    cross_color = "#2ca02c"
    for idx, (arc, safe, fa) in enumerate(all_arcs):
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color=raw_color,
            linewidth=1.5,
            alpha=0.4,
        )
        ax.plot(
            [p[0] for p in fa],
            [p[1] for p in fa],
            color=fillet_color,
            linewidth=2.5,
            alpha=0.9,
            label="Trimmed" if idx == 0 else "",
        )
        if safe is not None:
            enter, exit_pt = safe
            ax.plot(
                enter[0],
                enter[1],
                "o",
                color=cross_color,
                markersize=3,
                label="Enter" if idx == 0 else "",
            )
            ax.plot(
                exit_pt[0],
                exit_pt[1],
                "s",
                color=cross_color,
                markersize=3,
                label="Exit" if idx == 0 else "",
            )

    ax.set_title("Filleted cutting arcs (no islands)")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


def generate_link_arcs():
    """Show linked filleted arcs as a continuous path."""
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [
            (
                80 + 10 * math.cos(2 * math.pi * i / 32),
                50 + 10 * math.sin(2 * math.pi * i / 32),
            )
            for i in range(32)
        ],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, islands)

    filleted_arcs = []
    for _ in range(80):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc and len(arc) >= 3:
                fa = fillet_arc_ends(arc, boundary, islands, tool_radius)
                if len(fa) >= 3:
                    filleted_arcs.append(fa)
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    vi = get_polygons_group_difference(va, ca.fragments())
    uncleared = vi + islands  # also avoid islands during travel

    # Compute MAT once for obstacle-aware travel links.
    mat_data = None
    try:
        mat = compute_medial_axis(
            boundary,
            islands,
            tool_radius,
            sampling_spacing=step_over * 0.5,
        )
        mat_data = (mat[0], mat[2])  # (nodes, edges)
    except Exception:
        pass

    linked = link_filleted_arcs(
        filleted_arcs,
        uncleared,
        z=0.0,
        safe_z=2.0,
        mat=mat_data,
        safe_margin=tool_radius,
    )

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1, alpha=0.3, label="Boundary")

    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(
            ix,
            iy,
            facecolor="lightgray",
            edgecolor="gray",
            hatch="///",
            linewidth=1,
        )

    if linked:
        seg_x, seg_y, z_last = [], [], 0.0
        for p in linked:
            if seg_x and (math.isnan(p[0]) or abs(p[2] - z_last) > 0.1):
                if len(seg_x) >= 2:
                    color = "#2ca02c" if z_last > 0.1 else "#e41a1c"
                    ls = "--" if z_last > 0.1 else "-"
                    ax.plot(
                        seg_x, seg_y, color=color, linewidth=0.5, linestyle=ls
                    )
                seg_x, seg_y = [], []
            if not math.isnan(p[0]):
                if not seg_x:
                    z_last = p[2]
                seg_x.append(p[0])
                seg_y.append(p[1])
        if len(seg_x) >= 2:
            color = "#2ca02c" if z_last > 0.1 else "#e41a1c"
            ls = "--" if z_last > 0.1 else "-"
            ax.plot(seg_x, seg_y, color=color, linewidth=0.5, linestyle=ls)

    ax.set_title("Linked filleted cutting arcs")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


def generate_find_safe_sweep_end():
    """Show cutting arcs trimmed by iterative sweep shortening."""
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [
            (
                80 + 10 * math.cos(2 * math.pi * i / 32),
                50 + 10 * math.sin(2 * math.pi * i / 32),
            )
            for i in range(32)
        ],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, islands)

    all_crossings = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc and len(arc) >= 3:
                safe = find_safe_sweep_end(arc, boundary, islands, tool_radius)
                if safe:
                    all_crossings.append((arc, safe))
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(
            ix,
            iy,
            facecolor="lightgray",
            edgecolor="gray",
            hatch="///",
            linewidth=1,
        )

    for idx, (arc, (enter, exit_pt)) in enumerate(all_crossings):
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color="#1f77b4",
            linewidth=1.5,
            alpha=0.25,
        )
        ax.plot(
            enter[0],
            enter[1],
            "o",
            color="#e41a1c",
            markersize=2,
            label="Enter" if idx == 0 else "",
        )
        ax.plot(
            exit_pt[0],
            exit_pt[1],
            "s",
            color="#e41a1c",
            markersize=2,
            label="Exit" if idx == 0 else "",
        )

    ax.set_title("Cutting arcs with safe entry/exit points")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


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
    return _plot_wavefront_2d(
        ys_paths,
        yshape,
        None,
        "Adaptive Wavefronts — Y-Shaped Channel",
    )


__docs_target__ = ["raygeo.geo.algo.hsm.md"]
__images__ = [
    {
        "heading": None,
        "caption": (
            "Adaptive clearing — Helix → Spiral in a pocket with three islands"
        ),
        "function": generate_entry_multi,
    },
    {
        "heading": None,
        "caption": (
            "Adaptive clearing — Helix → Spiral in an L-shaped pocket"
        ),
        "function": generate_entry_lshape,
    },
    {
        "heading": None,
        "caption": "Adaptive clearing — ZigZag Ramp in a tight slot",
        "function": generate_entry_tight,
    },
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
    {
        "heading": "adaptive_peeling",
        "caption": (
            "Adaptive peeling (D-biting) in a rectangular pocket —"
            " outer (cutting) arc at depth and inner (return) arc"
            " at lift Z form the characteristic D-shape"
        ),
        "function": generate_peeling_rect_2d,
    },
    {
        "heading": "adaptive_peeling",
        "caption": (
            "Adaptive peeling (D-biting) in a rectangular pocket"
            " — 3-D view with colour by Z: the cutting arc at"
            " depth (blue) and the return arc at lift (red)"
        ),
        "function": generate_peeling_rect_3d,
    },
    {
        "heading": "adaptive_peeling",
        "caption": (
            "Adaptive peeling (D-biting) in a pocket with three islands"
        ),
        "function": generate_peeling_multi,
    },
    {
        "heading": "find_cutting_arc",
        "caption": (
            "Bite polygons from the first peeling iteration with the"
            " cutting arc (outer edge) highlighted in red — the cleared"
            " area is shown in blue"
        ),
        "function": generate_find_cutting_arc,
    },
    {
        "heading": "find_cutting_arc",
        "caption": ("Cutting arcs from passes without islands"),
        "function": generate_find_cutting_arc_simple,
    },
    {
        "heading": "fillet_arc_ends",
        "caption": (
            "Cutting arcs (blue) with their ends rounded (red) to"
            " flow tangentially into the frontier"
        ),
        "function": generate_fillet_arc_ends,
    },
    {
        "heading": "fillet_arc_ends",
        "caption": ("Filleted cutting arcs without islands"),
        "function": generate_fillet_arc_ends_simple,
    },
    {
        "heading": "link_filleted_arcs",
        "caption": (
            "Filleted cutting arcs linked end-to-start into"
            " a single continuous polyline"
        ),
        "function": generate_link_arcs,
    },
    {
        "heading": "find_safe_sweep_end",
        "caption": (
            "Cutting arcs trimmed (red) by iterative sweep shortening"
            " until the tool sweep no longer collides with the boundary"
            " or islands — original arc shown in blue"
        ),
        "function": generate_find_safe_sweep_end,
    },
]
