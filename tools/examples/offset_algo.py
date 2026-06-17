"""Generate concentric offset example images."""

import matplotlib.pyplot as plt

from raygeo.geo import Geometry
from raygeo.geo.algo.offset import concentric_offsets
from tools.plot import plot_geometry


def generate_examples(output_dir):
    images = []

    # 100x100 square with concentric inward offsets
    g = Geometry()
    g.move_to(10, 10)
    g.line_to(110, 10)
    g.line_to(110, 110)
    g.line_to(10, 110)
    g.close_path()

    offsets = concentric_offsets(g, step=10, max_passes=10, min_area=1)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_geometry(ax, g, color="black", linewidth=2, label="Original")
    colors = plt.cm.plasma(
        [i / max(len(offsets), 1) for i in range(len(offsets))]
    )
    for i, off in enumerate(offsets):
        plot_geometry(
            ax,
            off,
            color=colors[i],
            linewidth=1.5,
            label=f"Offset {i + 1}" if i < 5 else None,
        )

    ax.set_aspect("equal")
    ax.set_xlim(0, 120)
    ax.set_ylim(0, 120)
    ax.set_title("Concentric inward offsets")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=8, loc="upper right")

    fig.tight_layout()
    path = output_dir / "concentric-offsets.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "concentric-offsets.png",
            "caption": "Concentric inward offsets for adaptive clearing / pocketing",  # noqa: E501
        }
    )

    return {
        "title": "Concentric Offsets",
        "description": (
            "Generate concentric inward offsets from a geometry for "
            "adaptive clearing and pocketing toolpath generation."
        ),
        "images": images,
    }
