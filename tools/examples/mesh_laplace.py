"""Mesh Laplace example images — Laplace solver visualisations."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.collections import LineCollection
from matplotlib.tri import Triangulation

from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.laplace import solve_laplace, solve_laplace_with_history


def _plot_mesh_wireframe(ax, mesh, edge_color="gray", edge_alpha=0.5):
    verts = mesh.vertices
    segments = []
    for a, b, c in mesh.triangles:
        segments.append((verts[a], verts[b]))
        segments.append((verts[b], verts[c]))
        segments.append((verts[c], verts[a]))
    lc = LineCollection(
        segments, colors=edge_color, linewidths=0.4, alpha=edge_alpha
    )
    ax.add_collection(lc)


def _plot_boundary(ax, mesh, tag, color, lw):
    verts = mesh.vertices
    boundary_edges = set()
    for ti in range(len(mesh.triangles)):
        for ei in range(3):
            if mesh.adjacency[ti * 3 + ei] == -1:
                a = mesh.triangles[ti][ei]
                b = mesh.triangles[ti][(ei + 1) % 3]
                if (
                    mesh.boundary_tags[a] == tag
                    and mesh.boundary_tags[b] == tag
                ):
                    boundary_edges.add((min(a, b), max(a, b)))
    for a, b in boundary_edges:
        ax.plot(
            [verts[a][0], verts[b][0]],
            [verts[a][1], verts[b][1]],
            color=color,
            linewidth=lw,
        )


def generate_overview():
    """Laplace overview."""
    outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
    hole = [(30, 30), (70, 30), (70, 70), (30, 70)]
    mesh = build_triangle_mesh(outer, [hole], tool_radius=0.0, min_angle=20.0)

    xs_o, ys_o = zip(*outer)
    xs_h, ys_h = zip(*hole)

    u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)

    verts = mesh.vertices
    x_vals = np.asarray([v[0] for v in verts])
    y_vals = np.asarray([v[1] for v in verts])
    tris = np.asarray(mesh.triangles)
    u_arr = np.asarray(u)
    triang = Triangulation(x_vals, y_vals, tris)

    fig, ax = plt.subplots(figsize=(7, 7))
    ax.set_aspect("equal")
    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 105)

    tcf = ax.tripcolor(triang, u_arr, cmap="coolwarm", shading="gouraud")
    cbar = fig.colorbar(tcf, ax=ax, shrink=0.8)
    cbar.set_label("Scalar field u(x,y)", fontsize=10)

    ax.plot(
        list(xs_o) + [xs_o[0]],
        list(ys_o) + [ys_o[0]],
        color="darkred",
        linewidth=2,
        label="Outer (u=1)",
    )
    ax.plot(
        list(xs_h) + [xs_h[0]],
        list(ys_h) + [ys_h[0]],
        color="darkblue",
        linewidth=2,
        label="Hole (u=0)",
    )
    levels = np.linspace(0, 1, 11)
    ax.tricontour(
        triang,
        u_arr,
        levels=levels,
        colors="black",
        linewidths=0.5,
        alpha=0.3,
    )
    ax.set_title(
        "Laplace solution Δu = 0 via linear FEM\n"
        "(u=0 on hole, u=1 on outer boundary)",
        fontsize=12,
    )
    ax.legend(fontsize=10, loc="upper right")
    ax.grid(True, alpha=0.2)
    fig.tight_layout()
    return fig


def generate_l_shape_solution():
    """L-shape laplace."""
    l_outer = [(0, 0), (80, 0), (80, 20), (20, 20), (20, 80), (0, 80)]
    l_mesh = build_triangle_mesh(l_outer, [], tool_radius=0.0, min_angle=20.0)
    l_u = solve_laplace(l_mesh, max_iter=2000, tolerance=1e-10)

    lx = np.asarray([v[0] for v in l_mesh.vertices])
    ly = np.asarray([v[1] for v in l_mesh.vertices])
    ltris = np.asarray(l_mesh.triangles)
    l_u_arr = np.asarray(l_u)
    l_triang = Triangulation(lx, ly, ltris)

    fig4, ax4 = plt.subplots(figsize=(7, 7))
    ax4.set_aspect("equal")
    ax4.set_xlim(-5, 85)
    ax4.set_ylim(-5, 85)

    tcf4 = ax4.tripcolor(l_triang, l_u_arr, cmap="coolwarm", shading="gouraud")
    cbar4 = fig4.colorbar(tcf4, ax=ax4, shrink=0.8)
    cbar4.set_label("Scalar field u(x,y)", fontsize=10)

    xs_l, ys_l = zip(*l_outer)
    ax4.plot(
        list(xs_l) + [xs_l[0]],
        list(ys_l) + [ys_l[0]],
        color="darkred",
        linewidth=2,
        label="Outer (u=1)",
    )
    levels_l = np.linspace(0, 1, 11)
    ax4.tricontour(
        l_triang,
        l_u_arr,
        levels=levels_l,
        colors="black",
        linewidths=0.5,
        alpha=0.3,
    )
    ax4.set_title(
        "Laplace solution Δu = 0 on L-shaped pocket\n(u=1 on outer boundary)",
        fontsize=12,
    )
    ax4.legend(fontsize=10, loc="upper right")
    ax4.grid(True, alpha=0.2)
    fig4.tight_layout()
    return fig4


def generate_convergence():
    """Convergence."""
    outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
    hole = [(30, 30), (70, 30), (70, 70), (30, 70)]
    mesh = build_triangle_mesh(outer, [hole], tool_radius=0.0, min_angle=20.0)

    _, residuals = solve_laplace_with_history(
        mesh, max_iter=500, tolerance=1e-12
    )

    fig6, ax6 = plt.subplots(figsize=(7, 4))
    ax6.semilogy(residuals, "b.-", markersize=3, linewidth=1.0)
    ax6.set_xlabel("CG iteration", fontsize=11)
    ax6.set_ylabel(r"Residual norm $\|r\|_2$", fontsize=11)
    ax6.set_title(
        f"Conjugate gradient convergence\n"
        f"({len(mesh.vertices)} vertices, {len(mesh.triangles)} triangles, "
        f"{len(residuals)} iterations)",
        fontsize=12,
    )
    ax6.grid(True, alpha=0.3)
    fig6.tight_layout()
    return fig6


def generate_stiffness_spy():
    """Stiffness spy."""
    outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
    hole = [(30, 30), (70, 30), (70, 70), (30, 70)]
    mesh = build_triangle_mesh(outer, [hole], tool_radius=0.0, min_angle=20.0)

    verts = mesh.vertices
    x_vals = np.asarray([v[0] for v in verts])
    y_vals = np.asarray([v[1] for v in verts])
    tris = np.asarray(mesh.triangles)
    u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
    u_arr = np.asarray(u)
    triang = Triangulation(x_vals, y_vals, tris)

    edge_stiffness = {}
    for ti, (a, b, c) in enumerate(mesh.triangles):
        vi = verts[a]
        vj = verts[b]
        vk = verts[c]
        bi = vj[1] - vk[1]
        ci = vk[0] - vj[0]
        bj = vk[1] - vi[1]
        cj = vi[0] - vk[0]
        bk = vi[1] - vj[1]
        ck = vj[0] - vi[0]
        area2 = bi * cj - bj * ci
        if area2 < 1e-30:
            continue
        inv_a = 0.5 / area2
        k_ab = abs((bi * bj + ci * cj) * inv_a)
        k_bc = abs((bj * bk + cj * ck) * inv_a)
        k_ca = abs((bk * bi + ck * ci) * inv_a)
        for (p, q), w in [
            ((min(a, b), max(a, b)), k_ab),
            ((min(b, c), max(b, c)), k_bc),
            ((min(c, a), max(c, a)), k_ca),
        ]:
            edge_stiffness[(p, q)] = edge_stiffness.get((p, q), 0.0) + w

    edge_vals = list(edge_stiffness.values())
    emin, emax = min(edge_vals), max(edge_vals) if edge_vals else (0.0, 1.0)

    fig7, ax7 = plt.subplots(figsize=(7, 7))
    ax7.set_aspect("equal")
    ax7.set_xlim(-5, 105)
    ax7.set_ylim(-5, 105)

    tcf7 = ax7.tripcolor(triang, u_arr, cmap="coolwarm", shading="gouraud")
    cbar7 = fig7.colorbar(tcf7, ax=ax7, shrink=0.8)
    cbar7.set_label("Scalar field u(x,y)", fontsize=10)

    xs_o, ys_o = zip(*outer)
    xs_h, ys_h = zip(*hole)

    for (a, b), w in edge_stiffness.items():
        frac = (w - emin) / (emax - emin + 1e-30)
        lw = 0.3 + 3.5 * frac
        alpha = 0.15 + 0.85 * frac
        ax7.plot(
            [verts[a][0], verts[b][0]],
            [verts[a][1], verts[b][1]],
            color="black",
            linewidth=lw,
            alpha=alpha,
        )

    ax7.plot(
        list(xs_o) + [xs_o[0]],
        list(ys_o) + [ys_o[0]],
        color="darkred",
        linewidth=2,
        label="Outer (u=1)",
    )
    ax7.plot(
        list(xs_h) + [xs_h[0]],
        list(ys_h) + [ys_h[0]],
        color="darkblue",
        linewidth=2,
        label="Hole (u=0)",
    )
    ax7.set_title(
        "Stiffness matrix edge weights on the mesh\n"
        "Line thickness ∝ |Kᵢⱼ| — thicker edges contribute"
        " more to the Laplacian",
        fontsize=12,
    )
    ax7.legend(fontsize=10, loc="upper right")
    ax7.grid(True, alpha=0.2)
    fig7.tight_layout()
    return fig7


def generate_multi_island():
    """Multi-island laplace."""
    outer_mi = [(0, 0), (100, 0), (100, 100), (0, 100)]
    holes_mi = [
        [(10, 60), (30, 60), (30, 80), (10, 80)],
        [(60, 55), (85, 55), (85, 85), (60, 85)],
        [(10, 10), (35, 10), (35, 30), (10, 30)],
        [(60, 15), (75, 15), (75, 35), (60, 35)],
    ]
    mi_mesh = build_triangle_mesh(
        outer_mi, holes_mi, tool_radius=0.0, min_angle=20.0
    )
    mi_u = solve_laplace(mi_mesh, max_iter=2000, tolerance=1e-10)

    mi_verts = mi_mesh.vertices
    mi_x = np.asarray([v[0] for v in mi_verts])
    mi_y = np.asarray([v[1] for v in mi_verts])
    mi_tris = np.asarray(mi_mesh.triangles)
    mi_u_arr = np.asarray(mi_u)
    mi_triang = Triangulation(mi_x, mi_y, mi_tris)

    xs_mo, ys_mo = zip(*outer_mi)

    fig9, ax9 = plt.subplots(figsize=(7, 7))
    ax9.set_aspect("equal")
    ax9.set_xlim(-5, 105)
    ax9.set_ylim(-5, 105)

    tcf9 = ax9.tripcolor(
        mi_triang, mi_u_arr, cmap="coolwarm", shading="gouraud"
    )
    cbar9 = fig9.colorbar(tcf9, ax=ax9, shrink=0.8)
    cbar9.set_label("Scalar field u(x,y)", fontsize=10)

    ax9.plot(
        list(xs_mo) + [xs_mo[0]],
        list(ys_mo) + [ys_mo[0]],
        color="darkred",
        linewidth=2,
        label="Outer (u=1)",
    )
    for hi, hole in enumerate(holes_mi):
        xs_h_mi, ys_h_mi = zip(*hole)
        label = "Inners (u=0)" if hi == 0 else None
        ax9.plot(
            list(xs_h_mi) + [xs_h_mi[0]],
            list(ys_h_mi) + [ys_h_mi[0]],
            color="darkblue",
            linewidth=2,
            label=label,
        )
    levels_mi = np.linspace(0, 1, 9)
    ax9.tricontour(
        mi_triang,
        mi_u_arr,
        levels=levels_mi,
        colors="black",
        linewidths=0.5,
        alpha=0.3,
    )
    ax9.set_title(
        "Laplace solution Δu = 0 on multi-island domain\n"
        "(u=0 on islands, u=1 on outer boundary)",
        fontsize=12,
    )
    ax9.legend(fontsize=10, loc="upper right")
    ax9.grid(True, alpha=0.2)
    fig9.tight_layout()
    return fig9


__docs_target__ = ["raygeo.mesh.laplace.md"]
__images__ = [
    {
        "heading": "solve_laplace",
        "caption": (
            "Laplace solution — contours morph smoothly from hole to boundary"
        ),
        "function": generate_overview,
    },
    {
        "heading": "solve_laplace",
        "caption": "Laplace solution on an L-shaped domain",
        "function": generate_l_shape_solution,
    },
    {
        "heading": "solve_laplace_with_history",
        "caption": "Conjugate gradient convergence — residual norm per"
        " iteration",
        "function": generate_convergence,
    },
    {
        "heading": "solve_laplace",
        "caption": (
            "Stiffness matrix edge weights on the mesh — line"
            " thickness ∝ |Kᵢⱼ|"
        ),
        "function": generate_stiffness_spy,
    },
    {
        "heading": "solve_laplace",
        "caption": "Laplace solution on a multi-island domain — contour"
        " lines morph smoothly between four inner islands and the outer"
        " boundary",
        "function": generate_multi_island,
    },
]
