"""Generate PDE mesh example images."""

__images__ = [
    {
        "stem": "pde-mesh-triangulation",
        "caption": "CDT triangulation of a square pocket with centred hole",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "build_triangle_mesh",
    },
    {
        "stem": "pde-mesh-l-shape",
        "caption": "CDT triangulation of an L-shaped pocket",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "build_triangle_mesh",
    },
    {
        "stem": "pde-mesh-laplace",
        "caption": "Laplace solution — contours morph smoothly from hole to"
        " boundary",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "solve_laplace",
    },
    {
        "stem": "pde-mesh-l-shape-solution",
        "caption": "Laplace solution on an L-shaped domain",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "solve_laplace",
    },
    {
        "stem": "pde-mesh-gradient-field",
        "caption": "Gradient field ∇u (red) and perpendicular flow ∇u⊥ (blue)"
        " on the Laplace solution",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "compute_gradient_field",
    },
    {
        "stem": "pde-mesh-convergence",
        "caption": "Conjugate gradient convergence — residual norm per"
        " iteration",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "solve_laplace_with_history",
    },
    {
        "stem": "pde-mesh-stiffness-spy",
        "caption": (
            "Stiffness matrix edge weights on the mesh — line"
            " thickness ∝ |Kᵢⱼ|"
        ),
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "solve_laplace",
    },
    {
        "stem": "pde-mesh-multi-island",
        "caption": "CDT triangulation of a square pocket with multiple"
        " islands",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "build_triangle_mesh",
    },
    {
        "stem": "pde-mesh-multi-island-laplace",
        "caption": "Laplace solution on a multi-island domain — contour"
        " lines morph smoothly between four inner islands and the outer"
        " boundary",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "solve_laplace",
    },
    {
        "stem": "pde-mesh-multi-island-gradient",
        "caption": "Gradient field ∇u (red) and perpendicular flow ∇u⊥"
        " (blue) on a multi-island domain",
        "doc": "raygeo.geo.algo.pde_mesh.md",
        "heading": "compute_gradient_field",
    },
]

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.collections import LineCollection
from matplotlib.tri import Triangulation

from raygeo.geo.algo.pde_mesh import (
    build_triangle_mesh,
    compute_gradient_field,
    solve_laplace,
    solve_laplace_with_history,
)


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
    # Collect all boundary edges from adjacency (-1 means boundary)
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


