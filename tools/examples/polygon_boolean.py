"""Generate polygon boolean operation example images."""

__images__ = [
    {
        "stem": "polygon-boolean-union",
        "caption": "Polygon union",
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "get_polygons_union",
    },
    {
        "stem": "polygon-boolean-intersection",
        "caption": "Polygon intersection",
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "get_polygons_intersection",
    },
    {
        "stem": "polygon-boolean-difference",
        "caption": "Polygon difference",
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "get_polygons_difference",
    },
]

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


def generate_examples(output_dir):
    images = []
    n_seg = 64
    a = _make_circle(10, n_seg)
    b = [(-4, 0), (12, 0), (12, 8), (-4, 8)]

    union_result = get_polygons_union([a, b])
    fig1 = _plot_boolean(a, b, union_result, "Union")
    path1 = output_dir / "polygon-boolean-union.png"
    fig1.savefig(path1, dpi=150)
    plt.close(fig1)
    images.append(
        {
            "path": "polygon-boolean-union.png",
            "caption": "Polygon union",
        }
    )

    inter_result = get_polygons_intersection(a, b)
    fig2 = _plot_boolean(a, b, inter_result, "Intersection")
    path2 = output_dir / "polygon-boolean-intersection.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "polygon-boolean-intersection.png",
            "caption": "Polygon intersection",
        }
    )

    diff_result = get_polygons_difference(a, b)
    fig3 = _plot_boolean(a, b, diff_result, "Difference")
    path3 = output_dir / "polygon-boolean-difference.png"
    fig3.savefig(path3, dpi=150)
    plt.close(fig3)
    images.append(
        {
            "path": "polygon-boolean-difference.png",
            "caption": "Polygon difference",
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
