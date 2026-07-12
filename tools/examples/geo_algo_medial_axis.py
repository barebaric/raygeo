"""Generate 2D visualisation of Medial Axis Transform."""

import math

import matplotlib.pyplot as plt
import numpy as np

from raygeo.cnc.machining.wavefront import build_wavefront_workplan
from raygeo.geo.algo.medial_axis import MedialAxis
from raygeo.ops.assembly.spiral import generate_spiral
from raygeo.ops.part import Part, StockRegion
from raygeo.ops.part.cleared_area import ClearedArea


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
    axis = MedialAxis.compute(
        boundary, holes=[], min_clearance=1.0, sampling_spacing=6.0
    )
    _plot_ma_2d(
        axis.nodes,
        axis.edges,
        axis.root,
        boundary,
        None,
        ax,
        "Medial Axis — Rectangle",
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
    axis = MedialAxis.compute(
        boundary, holes=islands, min_clearance=1.0, sampling_spacing=8.0
    )
    _plot_ma_2d(
        axis.nodes,
        axis.edges,
        axis.root,
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
    axis = MedialAxis.compute(
        yshape, holes=[], min_clearance=1.0, sampling_spacing=6.0
    )
    _plot_ma_2d(
        axis.nodes,
        axis.edges,
        axis.root,
        yshape,
        None,
        ax,
        "Medial Axis — Y-Shape",
    )
    fig.tight_layout()
    return fig


def generate_mat_path():
    """MAT path routing around an island."""
    fig, ax = plt.subplots(figsize=(6, 5))
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    island = [(35, 20), (65, 20), (65, 60), (35, 60)]

    axis = MedialAxis.compute(
        boundary, holes=[island], min_clearance=1.0, sampling_spacing=6.0
    )

    from_pt, to_pt = (10, 10), (90, 70)
    path = axis.path_between(from_pt, to_pt)

    _plot_ma_2d(
        axis.nodes,
        axis.edges,
        axis.root,
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


def generate_mat_trimming():
    """Visualise how the Medial Axis is trimmed to the cleared area
    for travel routing.

    Uses a pocket with three islands.  Only the first 10 passes of
    adaptive clearing are applied, leaving most of the pocket
    unmachined.  The full MAT skeleton (gray) spans the whole pocket,
    while the trimmed nodes (blue) are the subset usable for travel
    routing through already-cleared territory (green).
    """
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

    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        step_over=step_over,
        target_z=-5.0,
        area_tolerance=1.0,
    )
    # Seed the cleared area from the FlatSpiral step only; the manual
    # bites/cut_fast loop below takes the place of the Wavefront step.
    seed_steps = [s for s in steps if s["kind"] == "FlatSpiral"]
    seed_step = seed_steps[0]
    part = Part.from_polygons(boundary, islands)
    generate_spiral(
        part,
        center=seed_step["center"],
        z=seed_step["z"],
        start_radius=seed_step["start_radius"],
        end_radius=seed_step["end_radius"],
        revolutions=seed_step["revolutions"],
        direction=seed_step["direction"],
        angular_step=seed_step["angular_step"],
    )
    region = StockRegion(boundary=boundary, islands=islands)
    ca = ClearedArea(
        initial=part.cleared.fragments(),
    )
    for _ in range(10):
        bites = ca.bites(region, step_over, tool_radius, 0.01)
        if not bites:
            break
        ca.cut_fast(bites)
    frags = ca.fragments()

    holes = [list(h) for h in islands]
    axis = MedialAxis.compute(boundary, holes, tool_radius, step_over * 0.5)
    trimmed_axis = MedialAxis.compute(
        boundary, holes, tool_radius, step_over * 0.5
    ).trim_to_polygons(frags)

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")
    ax.set_title("MAT skeleton trimmed to cleared area", fontsize=10)

    bnd = np.array(boundary + [boundary[0]])
    ax.plot(bnd[:, 0], bnd[:, 1], "k-", linewidth=1.5, label="Pocket")
    for isl in islands:
        ia = np.array(isl + [isl[0]])
        ax.fill(
            ia[:, 0],
            ia[:, 1],
            facecolor="#ddd",
            edgecolor="#999",
            linewidth=1,
        )

    for i, frag in enumerate(frags):
        fa = np.array(frag + [frag[0]])
        ax.fill(
            fa[:, 0],
            fa[:, 1],
            color="#2ca02c",
            alpha=0.15,
            label="Cleared area" if i == 0 else "",
        )

    full_mat_labeled = False
    for a, b in axis.edges:
        ax.plot(
            [axis.nodes[a][0], axis.nodes[b][0]],
            [axis.nodes[a][1], axis.nodes[b][1]],
            color="#bbb",
            linewidth=0.3,
            alpha=0.5,
            label="Full MAT" if not full_mat_labeled else None,
        )
        full_mat_labeled = True

    ax.scatter(
        [n[0] for n in trimmed_axis.nodes],
        [n[1] for n in trimmed_axis.nodes],
        s=4,
        c="#1f77b4",
        alpha=0.7,
        label=f"Trimmed ({len(trimmed_axis.nodes)})",
    )

    ax.legend(fontsize=7, loc="upper right")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")

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
        "heading": "compute",
        "caption": (
            "Medial axis with three rectangular islands — skeleton"
            " branches around each obstacle."
        ),
        "function": generate_mat_multi,
    },
    {
        "heading": "compute",
        "caption": (
            "Medial axis of a Y-shaped channel — skeleton follows"
            " the branching topology."
        ),
        "function": generate_mat_yshape,
    },
    {
        "heading": "path_between",
        "caption": (
            "MAT path routing: a path between two points along the medial"
            " axis skeleton, avoiding the island"
        ),
        "function": generate_mat_path,
    },
    {
        "heading": "trim_to_polygons",
        "caption": (
            "MAT trimmed to cleared area: original (gray) and"
            " trimmed nodes (blue) after multiple clearing passes"
        ),
        "function": generate_mat_trimming,
    },
]
