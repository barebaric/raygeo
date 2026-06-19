"""Generate polygon offset example images."""

__images__ = [
    {
        "stem": "polygon-offset",
        "caption": "Polygon offset (outward)",
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "offset_polygon",
    },
]

import math

import matplotlib.pyplot as plt

from raygeo.geo.shape.polygon import offset_polygon
from tools.plot import plot_polygon


def generate_examples(output_dir):
    images = []
    n_seg = 64
    r = 10
    pts = [
        (
            r * math.cos(2 * math.pi * i / n_seg),
            r * math.sin(2 * math.pi * i / n_seg),
        )
        for i in range(n_seg)
    ]

    fig, ax = plt.subplots(figsize=(7, 7))
    plot_polygon(ax, pts, "steelblue", "Original")
    result = offset_polygon(pts, 2.0)
    for i, poly in enumerate(result):
        plot_polygon(ax, poly, "limegreen", f"Offset {i}", linewidth=2.5)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend()

    fig.tight_layout()
    path = output_dir / "polygon-offset.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {"path": "polygon-offset.png", "caption": "Polygon offset (outward)"}
    )

    return {
        "title": "Polygon Offset",
        "description": "Offset (grow/shrink) polygon boundaries.",
        "images": images,
    }
