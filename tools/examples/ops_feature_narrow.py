"""Visualisation for ops/feature/narrow — narrow-passage classification."""

import matplotlib.pyplot as plt

from raygeo.ops.feature.narrow import analyze_pocket

CLASS_COLORS = {
    "slot": "mediumblue",
    "narrow": "darkorange",
    "unreachable": "crimson",
}


def _plot_pocket(ax, boundary, islands):
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "-", color="k", linewidth=1.5)
    ax.fill(bx, by, facecolor="#f0f0f0", alpha=0.3)
    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(ix, iy, color="dimgray", alpha=0.5)
        ax.plot(ix, iy, color="dimgray", linewidth=1.5)


def _plot_regions(ax, regions):
    for poly, cls, min_w, _entry in regions:
        color = CLASS_COLORS.get(cls, "gray")
        xs = [p[0] for p in poly] + [poly[0][0]]
        ys = [p[1] for p in poly] + [poly[0][1]]
        ax.fill(xs, ys, color=color, alpha=0.4)
        ax.plot(xs, ys, "-", color=color, linewidth=2.5)


def generate_classification_by_width():
    """A pocket with an L-shaped island creates passages of three widths."""

    tool_radius = 3.0
    tolerance = 0.5

    # Outer pocket
    pocket = [(0, 0), (100, 0), (100, 60), (0, 60)]

    # Staircase island whose right side creates gaps of different widths
    # to the right pocket wall (x=100):
    #   y=10..18 → x=97  → gap =  3 → Unreachable
    #   y=32..40 → x=93  → gap =  7 → Narrow
    # Top side (y=55) → gap to pocket top (y=60) = 5 → Slot
    island = [
        (25, 10),
        (97, 10),
        (97, 20),
        (86, 20),
        (86, 32),
        (93.5, 32),
        (93.5, 40),
        (86, 40),
        (86, 50),
        (88, 50),
        (88, 53),
        (25, 53),
    ]

    regions = analyze_pocket(
        pocket,
        holes=[island],
        tool_radius=tool_radius,
        tolerance=tolerance,
    )

    fig, ax = plt.subplots(figsize=(8, 6))
    _plot_pocket(ax, pocket, [island])
    _plot_regions(ax, regions)

    # Annotation for each classified passage
    for poly, cls, min_w, _entry in regions:
        cx = sum(p[0] for p in poly) / len(poly)
        cy = sum(p[1] for p in poly) / len(poly)
        color = CLASS_COLORS.get(cls, "gray")
        ax.text(
            cx,
            cy,
            f"{cls}\n{min_w:.1f} mm",
            ha="center",
            va="center",
            fontsize=10,
            fontweight="bold",
            color=color,
        )

    # Legend
    for label, color in CLASS_COLORS.items():
        ax.plot([], [], "-", color=color, linewidth=2.5, label=label)

    ax.set_title(
        "analyze_pocket — passages of varying width in one pocket\n"
        f"(tool_radius={tool_radius}, tolerance={tolerance})"
    )
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9, loc="upper right")
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.feature.narrow.md"]
__images__ = [
    {
        "heading": "analyze_pocket",
        "caption": (
            "A pocket with staircase island produces passages of"
            " three widths, showing all classification levels"
        ),
        "function": generate_classification_by_width,
    },
]
