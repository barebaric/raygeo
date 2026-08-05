"""Generate flat-spiral example images."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.helix import HelixDirection
from raygeo.geo.algo.spiral import generate_spiral_3d


def _extract(pts):
    return [p[0] for p in pts], [p[1] for p in pts], [p[2] for p in pts]


def generate_inward_outward():
    pts = generate_spiral_3d(
        center=(0, 0),
        z=0,
        start_radius=5,
        end_radius=30,
        revolutions=3,
        direction=HelixDirection.CCW,
        angular_step=0.05,
    )

    fig = plt.figure(figsize=(14, 7))

    ax1 = fig.add_subplot(121, projection="3d")
    xs, ys, zs = _extract(pts)
    ax1.plot(xs, ys, zs, "steelblue", linewidth=2)
    ax1.set_title("Outward Spiral (CCW)")
    ax1.set_xlabel("X")
    ax1.set_ylabel("Y")
    ax1.set_zlabel("Z")
    ax1.view_init(elev=25, azim=-60)

    pts2 = generate_spiral_3d(
        center=(0, 0),
        z=0,
        start_radius=30,
        end_radius=5,
        revolutions=3,
        direction=HelixDirection.CW,
        angular_step=0.05,
    )
    ax2 = fig.add_subplot(122, projection="3d")
    xs, ys, zs = _extract(pts2)
    ax2.plot(xs, ys, zs, "crimson", linewidth=2)
    ax2.set_title("Inward Spiral (CW)")
    ax2.set_xlabel("X")
    ax2.set_ylabel("Y")
    ax2.set_zlabel("Z")
    ax2.view_init(elev=25, azim=-60)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.spiral.md"]
__images__ = [
    {
        "heading": "generate_spiral_3d",
        "caption": "Outward (CCW) and inward (CW) flat Archimedean spirals",
        "function": generate_inward_outward,
    },
]
