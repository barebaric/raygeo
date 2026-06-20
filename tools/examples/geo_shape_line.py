"""Generate line intersection example images."""

import matplotlib.pyplot as plt

from raygeo.geo.shape.line import (
    get_line_closest_point,
    get_line_line_intersection,
    get_line_segment_intersection,
    get_point_line_distance,
)


def _draw_line(ax, p1, p2, color, linewidth=1.5, label=None, linestyle="-"):
    ax.plot(
        [p1[0], p2[0]],
        [p1[1], p2[1]],
        color=color,
        linewidth=linewidth,
        label=label,
        linestyle=linestyle,
    )


def generate_intersections():
    fig, axes = plt.subplots(1, 3, figsize=(20, 6))

    l1_a, l1_b = (2.0, 1.0), (10.0, 8.0)
    l2_a, l2_b = (2.0, 8.0), (10.0, 1.0)

    _draw_line(axes[0], l1_a, l1_b, "steelblue", linewidth=2.5, label="Line 1")
    _draw_line(axes[0], l2_a, l2_b, "tomato", linewidth=2.5, label="Line 2")

    for pt, clr in [
        (l1_a, "steelblue"),
        (l1_b, "steelblue"),
        (l2_a, "tomato"),
        (l2_b, "tomato"),
    ]:
        axes[0].plot(pt[0], pt[1], "o", color=clr, markersize=8, zorder=5)

    inter = get_line_line_intersection(l1_a, l1_b, l2_a, l2_b)
    if inter:
        axes[0].plot(
            inter[0], inter[1], "*", color="gold", markersize=18, zorder=6
        )
        axes[0].annotate(
            f"({inter[0]:.1f}, {inter[1]:.1f})",
            inter,
            xytext=(5, 8),
            textcoords="offset points",
            fontsize=11,
            color="gold",
            fontweight="bold",
        )
    axes[0].set_title("Infinite line intersection", fontsize=14)
    axes[0].legend(fontsize=11)

    s1_a, s1_b = (2.0, 1.0), (8.0, 7.0)
    s2_a, s2_b = (2.0, 6.0), (9.0, 2.0)

    _draw_line(
        axes[1], s1_a, s1_b, "steelblue", linewidth=3, label="Segment 1"
    )
    _draw_line(axes[1], s2_a, s2_b, "tomato", linewidth=3, label="Segment 2")

    seg_inter = get_line_segment_intersection(s1_a, s1_b, s2_a, s2_b)
    if seg_inter:
        axes[1].plot(
            seg_inter[0],
            seg_inter[1],
            "*",
            color="gold",
            markersize=18,
            zorder=6,
        )
        axes[1].annotate(
            f"({seg_inter[0]:.1f}, {seg_inter[1]:.1f})",
            seg_inter,
            xytext=(5, 8),
            textcoords="offset points",
            fontsize=11,
            color="gold",
            fontweight="bold",
        )
    axes[1].set_title("Segment intersection (hit)", fontsize=14)
    axes[1].legend(fontsize=11)

    s3_a, s3_b = (2.0, 1.0), (5.0, 4.0)
    s4_a, s4_b = (7.0, 2.0), (10.0, 5.0)

    _draw_line(
        axes[2], s3_a, s3_b, "steelblue", linewidth=3, label="Segment 1"
    )
    _draw_line(axes[2], s4_a, s4_b, "tomato", linewidth=3, label="Segment 2")

    seg_inter2 = get_line_segment_intersection(s3_a, s3_b, s4_a, s4_b)
    if seg_inter2 is None:
        axes[2].text(
            6,
            3,
            "No intersection",
            fontsize=13,
            color="gray",
            ha="center",
            fontstyle="italic",
        )
    axes[2].set_title("Segment intersection (miss)", fontsize=14)
    axes[2].legend(fontsize=11)

    for i in range(3):
        axes[i].set_aspect("equal")
        axes[i].grid(True, alpha=0.3)
        axes[i].set_xlim(0, 11)
        axes[i].set_ylim(0, 9)

    fig.tight_layout()
    return fig


def generate_point_distance():
    fig2, ax2 = plt.subplots(figsize=(7, 7))

    line_pt1, line_pt2 = (2.0, 1.0), (10.0, 7.0)
    test_point = (4.0, 6.0)
    dist = get_point_line_distance(test_point, line_pt1, line_pt2)

    closest = get_line_closest_point(
        line_pt1, line_pt2, test_point[0], test_point[1]
    )

    _draw_line(ax2, line_pt1, line_pt2, "steelblue", linewidth=3, label="Line")
    ax2.plot(
        test_point[0],
        test_point[1],
        "o",
        color="tomato",
        markersize=10,
        zorder=5,
        label="Point",
    )
    ax2.plot(
        closest[0],
        closest[1],
        "o",
        color="forestgreen",
        markersize=8,
        zorder=5,
        label="Closest",
    )
    ax2.plot(
        [test_point[0], closest[0]],
        [test_point[1], closest[1]],
        color="forestgreen",
        linewidth=2,
        linestyle="--",
        label=f"Distance = {dist:.2f}",
    )

    ax2.set_aspect("equal")
    ax2.grid(True, alpha=0.3)
    ax2.set_xlim(0, 12)
    ax2.set_ylim(0, 9)
    ax2.set_title("Point-Line Distance", fontsize=14)
    ax2.legend(fontsize=11)

    fig2.tight_layout()
    return fig2


__docs_target__ = ["raygeo.geo.shape.line.md"]
__images__ = [
    {
        "heading": "get_line_line_intersection",
        "caption": "Line-line and segment intersection",
        "function": generate_intersections,
    },
    {
        "heading": "get_point_line_distance",
        "caption": "Perpendicular distance from a point to a line",
        "function": generate_point_distance,
    },
]
