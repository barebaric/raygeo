"""Mesh build example images — uniform mesh, CDT triangulations."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.collections import LineCollection

from raygeo.mesh.build import (
    build_prism_mesh,
    build_triangle_mesh,
    build_uniform_mesh,
)
from raygeo.mesh.gradient import compute_gradient_field
from raygeo.mesh.laplace import solve_laplace


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


def generate_uniform():
    """Uniform mesh."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]

    ug_mesh = build_uniform_mesh(boundary, [], 3.0, target_edge_len=5.0)

    fig_ug, (ax_t, ax_b) = plt.subplots(
        2, 1, figsize=(7, 9), gridspec_kw={"height_ratios": [1, 1]}
    )

    verts = np.array(ug_mesh.vertices)
    tris = np.array(ug_mesh.triangles)
    for ti in tris:
        poly = verts[list(ti) + [ti[0]]]
        ax_t.plot(poly[:, 0], poly[:, 1], "k-", linewidth=0.3, alpha=0.5)
    bnd = np.array(list(boundary) + [boundary[0]])
    ax_t.plot(bnd[:, 0], bnd[:, 1], "r-", linewidth=2, alpha=0.7)
    ax_t.set_aspect("equal")
    nv = len(ug_mesh.vertices)
    nt = len(ug_mesh.triangles)
    ax_t.set_title(f"Uniform Mesh ({nv} verts, {nt} tris)")
    ax_t.axis("off")

    ug2 = build_uniform_mesh(boundary, [], 3.0, target_edge_len=8.0)
    u2 = solve_laplace(ug2, 1000, 1e-8)
    g2 = compute_gradient_field(ug2, u2)
    verts2 = np.array(ug2.vertices)
    tris2 = np.array(ug2.triangles)
    for ti in tris2:
        poly = verts2[list(ti) + [ti[0]]]
        ax_b.plot(poly[:, 0], poly[:, 1], "k-", linewidth=0.2, alpha=0.3)
    bnd2 = np.array(list(boundary) + [boundary[0]])
    ax_b.plot(bnd2[:, 0], bnd2[:, 1], "r-", linewidth=2, alpha=0.7)
    centroids = verts2[tris2].mean(axis=1)
    gx = np.array([g[0] for g in g2])
    gy = np.array([g[1] for g in g2])
    ax_b.quiver(
        centroids[:, 0],
        centroids[:, 1],
        gx,
        gy,
        alpha=0.6,
        scale=0.1,
        width=0.003,
    )
    ax_b.set_aspect("equal")
    ax_b.set_title("Laplace Gradient (quiver)")
    ax_b.axis("off")

    fig_ug.tight_layout()
    return fig_ug


def generate_triangulation():
    """CDT square with hole."""
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
    return fig


def generate_l_shape():
    """CDT L-shape."""
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
    return fig3


def generate_multi_island():
    """CDT multi-island."""
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
        xs_h_mi, ys_h_mi = zip(*hole)
        label = "Inner boundaries (u=0)" if hi == 0 else None
        ax8.fill(xs_h_mi, ys_h_mi, alpha=0.08, color="royalblue")
        ax8.plot(
            list(xs_h_mi) + [xs_h_mi[0]],
            list(ys_h_mi) + [ys_h_mi[0]],
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
    return fig8


def generate_prism():
    """Prism extrusion of a rectangular stock with a hole."""
    outer = [(0, 0), (200, 0), (200, 100), (0, 100)]
    hole = [(50, 25), (150, 25), (150, 75), (50, 75)]
    mesh = build_prism_mesh(outer, [hole], thickness=18.0, uv_scale=50.0)

    pos = np.asarray(mesh.positions)
    tris = np.asarray(mesh.indices).reshape(-1, 3)
    top_z = pos[:, 2].max()
    top_tris = tris[
        (pos[tris[:, 0], 2] == top_z)
        & (pos[tris[:, 1], 2] == top_z)
        & (pos[tris[:, 2], 2] == top_z)
    ]

    fig, ax = plt.subplots(figsize=(7, 7))
    ax.set_aspect("equal")
    ax.set_xlim(-10, 210)
    ax.set_ylim(-10, 110)

    for a, b, c in top_tris:
        poly = np.vstack([pos[a, :2], pos[b, :2], pos[c, :2], pos[a, :2]])
        ax.plot(poly[:, 0], poly[:, 1], "k-", linewidth=0.3, alpha=0.5)

    xs_o, ys_o = zip(*outer)
    ax.plot(
        list(xs_o) + [xs_o[0]],
        list(ys_o) + [ys_o[0]],
        color="crimson",
        linewidth=2.5,
        label="Outer boundary",
    )
    xs_h, ys_h = zip(*hole)
    ax.fill(xs_h, ys_h, alpha=0.08, color="royalblue")
    ax.plot(
        list(xs_h) + [xs_h[0]],
        list(ys_h) + [ys_h[0]],
        color="royalblue",
        linewidth=2.5,
        label="Hole boundary",
    )
    ax.set_title(
        f"Prism top face (earcut triangulation)\n"
        f"({len(pos)} vertices, {len(tris)} triangles total,"
        f" {len(top_tris)} on the top face)",
        fontsize=12,
    )
    ax.legend(fontsize=10, loc="upper right")
    ax.grid(True, alpha=0.2)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.mesh.build.md"]
__images__ = [
    {
        "heading": None,
        "caption": (
            "Uniform mesh (top) and Laplace gradient field (bottom)"
            " from build_uniform_mesh."
        ),
        "function": generate_uniform,
    },
    {
        "heading": "build_triangle_mesh",
        "caption": "CDT triangulation of a square pocket with centred hole",
        "function": generate_triangulation,
    },
    {
        "heading": "build_triangle_mesh",
        "caption": "CDT triangulation of an L-shaped pocket",
        "function": generate_l_shape,
    },
    {
        "heading": "build_triangle_mesh",
        "caption": "CDT triangulation of a square pocket with multiple"
        " islands",
        "function": generate_multi_island,
    },
    {
        "heading": "build_prism_mesh",
        "caption": "Earcut triangulation of the prism top face with hole",
        "function": generate_prism,
    },
]
