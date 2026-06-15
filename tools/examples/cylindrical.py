"""Generate cylindrical transform example images."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.algo.cylindrical import transform_to_cylinder


def generate_examples(output_dir):
    images = []
    diameter = 20.0
    radius = diameter / 2.0

    verts = []
    for x in range(10, 50, 5):
        verts.extend([(x, -80, 0), (x, 80, 0)])
    for y in range(-60, 70, 20):
        verts.extend([(10, y, 0), (45, y, 0)])

    verts_np = np.array(verts, dtype=np.float32)

    transformed, _, _ = transform_to_cylinder(
        verts_np, diameter, colors=None, degrees_input=True
    )
    t = transformed.reshape(-1, 3)

    fig = plt.figure(figsize=(14, 7))

    ax1 = fig.add_subplot(121)
    for i in range(0, len(verts), 2):
        xs = [verts[i][0], verts[i + 1][0]]
        ys = [verts[i][1], verts[i + 1][1]]
        ax1.plot(xs, ys, "steelblue", linewidth=2)
    ax1.set_xlabel("Linear (mm)")
    ax1.set_ylabel("Angle (degrees)")
    ax1.set_title("Input: flat (X, Y_deg)")
    ax1.grid(True, alpha=0.3)
    ax1.set_xlim(5, 50)
    ax1.set_ylim(-90, 90)

    ax2 = fig.add_subplot(122, projection="3d")

    theta = np.linspace(-np.pi, np.pi, 32)
    z_cyl = np.linspace(5, 50, 20)
    th, zz = np.meshgrid(theta, z_cyl)
    xx = zz
    yy = radius * np.sin(th)
    zz2 = radius * np.cos(th)
    ax2.plot_surface(
        xx,
        yy,
        zz2,
        alpha=0.12,
        color="gray",
        edgecolors="gray",
        linewidth=0.25,
    )

    for i in range(0, len(t), 2):
        ax2.plot(
            t[i : i + 2, 0],
            t[i : i + 2, 1],
            t[i : i + 2, 2],
            "tomato",
            linewidth=2,
        )

    ax2.set_xlabel("X (linear)")
    ax2.set_ylabel("Y")
    ax2.set_zlabel("Z")
    ax2.set_title("Cylindrical projection")
    ax2.view_init(elev=25, azim=-65)
    ax2.set_box_aspect((1.5, 1, 1))

    fig.tight_layout()
    path = output_dir / "cylindrical-transform.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "cylindrical-transform.png",
            "caption": "Flat vertex pairs wrapped onto a cylinder surface",
        }
    )

    return {
        "title": "Cylindrical Transform",
        "description": (
            "Transform flat vertex pairs to cylindrical coordinates, "
            "subdividing segments as needed to follow the cylinder surface."
        ),
        "images": images,
    }
