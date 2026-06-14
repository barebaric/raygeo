"""Generate smoothing example images."""

import math

import matplotlib.pyplot as plt

from raygeo.geo.algo.smooth import smooth_polyline


def generate_examples(output_dir):
    images = []

    n = 30
    pts = [
        (
            50 + 30 * math.cos(2 * math.pi * i / n) + (i % 3) * 5,
            50 + 30 * math.sin(2 * math.pi * i / n) + (i % 4) * 4,
        )
        for i in range(n)
    ]
    pts_3d = [(x, y, 0.0) for x, y in pts]

    fig, axes = plt.subplots(2, 4, figsize=(20, 9))

    def draw(ax, points, title, color, xlim=(0, 100), ylim=(0, 100)):
        sx, sy = zip(*[(p[0], p[1]) for p in points])
        ax.plot(
            sx + (sx[0],),
            sy + (sy[0],),
            color=color,
            linewidth=2.5,
        )
        ax.set_title(title)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_xlim(*xlim)
        ax.set_ylim(*ylim)

    amounts = [50, 100, 200]

    xs, ys = zip(*pts)
    draw(axes[0, 0], pts, "Original", "gray")
    draw(axes[1, 0], pts, "Original", "gray")

    for col, amount in enumerate(amounts, 1):
        smoothed_no_preserve = smooth_polyline(pts_3d, amount, 0.0, True)
        draw(
            axes[0, col],
            smoothed_no_preserve,
            f"Smooth {amount}, no preserve",
            "tomato",
        )

        smoothed_preserve = smooth_polyline(pts_3d, amount, 120.0, True)
        draw(
            axes[1, col],
            smoothed_preserve,
            f"Smooth {amount}, preserve<120°",
            "forestgreen",
        )

    fig.tight_layout()
    path = output_dir / "smooth.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "smooth.png",
            "caption": "Gaussian smoothing with corner preservation",
        }
    )

    return {
        "title": "Smooth",
        "description": (
            "Smooth polylines using Gaussian kernels with corner angle "
            "threshold preservation."
        ),
        "images": images,
    }
