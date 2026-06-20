"""Generate SVG parsing example images."""

import math

import matplotlib.pyplot as plt
from matplotlib.colors import to_hex

from raygeo.svg import parse_svg_path_data
from tools.plot import auto_limits, plot_geometry


def generate_parsing():
    n_circle = 48
    circle_pts = " ".join(
        f"L {50 + 20 * math.cos(2 * math.pi * i / n_circle):.1f}"
        f" {50 + 20 * math.sin(2 * math.pi * i / n_circle):.1f}"
        for i in range(1, n_circle + 1)
    )
    rect_circle = f"M 10 10 L 90 10 L 90 90 L 10 90 Z M 50 50 {circle_pts} Z"

    star = (
        "M 50 5 L 61 35 L 95 35 L 68 57 L 79 91"
        " L 50 70 L 21 91 L 32 57 L 5 35 L 39 35 Z"
    )

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    for ax, (title, path_data) in enumerate(
        [
            ("Rectangle + Circle", rect_circle),
            ("Star path", star),
        ]
    ):
        ax = axes[ax]
        geoms = parse_svg_path_data(path_data)
        cmap = plt.get_cmap("tab10")
        colors = [to_hex(cmap(i / 10)) for i in range(10)]
        for i, g in enumerate(geoms):
            plot_geometry(
                ax, g, color=colors[i % len(colors)], label=f"Path {i}"
            )
        xmin, xmax, ymin, ymax = (
            auto_limits(geoms) if geoms else (0, 100, 0, 100)
        )
        ax.set_xlim(xmin, xmax)
        ax.set_ylim(ymin, ymax)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend()
        ax.set_title(title)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.svg.md"]
__images__ = [
    {
        "heading": None,
        "caption": "SVG path data parsed into geometries",
        "function": generate_parsing,
    },
]
