"""Generate polygon offset example images."""

__images__ = [
    {
        "stem": "polygon-offset",
        "caption": "Polygon offset — miter vs round vs square join styles",
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "offset_polygon",
    },
]

import matplotlib.pyplot as plt

from raygeo.geo.shape.polygon import offset_polygon
from tools.plot import plot_polygon


def generate_examples(output_dir):
    images = []
    triangle = [(0, 0), (20, 0), (10, 18)]

    styles = [("miter", "Miter"), ("round", "Round"), ("square", "Square")]
    colors = ["limegreen", "tomato", "dodgerblue"]

    fig, axes = plt.subplots(1, 3, figsize=(14, 4.5))

    for ax, (style_key, style_label), color in zip(axes, styles, colors):
        plot_polygon(ax, triangle, "steelblue", "Original", linewidth=2)
        result = offset_polygon(triangle, 2.0, join_style=style_key)
        for poly in result:
            plot_polygon(ax, poly, color, f"{style_label}", linewidth=2.5)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)
        ax.set_title(f"{style_label} join", fontsize=11, fontweight="bold")

    fig.tight_layout()
    path = output_dir / "polygon-offset.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon-offset.png",
            "caption": "Polygon offset — miter vs round vs square join styles",
        }
    )

    return {
        "title": "Polygon Offset",
        "description": "Offset (grow/shrink) polygon boundaries"
        " with configurable join style.",
        "images": images,
    }
