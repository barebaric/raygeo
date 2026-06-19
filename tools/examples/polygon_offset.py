"""Generate polygon offset example images."""

__images__ = [
    {
        "stem": "polygon-offset",
        "caption": "Polygon offset — miter vs round vs square join styles",
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "offset_polygon",
    },
    {
        "stem": "polygon-min-curvature",
        "caption": "Minimum curvature fillet applied to a triangle",
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "apply_minimum_curvature",
    },
]

import matplotlib.pyplot as plt

from raygeo.geo.shape.polygon import apply_minimum_curvature, offset_polygon
from tools.plot import plot_polygon


def generate_examples(output_dir):
    images = []
    triangle = [(0, 0), (20, 0), (10, 18)]

    # ── offset_polygon: miter vs round vs square join styles ────────────
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

    # ── apply_minimum_curvature ──────────────────────────────────────────
    fig2, ax2 = plt.subplots(figsize=(7, 7))
    sharp = [(0, 0), (20, 0), (10, 18)]
    plot_polygon(ax2, sharp, "steelblue", "Original", linewidth=2)
    filleted = apply_minimum_curvature(sharp, 2.0)
    for poly in filleted:
        plot_polygon(ax2, poly, "tomato", "Filleted (r_min=2)", linewidth=2.5)
    ax2.set_aspect("equal")
    ax2.grid(True, alpha=0.3)
    ax2.legend(fontsize=10)
    ax2.set_title("apply_minimum_curvature", fontsize=11, fontweight="bold")

    fig2.tight_layout()
    path2 = output_dir / "polygon-min-curvature.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "polygon-min-curvature.png",
            "caption": "Minimum curvature fillet applied to a triangle",
        }
    )

    return {
        "title": "Polygon Offset",
        "description": "Offset (grow/shrink) polygon boundaries"
        " with configurable join style.",
        "images": images,
    }
