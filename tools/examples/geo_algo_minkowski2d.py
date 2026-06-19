"""Generate Minkowski sum example images."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.minkowski2d import get_polygon_minkowski_sum_convex
from tools.plot import plot_polygon


def generate_minkowski_sum():
    triangle = [(0.0, 0.0), (40.0, 0.0), (20.0, 35.0)]
    square = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]
    result = get_polygon_minkowski_sum_convex(triangle, square)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_polygon(ax, triangle, "steelblue", "Triangle (A)", linewidth=2.5)
    plot_polygon(ax, square, "tomato", "Square (B)", linewidth=2.5)
    for poly in result:
        plot_polygon(
            ax, poly, "limegreen", "A ⊕ B (Minkowski sum)", linewidth=2.5
        )

    ax.set_aspect("equal")
    ax.set_xlim(-10, 80)
    ax.set_ylim(-10, 65)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=11)

    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": "get_polygon_minkowski_sum_convex",
        "caption": "Minkowski sum of two convex polygons",
        "function": generate_minkowski_sum,
    },
]
