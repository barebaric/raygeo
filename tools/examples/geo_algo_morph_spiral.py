"""Generate 2D visualisation of MAT-driven morphing spiral."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import Normalize

from raygeo.geo.algo.morph_spiral import morph_spiral, morph_spiral_from_branch


def _plot_spiral_2d(spiral, boundary, islands, ax, title):
    """Plot a morphing spiral coloured by path progress."""
    pts = np.array(spiral)
    n = len(pts)
    if n == 0:
        return

    cmap = "plasma"
    for i in range(n - 1):
        seg = pts[i : i + 2]
        ax.plot(
            seg[:, 0],
            seg[:, 1],
            color=plt.get_cmap(cmap)(i / max(n - 1, 1)),
            linewidth=0.8,
            alpha=0.7,
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
                label="Island" if isl is islands[0] else None,
            )
    ax.set_aspect("equal")
    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)

    sm = plt.cm.ScalarMappable(cmap=cmap, norm=Normalize(0, n - 1))
    sm.set_array([])
    fig = ax.figure
    fig.colorbar(sm, ax=ax, label="Step")


def generate_spiral_rect():
    """Morphing spiral in a rectangular pocket."""
    fig, ax = plt.subplots(figsize=(6, 5))
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    tp, _ = morph_spiral(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        z=0.0,
        sampling_spacing=8.0,
    )
    _plot_spiral_2d(tp, boundary, None, ax, "Morphing Spiral — Rectangle")
    fig.tight_layout()
    return fig


def generate_spiral_multi():
    """Morphing spiral in a multi-island pocket."""
    fig, ax = plt.subplots(figsize=(7, 5))
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [(70, 40), (90, 40), (90, 60), (70, 60)],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    tp, _ = morph_spiral(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        z=0.0,
        sampling_spacing=8.0,
    )
    _plot_spiral_2d(
        tp, boundary, islands, ax, "Morphing Spiral — Multi-Island Pocket"
    )
    fig.tight_layout()
    return fig


def generate_spiral_yshape():
    """Morphing spiral in a Y-shaped channel."""
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
    tp, _ = morph_spiral(
        pocket_boundary=yshape,
        tool_radius=3.0,
        step_over=2.0,
        z=0.0,
        sampling_spacing=8.0,
    )
    _plot_spiral_2d(tp, yshape, None, ax, "Morphing Spiral — Y-Shape")
    fig.tight_layout()
    return fig


def generate_spiral_lshape():
    """Morphing spiral in an L-shaped pocket."""
    fig, ax = plt.subplots(figsize=(6, 5))
    lshape = [(0, 0), (120, 0), (120, 40), (40, 40), (40, 80), (0, 80)]
    tp, _ = morph_spiral(
        pocket_boundary=lshape,
        tool_radius=3.0,
        step_over=2.0,
        z=0.0,
        sampling_spacing=6.0,
    )
    _plot_spiral_2d(tp, lshape, None, ax, "Morphing Spiral — L-Shape")
    fig.tight_layout()
    return fig


def generate_spiral_rect_trochoid():
    """Morphing spiral in a small rectangle."""
    fig, ax = plt.subplots(figsize=(6, 5))
    boundary = [(0, 0), (60, 0), (60, 50), (0, 50)]
    tp, _ = morph_spiral(
        pocket_boundary=boundary,
        tool_radius=2.0,
        step_over=2.0,
        z=0.0,
        sampling_spacing=6.0,
    )
    _plot_spiral_2d(
        tp, boundary, None, ax, "Morphing Spiral — Small Rectangle"
    )
    fig.tight_layout()
    return fig


def generate_spiral_from_branch():
    """Boustrophedon spiral from a single MAT branch (tapered channel)."""
    fig, ax = plt.subplots(figsize=(6, 5))
    centerline = [
        (0.0, 0.0),
        (15.0, 0.0),
        (30.0, 0.0),
        (45.0, 0.0),
        (60.0, 0.0),
    ]
    clearances = [8.0, 7.0, 5.0, 3.0, 1.5]
    path = morph_spiral_from_branch(
        centerline, clearances, step_over=2.0, z=0.0
    )
    pts = np.array(path)
    n = len(pts)
    cmap = "plasma"
    for i in range(n - 1):
        seg = pts[i : i + 2]
        ax.plot(
            seg[:, 0],
            seg[:, 1],
            color=plt.get_cmap(cmap)(i / max(n - 1, 1)),
            linewidth=1.2,
            alpha=0.8,
        )
    cl_arr = np.array(centerline)
    ax.plot(
        cl_arr[:, 0],
        cl_arr[:, 1],
        "k--",
        linewidth=1,
        alpha=0.5,
        label="Centerline",
    )
    ax.plot(
        cl_arr[:, 0], cl_arr[:, 1] + clearances, "r:", linewidth=0.8, alpha=0.4
    )
    ax.plot(
        cl_arr[:, 0],
        cl_arr[:, 1] - clearances,
        "r:",
        linewidth=0.8,
        alpha=0.4,
        label="Bound",
    )
    ax.set_aspect("equal")
    ax.set_title("morph_spiral_from_branch — Tapered Channel")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    sm = plt.cm.ScalarMappable(cmap=cmap, norm=Normalize(0, n - 1))
    sm.set_array([])
    fig.colorbar(sm, ax=ax, label="Step")
    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": "morph_spiral_from_branch",
        "caption": (
            "Boustrophedon spiral from a single MAT branch — the path"
            " weaves back and forth along the centerline, with passes"
            " truncated as the channel narrows."
        ),
        "function": generate_spiral_from_branch,
    },
    {
        "heading": "morph_spiral",
        "caption": (
            "MAT-driven morphing spiral in a rectangular pocket —"
            " continuous toolpath fills area with constant step-over."
        ),
        "function": generate_spiral_rect,
    },
    {
        "heading": "morph_spiral",
        "caption": (
            "Morphing spiral in a three-island pocket — wraps"
            " around each island following the medial axis."
        ),
        "function": generate_spiral_multi,
    },
    {
        "heading": "morph_spiral",
        "caption": (
            "Morphing spiral in a Y-shaped channel — flows into"
            " both arms of the Y."
        ),
        "function": generate_spiral_yshape,
    },
    {
        "heading": "morph_spiral",
        "caption": (
            "Morphing spiral in an L-shaped pocket — fills the"
            " corner naturally."
        ),
        "function": generate_spiral_lshape,
    },
    {
        "heading": "morph_spiral",
        "caption": (
            "Morphing spiral in a small rectangle — boustrophedon"
            " pattern visible at branch level."
        ),
        "function": generate_spiral_rect_trochoid,
    },
]
