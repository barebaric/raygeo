"""Generate arc linearization example images."""

__images__ = [
    {
        "stem": "arc-linearize",
        "caption": "Arc linearization: coarse and fine resolution",
        "doc": "raygeo.geo.shape.arc.md",
        "heading": "linearize_arc",
    },
]

import math

import matplotlib.pyplot as plt

from raygeo.geo import Arc, Geometry
from raygeo.geo.shape.arc import linearize_arc
from tools.plot import plot_geometry


def generate_examples(output_dir):
    images = []

    r = 8
    arc_angle = 270
    sweep_rad = math.radians(arc_angle)
    end_x = r * math.cos(sweep_rad)
    end_y = r * math.sin(sweep_rad)

    geom = Geometry()
    geom.move_to(r, 0, 0)
    geom.arc_to(end_x, end_y, -r, 0, False, 0)

    cmds = geom.iter_typed_commands()
    arc_cmd = None
    for cmd in cmds:
        if isinstance(cmd, Arc):
            arc_cmd = cmd
            break

    fig, axes = plt.subplots(1, 3, figsize=(18, 6))

    plot_geometry(axes[0], geom, color="steelblue", linewidth=2)
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)
    axes[0].set_title(f"Original arc ({arc_angle}°)", fontsize=14)
    axes[0].set_xlim(-12, 12)
    axes[0].set_ylim(-12, 12)

    coarse_segments = linearize_arc(arc_cmd, (r, 0.0, 0.0), 4)
    for (sx, sy, _), (ex, ey, _) in coarse_segments:
        axes[1].plot(
            [sx, ex],
            [sy, ey],
            color="tomato",
            linewidth=2,
        )

    fine_segments = linearize_arc(arc_cmd, (r, 0.0, 0.0), 2)
    for (sx, sy, _), (ex, ey, _) in fine_segments:
        axes[2].plot(
            [sx, ex],
            [sy, ey],
            color="forestgreen",
            linewidth=2,
        )

    for ax in axes[1:]:
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_xlim(-12, 12)
        ax.set_ylim(-12, 12)
    axes[1].set_title("Coarse (res=4)", fontsize=14)
    axes[2].set_title("Fine (res=2)", fontsize=14)

    fig.tight_layout()
    path = output_dir / "arc-linearize.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "arc-linearize.png",
            "caption": "Arc linearization: coarse and fine resolution",
        }
    )

    return {
        "title": "Arc Linearization",
        "description": (
            "Convert arcs into line segments at configurable resolution."
        ),
        "images": images,
    }
