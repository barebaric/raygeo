"""Mesh PDE spiral example images."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.tri import Triangulation

from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.laplace import solve_laplace
from raygeo.mesh.pde import trace_spiral


def generate_spiral_path():
    # ── Spiral path on Laplace solution ────────────────────────────────
    outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
    hole = [(30, 30), (70, 30), (70, 70), (30, 70)]
    mesh = build_triangle_mesh(outer, [hole], min_angle=20.0)
    u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)
    path = trace_spiral(mesh, u, step_over=1.5)

    verts = mesh.vertices
    x_vals = np.asarray([v[0] for v in verts])
    y_vals = np.asarray([v[1] for v in verts])
    tris = np.asarray(mesh.triangles)
    u_arr = np.asarray(u)
    triang = Triangulation(x_vals, y_vals, tris)

    px = np.asarray([p[0] for p in path])
    py = np.asarray([p[1] for p in path])

    fig, ax = plt.subplots(figsize=(7.5, 7.5))
    ax.set_aspect("equal")
    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 105)

    tcf = ax.tripcolor(triang, u_arr, cmap="coolwarm", shading="gouraud")
    cbar = fig.colorbar(tcf, ax=ax, shrink=0.8)
    cbar.set_label("Scalar field u(x,y)", fontsize=10)

    ax.plot(px, py, "k-", linewidth=0.8, label="Spiral toolpath")

    xs_o = [p[0] for p in outer] + [outer[0][0]]
    ys_o = [p[1] for p in outer] + [outer[0][1]]
    xs_h = [p[0] for p in hole] + [hole[0][0]]
    ys_h = [p[1] for p in hole] + [hole[0][1]]
    ax.plot(xs_o, ys_o, color="darkred", linewidth=2, label="Outer (u=1)")
    ax.plot(xs_h, ys_h, color="darkblue", linewidth=2, label="Hole (u=0)")

    start_pt = path[0]
    end_pt = path[-1]
    ax.plot(
        start_pt[0],
        start_pt[1],
        "o",
        color="green",
        markersize=6,
        label="Start",
    )
    ax.plot(
        end_pt[0],
        end_pt[1],
        "s",
        color="darkorange",
        markersize=6,
        label="End",
    )

    ax.set_title(
        f"PDE spiral toolpath (step_over=1.5)\n"
        f"{len(path)} vertices, "
        f"{len(mesh.vertices)} mesh vertices",
        fontsize=12,
    )
    ax.legend(fontsize=9, loc="upper right")
    ax.grid(True, alpha=0.2)
    fig.tight_layout()
    return fig


def generate_spiral_multi_island():
    # ── Multi-island spiral path ─────────────────────────────────────────
    outer_mi = [(0, 0), (100, 0), (100, 100), (0, 100)]
    holes_mi = [
        [(10, 60), (30, 60), (30, 80), (10, 80)],
        [(60, 55), (85, 55), (85, 85), (60, 85)],
        [(10, 10), (35, 10), (35, 30), (10, 30)],
        [(60, 15), (75, 15), (75, 35), (60, 35)],
    ]
    mi_mesh = build_triangle_mesh(outer_mi, holes_mi, min_angle=20.0)
    mi_u = solve_laplace(mi_mesh, max_iter=2000, tolerance=1e-10)
    mi_path = trace_spiral(mi_mesh, mi_u, step_over=1.5)

    mi_verts = mi_mesh.vertices
    mi_x = np.asarray([v[0] for v in mi_verts])
    mi_y = np.asarray([v[1] for v in mi_verts])
    mi_tris = np.asarray(mi_mesh.triangles)
    mi_u_arr = np.asarray(mi_u)
    mi_triang = Triangulation(mi_x, mi_y, mi_tris)

    mi_px = np.asarray([p[0] for p in mi_path])
    mi_py = np.asarray([p[1] for p in mi_path])

    fig2, ax2 = plt.subplots(figsize=(7.5, 7.5))
    ax2.set_aspect("equal")
    ax2.set_xlim(-5, 105)
    ax2.set_ylim(-5, 105)

    tcf2 = ax2.tripcolor(
        mi_triang, mi_u_arr, cmap="coolwarm", shading="gouraud"
    )
    cbar2 = fig2.colorbar(tcf2, ax=ax2, shrink=0.8)
    cbar2.set_label("Scalar field u(x,y)", fontsize=10)

    ax2.plot(mi_px, mi_py, "k-", linewidth=0.8, label="Spiral toolpath")

    xs_mo = [p[0] for p in outer_mi] + [outer_mi[0][0]]
    ys_mo = [p[1] for p in outer_mi] + [outer_mi[0][1]]
    ax2.plot(xs_mo, ys_mo, color="darkred", linewidth=2, label="Outer (u=1)")

    for hi, hole in enumerate(holes_mi):
        xs_h = [p[0] for p in hole] + [hole[0][0]]
        ys_h = [p[1] for p in hole] + [hole[0][1]]
        label = "Inners (u=0)" if hi == 0 else None
        ax2.plot(xs_h, ys_h, color="darkblue", linewidth=2, label=label)

    mi_start = mi_path[0]
    mi_end = mi_path[-1]
    ax2.plot(
        mi_start[0],
        mi_start[1],
        "o",
        color="green",
        markersize=6,
        label="Start",
    )
    ax2.plot(
        mi_end[0],
        mi_end[1],
        "s",
        color="darkorange",
        markersize=6,
        label="End",
    )

    ax2.set_title(
        f"PDE spiral toolpath on multi-island domain (step_over=1.5)\n"
        f"{len(mi_path)} vertices,"
        f" {len(mi_mesh.vertices)} mesh vertices",
        fontsize=12,
    )
    ax2.legend(fontsize=9, loc="upper right")
    ax2.grid(True, alpha=0.2)
    fig2.tight_layout()
    return fig2


__images__ = [
    {
        "heading": "trace_spiral",
        "caption": (
            "Spiral toolpath traced on the Laplace solution — path"
            " morphs smoothly from the inner hole outward"
        ),
        "function": generate_spiral_path,
    },
    {
        "heading": "trace_spiral",
        "caption": "Spiral toolpath traced on a multi-island Laplace"
        " solution — path navigates around four inner islands",
        "function": generate_spiral_multi_island,
    },
]
