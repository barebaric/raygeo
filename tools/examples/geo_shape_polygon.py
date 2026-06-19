"""Generate examples for polygon operations (construction, boolean, offset)."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle as CirclePatch

from raygeo.geo.shape.polygon import (
    apply_minimum_curvature,
    get_circle_polygon,
    get_polygons_difference,
    get_polygons_intersection,
    get_polygons_union,
    get_segment_swept_polygon,
    offset_polygon,
)
from tools.plot import plot_polygon


def _make_circle(r, n, ox=0.0, oy=0.0):
    return [
        (
            ox + r * math.cos(2 * math.pi * i / n),
            oy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _make_square(r, ox=0.0, oy=0.0):
    return [
        (ox - r, oy - r),
        (ox + r, oy - r),
        (ox + r, oy + r),
        (ox - r, oy + r),
    ]


def _plot_boolean(a, b, result, title):
    fig, ax = plt.subplots(figsize=(7, 7))
    plot_polygon(ax, a, "steelblue", "A")
    plot_polygon(ax, b, "tomato", "B")
    if result:
        plot_polygon(ax, result[0], "limegreen", "Result", linewidth=2.5)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend()
    ax.set_title(title)
    fig.tight_layout()
    return fig


def generate_circle_polygon():
    """Circle polygon."""
    center = (50, 50)
    radius = 30.0
    poly = get_circle_polygon(center, radius, 32)
    poly_arr = np.array(poly)

    fig1, ax1 = plt.subplots(figsize=(6, 6))
    ax1.plot(
        *np.vstack([poly_arr, poly_arr[0:1]]).T,
        "b-",
        linewidth=2,
        label="64-gon",
    )
    ax1.add_patch(
        CirclePatch(
            center,
            radius,
            fill=False,
            edgecolor="red",
            linestyle="--",
            linewidth=1.5,
            label="Ideal circle",
        )
    )
    ax1.plot(center[0], center[1], "k+", markersize=10, label="Centre")
    ax1.set_xlim(15, 85)
    ax1.set_ylim(15, 85)
    ax1.set_aspect("equal")
    ax1.set_title("get_circle_polygon — 64-gon approximation")
    ax1.set_xlabel("X")
    ax1.set_ylabel("Y")
    ax1.legend(fontsize=9)
    ax1.grid(True, alpha=0.3)
    fig1.tight_layout()
    return fig1


def generate_segment_swept():
    """Segment swept."""
    a = (20, 30)
    b = (80, 70)
    r = 10.0
    swept = get_segment_swept_polygon(a, b, r)

    fig2, ax2 = plt.subplots(figsize=(7, 6))
    colors = ["#4ecdc4", "#ff6b6b", "#ffd93d"]
    labels = ["Swept rect", "Start cap", "End cap"]
    for i, poly in enumerate(swept):
        arr = np.array(poly)
        ax2.fill(*np.vstack([arr, arr[0:1]]).T, alpha=0.5, color=colors[i])
        ax2.plot(
            *np.vstack([arr, arr[0:1]]).T,
            "-",
            linewidth=2,
            color=colors[i],
            label=labels[i],
        )
    ax2.plot([a[0], b[0]], [a[1], b[1]], "k--", linewidth=1.5, label="Segment")
    ax2.plot(a[0], a[1], "ko", markersize=8)
    ax2.plot(b[0], b[1], "ko", markersize=8)
    ax2.set_xlim(0, 100)
    ax2.set_ylim(0, 100)
    ax2.set_aspect("equal")
    ax2.set_title("get_segment_swept_polygon — swept area")
    ax2.set_xlabel("X")
    ax2.set_ylabel("Y")
    ax2.legend(fontsize=9)
    ax2.grid(True, alpha=0.3)
    fig2.tight_layout()
    return fig2


def generate_boolean_union():
    """Boolean union."""
    n_seg = 64
    union_a = _make_circle(10.0, n_seg)
    union_b = [(-4.0, 0.0), (12.0, 0.0), (12.0, 8.0), (-4.0, 8.0)]
    union_result = get_polygons_union([union_a, union_b])
    return _plot_boolean(union_a, union_b, union_result, "Union")


def generate_boolean_intersection():
    """Boolean intersection."""
    n_seg = 64
    union_a = _make_circle(10.0, n_seg)
    union_b = [(-4.0, 0.0), (12.0, 0.0), (12.0, 8.0), (-4.0, 8.0)]
    inter_result = get_polygons_intersection(union_a, union_b)
    return _plot_boolean(union_a, union_b, inter_result, "Intersection")


def generate_boolean_difference():
    """Boolean difference."""
    n_seg = 64
    union_a = _make_circle(10.0, n_seg)
    union_b = [(-4.0, 0.0), (12.0, 0.0), (12.0, 8.0), (-4.0, 8.0)]
    diff_result = get_polygons_difference(union_a, union_b)
    return _plot_boolean(union_a, union_b, diff_result, "Difference")


def generate_offset():
    """Polygon offset."""
    triangle = [(0.0, 0.0), (20.0, 0.0), (10.0, 18.0)]
    styles = [("miter", "Miter"), ("round", "Round"), ("square", "Square")]
    style_colors = ["limegreen", "tomato", "dodgerblue"]

    fig6, axes = plt.subplots(1, 3, figsize=(14, 4.5))
    for ax, (style_key, style_label), color in zip(axes, styles, style_colors):
        plot_polygon(ax, triangle, "steelblue", "Original", linewidth=2)
        result = offset_polygon(triangle, 2.0, join_style=style_key)
        for poly in result:
            plot_polygon(ax, poly, color, f"{style_label}", linewidth=2.5)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)
        ax.set_title(f"{style_label} join", fontsize=11, fontweight="bold")
    fig6.tight_layout()
    return fig6


def generate_min_curvature():
    """Min curvature."""
    fig7, ax7 = plt.subplots(figsize=(7, 7))
    sharp = [(0, 0), (20, 0), (10, 18)]
    plot_polygon(ax7, sharp, "steelblue", "Original", linewidth=2)
    filleted = apply_minimum_curvature(sharp, 2.0)
    for poly in filleted:
        plot_polygon(ax7, poly, "tomato", "Filleted (r_min=2)", linewidth=2.5)
    ax7.set_aspect("equal")
    ax7.grid(True, alpha=0.3)
    ax7.legend(fontsize=10)
    ax7.set_title("apply_minimum_curvature", fontsize=11, fontweight="bold")
    fig7.tight_layout()
    return fig7


__images__ = [
    {
        "heading": "get_circle_polygon",
        "caption": (
            "``get_circle_polygon`` approximates a circle"
            " as an n-sided polygon"
        ),
        "function": generate_circle_polygon,
    },
    {
        "heading": "get_segment_swept_polygon",
        "caption": (
            "``get_segment_swept_polygon`` computes the swept area of a line "
            "segment with a given radius"
        ),
        "function": generate_segment_swept,
    },
    {
        "heading": "get_polygons_union",
        "caption": "Polygon union",
        "function": generate_boolean_union,
    },
    {
        "heading": "get_polygons_intersection",
        "caption": "Polygon intersection",
        "function": generate_boolean_intersection,
    },
    {
        "heading": "get_polygons_difference",
        "caption": "Polygon difference",
        "function": generate_boolean_difference,
    },
    {
        "heading": "offset_polygon",
        "caption": "Polygon offset — miter vs round vs square join styles",
        "function": generate_offset,
    },
    {
        "heading": "apply_minimum_curvature",
        "caption": "Minimum curvature fillet applied to a triangle",
        "function": generate_min_curvature,
    },
]
