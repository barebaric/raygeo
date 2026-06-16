"""Generate simplify example images."""

import math

import matplotlib.pyplot as plt

from raygeo.geo import Geometry
from raygeo.geo.algo.simplify import simplify_polyline_3d
from tools.plot import plot_geometry


def generate_examples(output_dir):
    images = []

    #
    # 2D simplify + linearize example (existing)
    #
    n = 100
    pts = [
        (
            50 + 30 * math.cos(2 * math.pi * i / n) + (i % 5) * 1.5,
            50 + 30 * math.sin(2 * math.pi * i / n) + (i % 7) * 1.0,
        )
        for i in range(n)
    ]
    geom = Geometry.from_points(pts, close=True)

    tol3 = 3.0
    tol5 = 5.0
    tol8 = 8.0

    simplified3 = geom.copy()
    simplified3.simplify(tol3)

    simplified5 = geom.copy()
    simplified5.simplify(tol5)

    linearized = geom.copy()
    linearized.fit_curves(tol8)
    linearized.linearize(tol8)

    fig, axes = plt.subplots(1, 4, figsize=(20, 5))

    axes[0].set_title(f"Original ({len(geom)} cmds)")
    plot_geometry(
        axes[0],
        geom,
        color="tomato",
        linewidth=2,
    )
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)

    axes[1].set_title(f"Simplify tol={tol3} ({len(simplified3)} cmds)")
    plot_geometry(
        axes[1],
        simplified3,
        color="tomato",
        linewidth=2,
        show_points=True,
    )
    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)

    axes[2].set_title(f"Simplify tol={tol5} ({len(simplified5)} cmds)")
    plot_geometry(
        axes[2],
        simplified5,
        color="steelblue",
        linewidth=2,
        show_points=True,
    )
    axes[2].set_aspect("equal")
    axes[2].grid(True, alpha=0.3)

    axes[3].set_title(f"Fit + Linearize tol={tol8} ({len(linearized)} cmds)")
    plot_geometry(
        axes[3],
        linearized,
        color="forestgreen",
        linewidth=2,
        show_points=True,
    )
    axes[3].set_aspect("equal")
    axes[3].grid(True, alpha=0.3)

    for ax in axes:
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)

    fig.tight_layout()
    path = output_dir / "simplify.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "simplify.png",
            "caption": "Geometry simplification and linearization",
        }
    )

    #
    # 3D simplify example
    #
    pts3d = [
        (10 + t, 30 + 20 * math.sin(t / 8), 10 * math.sin(t / 4))
        for t in [i * 0.0625 for i in range(1280)]
    ]
    result = simplify_polyline_3d(pts3d, tolerance=1.0)

    fig3d = plt.figure(figsize=(10, 8))
    ax3d = fig3d.add_subplot(111, projection="3d")
    xs, ys, zs = zip(*pts3d)
    rx, ry, rz = zip(*result)
    ax3d.plot(xs, ys, zs, "b-", linewidth=1.5, alpha=0.4, label="Original")
    ax3d.plot(rx, ry, rz, "r-o", linewidth=2, markersize=3, label="Simplified")
    title = f"3D polyline simplification: {len(pts3d)} → {len(result)} pts"
    ax3d.set_title(title)
    ax3d.set_xlabel("X")
    ax3d.set_ylabel("Y")
    ax3d.set_zlabel("Z")
    ax3d.legend()
    fig3d.tight_layout()
    path3d = output_dir / "simplify-3d.png"
    fig3d.savefig(path3d, dpi=150)
    plt.close(fig3d)
    images.append(
        {
            "path": "simplify-3d.png",
            "caption": "3D polyline simplification preserving Z coordinates",
        }
    )

    return {
        "title": "Simplify",
        "description": (
            "Reduce the number of points in a geometry using "
            "Ramer-Douglas-Peucker simplification, or convert curves "
            "to line segments via linearization."
        ),
        "images": images,
    }
