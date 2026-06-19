"""Generate PDE spiral example images."""

__images__ = [
    {
        "stem": "pde-spiral-path",
        "caption": (
            "Spiral toolpath traced on the Laplace solution — path"
            " morphs smoothly from the inner hole outward"
        ),
        "doc": "raygeo.geo.algo.pde_spiral.md",
        "heading": "trace_spiral",
    },
]

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.tri import Triangulation

from raygeo.geo.algo.pde_mesh import build_triangle_mesh, solve_laplace
from raygeo.geo.algo.pde_spiral import trace_spiral


def generate_examples(output_dir):
    images = []

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
    fig_path = output_dir / "pde-spiral-path.png"
    fig.savefig(fig_path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "pde-spiral-path.png",
            "caption": (
                "PDE spiral toolpath on the Laplace solution — starts on"
                " the inner hole boundary (green dot) and spirals outward"
                " to the outer boundary (orange square)"
            ),
        }
    )

    return {
        "title": "PDE Spiral",
        "description": (
            "Spiral toolpath tracing on a triangulated Laplace solution."
        ),
        "images": images,
    }
