"""Generate clipping example images."""

import math

import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

from raygeo.geo.algo.clipping import (
    clip_line_segment_with_polygons,
    clip_line_segment_with_rect,
    subtract_polygons_from_line_segment,
)
from tools.plot import plot_polygon


def generate_rect():
    """Line clipped to rectangle."""
    rect = (10.0, 10.0, 90.0, 90.0)
    p1 = (5.0, 50.0, 0.0)
    p2 = (95.0, 50.0, 0.0)

    clipped = clip_line_segment_with_rect(p1, p2, rect)

    fig, ax = plt.subplots(figsize=(7, 7))
    ax.add_patch(
        Rectangle(
            (rect[0], rect[1]),
            rect[2] - rect[0],
            rect[3] - rect[1],
            fill=False,
            edgecolor="steelblue",
            linewidth=2,
            label="Rect",
        )
    )
    ax.plot(
        [p1[0], p2[0]],
        [p1[1], p2[1]],
        color="tomato",
        linewidth=2,
        label="Original",
    )
    if clipped:
        seg_start, seg_end = clipped
        ax.plot(
            [seg_start[0], seg_end[0]],
            [seg_start[1], seg_end[1]],
            color="forestgreen",
            linewidth=3,
            label="Clipped",
        )
    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.grid(True, alpha=0.3)
    ax.legend()

    fig.tight_layout()
    return fig


def generate_polygon():
    """Line clipped to polygon."""
    n_star = 5
    polygon = []
    for i in range(n_star * 2):
        a = -math.pi / 2 + math.pi * i / n_star
        r = 40 if i % 2 == 0 else 16
        polygon.append((50 + r * math.cos(a), 50 + r * math.sin(a)))
    p3 = (0.0, 50.0, 0.0)
    p4 = (100.0, 50.0, 0.0)
    segs = clip_line_segment_with_polygons(p3, p4, [polygon])

    fig2, ax2 = plt.subplots(figsize=(7, 7))
    plot_polygon(ax2, polygon, "steelblue", "Polygon")
    ax2.plot(
        [p3[0], p4[0]],
        [p3[1], p4[1]],
        color="tomato",
        linewidth=2,
        label="Original",
    )
    for seg in segs:
        ax2.plot(
            [seg[0][0], seg[1][0]],
            [seg[0][1], seg[1][1]],
            color="forestgreen",
            linewidth=3,
            label="Clipped" if seg == segs[0] else None,
        )
    ax2.set_aspect("equal")
    ax2.set_xlim(0, 100)
    ax2.set_ylim(0, 100)
    ax2.grid(True, alpha=0.3)
    ax2.legend()

    fig2.tight_layout()
    return fig2


def generate_subtract():
    """Line clipped to polygon (subtract variant)."""
    triangle = [(50, 10), (90, 80), (10, 80)]
    p5 = (10.0, 60.0, 0.0)
    p6 = (90.0, 60.0, 0.0)
    subtracted = subtract_polygons_from_line_segment(p5, p6, [triangle])

    fig3, ax3 = plt.subplots(figsize=(7, 7))
    plot_polygon(ax3, triangle, "steelblue", "Triangle region")
    ax3.plot(
        [p5[0], p6[0]],
        [p5[1], p6[1]],
        color="tomato",
        linewidth=2,
        label="Original",
    )
    for seg in subtracted:
        ax3.plot(
            [seg[0][0], seg[1][0]],
            [seg[0][1], seg[1][1]],
            color="forestgreen",
            linewidth=3,
            label="Remaining" if seg == subtracted[0] else None,
        )
    ax3.set_aspect("equal")
    ax3.set_xlim(0, 100)
    ax3.set_ylim(0, 100)
    ax3.grid(True, alpha=0.3)
    ax3.legend()
    ax3.set_title("Subtract polygons from line segment")

    fig3.tight_layout()
    return fig3


__images__ = [
    {
        "heading": "clip_line_segment_with_rect",
        "caption": "Line clipped to rectangle",
        "function": generate_rect,
    },
    {
        "heading": "clip_line_segment_with_polygons",
        "caption": "Line clipped to polygon",
        "function": generate_polygon,
    },
    {
        "heading": "subtract_polygons_from_line_segment",
        "caption": "Subtract polygon from line",
        "function": generate_subtract,
    },
]
