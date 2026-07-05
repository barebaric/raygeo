"""Generate visualisations of spiral entry motion assembly."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.ops.assembly.spiral import generate_spiral


def _all_moving_pts(result):
    pts = []
    for i in range(result.ops.len()):
        if result.ops.is_travel(i) or result.ops.is_cutting(i):
            ep = result.ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2]))
    return pts


def generate_spiral_example():
    """Spiral to ops."""
    result = generate_spiral(
        center=(0.0, 0.0),
        z=-5.0,
        start_radius=3.0,
        end_radius=25.0,
        revolutions=3.5,
        direction="CW",
        angular_step=0.1,
    )

    pts = _all_moving_pts(result)
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]

    fig, ax = plt.subplots(figsize=(8, 8))
    colors = plt.cm.viridis(np.linspace(0, 1, len(pts)))
    for i in range(len(pts) - 1):
        ax.plot(
            xs[i : i + 2],
            ys[i : i + 2],
            color=colors[i],
            linewidth=1.0,
        )

    # Draw start/end markers
    ax.scatter(xs[0], ys[0], c="forestgreen", s=80, label="Start")
    ax.scatter(xs[-1], ys[-1], c="crimson", s=80, label="End")

    ax.set_aspect("equal")
    ax.set_title("Spiral Entry Path")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend()
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.spiral.md"]
__images__ = [
    {
        "heading": "generate_spiral",
        "caption": "Flat Archimedean spiral with smoothing circular pass",
        "function": generate_spiral_example,
    },
]
