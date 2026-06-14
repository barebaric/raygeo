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

    smooth_amounts = [2, 5, 10]

    fig, axes = plt.subplots(1, 4, figsize=(18, 5))

    xs, ys = zip(*pts)
    axes[0].plot(
        xs + (xs[0],),
        ys + (ys[0],),
        color="tomato",
        linewidth=2,
        label="Original",
    )
    axes[0].scatter(xs, ys, color="tomato", s=15)
    axes[0].set_title("Original (jagged)")
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)
    axes[0].set_xlim(0, 100)
    axes[0].set_ylim(0, 100)

    for ax_i, amount in zip(axes[1:], smooth_amounts):
        smoothed = smooth_polyline(pts_3d, amount, 120.0, True)
        sx, sy = zip(*[(p[0], p[1]) for p in smoothed])
        ax_i.plot(
            sx + (sx[0],),
            sy + (sy[0],),
            color="forestgreen",
            linewidth=2.5,
            label=f"Amount={amount}",
        )
        ax_i.scatter(sx, sy, color="forestgreen", s=10)
        ax_i.set_title(f"Smooth amount={amount}")
        ax_i.set_aspect("equal")
        ax_i.grid(True, alpha=0.3)
        ax_i.set_xlim(0, 100)
        ax_i.set_ylim(0, 100)

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
