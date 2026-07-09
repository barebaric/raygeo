"""Visualisation for geo/algo/narrow — narrow-passage detection."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.narrow import find_narrow_passages


def _plot_regions(ax, regions):
    for poly in regions:
        if len(poly) >= 3:
            xs = [p[0] for p in poly] + [poly[0][0]]
            ys = [p[1] for p in poly] + [poly[0][1]]
            ax.fill(xs, ys, color="crimson", alpha=0.35)
            ax.plot(xs, ys, "-", color="crimson", linewidth=1.5)


def generate_threshold_comparison():
    """Effect of max_width on a single pocket.

    Left: max_width=8 — the 8 mm channel is at the threshold edge.
    Right: max_width=20 — both the channel and parts of the rooms near
    corners are flagged, showing how the threshold controls sensitivity.
    """
    boundary = [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 21.0),
        (60.0, 21.0),
        (60.0, 0.0),
        (100.0, 0.0),
        (100.0, 50.0),
        (60.0, 50.0),
        (60.0, 29.0),
        (40.0, 29.0),
        (40.0, 50.0),
        (0.0, 50.0),
    ]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    for ax, mw in [(ax1, 8.0), (ax2, 20.0)]:
        regions = find_narrow_passages(boundary, holes=None, max_width=mw)

        bx = [p[0] for p in boundary] + [boundary[0][0]]
        by = [p[1] for p in boundary] + [boundary[0][1]]
        ax.plot(bx, by, "k-", linewidth=1.5)
        ax.fill(bx, by, facecolor="#f5f5f5", alpha=0.5)
        _plot_regions(ax, regions)

        ax.set_title(f"max_width = {mw:.0f}  →  {len(regions)} passage(s)")
        ax.set_xlabel("X (mm)")
        ax.set_ylabel("Y (mm)")
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)

    fig.suptitle("find_narrow_passages — Threshold sensitivity", fontsize=12)
    fig.tight_layout()
    return fig


def generate_with_island():
    """Pocket with an island: passages form in the necks."""
    boundary = [(0.0, 0.0), (80.0, 0.0), (80.0, 50.0), (0.0, 50.0)]
    islands = [[(30.0, 20.0), (50.0, 20.0), (50.0, 30.0), (30.0, 30.0)]]
    regions = find_narrow_passages(boundary, holes=islands, max_width=24.0)

    fig, ax = plt.subplots(figsize=(8, 5))

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.8, label="Pocket boundary")
    ax.fill(bx, by, facecolor="#f5f5f5", alpha=0.5)

    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(ix, iy, color="dimgray", alpha=0.45)
        ax.plot(ix, iy, color="dimgray", linewidth=1.2, label="Island")

    _plot_regions(ax, regions)

    ax.plot([], [], "-", color="crimson", linewidth=3, label="Narrow passage")
    ax.set_title(
        f"find_narrow_passages — Pocket with island (max_width=24)\n"
        f"{len(regions)} passage(s)"
    )
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.legend(fontsize=9, loc="upper right")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_triangular_islands():
    """Two triangular islands pointing at each other with a 4 mm gap.

    The morphological opening (max_width=6, r=3) expands the islands
    by r and shrinks the pocket by r. The expanded islands overlap at
    the tips, creating a barrier that splits the inset. After dilation
    the narrow passage is the region between the two islands that
    could not be restored.
    """
    boundary = [(0.0, 0.0), (80.0, 0.0), (80.0, 50.0), (0.0, 50.0)]
    islands = [
        [(5.0, 5.0), (5.0, 45.0), (37.0, 25.0)],
        [(75.0, 5.0), (75.0, 45.0), (41.0, 25.0)],
    ]
    regions = find_narrow_passages(boundary, holes=islands, max_width=6.0)

    fig, ax = plt.subplots(figsize=(8, 5))

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.8, label="Pocket boundary")
    ax.fill(bx, by, facecolor="#f5f5f5", alpha=0.5)

    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(ix, iy, color="dimgray", alpha=0.45)
        ax.plot(ix, iy, color="dimgray", linewidth=1.2, label="Island")

    # Annotate the gap distance
    tip_left = islands[0][2]
    tip_right = islands[1][2]
    gap = tip_right[0] - tip_left[0]
    ax.annotate(
        "",
        xy=(tip_left[0], tip_left[1] - 2),
        xytext=(tip_right[0], tip_right[1] - 2),
        arrowprops=dict(arrowstyle="<->", color="green", lw=1.8),
    )
    ax.text(
        (tip_left[0] + tip_right[0]) / 2,
        tip_left[1] - 4.5,
        f"gap = {gap:.0f} mm",
        ha="center",
        fontsize=10,
        color="green",
        fontweight="bold",
    )

    _plot_regions(ax, regions)

    ax.plot([], [], "-", color="crimson", linewidth=3, label="Narrow passage")
    ax.set_title(
        f"find_narrow_passages — Triangular islands (max_width=6)\n"
        f"gap={gap:.0f} mm  →  {len(regions)} passage(s)"
    )
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.legend(fontsize=9, loc="upper right")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.narrow.md"]
__images__ = [
    {
        "heading": "find_narrow_passages",
        "caption": (
            "Threshold sensitivity: at max_width=8 barely registers;"
            " at max_width=20 more qualifies"
        ),
        "function": generate_threshold_comparison,
    },
    {
        "heading": "find_narrow_passages",
        "caption": (
            "Pocket with a central island: narrow passages (crimson)"
            " form in the necks around the island"
        ),
        "function": generate_with_island,
    },
    {
        "heading": "find_narrow_passages",
        "caption": (
            "Two triangular islands (4 mm gap); morphological opening"
            " creates narrow passage (crimson)"
        ),
        "function": generate_triangular_islands,
    },
]
