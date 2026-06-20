"""Generate ray-line intersection example images."""

import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch

from raygeo.geo.algo.intersect import ray_line_intersection


def generate_ray_line():
    origin = (1.0, 2.0)
    direction = (1.0, 0.3)

    segments = [
        ((3, 0), (3, 5)),  # vertical, hit
        ((4, 3), (7, 3.5)),  # diagonal, hit
        ((0, 5), (2, 4)),  # behind ray, miss
        ((8, 0), (8, 6)),  # far right, miss (ray goes above)
    ]

    results = [
        ray_line_intersection(origin, direction, a, b) for a, b in segments
    ]

    fig, ax = plt.subplots(figsize=(7, 5))
    ax.set_aspect("equal")
    ax.set_xlim(-1, 10)
    ax.set_ylim(-1, 7)

    ray_len = 12.0
    mag = (direction[0] ** 2 + direction[1] ** 2) ** 0.5
    dx = direction[0] / mag * ray_len
    dy = direction[1] / mag * ray_len

    ax.add_patch(
        FancyArrowPatch(
            origin,
            (origin[0] + dx, origin[1] + dy),
            arrowstyle="->",
            color="darkorange",
            linewidth=1.8,
            zorder=1,
        )
    )
    ax.plot(
        origin[0],
        origin[1],
        "o",
        color="darkorange",
        markersize=8,
        zorder=5,
        label="Origin O",
    )

    for i, ((a, b), hit) in enumerate(zip(segments, results)):
        if hit is not None:
            ax.plot(
                [a[0], b[0]],
                [a[1], b[1]],
                "g-",
                linewidth=2.5,
                label=f"Segment $S_{i + 1}$ (hit)",
            )
            ax.plot(hit[0], hit[1], "ro", markersize=7, zorder=5)
        else:
            ax.plot(
                [a[0], b[0]],
                [a[1], b[1]],
                "gray",
                linestyle="--",
                linewidth=1.5,
                label=f"Segment $S_{i + 1}$ (miss)",
            )

    ax.set_title("ray_line_intersection — ray vs. line segments", fontsize=13)
    ax.legend(fontsize=9, loc="lower right")
    ax.grid(True, alpha=0.2)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.intersect.md"]
__images__ = [
    {
        "heading": "ray_line_intersection",
        "caption": (
            "Ray–line segment intersection: the ray from origin O"
            " hits segments S₁ and S₂ (marked), misses S₃"
        ),
        "function": generate_ray_line,
    },
]
