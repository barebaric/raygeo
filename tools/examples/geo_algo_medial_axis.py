"""Generate 2D visualisation of Medial Axis Transform."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.algo.medial_axis import compute_medial_axis, mat_path


def _plot_ma_2d(nodes, edges, root, boundary, islands, ax, title):
    """Plot medial axis overlay on pocket geometry."""
    nodes_arr = np.array(nodes)
    ax.scatter(
        nodes_arr[:, 0],
        nodes_arr[:, 1],
        c=nodes_arr[:, 0] * 0,
        cmap="viridis",
        s=8,
        alpha=0.6,
    )
    for i, j in edges:
        ax.plot(
            [nodes[i][0], nodes[j][0]],
            [nodes[i][1], nodes[j][1]],
            "r-",
            linewidth=0.5,
            alpha=0.5,
        )
    ax.plot(
        nodes[root][0],
        nodes[root][1],
        "r*",
        markersize=12,
        label="Root",
    )
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
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)


def generate_mat_rect():
    """Medial axis of a rectangular pocket."""
    fig, ax = plt.subplots(figsize=(6, 5))
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    nodes, clearances, edges, root, branches = compute_medial_axis(
        boundary, holes=[], tool_radius=1.0, sampling_spacing=6.0
    )
    _plot_ma_2d(
        nodes, edges, root, boundary, None, ax, "Medial Axis — Rectangle"
    )
    fig.tight_layout()
    return fig


def generate_mat_multi():
    """Medial axis of a pocket with three islands."""
    fig, ax = plt.subplots(figsize=(7, 5))
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [(70, 40), (90, 40), (90, 60), (70, 60)],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    nodes, clearances, edges, root, branches = compute_medial_axis(
        boundary, holes=islands, tool_radius=1.0, sampling_spacing=8.0
    )
    _plot_ma_2d(
        nodes,
        edges,
        root,
        boundary,
        islands,
        ax,
        "Medial Axis — Multi-Island Pocket",
    )
    fig.tight_layout()
    return fig


def generate_mat_yshape():
    """Medial axis of a Y-shaped channel."""
    fig, ax = plt.subplots(figsize=(6, 6))
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
    nodes, clearances, edges, root, branches = compute_medial_axis(
        yshape, holes=[], tool_radius=1.0, sampling_spacing=6.0
    )
    _plot_ma_2d(nodes, edges, root, yshape, None, ax, "Medial Axis — Y-Shape")
    fig.tight_layout()
    return fig


def generate_mat_path():
    """MAT path routing around an island."""
    fig, ax = plt.subplots(figsize=(6, 5))
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    island = [(35, 20), (65, 20), (65, 60), (35, 60)]

    nodes, clearances, edges, root, branches = compute_medial_axis(
        boundary, holes=[island], tool_radius=1.0, sampling_spacing=6.0
    )

    from_pt, to_pt = (10, 10), (90, 70)
    path = mat_path(
        boundary,
        from_pt,
        to_pt,
        holes=[island],
        tool_radius=1.0,
        sampling_spacing=6.0,
    )

    _plot_ma_2d(
        nodes,
        edges,
        root,
        boundary,
        [island],
        ax,
        "MAT Path — with island",
    )

    if path:
        path_arr = np.array(path)
        ax.plot(
            path_arr[:, 0],
            path_arr[:, 1],
            "g-",
            linewidth=3,
            label="MAT Path",
        )
        ax.plot(
            from_pt[0],
            from_pt[1],
            "go",
            markersize=10,
            zorder=5,
        )
        ax.plot(
            to_pt[0],
            to_pt[1],
            "gs",
            markersize=10,
            zorder=5,
        )
        ax.annotate("From", from_pt, xytext=(4, 4), textcoords="offset points")
        ax.annotate("To", to_pt, xytext=(4, 4), textcoords="offset points")

    ax.legend(fontsize=8)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.medial_axis.md"]
__images__ = [
    {
        "heading": None,
        "caption": (
            "Medial axis of a rectangular pocket — skeleton from center"
            " to corners."
        ),
        "function": generate_mat_rect,
    },
    {
        "heading": "compute_medial_axis",
        "caption": (
            "Medial axis with three rectangular islands — skeleton"
            " branches around each obstacle."
        ),
        "function": generate_mat_multi,
    },
    {
        "heading": "compute_medial_axis",
        "caption": (
            "Medial axis of a Y-shaped channel — skeleton follows"
            " the branching topology."
        ),
        "function": generate_mat_yshape,
    },
    {
        "heading": "mat_path",
        "caption": (
            "MAT path routing: a path between two points (green) along"
            " the medial axis skeleton (red). The path avoids the island"
            " by following the skeleton topology."
        ),
        "function": generate_mat_path,
    },
]
