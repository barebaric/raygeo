"""Generate ramp example images."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.ramp import RampStyle, generate_ramp_3d


def _extract(pts):
    return [p[0] for p in pts], [p[1] for p in pts], [p[2] for p in pts]


def generate_linear_zigzag():
    fig = plt.figure(figsize=(16, 7))

    ax1 = fig.add_subplot(121, projection="3d")
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(50, 0),
        z_start=0,
        z_end=-10,
        max_ramp_angle_deg=45,
        style=RampStyle.LINEAR,
    )
    xs, ys, zs = _extract(pts)
    ax1.plot(xs, ys, zs, "steelblue", linewidth=2.5)
    ax1.set_title("Linear Ramp")
    ax1.set_xlabel("X")
    ax1.set_ylabel("Y")
    ax1.set_zlabel("Z")
    ax1.view_init(elev=30, azim=-50)

    ax2 = fig.add_subplot(122, projection="3d")
    pts = generate_ramp_3d(
        start=(0, 0),
        end=(50, 0),
        z_start=0,
        z_end=-10,
        max_ramp_angle_deg=45,
        style=RampStyle.ZIG_ZAG,
        lateral_amplitude=5,
    )
    xs, ys, zs = _extract(pts)
    ax2.plot(xs, ys, zs, "crimson", linewidth=2.5)
    ax2.set_title("ZigZag Ramp")
    ax2.set_xlabel("X")
    ax2.set_ylabel("Y")
    ax2.set_zlabel("Z")
    ax2.view_init(elev=30, azim=-50)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.ramp.md"]
__images__ = [
    {
        "heading": "generate_ramp_3d",
        "caption": "Linear (left) and ZigZag (right) ramp entry paths",
        "function": generate_linear_zigzag,
    },
]
