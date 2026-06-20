"""Generate circle intersection example images."""

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Circle as MplCircle

from raygeo.geo.shape.circle import (
    find_tangent_circle_centers,
    get_circle_circle_intersections,
    get_line_circle_intersections,
    nearest_tangent_circle_on_polyline,
)


def generate_intersections():
    fig, axes = plt.subplots(1, 2, figsize=(14, 7))

    c1, r1 = (0.0, 0.0), 5.0
    c2, r2 = (6.0, 0.0), 5.0

    circ1 = MplCircle(c1, r1, fill=False, edgecolor="steelblue", linewidth=2)
    circ2 = MplCircle(c2, r2, fill=False, edgecolor="tomato", linewidth=2)
    axes[0].add_patch(circ1)
    axes[0].add_patch(circ2)
    axes[0].plot(c1[0], c1[1], "o", color="steelblue", markersize=6)
    axes[0].plot(c2[0], c2[1], "o", color="tomato", markersize=6)

    pts = get_circle_circle_intersections(c1, r1, c2, r2)
    for pt in pts:
        axes[0].plot(pt[0], pt[1], "*", color="gold", markersize=15, zorder=5)

    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)
    axes[0].set_xlim(-7, 13)
    axes[0].set_ylim(-7, 7)
    axes[0].set_title("Circle-Circle Intersection", fontsize=14)

    circ3 = MplCircle(
        (3.0, 0.0), 5.0, fill=False, edgecolor="steelblue", linewidth=2
    )
    axes[1].add_patch(circ3)
    line_p1, line_p2 = (-4.0, 3.0), (10.0, -2.0)
    axes[1].plot(
        [line_p1[0], line_p2[0]],
        [line_p1[1], line_p2[1]],
        color="tomato",
        linewidth=2,
        label="Line segment",
    )

    inter_pts = get_line_circle_intersections(
        line_p1, line_p2, (3.0, 0.0), 5.0
    )
    for pt in inter_pts:
        axes[1].plot(pt[0], pt[1], "*", color="gold", markersize=15, zorder=5)

    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)
    axes[1].set_xlim(-6, 12)
    axes[1].set_ylim(-6, 6)
    axes[1].set_title("Line-Circle Intersection", fontsize=14)
    axes[1].legend(fontsize=11)

    fig.tight_layout()
    return fig


def generate_tangent_circles():
    """Find circles tangent to a segment through a point."""
    seg_a, seg_b = (2.0, 0.0), (10.0, 0.0)
    pass_through = (6.0, 5.0)
    radius = 3.0
    results = find_tangent_circle_centers(pass_through, seg_a, seg_b, radius)

    fig, ax = plt.subplots(figsize=(8, 8))

    ax.plot(
        [seg_a[0], seg_b[0]],
        [seg_a[1], seg_b[1]],
        color="steelblue",
        linewidth=3,
        label="Segment",
    )
    ax.plot(
        pass_through[0],
        pass_through[1],
        "o",
        color="k",
        markersize=10,
        label="Pass-through point",
    )
    colors = ["tomato", "limegreen", "gold", "mediumpurple"]
    for i, (center, tangent) in enumerate(results):
        c = colors[i % len(colors)]
        circle = MplCircle(
            center,
            radius,
            fill=False,
            edgecolor=c,
            linewidth=2,
            linestyle="--",
        )
        ax.add_patch(circle)
        ax.plot(
            center[0],
            center[1],
            "s",
            color=c,
            markersize=8,
            label=f"Centre {i + 1}" if i == 0 else None,
        )
        ax.plot(
            tangent[0],
            tangent[1],
            "*",
            color=c,
            markersize=12,
            label=f"Tangent {i + 1}" if i == 0 else None,
        )

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_xlim(-2, 14)
    ax.set_ylim(-6, 10)
    ax.set_title(
        f"Circles r={radius} tangent to segment through point", fontsize=14
    )
    handles = [
        Line2D([], [], color="steelblue", linewidth=3, label="Segment"),
        Line2D(
            [],
            [],
            marker="o",
            color="k",
            linestyle="None",
            markersize=10,
            label="Pass-through point",
        ),
    ]
    if results:
        handles.append(
            Line2D(
                [],
                [],
                marker="s",
                color=colors[0],
                linestyle="None",
                markersize=8,
                label="Center",
            )
        )
        handles.append(
            Line2D(
                [],
                [],
                marker="*",
                color=colors[0],
                linestyle="None",
                markersize=12,
                label="Tangent point",
            )
        )
    ax.legend(handles=handles, fontsize=11)
    fig.tight_layout()
    return fig


def generate_nearest_tangent():
    """Nearest tangent circle on a polyline."""
    polyline = [(2.0, 0.0), (10.0, 0.0), (10.0, 8.0)]
    point = (6.0, 6.0)
    radius = 3.0
    containment = [(0.0, -5.0), (14.0, -5.0), (14.0, 12.0), (0.0, 12.0)]

    fig, axes = plt.subplots(1, 2, figsize=(14, 7))

    for ax, from_end, title in [
        (axes[0], False, "Search from start"),
        (axes[1], True, "Search from end"),
    ]:
        result = nearest_tangent_circle_on_polyline(
            point, polyline, radius, from_end, containment
        )

        xs = [p[0] for p in polyline]
        ys = [p[1] for p in polyline]
        ax.plot(
            xs,
            ys,
            "-o",
            color="steelblue",
            lw=2.5,
            markerfacecolor="lightblue",
            markeredgecolor="steelblue",
            markersize=7,
            label="Polyline",
        )
        ax.plot(
            point[0], point[1], "o", color="k", markersize=9, label="Point"
        )

        if result:
            center, tangent, idx = result
            circ = MplCircle(
                center,
                radius,
                fill=False,
                edgecolor="tomato",
                lw=2,
                linestyle="--",
            )
            ax.add_patch(circ)
            ax.plot(
                center[0],
                center[1],
                "s",
                color="tomato",
                markersize=8,
                label="Centre",
            )
            ax.plot(
                tangent[0],
                tangent[1],
                "*",
                color="gold",
                markersize=14,
                label="Tangent",
            )
            ax.plot(
                [center[0], point[0]],
                [center[1], point[1]],
                color="gray",
                lw=1,
                ls=":",
            )
            ax.plot(
                [center[0], tangent[0]],
                [center[1], tangent[1]],
                color="gray",
                lw=1,
                ls=":",
            )
            ax.set_title(f"{title} (seg {idx})", fontsize=13)
        else:
            ax.set_title(f"{title} — no solution", fontsize=13)

        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_xlim(0, 13)
        ax.set_ylim(-2, 10)
        ax.legend(fontsize=9)

    fig.suptitle(
        f"Nearest tangent circle (r={radius}) on polyline",
        fontsize=14,
        y=1.02,
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.shape.circle.md"]
__images__ = [
    {
        "heading": "get_circle_circle_intersections",
        "caption": "Circle-circle and line-circle intersection points",
        "function": generate_intersections,
    },
    {
        "heading": "find_tangent_circle_centers",
        "caption": "Find circles tangent to a segment through a given point",
        "function": generate_tangent_circles,
    },
    {
        "heading": "nearest_tangent_circle_on_polyline",
        "caption": "Nearest tangent circle on a polyline",
        "function": generate_nearest_tangent,
    },
]
