"""Generate fitting example images."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle

from raygeo.geo import Geometry
from raygeo.geo.algo.fitting import (
    fit_circle_to_points,
    flatten_to_points,
)


def generate_examples(output_dir):
    images = []

    rng = np.random.default_rng(42)
    angles = np.linspace(0, 2 * math.pi, 50)
    cx, cy, cr = 50, 50, 30
    pts = [
        (
            cx + cr * math.cos(a) + rng.normal(0, 0.5),
            cy + cr * math.sin(a) + rng.normal(0, 0.5),
        )
        for a in angles
    ]

    result = fit_circle_to_points([(x, y, 0.0) for x, y in pts])
    fc, fr, ferr = result if result else ((0.0, 0.0), 0.0, 0.0)

    fig, ax = plt.subplots(figsize=(7, 7))
    xs, ys = zip(*pts)
    ax.scatter(xs, ys, color="tomato", s=10, label="Noisy points")
    if result:
        circle = Circle(
            (fc[0], fc[1]),
            fr,
            fill=False,
            color="forestgreen",
            linewidth=2,
            label="Fitted circle",
        )
        ax.add_patch(circle)
        ax.scatter(
            fc[0], fc[1], color="forestgreen", marker="x", s=100, linewidths=2
        )
    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.grid(True, alpha=0.3)
    ax.legend()
    ax.set_title(
        f"Fit circle to points (error: {ferr:.4f})"
        if result
        else "Circle fit failed"
    )

    fig.tight_layout()
    path = output_dir / "fitting-circle.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "fitting-circle.png",
            "caption": "Circle fitted to noisy point cloud",
        }
    )

    n_arc = 30
    arc_pts = [
        (
            50 + 30 * math.cos(math.pi * i / n_arc),
            50 + 30 * math.sin(math.pi * i / n_arc),
        )
        for i in range(n_arc + 1)
    ]

    raw_geom = Geometry.from_points(arc_pts, close=False)
    fit_geom = raw_geom.fit_curves(3.0, arcs=True, beziers=False)

    fit_flat = flatten_to_points(fit_geom, 0.5)
    fit_pts = fit_flat[0] if fit_flat else []

    fig2, axes2 = plt.subplots(1, 2, figsize=(14, 6))
    for ax_i in axes2:
        ax_i.set_aspect("equal")
        ax_i.set_xlim(0, 100)
        ax_i.set_ylim(0, 100)
        ax_i.grid(True, alpha=0.3)

    axes2[0].plot(
        [p[0] for p in arc_pts],
        [p[1] for p in arc_pts],
        "o-",
        color="tomato",
        markersize=3,
        linewidth=1,
        label="Original points",
    )
    axes2[0].set_title("Original polyline")

    if fit_pts:
        axes2[1].plot(
            [p[0] for p in fit_pts],
            [p[1] for p in fit_pts],
            color="forestgreen",
            linewidth=2.5,
            label=f"Fitted ({len(fit_geom)} cmds)",
        )
        axes2[1].legend()
    axes2[1].set_title("Fitted primitives (tol=3.0)")

    fig2.tight_layout()
    path2 = output_dir / "fitting-primitives.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "fitting-primitives.png",
            "caption": "Polyline fitted with arc and line primitives",
        }
    )

    return {
        "title": "Fitting",
        "description": (
            "Fit circles to point clouds and fit polylines with arc/line "
            "primitives."
        ),
        "images": images,
    }
