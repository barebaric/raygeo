"""Generate visualisations of helix→spiral entry motion assembly."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.ops.assembly.entry import generate_helix_spiral


def _all_moving_pts(result):
    pts = []
    for i in range(result.ops.len()):
        if result.ops.is_travel(i) or result.ops.is_cutting(i):
            ep = result.ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2]))
    return pts


def generate_helix_spiral_example():
    """Helix spiral to ops."""
    result = generate_helix_spiral(
        entry_pt=(80.0, 50.0),
        r_max=50.0,
        tool_radius=3.0,
        step_over=2.0,
        safe_z=2.0,
        target_z=-8.0,
    )

    pts = _all_moving_pts(result)
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    zs = [p[2] for p in pts]

    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")
    colors = plt.cm.viridis(np.linspace(0, 1, len(pts)))
    for i in range(len(pts) - 1):
        ax.plot(
            xs[i : i + 2],
            ys[i : i + 2],
            zs[i : i + 2],
            color=colors[i],
            linewidth=1.5,
        )

    ax.scatter(xs[0], ys[0], zs[0], c="forestgreen", s=80, label="Start")
    ax.scatter(xs[-1], ys[-1], zs[-1], c="crimson", s=80, label="End")

    ax.set_title("Helix → Spiral Entry")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.legend()
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.entry.md"]
__images__ = [
    {
        "heading": "generate_helix_spiral",
        "caption": (
            "Helix → Spiral: helical plunge followed by Archimedean spiral"
        ),
        "function": generate_helix_spiral_example,
    },
]
