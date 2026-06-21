"""Generate line intersection example images."""

import matplotlib.pyplot as plt

from raygeo.geo.shape.line import (
    does_line_cross_polygon,
    get_line_closest_point,
    get_line_line_intersection,
    get_line_segment_intersection,
    get_point_line_distance,
    get_segment_segment_distance,
    interpolated_segment_3d,
    longest_line_through_point,
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


def generate_interpolated_segment():
    """Interpolated 3D segment."""
    from_pt, to_pt = (2.0, 2.0), (10.0, 8.0)
    z, n = 5.0, 8
    pts = interpolated_segment_3d(
        from_pt[0], from_pt[1], to_pt[0], to_pt[1], z, n
    )

    fig, ax = plt.subplots(figsize=(7, 7))
    ax.plot(
        [from_pt[0], to_pt[0]],
        [from_pt[1], to_pt[1]],
        color="steelblue",
        lw=2,
        label="Segment (XY)",
    )
    ax.plot(
        [p[0] for p in pts],
        [p[1] for p in pts],
        "o",
        color="tomato",
        markersize=8,
        label=f"Interpolated ({n} pts, Z={z})",
    )
    for i, p in enumerate(pts):
        ax.annotate(
            str(i),
            (p[0], p[1]),
            xytext=(4, 4),
            textcoords="offset points",
            fontsize=8,
            color="tomato",
        )
    ax.plot(from_pt[0], from_pt[1], "o", color="k", markersize=8, label="From")
    ax.plot(to_pt[0], to_pt[1], "s", color="k", markersize=8, label="To")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_xlim(0, 12)
    ax.set_ylim(0, 10)
    ax.set_title(f"Interpolated segment 3D (n={n}, z={z})", fontsize=14)
    ax.legend(fontsize=10)
    fig.tight_layout()
    return fig


def generate_line_crosses_polygon():
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

    polygon = [(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)]
    poly_xs = [p[0] for p in polygon] + [polygon[0][0]]
    poly_ys = [p[1] for p in polygon] + [polygon[0][1]]

    crossing_a, crossing_b = (0.0, 5.0), (10.0, 5.0)
    crosses = does_line_cross_polygon(crossing_a, crossing_b, polygon)

    ax1.fill(
        poly_xs,
        poly_ys,
        facecolor="lightblue",
        alpha=0.4,
        edgecolor="steelblue",
        linewidth=2,
        label="Polygon",
    )
    ax1.plot(
        [crossing_a[0], crossing_b[0]],
        [crossing_a[1], crossing_b[1]],
        "tomato",
        linewidth=2.5,
        label=f"Crosses = {crosses}",
    )
    ax1.plot(crossing_a[0], crossing_a[1], "o", color="tomato", markersize=6)
    ax1.plot(crossing_b[0], crossing_b[1], "o", color="tomato", markersize=6)
    ax1.set_title("Line segment crosses polygon", fontsize=13)

    touching_a, touching_b = (2.0, 2.0), (0.0, 0.0)
    touches = does_line_cross_polygon(touching_a, touching_b, polygon)

    ax2.fill(
        poly_xs,
        poly_ys,
        facecolor="lightblue",
        alpha=0.4,
        edgecolor="steelblue",
        linewidth=2,
        label="Polygon",
    )
    ax2.plot(
        [touching_a[0], touching_b[0]],
        [touching_a[1], touching_b[1]],
        "gray",
        linewidth=2.5,
        linestyle="--",
        label=f"Crosses = {touches}",
    )
    ax2.plot(touching_a[0], touching_a[1], "o", color="gray", markersize=6)
    ax2.plot(touching_b[0], touching_b[1], "o", color="gray", markersize=6)
    ax2.set_title("Line segment touches vertex (no cross)", fontsize=13)

    ax1.set_xlim(-1, 11)
    ax1.set_ylim(0, 10)
    ax2.set_xlim(-1, 11)
    ax2.set_ylim(-1, 10)
    for ax in (ax1, ax2):
        ax.set_aspect("equal")
        ax.legend(fontsize=10)
        ax.grid(True, alpha=0.2)

    fig.tight_layout()
    return fig


def generate_longest_line():
    """Longest axis-aligned line through a point within a bounding box."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

    bbox = (0.0, 0.0, 10.0, 6.0)
    pt = (4.0, 3.0)
    (sx, sy), (ex, ey) = longest_line_through_point(pt, bbox)

    ax1.plot(
        [bbox[0], bbox[2], bbox[2], bbox[0], bbox[0]],
        [bbox[1], bbox[1], bbox[3], bbox[3], bbox[1]],
        "k-",
        linewidth=2,
        label="Bounding box",
    )
    ax1.plot(
        pt[0],
        pt[1],
        "o",
        color="tomato",
        markersize=10,
        zorder=5,
        label="Point",
    )
    ax1.plot(
        [sx, ex], [sy, ey], "steelblue", linewidth=3, label="Longest line"
    )
    ax1.set_title("Wider bbox → horizontal line", fontsize=13)
    ax1.set_aspect("equal")
    ax1.legend(fontsize=10)
    ax1.grid(True, alpha=0.3)
    ax1.set_xlim(-1, 11)
    ax1.set_ylim(-1, 7)

    bbox2 = (0.0, 0.0, 6.0, 10.0)
    pt2 = (3.0, 5.0)
    (sx2, sy2), (ex2, ey2) = longest_line_through_point(pt2, bbox2)

    ax2.plot(
        [bbox2[0], bbox2[2], bbox2[2], bbox2[0], bbox2[0]],
        [bbox2[1], bbox2[1], bbox2[3], bbox2[3], bbox2[1]],
        "k-",
        linewidth=2,
        label="Bounding box",
    )
    ax2.plot(
        pt2[0],
        pt2[1],
        "o",
        color="tomato",
        markersize=10,
        zorder=5,
        label="Point",
    )
    ax2.plot(
        [sx2, ex2], [sy2, ey2], "steelblue", linewidth=3, label="Longest line"
    )
    ax2.set_title("Taller bbox → vertical line", fontsize=13)
    ax2.set_aspect("equal")
    ax2.legend(fontsize=10)
    ax2.grid(True, alpha=0.3)
    ax2.set_xlim(-1, 7)
    ax2.set_ylim(-1, 11)

    fig.tight_layout()
    return fig


def generate_segment_distance():
    """Minimum distance between two line segments."""
    fig, (ax1, ax2, ax3) = plt.subplots(1, 3, figsize=(18, 5))

    # Crossing segments — distance 0
    ax1.plot([0, 10], [0, 10], "steelblue", linewidth=2.5, label="Seg 1")
    ax1.plot([0, 10], [10, 0], "tomato", linewidth=2.5, label="Seg 2")
    ax1.plot(5, 5, "*", color="gold", markersize=16, zorder=6)
    ax1.annotate(
        "intersect → d=0",
        (5, 5),
        xytext=(5, 8),
        textcoords="offset points",
        fontsize=11,
        ha="center",
    )
    ax1.set_title("Crossing segments (d=0)", fontsize=13)
    ax1.set_aspect("equal")
    ax1.legend(fontsize=10)
    ax1.grid(True, alpha=0.3)
    ax1.set_xlim(-1, 11)
    ax1.set_ylim(-1, 11)

    # Parallel separated segments
    d = get_segment_segment_distance(
        (0.0, 1.0), (8.0, 1.0), (2.0, 5.0), (10.0, 5.0)
    )
    ax2.plot([0, 8], [1, 1], "steelblue", linewidth=2.5, label="Seg 1")
    ax2.plot([2, 10], [5, 5], "tomato", linewidth=2.5, label="Seg 2")
    ax2.annotate(
        "",
        (4, 1),
        (4, 5),
        arrowprops=dict(arrowstyle="<->", color="forestgreen", lw=2),
    )
    ax2.text(4.3, 2.8, f"d={d:.1f}", fontsize=12, color="forestgreen")
    ax2.set_title("Parallel separated segments", fontsize=13)
    ax2.set_aspect("equal")
    ax2.legend(fontsize=10)
    ax2.grid(True, alpha=0.3)
    ax2.set_xlim(-1, 11)
    ax2.set_ylim(-1, 7)

    # Skew (non-parallel, non-intersecting) segments
    # Seg1: (0,0)→(10,0) along bottom.  Seg2: (1,4)→(9,1) slopes down
    # toward seg1's right end.  Closest approach: (9,0)↔(9,1) = 1.0
    d3 = get_segment_segment_distance(
        (0.0, 0.0), (10.0, 0.0), (1.0, 4.0), (9.0, 1.0)
    )
    ax3.plot([0, 10], [0, 0], "steelblue", linewidth=2.5, label="Seg 1")
    ax3.plot([1, 9], [4, 1], "tomato", linewidth=2.5, label="Seg 2")
    ax3.annotate(
        "",
        (9, 0),
        (9, 1),
        arrowprops=dict(arrowstyle="<->", color="forestgreen", lw=2),
    )
    ax3.text(9.3, 0.4, f"d={d3:.1f}", fontsize=12, color="forestgreen")
    ax3.set_title("Skew segments (non-parallel)", fontsize=13)
    ax3.set_aspect("equal")
    ax3.legend(fontsize=10)
    ax3.grid(True, alpha=0.3)
    ax3.set_xlim(-1, 11)
    ax3.set_ylim(-1, 7)

    fig.tight_layout()
    return fig


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
    {
        "heading": "interpolated_segment_3d",
        "caption": "Linearly interpolated 3D points along a 2D segment",
        "function": generate_interpolated_segment,
    },
    {
        "heading": "does_line_cross_polygon",
        "caption": (
            "Check whether a line segment crosses the interior of a polygon."
            " Left: crossing segment (red). Right: segment that only touches"
            " the boundary (gray, no cross)."
        ),
        "function": generate_line_crosses_polygon,
    },
    {
        "heading": "longest_line_through_point",
        "caption": (
            "Find the longest axis-aligned line through a point within a"
            " bounding box. Left: wider box gives a horizontal line."
            " Right: taller box gives a vertical line."
        ),
        "function": generate_longest_line,
    },
    {
        "heading": "get_segment_segment_distance",
        "caption": (
            "Minimum Euclidean distance between two line segments."
            " Left: crossing segments (distance 0)."
            " Centre: parallel separated segments."
            " Right: skew (non-parallel) segments."
        ),
        "function": generate_segment_distance,
    },
]
