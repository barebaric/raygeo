"""Generate PDE mesh example images."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.collections import LineCollection

from raygeo.geo.algo.pde_mesh import build_triangle_mesh, solve_laplace


def _plot_mesh_wireframe(ax, mesh, edge_color="gray", edge_alpha=0.5):
    verts = mesh.vertices
    segments = []
    for a, b, c in mesh.triangles:
        segments.append((verts[a], verts[b]))
        segments.append((verts[b], verts[c]))
        segments.append((verts[c], verts[a]))
    lc = LineCollection(segments, colors=edge_color, linewidths=0.4, alpha=edge_alpha)
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
                if mesh.boundary_tags[a] == tag and mesh.boundary_tags[b] == tag:
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
    ax.plot(list(xs_o) + [xs_o[0]], list(ys_o) + [ys_o[0]], color="crimson", linewidth=2.5, label="Outer boundary (u=1)")
    ax.plot(list(xs_h) + [xs_h[0]], list(ys_h) + [ys_h[0]], color="royalblue", linewidth=2.5, label="Hole boundary (u=0)")

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
            "caption": "CDT triangulation of a square pocket with centred hole",
        }
    )

    # ── Example 2: Laplace solution as filled contour ─────────────────────
    u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)

    verts = mesh.vertices
    x_vals = [v[0] for v in verts]
    y_vals = [v[1] for v in verts]
    tris = mesh.triangles

    fig2, ax2 = plt.subplots(figsize=(7, 7))
    ax2.set_aspect("equal")
    ax2.set_xlim(-5, 105)
    ax2.set_ylim(-5, 105)

    tcf = ax2.tripcolor(x_vals, y_vals, tris, u, cmap="coolwarm", shading="gouraud")
    cbar = fig2.colorbar(tcf, ax=ax2, shrink=0.8)
    cbar.set_label("Scalar field u(x,y)", fontsize=10)

    # Overlay outer/hole boundaries
    ax2.plot(list(xs_o) + [xs_o[0]], list(ys_o) + [ys_o[0]], color="darkred", linewidth=2, label="Outer (u=1)")
    ax2.plot(list(xs_h) + [xs_h[0]], list(ys_h) + [ys_h[0]], color="darkblue", linewidth=2, label="Hole (u=0)")

    # Draw a few contour lines
    levels = np.linspace(0, 1, 11)
    ax2.tricontour(x_vals, y_vals, tris, u, levels=levels, colors="black", linewidths=0.5, alpha=0.3)

    ax2.set_title(
        f"Laplace solution Δu = 0 via linear FEM\n"
        f"(u=0 on hole, u=1 on outer boundary)",
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
    ax3.plot(list(xs_l) + [xs_l[0]], list(ys_l) + [ys_l[0]], color="crimson", linewidth=2.5, label="Outer boundary (u=1)")

    ax3.set_title(
        f"CDT triangulation of an L-shaped pocket\n"
        f"({len(l_mesh.vertices)} vertices, {len(l_mesh.triangles)} triangles)",
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

    lx = [v[0] for v in l_mesh.vertices]
    ly = [v[1] for v in l_mesh.vertices]
    ltris = l_mesh.triangles

    fig4, ax4 = plt.subplots(figsize=(7, 7))
    ax4.set_aspect("equal")
    ax4.set_xlim(-5, 85)
    ax4.set_ylim(-5, 85)

    tcf4 = ax4.tripcolor(lx, ly, ltris, l_u, cmap="coolwarm", shading="gouraud")
    cbar4 = fig4.colorbar(tcf4, ax=ax4, shrink=0.8)
    cbar4.set_label("Scalar field u(x,y)", fontsize=10)

    ax4.plot(list(xs_l) + [xs_l[0]], list(ys_l) + [ys_l[0]], color="darkred", linewidth=2, label="Outer (u=1)")
    levels_l = np.linspace(0, 1, 11)
    ax4.tricontour(lx, ly, ltris, l_u, levels=levels_l, colors="black", linewidths=0.5, alpha=0.3)

    ax4.set_title(
        f"Laplace solution Δu = 0 on L-shaped pocket\n"
        f"(u=1 on outer boundary)",
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

    return {
        "title": "PDE Mesh",
        "description": (
            "Constrained Delaunay triangulation (CDT) of 2D polygon domains"
            " and FEM Laplace equation solving for scalar field generation."
        ),
        "images": images,
    }
