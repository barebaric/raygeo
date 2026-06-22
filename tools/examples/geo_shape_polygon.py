"""Generate examples for polygon operations (construction, boolean, offset)."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle as CirclePatch

from raygeo.geo.shape.line import get_interior_angle
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
    split_polyline_at_v_junctions,
    trim_polyline_angular_ends,
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


def generate_trim_polyline_angular_ends():
    """Trim transition vertices from the ends of a subsequence."""
    # 13-point polyline: hook → opens up → straight → tightens → hook.
    # Cut indices 1..10 (length 10).  The 25° threshold detects both
    # transitions → P1 and P10 get trimmed, leaving indices 2..9.
    poly = [
        (0.0, 8.0),
        (0.0, 0.0),
        (2.5, 1.5),
        (5.0, 0.3),
        (7.5, 0.0),
        (10.0, 0.0),
        (12.5, 0.0),
        (15.0, 0.0),
        (17.5, 0.0),
        (20.0, -0.3),
        (22.5, 1.5),
        (25.0, 0.0),
        (25.0, 8.0),
    ]
    n = len(poly)
    idxs = list(range(n))
    threshold_rad = math.radians(25)
    threshold_deg = 25

    cut_start_in = 1
    cut_len_in = n - 3  # indices 1..10 (length 10)

    (new_start, new_len) = trim_polyline_angular_ends(
        poly,
        cut_start_in,
        cut_len_in,
        threshold_rad,
    )

    cut_before = list(range(cut_start_in, cut_start_in + cut_len_in))
    cut_after = list(range(new_start, new_start + new_len))
    trimmed = [i for i in cut_before if i not in cut_after]

    angles = [
        math.degrees(
            get_interior_angle(poly[(i - 1) % n], poly[i], poly[(i + 1) % n])
        )
        for i in range(n)
    ]

    # ── figure layout ──
    fig = plt.figure(figsize=(12, 8))
    gs = fig.add_gridspec(1, 2, width_ratios=[1, 1], wspace=0.3)
    ax1 = fig.add_subplot(gs[0])
    ax2 = fig.add_subplot(gs[1])

    # ── draw one subplot ──
    def draw_panel(ax, ids, label_drop, title_text):
        arr = np.array(poly)

        # full polygon outline (thin)
        closed = np.vstack([arr, arr[0:1]])
        ax.plot(closed[:, 0], closed[:, 1], "-", color="#ccc", lw=1, zorder=1)
        ax.plot(arr[:, 0], arr[:, 1], "o", color="#ccc", ms=4, zorder=2)

        # highlight the cut
        for k in range(len(ids) - 1):
            a = poly[ids[k]]
            b = poly[ids[k + 1]]
            ax.plot(
                [a[0], b[0]],
                [a[1], b[1]],
                "-",
                color="#d62728",
                lw=3.5,
                zorder=3,
            )
        cut_pts = np.array([poly[i] for i in ids])
        ax.plot(
            cut_pts[:, 0],
            cut_pts[:, 1],
            "o",
            color="#d62728",
            ms=8,
            zorder=4,
        )

        # trimmed vertices  (only in Before panel)
        for i in label_drop:
            p = poly[i]
            ax.plot(p[0], p[1], "x", color="#888", ms=10, mew=2.5, zorder=5)
            ax.annotate(
                "trimmed",
                xy=(p[0], p[1]),
                xytext=(p[0] + (1.5 if i > 3 else -1.5), p[1] - 1.8),
                fontsize=8,
                color="#666",
                ha="center",
                arrowprops=dict(arrowstyle="->", color="#888", lw=0.8),
            )

        # ── angle annotations ──
        for i in idxs:
            p = poly[i]
            ang = angles[i]
            inside = i in ids
            clr = "#d62728" if inside else "#aaa"
            # offset: left (x<3), right (x>17), centre; above/below y
            ox = -2.8 if p[0] < 3 else (2.2 if p[0] > 17 else 0.6)
            oy = 1.6 if p[1] >= 0 else -2.2
            ax.annotate(
                f"{ang:.0f}°",
                xy=p,
                xytext=(p[0] + ox, p[1] + oy),
                fontsize=9,
                color=clr,
                fontweight="bold" if inside else "normal",
                bbox=dict(
                    boxstyle="round,pad=0.15",
                    fc="white" if inside else "none",
                    ec="none",
                    alpha=0.85,
                ),
            )

        # ── highlight the angular jumps ──
        # start jump: from second → third vertex of the cut
        if len(ids) >= 3:
            i2, i3 = ids[1], ids[2]
            mid = (
                (poly[i2][0] + poly[i3][0]) / 2,
                (poly[i2][1] + poly[i3][1]) / 2,
            )
            ax.annotate(
                f"Δ {abs(angles[i3] - angles[i2]):.0f}°",
                xy=mid,
                xytext=(mid[0], mid[1] + (2.0 if poly[i2][1] >= 0 else -2.0)),
                fontsize=8,
                color="#d62728",
                ha="center",
                fontweight="bold",
                arrowprops=dict(
                    arrowstyle="->",
                    color="#d62728",
                    lw=1.2,
                    connectionstyle="arc3,rad=0",
                ),
            )
            # end jump: from third-to-last → second-to-last of the cut
            i_a, i_b = ids[-3], ids[-2]
            mid2 = (
                (poly[i_a][0] + poly[i_b][0]) / 2,
                (poly[i_a][1] + poly[i_b][1]) / 2,
            )
            ax.annotate(
                f"Δ {abs(angles[i_b] - angles[i_a]):.0f}°",
                xy=mid2,
                xytext=(
                    mid2[0],
                    mid2[1] + (2.0 if poly[i_b][1] >= 0 else -2.0),
                ),
                fontsize=8,
                color="#d62728",
                ha="center",
                fontweight="bold",
                arrowprops=dict(
                    arrowstyle="->",
                    color="#d62728",
                    lw=1.2,
                    connectionstyle="arc3,rad=0",
                ),
            )

        # ── labels ──
        ax.set_aspect("equal")
        ax.set_ylim(-6, 12)
        ax.set_xlim(-3, 28)
        ax.grid(True, alpha=0.15, ls=":")
        ax.set_title(title_text, fontsize=12, fontweight="bold")
        ax.set_xlabel("x")
        ax.set_ylabel("y")

    draw_panel(
        ax1,
        cut_before,
        trimmed,
        "Before:  cut = vertices 1…10",
    )
    draw_panel(
        ax2,
        cut_after,
        [],
        "After:  trimmed cut = vertices 2…9",
    )

    # ── panel annotations ──
    ax1.annotate(
        f"threshold = {threshold_deg}°",
        xy=(0.96, 0.96),
        xycoords="axes fraction",
        va="top",
        ha="right",
        fontsize=9,
        bbox=dict(boxstyle="round,pad=0.3", fc="#fff9c4", ec="#f9a825"),
    )
    annot_after = (
        f"cut index {cut_start_in}→{new_start}\n"
        f"cut length {cut_len_in}→{new_len}\n"
        f"dropped vertices: {trimmed}"
    )
    ax2.annotate(
        annot_after,
        xy=(0.96, 0.96),
        xycoords="axes fraction",
        va="top",
        ha="right",
        fontsize=9,
        family="monospace",
        bbox=dict(boxstyle="round,pad=0.3", fc="#e8f5e9", ec="#43a047"),
    )

    fig.suptitle(
        "trim_polyline_angular_ends",
        fontsize=14,
        fontweight="bold",
        y=1.02,
    )
    fig.subplots_adjust(left=0.05, right=0.98, bottom=0.06, top=0.93)
    return fig


def generate_split_v_junctions():
    #
    # Three high-resolution semi-arcs (hills) meeting at V-junctions
    #
    n = 25  # points per hill

    def semi_arc(x0, y0, x1, y1, n):
        pts = []
        cx = (x0 + x1) / 2
        r = abs(x1 - x0) / 2
        amp = r * 0.2
        for i in range(n):
            t = i / (n - 1)
            a = math.pi * t
            x = cx - r * math.cos(a)
            y = y0 + amp * math.sin(a)
            pts.append((x, round(y, 6)))
        return pts

    hill1 = semi_arc(10, 30, 40, 30, n)
    hill2 = semi_arc(40, 30, 70, 30, n)
    hill3 = semi_arc(70, 30, 100, 30, n)

    polyline = hill1 + hill2[1:] + hill3[1:]
    vj1_idx = len(hill1)
    vj2_idx = len(hill1) + len(hill2) - 1

    angle_thresh = math.radians(25)
    segments = split_polyline_at_v_junctions(polyline, angle_thresh)

    fig, axes = plt.subplots(1, 2, figsize=(12, 5))

    xs = [p[0] for p in polyline]
    ys = [p[1] for p in polyline]
    axes[0].plot(xs, ys, "-", color="steelblue", linewidth=2, alpha=0.7)
    axes[0].plot(xs, ys, "o", color="steelblue", markersize=2)
    for idx, label in [(vj1_idx, "V₁"), (vj2_idx, "V₂")]:
        axes[0].plot(
            xs[idx], ys[idx], "v", color="red", markersize=12, zorder=5
        )
        axes[0].annotate(
            label,
            (xs[idx], ys[idx]),
            xytext=(0, -18),
            textcoords="offset points",
            ha="center",
            fontsize=11,
            fontweight="bold",
            color="red",
        )
    axes[0].set_title(
        f"Original polyline — {len(polyline)} pts, 2 V-junctions (▼)"
    )
    axes[0].set_aspect("equal")
    axes[0].set_xlim(5, 105)
    axes[0].set_ylim(25, 50)
    axes[0].grid(True, alpha=0.3)

    cmap = plt.get_cmap("tab10")
    for si, seg in enumerate(segments):
        xs = [p[0] for p in seg]
        ys = [p[1] for p in seg]
        axes[1].plot(
            xs,
            ys,
            "o-",
            color=cmap(si % 10),
            linewidth=2.5,
            markersize=4,
            label=f"Segment {si + 1}",
        )
    axes[1].set_title(f"After split — {len(segments)} segments")
    axes[1].set_aspect("equal")
    axes[1].set_xlim(5, 105)
    axes[1].set_ylim(25, 50)
    axes[1].grid(True, alpha=0.3)
    axes[1].legend(fontsize=9)

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
    {
        "heading": "trim_polyline_angular_ends",
        "caption": (
            "``trim_polyline_angular_ends`` removes transition vertices from"
            " both ends of a contiguous subsequence where the interior angle"
            " jumps sharply.  Here a 10-vertex cut (indices 1–10) with"
            " angles ranging 59°→180°→59° is trimmed to 8 vertices"
            " using a 25° threshold."
        ),
        "function": generate_trim_polyline_angular_ends,
    },
    {
        "heading": "split_polyline_at_v_junctions",
        "caption": (
            "Three semi-arcs (hills) form two V-junctions where they meet."
            " The function splits the polyline at those points and trims"
            " each segment's angular ends."
        ),
        "function": generate_split_v_junctions,
    },
]