def generate_examples(output_dir):
    images = []

    # ── Example 1: square with hole triangulation ─────────────────────────
    outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
    hole = [(30, 30), (70, 30), (70, 70), (30, 70)]
    mesh = build_triangle_mesh(outer, [hole], tool_radius=0.0, min_angle=20.0)

    fig, ax = plt.subplots(figsize=(7, 7))
    ax.set_aspect("equal")
    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 105)

    _plot_mesh_wireframe(ax, mesh)
    _plot_boundary(ax, mesh, "outer", "crimson", 2.0)
    _plot_boundary(ax, mesh, "inner", "royalblue", 2.0)

    # Plot original boundaries as filled regions for context
    xs_o, ys_o = zip(*outer)
    ax.fill(xs_o, ys_o, alpha=0.04, color="crimson")
    xs_h, ys_h = zip(*hole)
    ax.fill(xs_h, ys_h, alpha=0.08, color="royalblue")
    ax.plot(
        list(xs_o) + [xs_o[0]],
        list(ys_o) + [ys_o[0]],
        color="crimson",
        linewidth=2.5,
        label="Outer boundary (u=1)",
    )
    ax.plot(
        list(xs_h) + [xs_h[0]],
        list(ys_h) + [ys_h[0]],
        color="royalblue",
        linewidth=2.5,
        label="Hole boundary (u=0)",
    )

    ax.set_title(
        f"Constrained Delaunay triangulation\n"
        f"({len(mesh.vertices)} vertices, {len(mesh.triangles)} triangles)",
        fontsize=12,
    )
    ax.legend(fontsize=10, loc="upper right")
    ax.grid(True, alpha=0.2)
    fig.tight_layout()
    path = output_dir / "pde-mesh-triangulation.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "pde-mesh-triangulation.png",
            "caption": (
                "CDT triangulation of a square pocket with centred hole"
            ),
        }
    )

    # ── Example 2: Laplace solution as filled contour ─────────────────────
    u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)

    verts = mesh.vertices
    x_vals = np.asarray([v[0] for v in verts])
    y_vals = np.asarray([v[1] for v in verts])
    tris = np.asarray(mesh.triangles)
    u_arr = np.asarray(u)
    triang = Triangulation(x_vals, y_vals, tris)

    fig2, ax2 = plt.subplots(figsize=(7, 7))
    ax2.set_aspect("equal")
    ax2.set_xlim(-5, 105)
    ax2.set_ylim(-5, 105)

    tcf = ax2.tripcolor(triang, u_arr, cmap="coolwarm", shading="gouraud")
    cbar = fig2.colorbar(tcf, ax=ax2, shrink=0.8)
    cbar.set_label("Scalar field u(x,y)", fontsize=10)

    # Overlay outer/hole boundaries
    ax2.plot(
        list(xs_o) + [xs_o[0]],
        list(ys_o) + [ys_o[0]],
        color="darkred",
        linewidth=2,
        label="Outer (u=1)",
    )
    ax2.plot(
        list(xs_h) + [xs_h[0]],
        list(ys_h) + [ys_h[0]],
        color="darkblue",
        linewidth=2,
        label="Hole (u=0)",
    )

    # Draw a few contour lines
    levels = np.linspace(0, 1, 11)
    ax2.tricontour(
        triang,
        u_arr,
        levels=levels,
        colors="black",
        linewidths=0.5,
        alpha=0.3,
    )

    ax2.set_title(
        "Laplace solution Δu = 0 via linear FEM\n"
        "(u=0 on hole, u=1 on outer boundary)",
        fontsize=12,
    )
    ax2.legend(fontsize=10, loc="upper right")
    ax2.grid(True, alpha=0.2)
    fig2.tight_layout()
    path2 = output_dir / "pde-mesh-laplace.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "pde-mesh-laplace.png",
            "caption": (
                "Laplace solution on triangle mesh — contours morph smoothly"
                " from the inner hole (u=0) to the outer boundary (u=1)"
            ),
        }
    )

    # ── Example 3: L-shape triangulation ──────────────────────────────────
    l_outer = [(0, 0), (80, 0), (80, 20), (20, 20), (20, 80), (0, 80)]
    l_mesh = build_triangle_mesh(l_outer, [], tool_radius=0.0, min_angle=20.0)

    fig3, ax3 = plt.subplots(figsize=(7, 7))
    ax3.set_aspect("equal")
    ax3.set_xlim(-5, 85)
    ax3.set_ylim(-5, 85)

    _plot_mesh_wireframe(ax3, l_mesh)
    _plot_boundary(ax3, l_mesh, "outer", "crimson", 2.0)

    xs_l, ys_l = zip(*l_outer)
    ax3.fill(xs_l, ys_l, alpha=0.04, color="crimson")
    ax3.plot(
        list(xs_l) + [xs_l[0]],
        list(ys_l) + [ys_l[0]],
        color="crimson",
        linewidth=2.5,
        label="Outer boundary (u=1)",
    )

    ax3.set_title(
        f"CDT triangulation of an L-shaped pocket\n"
        f"({len(l_mesh.vertices)} vertices,"
        f" {len(l_mesh.triangles)} triangles)",
        fontsize=12,
    )
    ax3.legend(fontsize=10, loc="upper right")
    ax3.grid(True, alpha=0.2)
    fig3.tight_layout()
    path3 = output_dir / "pde-mesh-l-shape.png"
    fig3.savefig(path3, dpi=150)
    plt.close(fig3)
    images.append(
        {
            "path": "pde-mesh-l-shape.png",
            "caption": (
                "CDT triangulation of an L-shaped pocket — the constrained"
                " edges preserve the re-entrant corner"
            ),
        }
    )

    # ── Example 4: L-shape Laplace solution ───────────────────────────────
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
    path4 = output_dir / "pde-mesh-l-shape-solution.png"
    fig4.savefig(path4, dpi=150)
    plt.close(fig4)
    images.append(
        {
            "path": "pde-mesh-l-shape-solution.png",
            "caption": (
                "Laplace solution on an L-shaped mesh — the scalar field"
                " captures the re-entrant geometry naturally"
            ),
        }
    )

    # ── Example 5: Gradient field quiver plot ────────────────────────────
    # Use the hole mesh + Laplace solution from example 2
    grad = compute_gradient_field(mesh, u)

    # Triangle centroids and gradient vectors
    cx_arr = np.empty(len(mesh.triangles))
    cy_arr = np.empty(len(mesh.triangles))
    gx_arr = np.empty(len(mesh.triangles))
    gy_arr = np.empty(len(mesh.triangles))
    for ti, (a, b, c) in enumerate(mesh.triangles):
        cx_arr[ti] = (verts[a][0] + verts[b][0] + verts[c][0]) / 3.0
        cy_arr[ti] = (verts[a][1] + verts[b][1] + verts[c][1]) / 3.0
        gx_arr[ti], gy_arr[ti] = grad[ti]

    # Normalize for quiver — skip zero-length vectors
    mag = np.hypot(gx_arr, gy_arr)
    valid = mag > 1e-10
    qx = np.where(valid, gx_arr / mag, 0.0)
    qy = np.where(valid, gy_arr / mag, 0.0)
    # Perpendicular (rotated 90° CCW)
    px = np.where(valid, -gy_arr / mag, 0.0)
    py = np.where(valid, gx_arr / mag, 0.0)

    fig5, ax5 = plt.subplots(figsize=(7, 7))
    ax5.set_aspect("equal")
    ax5.set_xlim(-5, 105)
    ax5.set_ylim(-5, 105)

    tcf5 = ax5.tripcolor(triang, u_arr, cmap="coolwarm", shading="gouraud")
    ax5.quiver(
        cx_arr[valid],
        cy_arr[valid],
        qx[valid],
        qy[valid],
        color="darkred",
        alpha=0.7,
        scale=25,
        width=0.002,
        label=r"$\nabla u$",
    )
    ax5.quiver(
        cx_arr[valid],
        cy_arr[valid],
        px[valid],
        py[valid],
        color="darkblue",
        alpha=0.5,
        scale=25,
        width=0.002,
        label=r"$\nabla u^\perp$",
    )
    ax5.plot(
        list(xs_o) + [xs_o[0]],
        list(ys_o) + [ys_o[0]],
        color="black",
        linewidth=1.5,
    )
    ax5.plot(
        list(xs_h) + [xs_h[0]],
        list(ys_h) + [ys_h[0]],
        color="black",
        linewidth=1.5,
    )
    cbar5 = fig5.colorbar(tcf5, ax=ax5, shrink=0.8)
    cbar5.set_label("u(x,y)", fontsize=10)
    ax5.set_title(
        "Gradient field on Laplace solution\n"
        r"$\nabla u$ (red, normal to contours), "
        r"$\nabla u^\perp$ (blue, along contours)",
        fontsize=12,
    )
    ax5.legend(fontsize=10, loc="upper right")
    ax5.grid(True, alpha=0.2)
    fig5.tight_layout()
    path5 = output_dir / "pde-mesh-gradient-field.png"
    fig5.savefig(path5, dpi=150)
    plt.close(fig5)
    images.append(
        {
            "path": "pde-mesh-gradient-field.png",
            "caption": (
                "Gradient field ∇u (red) and perpendicular flow ∇u⊥"
                " (blue) on the FEM Laplace solution"
            ),
        }
    )

    # ── Example 6: Convergence plot ─────────────────────────────────────
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
    path6 = output_dir / "pde-mesh-convergence.png"
    fig6.savefig(path6, dpi=150)
    plt.close(fig6)
    images.append(
        {
            "path": "pde-mesh-convergence.png",
            "caption": (
                "Conjugate gradient convergence — residual norm decreases"
                " exponentially as the solver progresses"
            ),
        }
    )

    # ── Example 7: Stiffness matrix edge weights ────────────────────────
    # Re-use the hole mesh from example 2. Compute the local stiffness
    # matrix per triangle and accumulate off-diagonal magnitudes onto mesh
    # edges. Thicker edges = larger |Kᵢⱼ| contribution.
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
        "Line thickness ∝ |K\u1d62\u2c7c| — thicker edges contribute"
        " more to the Laplacian",
        fontsize=12,
    )
    ax7.legend(fontsize=10, loc="upper right")
    ax7.grid(True, alpha=0.2)
    fig7.tight_layout()
    path7 = output_dir / "pde-mesh-stiffness-spy.png"
    fig7.savefig(path7, dpi=150)
    plt.close(fig7)
    images.append(
        {
            "path": "pde-mesh-stiffness-spy.png",
            "caption": (
                "Stiffness matrix visualised directly on the mesh — edge"
                " thickness is proportional to |K\u1d62\u2c7c|. Shorter"
                " edges (in denser regions) produce larger stiffness values,"
                " driving the Laplace solution\u2019s smoothness."
            ),
        }
    )

    # ── Example 8: Multi-island triangulation ────────────────────────────
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

    fig8, ax8 = plt.subplots(figsize=(7, 7))
    ax8.set_aspect("equal")
    ax8.set_xlim(-5, 105)
    ax8.set_ylim(-5, 105)

    _plot_mesh_wireframe(ax8, mi_mesh)
    _plot_boundary(ax8, mi_mesh, "outer", "crimson", 2.0)
    _plot_boundary(ax8, mi_mesh, "inner", "royalblue", 2.0)

    xs_mo, ys_mo = zip(*outer_mi)
    ax8.fill(xs_mo, ys_mo, alpha=0.04, color="crimson")
    ax8.plot(
        list(xs_mo) + [xs_mo[0]],
        list(ys_mo) + [ys_mo[0]],
        color="crimson",
        linewidth=2.5,
        label="Outer boundary (u=1)",
    )
    for hi, hole in enumerate(holes_mi):
        xs_h, ys_h = zip(*hole)
        label = "Inner boundaries (u=0)" if hi == 0 else None
        ax8.fill(xs_h, ys_h, alpha=0.08, color="royalblue")
        ax8.plot(
            list(xs_h) + [xs_h[0]],
            list(ys_h) + [ys_h[0]],
            color="royalblue",
            linewidth=2.5,
            label=label,
        )

    ax8.set_title(
        f"CDT triangulation with multiple islands\n"
        f"({len(mi_mesh.vertices)} vertices,"
        f" {len(mi_mesh.triangles)} triangles)",
        fontsize=12,
    )
    ax8.legend(fontsize=10, loc="upper right")
    ax8.grid(True, alpha=0.2)
    fig8.tight_layout()
    path8 = output_dir / "pde-mesh-multi-island.png"
    fig8.savefig(path8, dpi=150)
    plt.close(fig8)
    images.append(
        {
            "path": "pde-mesh-multi-island.png",
            "caption": (
                "CDT triangulation of a square pocket with four inner"
                " islands — each island is treated as an inner boundary"
                " (u=0)"
            ),
        }
    )

    # ── Example 9: Multi-island Laplace solution ─────────────────────────
    mi_u = solve_laplace(mi_mesh, max_iter=2000, tolerance=1e-10)

    mi_verts = mi_mesh.vertices
    mi_x = np.asarray([v[0] for v in mi_verts])
    mi_y = np.asarray([v[1] for v in mi_verts])
    mi_tris = np.asarray(mi_mesh.triangles)
    mi_u_arr = np.asarray(mi_u)
    mi_triang = Triangulation(mi_x, mi_y, mi_tris)

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
        xs_h, ys_h = zip(*hole)
        label = "Inners (u=0)" if hi == 0 else None
        ax9.plot(
            list(xs_h) + [xs_h[0]],
            list(ys_h) + [ys_h[0]],
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
    path9 = output_dir / "pde-mesh-multi-island-laplace.png"
    fig9.savefig(path9, dpi=150)
    plt.close(fig9)
    images.append(
        {
            "path": "pde-mesh-multi-island-laplace.png",
            "caption": (
                "Laplace solution on a triangle mesh with four inner"
                " islands — the scalar field smoothly transitions from"
                " u=0 on each island to u=1 on the outer boundary"
            ),
        }
    )

    # ── Example 10: Multi-island gradient field ──────────────────────────
    mi_grad = compute_gradient_field(mi_mesh, mi_u)

    mi_cx = np.empty(len(mi_mesh.triangles))
    mi_cy = np.empty(len(mi_mesh.triangles))
    mi_gx = np.empty(len(mi_mesh.triangles))
    mi_gy = np.empty(len(mi_mesh.triangles))
    for ti, (a, b, c) in enumerate(mi_mesh.triangles):
        mi_cx[ti] = (mi_verts[a][0] + mi_verts[b][0] + mi_verts[c][0]) / 3.0
        mi_cy[ti] = (mi_verts[a][1] + mi_verts[b][1] + mi_verts[c][1]) / 3.0
        mi_gx[ti], mi_gy[ti] = mi_grad[ti]

    mi_mag = np.hypot(mi_gx, mi_gy)
    mi_valid = mi_mag > 1e-10
    mi_qx = np.where(mi_valid, mi_gx / mi_mag, 0.0)
    mi_qy = np.where(mi_valid, mi_gy / mi_mag, 0.0)
    mi_px = np.where(mi_valid, -mi_gy / mi_mag, 0.0)
    mi_py = np.where(mi_valid, mi_gx / mi_mag, 0.0)

    fig10, ax10 = plt.subplots(figsize=(7, 7))
    ax10.set_aspect("equal")
    ax10.set_xlim(-5, 105)
    ax10.set_ylim(-5, 105)

    tcf10 = ax10.tripcolor(
        mi_triang, mi_u_arr, cmap="coolwarm", shading="gouraud"
    )
    ax10.quiver(
        mi_cx[mi_valid],
        mi_cy[mi_valid],
        mi_qx[mi_valid],
        mi_qy[mi_valid],
        color="darkred",
        alpha=0.7,
        scale=25,
        width=0.002,
        label=r"$\nabla u$",
    )
    ax10.quiver(
        mi_cx[mi_valid],
        mi_cy[mi_valid],
        mi_px[mi_valid],
        mi_py[mi_valid],
        color="darkblue",
        alpha=0.5,
        scale=25,
        width=0.002,
        label=r"$\nabla u^\perp$",
    )
    ax10.plot(
        list(xs_mo) + [xs_mo[0]],
        list(ys_mo) + [ys_mo[0]],
        color="black",
        linewidth=1.5,
    )
    for hole in holes_mi:
        xs_h, ys_h = zip(*hole)
        ax10.plot(
            list(xs_h) + [xs_h[0]],
            list(ys_h) + [ys_h[0]],
            color="black",
            linewidth=1.5,
        )

    cbar10 = fig10.colorbar(tcf10, ax=ax10, shrink=0.8)
    cbar10.set_label("u(x,y)", fontsize=10)
    ax10.set_title(
        "Gradient field on multi-island Laplace solution\n"
        r"$\nabla u$ (red), $\nabla u^\perp$ (blue)",
        fontsize=12,
    )
    ax10.legend(fontsize=10, loc="upper right")
    ax10.grid(True, alpha=0.2)
    fig10.tight_layout()
    path10 = output_dir / "pde-mesh-multi-island-gradient.png"
    fig10.savefig(path10, dpi=150)
    plt.close(fig10)
    images.append(
        {
            "path": "pde-mesh-multi-island-gradient.png",
            "caption": (
                "Gradient field on a multi-island Laplace solution —"
                " the vector fields flow between the four inner islands"
                " and the outer boundary"
            ),
        }
    )

    return {
        "title": "PDE Mesh",
        "description": (
            "Constrained Delaunay triangulation (CDT) of 2D polygon domains"
            " and FEM Laplace equation solving for scalar field generation."
        ),
        "images": images,
    }
