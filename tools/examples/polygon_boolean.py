"""Generate polygon boolean operation example images."""

import math

import matplotlib.pyplot as plt

from raygeo.geo.shape.polygon import (
    get_polygons_difference,
    get_polygons_intersection,
    get_polygons_union,
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


def generate_examples(output_dir):
    images = []
    n_seg = 64
    a = _make_circle(10, n_seg)
    b = [(-4, 0), (12, 0), (12, 8), (-4, 8)]

    ops = [
        ("Union", get_polygons_union([a, b])),
        ("Intersection", get_polygons_intersection(a, b)),
        ("Difference", get_polygons_difference(a, b)),
    ]

    fig, axes = plt.subplots(1, 3, figsize=(16, 5))
    for ax, (title, result) in zip(axes, ops):
        plot_polygon(ax, a, "steelblue", "A")
        plot_polygon(ax, b, "tomato", "B")
        if result:
            plot_polygon(ax, result[0], "limegreen", "Result", linewidth=2.5)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend()
        ax.set_title(title)

    fig.tight_layout()
    path = output_dir / "polygon-boolean.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon-boolean.png",
            "caption": (
                "Polygon boolean operations: union, intersection, difference"
            ),
        }
    )

    return {
        "title": "Polygon Boolean Operations",
        "description": (
            "Boolean operations on polygons: union, intersection, and "
            "difference."
        ),
        "images": images,
    }
