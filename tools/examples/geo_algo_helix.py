"""Generate helix example images."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.helix import HelixDirection, generate_helix_3d


def _extract(pts):
    return [p[0] for p in pts], [p[1] for p in pts], [p[2] for p in pts]


def generate_helical():
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=20,
        end_radius=20,
        z_start=0,
        z_end=-30,
        pitch=10,
        direction=HelixDirection.Ccw,
        angular_step=0.05,
        min_revolutions=3,
    )

    fig = plt.figure(figsize=(14, 7))

    ax1 = fig.add_subplot(121, projection="3d")
    xs, ys, zs = _extract(pts)
    ax1.plot(xs, ys, zs, "steelblue", linewidth=2)
    ax1.set_title("Cylindrical Helix (CCW)")
    ax1.set_xlabel("X")
    ax1.set_ylabel("Y")
    ax1.set_zlabel("Z")
    ax1.view_init(elev=25, azim=-60)

    ax2 = fig.add_subplot(122, projection="3d")
    pts2 = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=30,
        z_start=0,
        z_end=-30,
        pitch=10,
        direction=HelixDirection.Cw,
        angular_step=0.05,
        min_revolutions=3,
    )
    xs, ys, zs = _extract(pts2)
    ax2.plot(xs, ys, zs, "crimson", linewidth=2)
    ax2.set_title("Conical Expand Helix (CW)")
    ax2.set_xlabel("X")
    ax2.set_ylabel("Y")
    ax2.set_zlabel("Z")
    ax2.view_init(elev=25, azim=-60)

    fig.subplots_adjust(top=0.85)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.helix.md"]
__images__ = [
    {
        "heading": "generate_helix_3d",
        "caption": "Cylindrical (CCW) and conical-expand (CW) helical paths",
        "function": generate_helical,
    },
]
