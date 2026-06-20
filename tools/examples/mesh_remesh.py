"""Mesh remesh example images."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.mesh.build import build_triangle_mesh
from raygeo.mesh.remesh import remesh


def generate_overview():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    rm_mesh = build_triangle_mesh(boundary, [], 3.0, 20.0)
    rm_refined = remesh(rm_mesh, boundary, max_edge_len=10.0)

    fig_rm, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))
    for ax, mesh, title in [
        (ax1, rm_mesh, f"Initial ({len(rm_mesh.vertices)} verts)"),
        (ax2, rm_refined, f"Refined ({len(rm_refined.vertices)} verts)"),
    ]:
        verts = np.array(mesh.vertices)
        tris = np.array(mesh.triangles)
        for ti in tris:
            poly = verts[list(ti) + [ti[0]]]
            ax.plot(poly[:, 0], poly[:, 1], "k-", linewidth=0.3, alpha=0.5)
        bnd = np.array(list(boundary) + [boundary[0]])
        ax.plot(bnd[:, 0], bnd[:, 1], "r-", linewidth=2, alpha=0.7)
        ax.set_aspect("equal")
        ax.set_title(title)
        ax.axis("off")

    fig_rm.tight_layout()
    return fig_rm


__docs_target__ = ["raygeo.mesh.remesh.md"]
__images__ = [
    {
        "heading": None,
        "caption": "Initial mesh (left) vs refined mesh (right) after remesh.",
        "function": generate_overview,
    },
]
