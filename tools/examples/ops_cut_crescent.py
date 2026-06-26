"""Visualise the sweep-line disk-increment area (cut_area)."""

import math

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.shape.polygon import (
    get_circle_polygon,
    get_polygons_group_difference,
)
from raygeo.ops.cut import cut_area


def generate_disk_increment():
    """Disk increment when stepping, with and without fragments."""
    c1 = (4.0, 5.0)
    c2 = (8.0, 5.0)
    r = 3.0

    # Disk polygons for visualisation.
    disk1 = get_circle_polygon(c1, r, 48)
    disk2 = get_circle_polygon(c2, r, 48)
    crescent = get_polygons_group_difference([disk2], [disk1])

    # Compute increment areas.
    area_no_frags, _ = cut_area(c1, c2, r, [], [])
    cleared_square = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]
    area_with_frags, _ = cut_area(c1, c2, r, cleared_square, [])

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    for ax, title, area, fragments in [
        (ax1, "No cleared fragments", area_no_frags, None),
        (
            ax2,
            "With cleared square",
            area_with_frags,
            [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        ),
    ]:
        # Draw crescent.
        if crescent:
            for poly in crescent:
                arr = np.array(poly)
                ax.fill(
                    arr[:, 0], arr[:, 1], "tomato", alpha=0.5, label="Crescent"
                )

        # Draw disks.
        for centre, style, label in [
            (c1, "b--", "Disk(c1)"),
            (c2, "b-", "Disk(c2)"),
        ]:
            theta = np.linspace(0, 2 * math.pi, 100)
            cx, cy = centre
            ax.plot(
                cx + r * np.cos(theta),
                cy + r * np.sin(theta),
                style,
                linewidth=1.5,
                label=label,
            )

        # Draw cleared fragments.
        if fragments:
            f_arr = np.array(fragments + [fragments[0]])
            ax.fill(
                f_arr[:, 0],
                f_arr[:, 1],
                "lightgray",
                alpha=0.6,
                label="Cleared",
            )
            ax.plot(f_arr[:, 0], f_arr[:, 1], "gray", linewidth=1)

        # Marker for centres.
        for centre, style in [(c1, "bs"), (c2, "bo")]:
            ax.plot(*centre, style, markersize=4)

        ax.set_aspect("equal")
        ax.set_xlim(0, 12)
        ax.set_ylim(0, 10)
        ax.set_title(f"{title}\ncut_area = {area:.2f} mm²")
        ax.legend(fontsize=7, loc="upper right")
        ax.grid(True, alpha=0.3)

    fig.suptitle("cut_area — Disk Increment Area", fontsize=13)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.cut.md"]
__images__ = [
    {
        "heading": "cut_area",
        "caption": (
            "Disk increment (in red) produced by stepping a disk from"
            " C1 to C2."
            " Left panel shows the full increment; right panel shows"
            " the reduction when a cleared fragment (gray) occupies part"
            " of the increment."
        ),
        "function": generate_disk_increment,
    },
]
