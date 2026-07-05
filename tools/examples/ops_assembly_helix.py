"""Generate visualisations of helix entry motion assembly."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.ops.assembly.helix import generate_helix


def _all_moving_pts(result):
    """Extract all (x, y, z) from moving commands (travel + cut)."""
    pts = []
    for i in range(result.ops.len()):
        if result.ops.is_travel(i) or result.ops.is_cutting(i):
            ep = result.ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2]))
    return pts


def generate_helix_example():
    """Helix to ops."""
    result = generate_helix(
        center=(0.0, 0.0),
        start_radius=8.0,
        z_start=2.0,
        z_end=-10.0,
        pitch=3.0,
        direction="CW",
        angular_step=0.1,
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

    # Draw center axis
    ax.plot([0, 0], [0, 0], [2, -10], "k--", alpha=0.3, linewidth=1)

    # Draw start/end markers
    z_start = zs[0] if pts else 0
    z_end = zs[-1] if pts else 0
    ax.scatter(
        xs[0],
        ys[0],
        z_start,
        c="forestgreen",
        s=80,
        label="Start",
    )
    ax.scatter(
        xs[-1],
        ys[-1],
        z_end,
        c="crimson",
        s=80,
        label="End",
    )

    ax.set_title("Helical Entry Path")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.legend()
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.helix.md"]
__images__ = [
    {
        "heading": "generate_helix",
        "caption": "Helical entry path from safe Z to target depth",
        "function": generate_helix_example,
    },
]
