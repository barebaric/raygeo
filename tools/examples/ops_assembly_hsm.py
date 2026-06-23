"""Generate visualisations of HSM motion assembly (Ops)."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import Normalize

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.fillet import append_end_fillets, trim_to_safe_fillet_span
from raygeo.geo.algo.medial_axis import MedialAxis
from raygeo.geo.algo.offset import compute_inset_region
from raygeo.geo.shape.arc import get_polyline_turn_sign
from raygeo.geo.shape.polygon import (
    get_polygon_centroid,
    get_polygons_group_difference,
)
from raygeo.geo.shape.polyline import trim_polyline_at
from raygeo.ops.assembly.hsm import (
    adaptive_entry,
    adaptive_peeling,
    adaptive_wavefronts,
    find_cutting_arc,
    link_arcs_to_ops,
    split_ordered_wavefronts,
)


def _ops_to_points(ops):
    """Extract (x, y, z, is_travel) for every moving command in *ops*."""
    pts = []
    for i in range(ops.len()):
        if ops.is_travel(i) or ops.is_cutting(i):
            ep = ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return pts


def _plot_ops_2d(ops, boundary, islands, title, cut_z=-5.0, safe_z=5.0):
    """Plot Ops as 2-D path — LineTo solid blue, MoveTo dashed orange."""
    fig, ax = plt.subplots(figsize=(7, 6))

    pts = _ops_to_points(ops)
    if pts:
        seg_x, seg_y, is_travel = [], [], False
        labeled = {}
        for x, y, z, travel in pts:
            if seg_x and travel != is_travel:
                if len(seg_x) >= 2:
                    key = "travel" if is_travel else "cut"
                    ax.plot(
                        seg_x,
                        seg_y,
                        color="#ff7f0e" if is_travel else "#1f77b4",
                        linewidth=2.0 if is_travel else 0.6,
                        linestyle="--" if is_travel else "-",
                        alpha=0.8,
                        label=key if key not in labeled else "",
                    )
                    labeled[key] = True
                seg_x, seg_y = [], []
            if not seg_x:
                is_travel = travel
            seg_x.append(x)
            seg_y.append(y)
        if len(seg_x) >= 2:
            key = "travel" if is_travel else "cut"
            ax.plot(
                seg_x,
                seg_y,
                color="#ff7f0e" if is_travel else "#1f77b4",
                linewidth=2.0 if is_travel else 0.6,
                linestyle="--" if is_travel else "-",
                alpha=0.8,
                label=key if key not in labeled else "",
            )
            labeled[key] = True

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
            )

    ax.set_aspect("equal")
    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def _plot_ops_3d(ops, boundary, islands, title, cut_z, safe_z):
    """Plot Ops as 3-D path coloured by Z height."""
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")

    pts = _ops_to_points(ops)
    if pts:
        arr = np.array([(p[0], p[1], p[2]) for p in pts])
        ax.plot(
            arr[:, 0],
            arr[:, 1],
            arr[:, 2],
            color="#1f77b4",
            linewidth=0.7,
            alpha=0.7,
        )
        ax.scatter(
            arr[:, 0],
            arr[:, 1],
            zs=arr[:, 2],  # type: ignore[call-arg]
            c=arr[:, 2],
            cmap="coolwarm",
            s=4,
            alpha=0.6,
            norm=Normalize(cut_z, safe_z),
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

    sm = plt.cm.ScalarMappable(cmap="coolwarm", norm=Normalize(cut_z, safe_z))
    sm.set_array([])
    fig.colorbar(sm, ax=ax, label="Z", shrink=0.6)
    fig.tight_layout()
    return fig


def generate_adaptive_peeling_2d():
    """adaptive_peeling on a rectangular pocket (2D)."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_peeling(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=5.0,
    )
    return _plot_ops_2d(
        ops,
        boundary,
        None,
        "adaptive_peeling (2-D) — Rectangular Pocket",
    )


