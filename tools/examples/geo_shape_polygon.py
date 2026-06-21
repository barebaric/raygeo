"""Generate examples for polygon operations (construction, boolean, offset)."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle as CirclePatch

from raygeo.geo.shape.polygon import (
    JoinStyle,
    apply_minimum_curvature,
    clean_polygon,
    does_path_sweep_intersect_polygon,
    get_circle_polygon,
    get_polygon_centroid,
    get_polygon_convex_hull,
    get_polygon_group_bounds,
    get_polygons_closest_point,
    get_polygons_difference,
    get_polygons_intersection,
    get_polygons_union,
    get_polyline_closest_point,
    get_segment_swept_polygon,
    offset_polygon,
    trim_polyline_at,
)
from raygeo.geo.types import Polygon
from tools.plot import plot_polygon


def _make_circle(r, n, ox=0.0, oy=0.0):
    return [
        (
            ox + r * math.cos(2 * math.pi * i / n),
            oy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _make_square(r, ox=0.0, oy=0.0):
    return [
        (ox - r, oy - r),
        (ox + r, oy - r),
        (ox + r, oy + r),
        (ox - r, oy + r),
    ]


def _plot_boolean(a, b, result, title):
    fig, ax = plt.subplots(figsize=(7, 7))
    plot_polygon(ax, a, "steelblue", "A")
    plot_polygon(ax, b, "tomato", "B")
    if result:
        plot_polygon(ax, result[0], "limegreen", "Result", linewidth=2.5)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend()
    ax.set_title(title)
    fig.tight_layout()
    return fig


def generate_circle_polygon():
    """Circle polygon."""
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
    return fig1


def generate_segment_swept():
    """Segment swept."""
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
    return fig2


def generate_path_sweep_intersect():
    """Path sweep intersect."""
    path = [(10, 30), (40, 30), (60, 60)]
    radius = 12.0
    obstacles: list[Polygon] = [
        [(35.0, 10.0), (50.0, 10.0), (50.0, 25.0), (35.0, 25.0)],
    ]
    result = does_path_sweep_intersect_polygon(path, radius, obstacles)

    fig, ax = plt.subplots(figsize=(7, 6))

    path_arr = np.array(path)
    ax.plot(
        path_arr[:, 0],
        path_arr[:, 1],
        "-o",
        color="k",
        lw=2,
        ms=6,
        label="Path",
    )

    for i, (a, b) in enumerate(zip(path, path[1:])):
        swept = get_segment_swept_polygon(a, b, radius)
        for poly in swept:
            arr = np.array(poly)
            ax.fill(*np.vstack([arr, arr[0:1]]).T, alpha=0.2, color="#4ecdc4")

    for obs in obstacles:
        plot_polygon(ax, obs, "tomato", None, linewidth=2.5)
        ax.fill(*np.array(obs + obs[:1]).T, alpha=0.15, color="tomato")

    status = "intersects" if result else "does NOT intersect"
    ax.set_title(f"Path sweep ({status})", fontsize=13)
    ax.set_aspect("equal")
    ax.set_xlim(0, 90)
    ax.set_ylim(0, 80)
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_boolean_union():
    """Boolean union."""
    n_seg = 64
    union_a = _make_circle(10.0, n_seg)
    union_b = [(-4.0, 0.0), (12.0, 0.0), (12.0, 8.0), (-4.0, 8.0)]
    union_result = get_polygons_union([union_a, union_b])
    return _plot_boolean(union_a, union_b, union_result, "Union")


def generate_boolean_intersection():
    """Boolean intersection."""
    n_seg = 64
    union_a = _make_circle(10.0, n_seg)
    union_b = [(-4.0, 0.0), (12.0, 0.0), (12.0, 8.0), (-4.0, 8.0)]
    inter_result = get_polygons_intersection(union_a, union_b)
    return _plot_boolean(union_a, union_b, inter_result, "Intersection")


def generate_boolean_difference():
    """Boolean difference."""
    n_seg = 64
    union_a = _make_circle(10.0, n_seg)
    union_b = [(-4.0, 0.0), (12.0, 0.0), (12.0, 8.0), (-4.0, 8.0)]
    diff_result = get_polygons_difference(union_a, union_b)
    return _plot_boolean(union_a, union_b, diff_result, "Difference")


def generate_offset():
    """Polygon offset."""

    triangle = [(0.0, 0.0), (20.0, 0.0), (10.0, 18.0)]
    styles = [
        (JoinStyle.Miter, "Miter"),
        (JoinStyle.Round, "Round"),
        (JoinStyle.Square, "Square"),
    ]
    style_colors = ["limegreen", "tomato", "dodgerblue"]

    fig6, axes = plt.subplots(1, 3, figsize=(14, 4.5))
    for ax, (style_key, style_label), color in zip(axes, styles, style_colors):
        plot_polygon(ax, triangle, "steelblue", "Original", linewidth=2)
        result = offset_polygon(triangle, 2.0, join_style=style_key)
        for poly in result:
            plot_polygon(ax, poly, color, f"{style_label}", linewidth=2.5)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)
        ax.set_title(f"{style_label} join", fontsize=11, fontweight="bold")
    fig6.tight_layout()
    return fig6


def generate_min_curvature():
    """Min curvature."""
    fig7, ax7 = plt.subplots(figsize=(7, 7))
    sharp = [(0, 0), (20, 0), (10, 18)]
    plot_polygon(ax7, sharp, "steelblue", "Original", linewidth=2)
    filleted = apply_minimum_curvature(sharp, 2.0)
    for poly in filleted:
        plot_polygon(ax7, poly, "tomato", "Filleted (r_min=2)", linewidth=2.5)
    ax7.set_aspect("equal")
    ax7.grid(True, alpha=0.3)
    ax7.legend(fontsize=10)
    ax7.set_title("apply_minimum_curvature", fontsize=11, fontweight="bold")
    fig7.tight_layout()
    return fig7


def generate_clean_polygon():
    """Polygon cleaning."""
    noisy = [
        (0, 0),
        (10, 0),
        (10, 0.001),
        (10.001, 0),
        (20, 0),
        (20, 10),
        (19.999, 10),
        (20, 20),
        (10, 20),
        (0, 20),
    ]
    cleaned = clean_polygon(noisy, tolerance=0.01)

    fig8, (ax8a, ax8b) = plt.subplots(1, 2, figsize=(10, 5))
    for ax, pts, title in [
        (ax8a, noisy, "Original (duplicates)"),
        (ax8b, cleaned, "Cleaned"),
    ]:
        arr = np.array(pts)
        ax.plot(*np.vstack([arr, arr[0:1]]).T, "b-", linewidth=2)
        ax.plot(arr[:, 0], arr[:, 1], "ro", markersize=4, label="Vertices")
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)
        ax.set_title(title)
    fig8.tight_layout()
    return fig8


def generate_centroid():
    """Polygon centroid."""
    base = _make_circle(10.0, 64, ox=0.0, oy=0.0)
    poly = list(base)
    for i in range(20, 44):
        t = (i - 20) / 24
        weight = 0.5 * (1 - math.cos(2 * math.pi * t))
        r = 10.0 - 3.0 * weight
        angle = 2 * math.pi * i / 64
        poly[i] = (r * math.cos(angle), r * math.sin(angle))
    cx, cy = get_polygon_centroid(poly)
    fig9, ax9 = plt.subplots(figsize=(6, 6))
    arr = np.array(poly)
    ax9.fill(*arr.T, alpha=0.15, color="steelblue")
    ax9.plot(*np.vstack([arr, arr[0:1]]).T, "b-", linewidth=2, label="Polygon")
    ax9.plot(
        cx,
        cy,
        "o",
        color="limegreen",
        markersize=10,
        label=f"Centroid ({cx:.2f}, {cy:.2f})",
    )
    ax9.plot(cx, cy, "k+", markersize=8)
    ax9.set_aspect("equal")
    ax9.grid(True, alpha=0.3)
    ax9.legend(fontsize=9)
    ax9.set_title("get_polygon_centroid")
    fig9.tight_layout()
    return fig9


def generate_convex_hull():
    """Polygon convex hull."""
    star = [
        (10, 0),
        (13, 7),
        (20, 7),
        (14, 12),
        (16, 20),
        (10, 15),
        (4, 20),
        (6, 12),
        (0, 7),
        (7, 7),
    ]
    hull = get_polygon_convex_hull(star)
    fig10, ax10 = plt.subplots(figsize=(6, 6))
    s_arr = np.array(star)
    ax10.fill(*s_arr.T, alpha=0.1, color="steelblue")
    ax10.plot(
        *np.vstack([s_arr, s_arr[0:1]]).T, "b-", linewidth=2, label="Original"
    )
    ax10.plot(s_arr[:, 0], s_arr[:, 1], "bo", markersize=4)
    h_arr = np.array(hull)
    ax10.plot(
        *np.vstack([h_arr, h_arr[0:1]]).T,
        "r-",
        linewidth=2.5,
        label="Convex Hull",
    )
    ax10.fill(*h_arr.T, alpha=0.2, color="tomato")
    ax10.set_aspect("equal")
    ax10.grid(True, alpha=0.3)
    ax10.legend(fontsize=9)
    ax10.set_title("get_polygon_convex_hull")
    fig10.tight_layout()
    return fig10


def generate_group_bounds():
    """Polygon group bounds."""
    polys = [
        _make_circle(4.0, 32, ox=4, oy=4),
        _make_square(5.0, ox=14, oy=10),
        _make_circle(3.0, 32, ox=8, oy=16),
    ]
    x_min, y_min, x_max, y_max = get_polygon_group_bounds(polys)
    fig11, ax11 = plt.subplots(figsize=(7, 7))
    colors = ["steelblue", "tomato", "limegreen"]
    for poly, color in zip(polys, colors):
        arr = np.array(poly)
        ax11.fill(*arr.T, alpha=0.2, color=color)
        ax11.plot(*np.vstack([arr, arr[0:1]]).T, "-", linewidth=2, color=color)
    rect = np.array(
        [
            [x_min, y_min],
            [x_max, y_min],
            [x_max, y_max],
            [x_min, y_max],
        ]
    )
    ax11.plot(
        *np.vstack([rect, rect[0:1]]).T,
        "r--",
        linewidth=2.5,
        label="Group bounds",
    )
    ax11.set_aspect("equal")
    ax11.grid(True, alpha=0.3)
    ax11.legend(fontsize=9)
    ax11.set_title("get_polygon_group_bounds")
    fig11.tight_layout()
    return fig11


def generate_closest_point():
    """Closest point on multiple polygons."""
    polys = [
        [(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)],
        [(12.0, 2.0), (18.0, 2.0), (18.0, 8.0), (12.0, 8.0)],
    ]
    query = (10.0, 15.0)

    fig, ax = plt.subplots(figsize=(8, 8))
    colors = ["steelblue", "tomato"]
    for poly, color in zip(polys, colors):
        arr = np.array(poly)
        ax.fill(*arr.T, alpha=0.15, color=color)
        ax.plot(*np.vstack([arr, arr[0:1]]).T, "-", linewidth=2, color=color)

    ax.plot(query[0], query[1], "o", color="k", markersize=10, label="Query")

    result = get_polygons_closest_point(polys, query[0], query[1])
    if result is not None:
        pi, t, pt, d2 = result
        ax.plot(
            pt[0],
            pt[1],
            "*",
            color="gold",
            markersize=16,
            label=f"Closest (poly {pi})",
        )
        ax.plot(
            [query[0], pt[0]], [query[1], pt[1]], color="gray", lw=1.5, ls="--"
        )
        ax.set_title(
            f"Closest point on polygon {pi}, d²={d2:.2f}", fontsize=13
        )
    else:
        ax.set_title("No closest point found", fontsize=13)

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_xlim(-1, 22)
    ax.set_ylim(-1, 18)
    ax.legend(fontsize=11)
    fig.tight_layout()
    return fig


def generate_polyline_closest_point():
    """Show closest point on an open polyline."""
    polyline = [
        (2.0, 12.0),
        (5.0, 14.0),
        (8.0, 10.0),
        (11.0, 13.0),
        (14.0, 10.0),
        (17.0, 14.0),
    ]
    queries = [
        (4.0, 12.5),
        (10.0, 11.0),
        (15.0, 13.0),
    ]

    fig, ax = plt.subplots(figsize=(8, 6))
    arr = np.array(polyline)
    ax.plot(
        arr[:, 0],
        arr[:, 1],
        "-o",
        color="gray",
        lw=2,
        alpha=0.6,
        label="Polyline",
    )

    for q in queries:
        res = get_polyline_closest_point(polyline, q)
        if res is None:
            continue
        ei, t = res
        p1 = np.array(polyline[ei])
        p2 = np.array(polyline[ei + 1])
        cp = p1 + (p2 - p1) * t

        ax.plot(q[0], q[1], "o", color="steelblue", ms=8)
        ax.plot(cp[0], cp[1], "r*", ms=12)
        ax.plot([q[0], cp[0]], [q[1], cp[1]], "-", color="crimson", alpha=0.5)

    ax.plot([], [], "o", color="steelblue", label="Query point")
    ax.plot([], [], "r*", ms=12, label="Closest point on polyline")
    ax.plot([], [], "-", color="crimson", alpha=0.5, label="Distance")

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=11)
    ax.set_title("get_polyline_closest_point — Open polyline", fontsize=13)
    fig.tight_layout()
    return fig


def generate_trim_polyline():
    """Trim a polyline between two points."""
    polyline = [
        (2.0, 12.0),
        (5.0, 14.0),
        (8.0, 10.0),
        (11.0, 13.0),
        (14.0, 10.0),
        (17.0, 14.0),
    ]
    a = (4.0, 13.0)
    b = (15.0, 11.5)

    trimmed = trim_polyline_at(polyline, a, b)

    fig, ax = plt.subplots(figsize=(8, 6))
    arr = np.array(polyline)
    ax.plot(
        arr[:, 0],
        arr[:, 1],
        "-o",
        color="gray",
        lw=1.5,
        alpha=0.5,
        label="Original",
    )
    ax.plot(*arr.T, "o", color="gray", ms=4, alpha=0.5)

    trimmed_arr = np.array(trimmed)
    ax.plot(
        trimmed_arr[:, 0],
        trimmed_arr[:, 1],
        "-o",
        color="#e41a1c",
        lw=2.5,
        label="Trimmed",
    )

    ax.plot(a[0], a[1], "s", color="green", ms=10, label="A")
    ax.plot(b[0], b[1], "s", color="blue", ms=10, label="B")

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=11)
    ax.set_title("trim_polyline_at", fontsize=13)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.shape.polygon.md"]
__images__ = [
    {
        "heading": "get_circle_polygon",
        "caption": (
            "``get_circle_polygon`` approximates a circle"
            " as an n-sided polygon"
        ),
        "function": generate_circle_polygon,
    },
    {
        "heading": "get_segment_swept_polygon",
        "caption": (
            "``get_segment_swept_polygon`` computes the swept area of a line "
            "segment with a given radius"
        ),
        "function": generate_segment_swept,
    },
    {
        "heading": "does_path_sweep_intersect_polygon",
        "caption": (
            "Tests whether the Minkowski sweep of a disk along a polyline"
            " intersects any obstacle polygon"
        ),
        "function": generate_path_sweep_intersect,
    },
    {
        "heading": "get_polygons_union",
        "caption": "Polygon union",
        "function": generate_boolean_union,
    },
    {
        "heading": "get_polygons_intersection",
        "caption": "Polygon intersection",
        "function": generate_boolean_intersection,
    },
    {
        "heading": "get_polygons_difference",
        "caption": "Polygon difference",
        "function": generate_boolean_difference,
    },
    {
        "heading": "offset_polygon",
        "caption": "Polygon offset — miter vs round vs square join styles",
        "function": generate_offset,
    },
    {
        "heading": "apply_minimum_curvature",
        "caption": "Minimum curvature fillet applied to a triangle",
        "function": generate_min_curvature,
    },
    {
        "heading": "clean_polygon",
        "caption": "``clean_polygon`` removes near-duplicate vertices",
        "function": generate_clean_polygon,
    },
    {
        "heading": "get_polygon_centroid",
        "caption": "``get_polygon_centroid`` computes the geometric center",
        "function": generate_centroid,
    },
    {
        "heading": "get_polygon_convex_hull",
        "caption": "``get_polygon_convex_hull`` wraps polygon in convex hull",
        "function": generate_convex_hull,
    },
    {
        "heading": "get_polygon_group_bounds",
        "caption": "``get_polygon_group_bounds`` all polygons within a rect",
        "function": generate_group_bounds,
    },
    {
        "heading": "get_polygons_closest_point",
        "caption": "Closest point on multiple polygons",
        "function": generate_closest_point,
    },
    {
        "heading": "get_polyline_closest_point",
        "caption": (
            "``get_polyline_closest_point`` finds the closest point on an open"
            " polyline to a query point, returning the edge index and"
            " parametric position"
        ),
        "function": generate_polyline_closest_point,
    },
    {
        "heading": "trim_polyline_at",
        "caption": "``trim_polyline_at`` trims a polyline between two points",
        "function": generate_trim_polyline,
    },
]
