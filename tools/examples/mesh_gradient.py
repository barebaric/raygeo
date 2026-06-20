"""Mesh gradient example images — gradient field visualisations."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.tri import Triangulation

from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.gradient import compute_gradient_field
from raygeo.mesh.laplace import solve_laplace


def generate_field():
    outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
    hole = [(30, 30), (70, 30), (70, 70), (30, 70)]
    mesh = build_triangle_mesh(outer, [hole], tool_radius=0.0, min_angle=20.0)
    u = solve_laplace(mesh, max_iter=2000, tolerance=1e-10)

    verts = mesh.vertices
    tris = np.asarray(mesh.triangles)
    u_arr = np.asarray(u)
    xs_o, ys_o = zip(*outer)
    xs_h, ys_h = zip(*hole)

    # ── Gradient field quiver plot ──────────────────────────────────────
    grad = compute_gradient_field(mesh, u)

    cx_arr = np.empty(len(mesh.triangles))
    cy_arr = np.empty(len(mesh.triangles))
    gx_arr = np.empty(len(mesh.triangles))
    gy_arr = np.empty(len(mesh.triangles))
    for ti, (a, b, c) in enumerate(mesh.triangles):
        cx_arr[ti] = (verts[a][0] + verts[b][0] + verts[c][0]) / 3.0
        cy_arr[ti] = (verts[a][1] + verts[b][1] + verts[c][1]) / 3.0
        gx_arr[ti], gy_arr[ti] = grad[ti]

    mag = np.hypot(gx_arr, gy_arr)
    valid = mag > 1e-10
    qx = np.where(valid, gx_arr / mag, 0.0)
    qy = np.where(valid, gy_arr / mag, 0.0)
    px = np.where(valid, -gy_arr / mag, 0.0)
    py = np.where(valid, gx_arr / mag, 0.0)

    fig, ax = plt.subplots(figsize=(7, 7))
    ax.set_aspect("equal")
    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 105)

    triang = Triangulation(
        np.asarray([v[0] for v in verts]),
        np.asarray([v[1] for v in verts]),
        tris,
    )
    tcf = ax.tripcolor(triang, u_arr, cmap="coolwarm", shading="gouraud")
    ax.quiver(
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
    ax.quiver(
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
    ax.plot(
        list(xs_o) + [xs_o[0]],
        list(ys_o) + [ys_o[0]],
        color="black",
        linewidth=1.5,
    )
    ax.plot(
        list(xs_h) + [xs_h[0]],
        list(ys_h) + [ys_h[0]],
        color="black",
        linewidth=1.5,
    )
    cbar = fig.colorbar(tcf, ax=ax, shrink=0.8)
    cbar.set_label("u(x,y)", fontsize=10)
    ax.set_title(
        "Gradient field on Laplace solution\n"
        r"$\nabla u$ (red, normal to contours), "
        r"$\nabla u^\perp$ (blue, along contours)",
        fontsize=12,
    )
    ax.legend(fontsize=10, loc="upper right")
    ax.grid(True, alpha=0.2)
    fig.tight_layout()
    return fig


def generate_multi_island():
    # ── Multi-island gradient field ─────────────────────────────────────
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
    mi_grad = compute_gradient_field(mi_mesh, mi_u)

    mi_verts = mi_mesh.vertices
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

    xs_mo, ys_mo = zip(*outer_mi)
    mi_x = np.asarray([v[0] for v in mi_verts])
    mi_y = np.asarray([v[1] for v in mi_verts])
    mi_tris = np.asarray(mi_mesh.triangles)
    mi_u_arr = np.asarray(mi_u)
    mi_triang = Triangulation(mi_x, mi_y, mi_tris)

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
        xs_h_mi, ys_h_mi = zip(*hole)
        ax10.plot(
            list(xs_h_mi) + [xs_h_mi[0]],
            list(ys_h_mi) + [ys_h_mi[0]],
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
    return fig10


__docs_target__ = ["raygeo.mesh.gradient.md"]
__images__ = [
    {
        "heading": "compute_gradient_field",
        "caption": "Gradient field ∇u (red) and perpendicular flow ∇u⊥ (blue)"
        " on the Laplace solution",
        "function": generate_field,
    },
    {
        "heading": "compute_gradient_field",
        "caption": "Gradient field ∇u (red) and perpendicular flow ∇u⊥"
        " (blue) on a multi-island domain",
        "function": generate_multi_island,
    },
]
