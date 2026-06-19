"""Generate circle intersection example images."""

import matplotlib.pyplot as plt
from matplotlib.patches import Circle as MplCircle

from raygeo.geo.shape.circle import (
    get_circle_circle_intersections,
    get_line_circle_intersections,
)


def generate_intersections():
    fig, axes = plt.subplots(1, 2, figsize=(14, 7))

    c1, r1 = (0.0, 0.0), 5.0
    c2, r2 = (6.0, 0.0), 5.0

    circ1 = MplCircle(c1, r1, fill=False, edgecolor="steelblue", linewidth=2)
    circ2 = MplCircle(c2, r2, fill=False, edgecolor="tomato", linewidth=2)
    axes[0].add_patch(circ1)
    axes[0].add_patch(circ2)
    axes[0].plot(c1[0], c1[1], "o", color="steelblue", markersize=6)
    axes[0].plot(c2[0], c2[1], "o", color="tomato", markersize=6)

    pts = get_circle_circle_intersections(c1, r1, c2, r2)
    for pt in pts:
        axes[0].plot(pt[0], pt[1], "*", color="gold", markersize=15, zorder=5)

    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)
    axes[0].set_xlim(-7, 13)
    axes[0].set_ylim(-7, 7)
    axes[0].set_title("Circle-Circle Intersection", fontsize=14)

    circ3 = MplCircle(
        (3.0, 0.0), 5.0, fill=False, edgecolor="steelblue", linewidth=2
    )
    axes[1].add_patch(circ3)
    line_p1, line_p2 = (-4.0, 3.0), (10.0, -2.0)
    axes[1].plot(
        [line_p1[0], line_p2[0]],
        [line_p1[1], line_p2[1]],
        color="tomato",
        linewidth=2,
        label="Line segment",
    )

    inter_pts = get_line_circle_intersections(
        line_p1, line_p2, (3.0, 0.0), 5.0
    )
    for pt in inter_pts:
        axes[1].plot(pt[0], pt[1], "*", color="gold", markersize=15, zorder=5)

    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)
    axes[1].set_xlim(-6, 12)
    axes[1].set_ylim(-6, 6)
    axes[1].set_title("Line-Circle Intersection", fontsize=14)
    axes[1].legend(fontsize=11)

    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": "get_circle_circle_intersections",
        "caption": "Circle-circle and line-circle intersection points",
        "function": generate_intersections,
    },
]
