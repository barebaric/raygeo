"""Generate examples for polygon operations (construction, boolean, offset)."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle as CirclePatch

from raygeo.geo.algo.narrow import find_narrow_passages
from raygeo.geo.shape.polygon import (
    CornerType,
    JoinStyle,
    apply_minimum_curvature,
    clean_polygon,
    does_path_sweep_intersect_polygon,
    find_entry_edges,
    find_polygon_corners,
    get_circle_polygon,
    get_polygon_centroid,
    get_polygon_closest_point,
    get_polygon_convex_hull,
    get_polygon_group_bounds,
    get_polygon_heading_at,
    get_polygons_closest_point,
    get_polygons_difference,
    get_polygons_intersection,
    get_polygons_union,
    get_polyline_swept_polygon,
    get_segment_swept_polygon,
    get_signed_boundary_distance,
    offset_polygon,
    walk_polygon_from_point,
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


def generate_polyline_swept():
    """Polyline swept."""
    path = [(10, 20), (30, 70), (60, 50), (80, 70), (90, 30), (50, 10)]
    radius = 10.0
    swept = get_polyline_swept_polygon(path, radius)

    fig, ax = plt.subplots(figsize=(7, 6))

    if swept:
        arr = np.array(swept[0])
        ax.fill(*np.vstack([arr, arr[0:1]]).T, alpha=0.35, color="#4ecdc4")
        ax.plot(
            *np.vstack([arr, arr[0:1]]).T,
            "-",
            linewidth=2,
            color="#4ecdc4",
            label="Swept area",
        )

    path_arr = np.array(path)
    ax.plot(
        path_arr[:, 0],
        path_arr[:, 1],
        "-o",
        color="k",
        lw=2,
        ms=6,
        label="Polyline path",
    )

    for pt in path:
        circle = CirclePatch(
            pt,
            radius,
            fill=False,
            edgecolor="gray",
            linestyle="--",
            linewidth=0.8,
            alpha=0.5,
        )
        ax.add_patch(circle)

    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.set_aspect("equal")
    ax.set_title("get_polyline_swept_polygon — swept area")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


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


def generate_polygon_closest_point():
    """Closest point on a single polygon."""
    poly = [(10, 10), (90, 10), (90, 70), (10, 70)]
    test_points = [(50, 60), (30, 20), (120, 40), (50, 40), (120, 80)]

    fig, ax = plt.subplots(figsize=(7, 6))

    arr = list(poly) + [poly[0]]
    ax.plot(*zip(*arr), "k-", linewidth=2, label="Polygon")
    ax.fill(*zip(*arr), facecolor="#eef", alpha=0.3)

    for pt in test_points:
        res = get_polygon_closest_point(poly, pt[0], pt[1])
        ax.plot(pt[0], pt[1], "o", color="steelblue", markersize=8)
        if res:
            _t, (cx, cy), _d2 = res
            ax.plot(cx, cy, "r*", markersize=10)
            ax.plot(
                [pt[0], cx],
                [pt[1], cy],
                "-",
                color="crimson",
                alpha=0.5,
                linewidth=1,
            )

    ax.plot([], [], "o", color="steelblue", label="Query point")
    ax.plot([], [], "r*", markersize=10, label="Closest boundary point")
    ax.plot([], [], "-", color="crimson", alpha=0.5, label="Distance")
    ax.set_title("get_polygon_closest_point — Boundary Distance")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_aspect("equal")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


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


# ── get_signed_boundary_distance ──────────────────────────────────


def generate_signed_boundary_distance_field():
    """Signed distance to a square polygon (heatmap + zero contour)."""
    square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
    r = 4.0
    n = 80
    xs = np.linspace(-r, 10 + r, n)
    ys = np.linspace(-r, 10 + r, n)

    field = np.zeros((n, n))
    for i, x in enumerate(xs):
        for j, y in enumerate(ys):
            field[j, i] = get_signed_boundary_distance((x, y), [square])

    fig, ax = plt.subplots(figsize=(7, 6))
    im = ax.pcolormesh(xs, ys, field, shading="auto", cmap="RdBu_r")
    cs = ax.contour(xs, ys, field, levels=[0], colors="k", linewidths=2)
    ax.clabel(cs, fmt={0: "boundary"}, inline=True, fontsize=9)
    cbar = fig.colorbar(im, ax=ax)
    cbar.set_label("Signed distance (mm)")

    sq = np.array(square + [square[0]])
    ax.plot(sq[:, 0], sq[:, 1], "k-", linewidth=1.5, alpha=0.5)

    ax.set_aspect("equal")
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("get_signed_boundary_distance — Square Polygon")
    fig.tight_layout()
    return fig


# ── get_polygon_heading_at ──────────────────────────────────────────


def generate_polygon_heading_at():
    """Outward-pointing heading arrows on an L‑shaped polygon with
    collinear and concave (reflex) vertices.

    * **Collinear points** — four vertices along the bottom edge
      *(0, 1, 2, 3)* — all have the same outward heading (downward).
    * **Convex corners** — vertices *4, 7, 9* are standard 90° turns.
    * **Concave (reflex) corners** — vertices *5 and 8* are the
      notched corners where the outward heading points into the notch.
    """
    poly = [
        (20.0, 10.0),  # 0  collinear start
        (45.0, 10.0),  # 1  ┐ collinear
        (65.0, 10.0),  # 2  ┘ collinear
        (80.0, 10.0),  # 3  collinear end, convex corner
        (80.0, 40.0),  # 4  convex — right shelf
        (60.0, 40.0),  # 5  CONCAVE — notch-right (reflex)
        (60.0, 30.0),  # 6  convex — notch inner
        (40.0, 30.0),  # 7  convex — notch inner
        (40.0, 40.0),  # 8  CONCAVE — notch-left (reflex)
        (20.0, 40.0),  # 9  convex — left shelf
    ]

    fig, ax = plt.subplots(figsize=(8, 6))

    arr = np.array(poly + [poly[0]])
    ax.plot(arr[:, 0], arr[:, 1], "k-", linewidth=2, label="Polygon")
    ax.fill(arr[:, 0], arr[:, 1], facecolor="#eef", alpha=0.3)

    arrow_len = 9.0
    for i, pt in enumerate(poly):
        h = get_polygon_heading_at(poly, pt)
        dx = arrow_len * math.cos(h)
        dy = arrow_len * math.sin(h)
        color = "crimson"
        ax.arrow(
            pt[0],
            pt[1],
            dx,
            dy,
            head_width=4,
            head_length=4,
            fc=color,
            ec=color,
            alpha=0.8,
        )
        ax.plot(pt[0], pt[1], "o", color=color, markersize=5)
        ax.text(
            pt[0] + 2,
            pt[1] - 5,
            str(i),
            fontsize=7,
            color="gray",
            fontweight="bold",
        )

    # Legend markers.
    ax.plot([], [], "o", color="crimson", label="Vertex + heading")
    ax.plot(
        [],
        [],
        "s",
        color="none",
        markeredgecolor="orange",
        markeredgewidth=2,
        markersize=10,
        label="Collinear",
    )
    ax.plot(
        [],
        [],
        "s",
        color="none",
        markeredgecolor="limegreen",
        markeredgewidth=2,
        markersize=10,
        label="Concave (reflex)",
    )

    # Highlight special vertices.
    for idx in (1, 2):
        p = poly[idx]
        ax.plot(
            p[0],
            p[1],
            "s",
            color="none",
            markeredgecolor="orange",
            markeredgewidth=2,
            markersize=10,
        )
    for idx in (5, 8):
        p = poly[idx]
        ax.plot(
            p[0],
            p[1],
            "s",
            color="none",
            markeredgecolor="limegreen",
            markeredgewidth=2,
            markersize=10,
        )

    ax.set_title("get_polygon_heading_at — Collinear & Concave Vertices")
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.legend(fontsize=8, loc="upper left")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


# ── walk_polygon_from_point ────────────────────────────────────────


def _arrow_between(ax, a, b, color, lw=1.5):
    """Draw an arrow from *a* to *b*."""
    dx = b[0] - a[0]
    dy = b[1] - a[1]
    ax.arrow(
        a[0],
        a[1],
        dx,
        dy,
        head_width=3,
        head_length=3,
        fc=color,
        ec=color,
        alpha=0.6,
        lw=lw,
        length_includes_head=True,
    )


def generate_walk_polygon_from_point():
    """Walk order around an irregular polygon with a concave notch.
    The start marker is placed closest to vertex *2*, and the walk
    proceeds forward (CCW) from there.

    The vertex indices in the walk sequence should form
    ``[2, 3, 4, 5, 6, 7, 0, 1]`` — *2* is closest, then all others
    in wrapping CCW order.
    """
    poly = [
        (20.0, 10.0),  # 0  bottom-left
        (80.0, 10.0),  # 1  bottom-right
        (80.0, 55.0),  # 2  right-side start (closest to marker)
        (50.0, 55.0),  # 3  notch-right (concave)
        (50.0, 35.0),  # 4  notch-inner-right
        (30.0, 35.0),  # 5  notch-inner-left
        (30.0, 55.0),  # 6  notch-left (concave)
        (20.0, 55.0),  # 7  left-side
    ]
    start = (85.0, 50.0)

    fig, ax = plt.subplots(figsize=(8, 6))

    arr = np.array(poly + [poly[0]])
    ax.plot(arr[:, 0], arr[:, 1], "k-", linewidth=2, label="Polygon")
    ax.fill(arr[:, 0], arr[:, 1], facecolor="#eef", alpha=0.3)

    walk = walk_polygon_from_point(poly, start)

    n = len(walk)
    for rank, (idx, x, y) in enumerate(walk):
        ax.plot(x, y, "o", color="steelblue", markersize=9, zorder=5)
        ax.text(
            x + 2,
            y + 3,
            str(rank + 1),
            fontsize=11,
            color="steelblue",
            fontweight="bold",
        )
        ax.text(
            x + 2,
            y - 3,
            f"v{idx}",
            fontsize=6,
            color="steelblue",
            alpha=0.7,
        )

    # Arrows between consecutive walk vertices + wraparound.
    for i in range(n - 1):
        a = (walk[i][1], walk[i][2])
        b = (walk[i + 1][1], walk[i + 1][2])
        _arrow_between(ax, a, b, "gray")
    _arrow_between(
        ax, (walk[-1][1], walk[-1][2]), (walk[0][1], walk[0][2]), "gray", lw=1
    )

    # Mark the start point.
    ax.plot(start[0], start[1], "r*", markersize=16, label="Start point")
    ax.plot(
        [start[0], walk[0][1]],
        [start[1], walk[0][2]],
        ":",
        color="gray",
        alpha=0.5,
        label="Closest vertex",
    )

    ax.set_title(
        "walk_polygon_from_point — Wrapping Walk from Closest Vertex\n"
        "(numbered labels show walk step order)"
    )
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.legend(
        fontsize=9, loc="lower center", bbox_to_anchor=(0.5, -0.2), ncol=3
    )
    ax.grid(True, alpha=0.3)
    fig.subplots_adjust(bottom=0.18)
    return fig


def generate_find_polygon_corners():
    """Concave and convex corners of an L-shaped polygon."""
    poly = [
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0, 10.0),
        (10.0, 10.0),
        (10.0, 20.0),
        (0.0, 20.0),
    ]

    concave = find_polygon_corners(poly, CornerType.Concave, 0.0)
    convex = find_polygon_corners(poly, CornerType.Convex, 0.0)

    fig, ax = plt.subplots(figsize=(7, 6))

    arr = np.array(poly + [poly[0]])
    ax.plot(arr[:, 0], arr[:, 1], "k-", linewidth=2, label="Polygon")
    ax.fill(arr[:, 0], arr[:, 1], facecolor="#eef", alpha=0.3)

    for idx, angle in convex:
        p = poly[idx]
        ax.plot(p[0], p[1], "o", color="steelblue", markersize=10, zorder=5)
        ax.text(
            p[0] + 0.8,
            p[1] - 1.5,
            f"v{idx}\n{angle:.0f}°",
            fontsize=8,
            color="steelblue",
            fontweight="bold",
        )

    for idx, angle in concave:
        p = poly[idx]
        ax.plot(p[0], p[1], "s", color="tomato", markersize=12, zorder=5)
        ax.text(
            p[0] + 0.8,
            p[1] - 1.5,
            f"v{idx}\n{angle:.0f}°",
            fontsize=8,
            color="tomato",
            fontweight="bold",
        )

    ax.plot([], [], "o", color="steelblue", label="Convex")
    ax.plot(
        [], [], "s", color="tomato", markersize=10, label="Concave (reflex)"
    )
    ax.set_title("find_polygon_corners — Convex vs Concave vertices")
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_aspect("equal")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_find_entry_edges():
    """Entry edges of narrow passages between two triangular islands."""
    # Two triangular islands pointing at each other with a 4 mm tip gap.
    pocket = [(0.0, 0.0), (80.0, 0.0), (80.0, 50.0), (0.0, 50.0)]
    islands = [
        [(5.0, 5.0), (5.0, 45.0), (37.0, 25.0)],
        [(75.0, 5.0), (75.0, 45.0), (41.0, 25.0)],
    ]
    passages = find_narrow_passages(pocket, holes=islands, max_width=6.0)
    if not passages:
        return plt.figure()

    dist_tol = 2.0

    fig, ax = plt.subplots(figsize=(7, 5))
    plot_polygon(ax, pocket, "grey", "Pocket boundary", linewidth=1.5)
    for isl in islands:
        plot_polygon(ax, isl, "dimgray", None, linewidth=1.2)

    total_entry = 0
    for passage in passages:
        entry = find_entry_edges(passage, [pocket] + islands, dist_tol)
        entry_set = set(entry)
        total_entry += len(entry)

        n = len(passage)
        for i in range(n):
            j = (i + 1) % n
            x = [passage[i][0], passage[j][0]]
            y = [passage[i][1], passage[j][1]]
            if i in entry_set:
                ax.plot(x, y, "tomato", linewidth=3, solid_capstyle="round")
            else:
                ax.plot(x, y, "steelblue", linewidth=2, solid_capstyle="round")

        for i in entry:
            j = (i + 1) % n
            mid = (
                (passage[i][0] + passage[j][0]) / 2,
                (passage[i][1] + passage[j][1]) / 2,
            )
            ax.plot(mid[0], mid[1], "ro", markersize=6)

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_title(
        "find_entry_edges — entry edges (red) vs wall edges (blue)\n"
        f"dist_tol={dist_tol}, {total_entry} entry edges"
        f" across {len(passages)} passages"
    )
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
        "heading": "get_polyline_swept_polygon",
        "caption": (
            "``get_polyline_swept_polygon`` computes the Minkowski sum of a "
            "polyline path with a disk"
        ),
        "function": generate_polyline_swept,
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
        "heading": "get_polygon_closest_point",
        "caption": (
            "``get_polygon_closest_point`` finds the nearest boundary"
            " point to a given coordinate"
        ),
        "function": generate_polygon_closest_point,
    },
    {
        "heading": "get_polygon_group_bounds",
        "caption": "``get_polygon_group_bounds`` all polygons within a rect",
        "function": generate_group_bounds,
    },
    {
        "heading": "get_signed_boundary_distance",
        "caption": (
            "Signed distance field around a square polygon."
            " Red = outside (positive), blue = inside (negative),"
            " black contour marks the boundary."
        ),
        "function": generate_signed_boundary_distance_field,
    },
    {
        "heading": "get_polygon_heading_at",
        "caption": (
            "``get_polygon_heading_at`` draws outward-facing heading arrows"
            " at each vertex of a CCW polygon."
        ),
        "function": generate_polygon_heading_at,
    },
    {
        "heading": "walk_polygon_from_point",
        "caption": (
            "``walk_polygon_from_point`` returns vertices in walk order"
            " starting from the vertex closest to a marker."
        ),
        "function": generate_walk_polygon_from_point,
    },
    {
        "heading": "find_polygon_corners",
        "caption": (
            "``find_polygon_corners`` labels convex (circle) and"
            " concave / reflex (square) vertices with their interior"
            " angles."
        ),
        "function": generate_find_polygon_corners,
    },
    {
        "heading": "find_entry_edges",
        "caption": (
            "``find_entry_edges`` identifies edges of a narrow-passage"
            " polygon that are not collinear with the pocket boundary"
            " (entry edges, marked red)."
        ),
        "function": generate_find_entry_edges,
    },
]
