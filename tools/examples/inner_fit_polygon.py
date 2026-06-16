"""Generate Inner Fit Polygon example images."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.nest2d.ifp import inner_fit_polygon
from tools.plot import plot_polygon


def generate_examples(output_dir):
    images = []

    bin_poly = [(0.0, 0.0), (100.0, 0.0), (100.0, 80.0), (0.0, 80.0)]
    part = [(0.0, 0.0), (30.0, 0.0), (30.0, 25.0), (0.0, 25.0)]

    ifp_result = inner_fit_polygon(bin_poly, part)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_polygon(ax, bin_poly, "black", "Bin (B)", linewidth=2.5)
    if ifp_result:
        plot_polygon(
            ax, ifp_result[0], "limegreen", "IFP (valid region)", linewidth=2.5
        )
        xs = [p[0] for p in ifp_result[0]] + [ifp_result[0][0][0]]
        ys = [p[1] for p in ifp_result[0]] + [ifp_result[0][0][1]]
        ax.fill(xs, ys, alpha=0.08, color="limegreen")

    cx, cy = 15, 12
    shifted = [(p[0] + cx, p[1] + cy) for p in part]
    plot_polygon(ax, shifted, "tomato", "Part (placed example)", linewidth=2.5)
    xs_s = [p[0] for p in shifted] + [shifted[0][0]]
    ys_s = [p[1] for p in shifted] + [shifted[0][1]]
    ax.fill(xs_s, ys_s, alpha=0.15, color="tomato")

    ax.set_aspect("equal")
    ax.set_xlim(-10, 120)
    ax.set_ylim(-10, 100)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    ax.set_title(
        "Inner Fit Polygon (IFP) — valid placement region", fontsize=12
    )

    fig.tight_layout()
    path = output_dir / "inner-fit-polygon.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "inner-fit-polygon.png",
            "caption": "Inner Fit Polygon showing valid placement region",
        }
    )

    return {
        "title": "Inner Fit Polygon",
        "description": "Inner Fit Polygon (IFP) for a part inside a bin.",
        "images": images,
    }
