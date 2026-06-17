"""Generate helix example images."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.helix import HelixDirection, generate_helix


def _extract(pts):
    return [p[0] for p in pts], [p[1] for p in pts], [p[2] for p in pts]


def generate_examples(output_dir):
    images = []

    # Cylindrical helix
    pts = generate_helix(
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
    pts2 = generate_helix(
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
    path = output_dir / "helix-cylindrical-conical.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "helix-cylindrical-conical.png",
            "caption": "Cylindrical (CCW) and conical-expand (CW) helical paths",  # noqa: E501
        }
    )

    return {
        "title": "Helical Path Generation",
        "description": (
            "Generate 3D helical polylines for helical boring and "
            "ramping operations, supporting cylindrical and conical geometry."
        ),
        "images": images,
    }
