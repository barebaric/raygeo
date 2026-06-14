"""Generate overcut example images."""

import matplotlib.pyplot as plt

from raygeo.geo import Geometry
from raygeo.geo.algo.overcut import apply_overcut
from tools.plot import plot_geometry


def generate_examples(output_dir):
    images = []

    geom = Geometry.from_points(
        [
            (20, 20),
            (80, 20),
            (80, 80),
            (20, 80),
        ]
    )

    overcut_dist = 10.0
    result = apply_overcut(geom, overcut_dist)

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    axes[0].set_title("Original contour")
    plot_geometry(
        axes[0], geom, color="steelblue", linewidth=2, show_points=True
    )
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)

    axes[1].set_title(f"With overcut ({overcut_dist} mm)")
    plot_geometry(
        axes[1], result, color="forestgreen", linewidth=2, show_points=True
    )
    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)

    for ax in axes:
        ax.set_xlim(5, 95)
        ax.set_ylim(5, 95)

    fig.tight_layout()
    path = output_dir / "overcut.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {"path": "overcut.png", "caption": "Overcut applied to closed contour"}
    )

    return {
        "title": "Overcut",
        "description": (
            "Extend a closed contour past its start point to ensure a "
            "clean cut through the material."
        ),
        "images": images,
    }