def generate_adaptive_peeling_3d():
    """adaptive_peeling on a rectangular pocket (3D)."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    cut_z = -5.0
    safe_z = 5.0
    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=cut_z,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    ops = adaptive_peeling(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        cut_z=cut_z,
        safe_z=safe_z,
    )
    return _plot_ops_3d(
        ops,
        boundary,
        None,
        "adaptive_peeling (3-D) — Rectangular Pocket",
        cut_z,
        safe_z,
    )


def generate_adaptive_peeling_multi():
    """adaptive_peeling on a multi-island pocket with directed bite graph."""
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
    va, _total = compute_inset_region(boundary, tool_radius, islands)

    if cp:
        centre = get_polygon_centroid(cp[0])
    else:
        centre = get_polygon_centroid(boundary)

    graph = split_ordered_wavefronts(
        ClearedArea(initial=cp), step_over, va, 0.01, centre
    )
    ops = adaptive_peeling(
        cleared=ClearedArea(initial=cp),
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        step_over=step_over,
        cut_z=-5.0,
        safe_z=5.0,
    )

    n_passes = len(graph.bite_polys)
    total_bites = len(graph.parent)

    # Map global bite index to pass.
    def _bite_pass(bi: int) -> int:
        for pi, off in enumerate(graph.bite_offsets):
            nxt = (
                graph.bite_offsets[pi + 1]
                if pi + 1 < n_passes
                else total_bites
            )
            if off <= bi < nxt:
                return pi
        return 0

    # Per-bite node position: first arc's spatial midpoint,
    # falling back to polygon centroid when the bite has no arcs.
    arc_mids: list[tuple[float, float]] = []
    for arc in graph.arcs:
        if len(arc) == 1:
            arc_mids.append((arc[0][0], arc[0][1]))
            continue
        seg_lens = []
        total_len = 0.0
        for i in range(len(arc) - 1):
            dl = math.hypot(
                arc[i + 1][0] - arc[i][0], arc[i + 1][1] - arc[i][1]
            )
            seg_lens.append(dl)
            total_len += dl
        if total_len < 1e-12:
            arc_mids.append((arc[0][0], arc[0][1]))
            continue
        half = total_len / 2.0
        acc = 0.0
        mid_pt = (arc[-1][0], arc[-1][1])
        for i, dl in enumerate(seg_lens):
            if acc + dl >= half:
                t = (half - acc) / dl if dl > 1e-12 else 0.0
                mid_pt = (
                    arc[i][0] + t * (arc[i + 1][0] - arc[i][0]),
                    arc[i][1] + t * (arc[i + 1][1] - arc[i][1]),
                )
                break
            acc += dl
        arc_mids.append(mid_pt)

    node_pos: list[tuple[float, float]] = []
    node_has_arc: list[bool] = []
    for bi in range(total_bites):
        dfs_arcs = graph.bite_arcs[bi]
        if dfs_arcs:
            node_pos.append(arc_mids[dfs_arcs[0]])
            node_has_arc.append(True)
        else:
            pi = _bite_pass(bi)
            local = bi - graph.bite_offsets[pi]
            poly = graph.bite_polys[pi][local]
            cx = sum(p[0] for p in poly) / len(poly)
            cy = sum(p[1] for p in poly) / len(poly)
            node_pos.append((cx, cy))
            node_has_arc.append(False)

    grad_cmap = plt.colormaps["turbo"]

    def _draw_common(ax, show_graph, show_ops):
        ax.set_aspect("equal")

        bnd = np.array(list(boundary) + [boundary[0]])
        ax.plot(bnd[:, 0], bnd[:, 1], "k-", linewidth=2, label="Boundary")
        for isl in islands:
            isl_arr = np.array(list(isl) + [isl[0]])
            ax.fill(
                isl_arr[:, 0],
                isl_arr[:, 1],
                facecolor="#ccc",
                edgecolor="#999",
                linewidth=1.5,
            )

        if show_graph:
            # Directed edges: parent -> child (tree, one parent each).
            for child, p in enumerate(graph.parent):
                if p is None:
                    continue
                px, py = node_pos[p]
                cx, cy = node_pos[child]
                ax.plot(
                    [px, cx],
                    [py, cy],
                    color="#2ca02c",
                    linewidth=0.4,
                    alpha=0.35,
                    zorder=1,
                )

            # Node markers coloured by pass (turbo), arc-bearing only.
            arc_node_pos = [
                node_pos[bi] for bi in range(total_bites) if node_has_arc[bi]
            ]
            if arc_node_pos:
                arc_pass_vals = [
                    _bite_pass(bi)
                    for bi in range(total_bites)
                    if node_has_arc[bi]
                ]
                ax.scatter(
                    [c[0] for c in arc_node_pos],
                    [c[1] for c in arc_node_pos],
                    c=arc_pass_vals,
                    cmap=grad_cmap,
                    s=6,
                    alpha=0.4,
                    zorder=2,
                )

            # Arcs in turbo by pass index.
            for ai, arc in enumerate(graph.arcs):
                pi = graph.arc_passes[ai]
                colour = grad_cmap(pi / max(n_passes - 1, 1))
                ax.plot(
                    [p[0] for p in arc],
                    [p[1] for p in arc],
                    color=colour,
                    linewidth=2.0,
                    alpha=0.7,
                    zorder=3,
                )

        if show_ops:
            pts = _ops_to_points(ops)
            if pts:
                seg_x, seg_y, is_travel = [], [], False
                for x, y, z, travel in pts:
                    if seg_x and travel != is_travel:
                        if len(seg_x) >= 2:
                            color = "#ff7f0e" if is_travel else "#1f77b4"
                            ls = "--" if is_travel else "-"
                            ax.plot(
                                seg_x,
                                seg_y,
                                color=color,
                                linewidth=0.6,
                                linestyle=ls,
                                alpha=0.8,
                                zorder=4,
                            )
                        seg_x, seg_y = [], []
                    if not seg_x:
                        is_travel = travel
                    seg_x.append(x)
                    seg_y.append(y)
                if len(seg_x) >= 2:
                    color = "#ff7f0e" if is_travel else "#1f77b4"
                    ls = "--" if is_travel else "-"
                    ax.plot(
                        seg_x,
                        seg_y,
                        color=color,
                        linewidth=0.6,
                        linestyle=ls,
                        alpha=0.8,
                        zorder=4,
                    )

        ax.set_xlabel("X")
        ax.set_ylabel("Y")
        ax.grid(True, alpha=0.3)

    fig, (ax1, ax2) = plt.subplots(
        1, 2, figsize=(18, 8), constrained_layout=True
    )

    _draw_common(ax1, show_graph=True, show_ops=False)
    ax1.set_title(
        f"Directed bite graph — {total_bites} bites, {n_passes} passes"
    )

    _draw_common(ax2, show_graph=False, show_ops=True)
    ax2.set_title("Ops toolpath (cut blue, travel orange dashed)")

    sm = plt.cm.ScalarMappable(
        cmap=grad_cmap, norm=Normalize(0, max(n_passes - 1, 1))
    )
    sm.set_array([])
    cbar = fig.colorbar(
        sm,
        ax=[ax1, ax2],
        orientation="vertical",
        pad=0.02,
        shrink=0.7,
    )
    cbar.set_label("Pass index")

    return fig


def generate_link_arcs():
    """Show linked filleted arcs as an Ops path."""
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
                safe = trim_to_safe_fillet_span(
                    arc, boundary, islands, tool_radius, 0.0
                )
                if safe:
                    enter, exit_pt = safe
                    trimmed = trim_polyline_at(arc, enter, exit_pt)
                    if len(trimmed) >= 3:
                        side = get_polyline_turn_sign(arc)
                        fa = append_end_fillets(
                            trimmed, tool_radius, math.pi / 2, side
                        )
                        if len(fa) >= 3:
                            filleted_arcs.append(fa)
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    vi = get_polygons_group_difference(va, ca.fragments())
    uncleared = vi + islands

    mat_data = None
    try:
        mat = MedialAxis.compute(
            boundary,
            islands,
            tool_radius,
            step_over * 0.5,
        )
        mat_data = (mat.nodes, mat.edges)
    except Exception:
        pass

    ops = link_arcs_to_ops(
        arcs=filleted_arcs,
        uncleared=uncleared,
        cut_z=0.0,
        safe_z=2.0,
        mat=mat_data,
        safe_margin=tool_radius,
        cleared=ca.fragments(),
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

    pts = _ops_to_points(ops)
    if pts:
        seg_x, seg_y, is_travel = [], [], False
        for x, y, z, travel in pts:
            if seg_x and travel != is_travel:
                if len(seg_x) >= 2:
                    color = "#ff7f0e" if is_travel else "#1f77b4"
                    ls = "--" if is_travel else "-"
                    ax.plot(
                        seg_x,
                        seg_y,
                        color=color,
                        linewidth=0.5,
                        linestyle=ls,
                    )
                seg_x, seg_y = [], []
            if not seg_x:
                is_travel = travel
            seg_x.append(x)
            seg_y.append(y)
        if len(seg_x) >= 2:
            color = "#ff7f0e" if is_travel else "#1f77b4"
            ls = "--" if is_travel else "-"
            ax.plot(seg_x, seg_y, color=color, linewidth=0.5, linestyle=ls)

    ax.set_title("link_arcs_to_ops — linked filleted cutting arcs")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


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


def generate_find_cutting_arc():
    """Show cutting arcs from peeling passes (multi-island)."""
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


def generate_split_ordered_wavefronts():
    """Visualize the directed bite graph from split_ordered_wavefronts()."""
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
    va, _total = compute_inset_region(boundary, tool_radius, islands)

    if cp:
        centre = get_polygon_centroid(cp[0])
    else:
        centre = get_polygon_centroid(boundary)

    ca = ClearedArea(initial=cp)
    graph = split_ordered_wavefronts(ca, step_over, va, 0.01, centre)

    n_passes = len(graph.bite_polys)
    total_bites = len(graph.parent)
    n_arcs = len(graph.arcs)

    # Colour by pass index (outward gradient).
    arc_cmap = plt.colormaps["turbo"]
    arc_norm = Normalize(vmin=0, vmax=max(n_passes - 1, 1))

    def _bite_pass(gb: int) -> int:
        for pi, off in enumerate(graph.bite_offsets):
            nxt = (
                graph.bite_offsets[pi + 1]
                if pi + 1 < n_passes
                else total_bites
            )
            if off <= gb < nxt:
                return pi
        return 0

    # Spatial midpoint along each arc (for number placement).
    arc_mids: list[tuple[float, float]] = []
    for arc in graph.arcs:
        if len(arc) == 1:
            arc_mids.append((arc[0][0], arc[0][1]))
            continue
        seg_lens = []
        total_len = 0.0
        for i in range(len(arc) - 1):
            dl = math.hypot(
                arc[i + 1][0] - arc[i][0], arc[i + 1][1] - arc[i][1]
            )
            seg_lens.append(dl)
            total_len += dl
        if total_len < 1e-12:
            arc_mids.append((arc[0][0], arc[0][1]))
            continue
        half = total_len / 2.0
        acc = 0.0
        mid_pt = (arc[-1][0], arc[-1][1])
        for i, dl in enumerate(seg_lens):
            if acc + dl >= half:
                t = (half - acc) / dl if dl > 1e-12 else 0.0
                mid_pt = (
                    arc[i][0] + t * (arc[i + 1][0] - arc[i][0]),
                    arc[i][1] + t * (arc[i + 1][1] - arc[i][1]),
                )
                break
            acc += dl
        arc_mids.append(mid_pt)

    # Per-bite node position = first arc's spatial midpoint (so it
    # coincides with a numbered arc).  Falls back to polygon centroid
    # when the bite has no arcs.
    node_pos: list[tuple[float, float]] = []
    node_has_arc: list[bool] = []
    for bi in range(total_bites):
        dfs_arcs = graph.bite_arcs[bi]
        if dfs_arcs:
            node_pos.append(arc_mids[dfs_arcs[0]])
            node_has_arc.append(True)
        else:
            pi = _bite_pass(bi)
            local = bi - graph.bite_offsets[pi]
            poly = graph.bite_polys[pi][local]
            cx = sum(p[0] for p in poly) / len(poly)
            cy = sum(p[1] for p in poly) / len(poly)
            node_pos.append((cx, cy))
            node_has_arc.append(False)

    fig, ax = plt.subplots(figsize=(12, 8))
    ax.set_aspect("equal")

    # Boundary + islands.
    bnd = np.array(list(boundary) + [boundary[0]])
    ax.plot(bnd[:, 0], bnd[:, 1], "k-", linewidth=2, label="Boundary")
    for isl in islands:
        isl_arr = np.array(list(isl) + [isl[0]])
        ax.fill(
            isl_arr[:, 0],
            isl_arr[:, 1],
            facecolor="#ddd",
            edgecolor="#999",
            linewidth=1,
        )

    # Initial cleared area.
    for fidx, frag in enumerate(cp):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(
            fx,
            fy,
            color="#4488dd",
            alpha=0.2,
            label="Initial" if fidx == 0 else "",
        )

    # Directed edges: parent -> child (tree, one parent each).
    for child, p in enumerate(graph.parent):
        if p is None:
            continue
        px, py = node_pos[p]
        cx, cy = node_pos[child]
        ax.annotate(
            "",
            xy=(cx, cy),
            xytext=(px, py),
            arrowprops=dict(
                arrowstyle="->",
                color="#444",
                lw=0.6,
                alpha=0.7,
                shrinkA=6,
                shrinkB=6,
            ),
        )

    # Node markers at arc-bearing bites only.
    arc_node_pos = [
        node_pos[bi] for bi in range(total_bites) if node_has_arc[bi]
    ]
    if arc_node_pos:
        ax.scatter(
            [p[0] for p in arc_node_pos],
            [p[1] for p in arc_node_pos],
            s=4,
            c="#333",
            alpha=0.3,
            zorder=3,
        )

    # Cutting arcs coloured by pass index.
    for ai, arc in enumerate(graph.arcs):
        pi = graph.arc_passes[ai]
        colour = arc_cmap(arc_norm(pi))
        xs = [p[0] for p in arc]
        ys = [p[1] for p in arc]
        ax.plot(xs, ys, color=colour, linewidth=1.2, alpha=0.85)

    # Number each arc at its spatial midpoint.
    for ai, mid in enumerate(arc_mids):
        ax.text(
            mid[0],
            mid[1],
            str(ai),
            fontsize=5,
            ha="center",
            va="center",
            zorder=5,
            bbox=dict(
                boxstyle="circle,pad=0.12",
                facecolor="white",
                alpha=0.7,
                edgecolor="none",
            ),
        )

    sm = plt.cm.ScalarMappable(cmap=arc_cmap, norm=arc_norm)
    sm.set_array([])
    fig.colorbar(sm, ax=ax, label="Pass index", fraction=0.04, pad=0.02)

    ax.set_title(
        f"split_ordered_wavefronts() - {total_bites} bites,"
        f" {n_passes} passes, {n_arcs} arcs"
    )
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.hsm.md"]

__images__ = [
    {
        "heading": "find_cutting_arc",
        "caption": (
            "Cutting arcs from peeling passes in a pocket with three islands"
            " — each arc is the outer edge of a bite polygon"
        ),
        "function": generate_find_cutting_arc,
    },
    {
        "heading": "find_cutting_arc",
        "caption": ("Cutting arcs from passes without islands"),
        "function": generate_find_cutting_arc_simple,
    },
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
            "adaptive_peeling on a rectangular pocket —"
            " cutting arcs (blue, solid) at cut depth and"
            " travel links (orange, dashed) at safe Z"
        ),
        "function": generate_adaptive_peeling_2d,
    },
    {
        "heading": "adaptive_peeling",
        "caption": (
            "adaptive_peeling (3-D) — Z colouring shows"
            " cutting depth (blue) vs travel height (red)"
        ),
        "function": generate_adaptive_peeling_3d,
    },
    {
        "heading": "adaptive_peeling",
        "caption": (
            "adaptive_peeling on a three-island pocket — left: directed"
            " bite graph (green parent→child edges, node markers at bite"
            " centroids coloured by pass, arcs in turbo); right:"
            " resulting Ops toolpath (cut blue, travel orange dashed)"
        ),
        "function": generate_adaptive_peeling_multi,
    },
    {
        "heading": "link_arcs_to_ops",
        "caption": (
            "Pre-computed filleted arcs linked into an Ops with"
            " MAT-routed travel segments"
        ),
        "function": generate_link_arcs,
    },
    {
        "heading": "split_ordered_wavefronts",
        "caption": (
            "Cutting arcs from split_ordered_wavefronts() coloured by"
            " pass (turbo), with parent→child edges (grey arrows) and"
            " numbered labels at each arc midpoint"
        ),
        "function": generate_split_ordered_wavefronts,
    },
]
