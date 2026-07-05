"""Generate visualisations of ramp entry motion assembly."""

import matplotlib.pyplot as plt

from raygeo.ops.assembly.ramp import generate_ramp


def _all_moving_pts(result):
    pts = []
    for i in range(result.ops.len()):
        if result.ops.is_travel(i) or result.ops.is_cutting(i):
            ep = result.ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2]))
    return pts


def generate_ramp_example():
    """Ramp to ops."""
    result = generate_ramp(
        start=(0.0, 0.0),
        end=(120.0, 0.0),
        z_start=2.0,
        z_end=-8.0,
        style="zigzag",
        lateral_amplitude=4.0,
    )

    pts = _all_moving_pts(result)
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    zs = [p[2] for p in pts]

    fig = plt.figure(figsize=(10, 6))
    ax = fig.add_subplot(111, projection="3d")
    ax.plot(xs, ys, zs, color="steelblue", linewidth=1.5)

    ax.scatter(xs[0], ys[0], zs[0], c="forestgreen", s=80, label="Start")
    ax.scatter(xs[-1], ys[-1], zs[-1], c="crimson", s=80, label="End")

    ax.set_title("ZigZag Ramp Entry Path")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.legend()
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.ramp.md"]
__images__ = [
    {
        "heading": "generate_ramp",
        "caption": "ZigZag ramp entry path from safe Z to target depth",
        "function": generate_ramp_example,
    },
]
