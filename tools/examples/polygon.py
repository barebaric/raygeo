"""Generate examples for polygon construction utilities."""

__images__ = [
    {
        "stem": "polygon-circle-polygon",
        "caption": (
            "``get_circle_polygon`` approximates a circle"
            " as an n-sided polygon"
        ),
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "get_circle_polygon",
    },
    {
        "stem": "polygon-segment-swept",
        "caption": (
            "``get_segment_swept_polygon`` computes the swept area of a line "
            "segment with a given radius"
        ),
        "doc": "raygeo.geo.shape.polygon.md",
        "heading": "get_segment_swept_polygon",
    },
]

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle as CirclePatch

from raygeo.geo.shape.polygon import (
    get_circle_polygon,
    get_segment_swept_polygon,
)


def generate_examples(output_dir):
    images = []

    # ----------------------------------------------------------------
    # Figure 1: get_circle_polygon — 64-gon vs ideal circle
    # ----------------------------------------------------------------
    center = (50, 50)
    radius = 30.0
    poly = get_circle_polygon(center, radius, 32)
    poly_arr = np.array(poly)

    fig1, ax1 = plt.subplots(figsize=(6, 6))
    ax1.plot(
        *np.vstack([poly_arr, poly_arr[0:1]]).T,
        "b-",
        linewidth=2,
        label="64-gon",
    )
    ax1.add_patch(
        CirclePatch(
            center,
            radius,
            fill=False,
            edgecolor="red",
            linestyle="--",
            linewidth=1.5,
            label="Ideal circle",
        )
    )
    ax1.plot(center[0], center[1], "k+", markersize=10, label="Centre")
    ax1.set_xlim(15, 85)
    ax1.set_ylim(15, 85)
    ax1.set_aspect("equal")
    ax1.set_title("get_circle_polygon — 64-gon approximation")
    ax1.set_xlabel("X")
    ax1.set_ylabel("Y")
    ax1.legend(fontsize=9)
    ax1.grid(True, alpha=0.3)
    fig1.tight_layout()

    path1 = output_dir / "polygon-circle-polygon.png"
    fig1.savefig(path1, dpi=150)
    plt.close(fig1)
    images.append(
        {
            "path": "polygon-circle-polygon.png",
            "caption": (
                "``get_circle_polygon`` approximates a circle as an n-sided "
                "polygon.  The 64-gon (blue) closely matches the ideal circle "
                "(red dashed)."
            ),
        }
    )

    # ----------------------------------------------------------------
    # Figure 2: get_segment_swept_polygon
    # ----------------------------------------------------------------
    a = (20, 30)
    b = (80, 70)
    r = 10.0

    swept = get_segment_swept_polygon(a, b, r)

    fig2, ax2 = plt.subplots(figsize=(7, 6))

    colors = ["#4ecdc4", "#ff6b6b", "#ffd93d"]
    labels = ["Swept rect", "Start cap", "End cap"]
    for i, poly in enumerate(swept):
        arr = np.array(poly)
        ax2.fill(*np.vstack([arr, arr[0:1]]).T, alpha=0.5, color=colors[i])
        ax2.plot(
            *np.vstack([arr, arr[0:1]]).T,
            "-",
            linewidth=2,
            color=colors[i],
            label=labels[i],
        )

    ax2.plot([a[0], b[0]], [a[1], b[1]], "k--", linewidth=1.5, label="Segment")
    ax2.plot(a[0], a[1], "ko", markersize=8)
    ax2.plot(b[0], b[1], "ko", markersize=8)

    ax2.set_xlim(0, 100)
    ax2.set_ylim(0, 100)
    ax2.set_aspect("equal")
    ax2.set_title("get_segment_swept_polygon — swept area")
    ax2.set_xlabel("X")
    ax2.set_ylabel("Y")
    ax2.legend(fontsize=9)
    ax2.grid(True, alpha=0.3)
    fig2.tight_layout()

    path2 = output_dir / "polygon-segment-swept.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "polygon-segment-swept.png",
            "caption": (
                "``get_segment_swept_polygon`` returns a rectangle (the "
                "Minkowski sum of the segment with a disk of *radius*) plus "
                "two disks at the endpoints."
            ),
        }
    )

    return {"images": images}
