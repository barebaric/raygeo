"""Generate 3D polygon operation example images."""

import math

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import art3d

from raygeo.geo.shape.polygon3d import (
    fillet_polyline_3d,
    flip_polygon_3d,
    get_polygon_area_3d,
    get_polygon_bounds_3d,
    get_polygon_centroid_3d,
    get_polygon_convex_hull_3d,
    get_polygon_edges_3d,
    get_polygon_perimeter_3d,
    get_polygon_signed_area_3d,
    get_polygons_closest_point_3d,
    get_polygons_difference_3d,
    get_polygons_intersection_3d,
    get_polygons_union_3d,
    get_polyline_end_tangent_3d,
    offset_polygon_3d,
    offset_polyline_3d,
    rotate_polygon_3d,
    scale_polygon_3d,
    translate_polygon_3d,
    walk_along_polygon_3d,
    walk_along_polyline_3d,
)


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


def _lift(poly, z):
    return [(x, y, float(z)) for x, y in poly]


def _draw_polygon3d(ax, poly3d, color, label, linewidth=2, alpha=0.3):
    xs = [p[0] for p in poly3d] + [poly3d[0][0]]
    ys = [p[1] for p in poly3d] + [poly3d[0][1]]
    zs = [p[2] for p in poly3d] + [poly3d[0][2]]
    ax.plot(xs, ys, zs, color=color, linewidth=linewidth, label=label)
    verts = [list(zip(xs, ys))]
    poly = mpatches.Polygon(verts[0], color=color, alpha=alpha)
    ax.add_patch(poly)
    art3d.pathpatch_2d_to_3d(poly, z=poly3d[0][2], zdir="z")


def _make_3d_ax(title, zlim=(0, 15)):
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title(title, fontsize=14)
    ax.set_zlim(*zlim)
    ax.view_init(elev=20, azim=-90)
    fig.tight_layout()
    return fig, ax


def _plot_3d_boolean(a, b, result, title):
    fig, ax = _make_3d_ax(title)
    _draw_polygon3d(ax, a, "steelblue", f"A (Z={a[0][2]:.0f})")
    _draw_polygon3d(ax, b, "tomato", f"B (Z={b[0][2]:.0f})")
    if result:
        _draw_polygon3d(
            ax,
            result[0],
            "limegreen",
            f"Result (Z={result[0][0][2]:.0f})",
            linewidth=3,
            alpha=0.5,
        )
    ax.legend(loc="upper left")
    return fig


def generate_boolean_union():
    """3D polygon union."""
    n_seg = 64
    r = 12
    a_xy = _make_circle(float(r), n_seg, ox=-2.0, oy=0.0)
    b_xy = _make_square(float(r), ox=2.0, oy=0.0)
    a = _lift(a_xy, 3.0)
    b = _lift(b_xy, 7.0)
    result = get_polygons_union_3d([a, b])
    return _plot_3d_boolean(a, b, result, "3D Union (Z from A)")


def generate_boolean_intersection():
    """3D polygon intersection."""
    n_seg = 64
    r = 12
    a_xy = _make_circle(r, n_seg, ox=-2, oy=0)
    b_xy = _make_square(r, ox=2, oy=0)
    a = _lift(a_xy, 3)
    b = _lift(b_xy, 7)
    result = get_polygons_intersection_3d(a, b)
    return _plot_3d_boolean(a, b, result, "3D Intersection (Z from A)")


def generate_boolean_difference():
    """3D polygon difference."""
    n_seg = 64
    r = 12
    a_xy = _make_circle(r, n_seg, ox=-2, oy=0)
    b_xy = _make_square(r, ox=2, oy=0)
    a = _lift(a_xy, 3)
    b = _lift(b_xy, 7)
    result = get_polygons_difference_3d(a, b)
    return _plot_3d_boolean(a, b, result, "3D Difference (Z from A)")


def generate_offset():
    """3D polygon offset."""
    n_seg = 64
    r = 12
    a_xy = _make_circle(r, n_seg, ox=0, oy=0)
    a = _lift(a_xy, 3)
    result = offset_polygon_3d(a, 2.0)
    fig, ax = _make_3d_ax("3D Polygon Offset (Z preserved)")
    _draw_polygon3d(ax, a, "steelblue", f"Original (Z={a[0][2]:.0f})")
    if result:
        _draw_polygon3d(
            ax,
            result[0],
            "limegreen",
            f"Offset (Z={result[0][0][2]:.0f})",
            linewidth=3,
            alpha=0.5,
        )
    ax.legend(loc="upper left")
    return fig


def generate_perimeter():
    """3D polygon perimeter."""
    poly = [(0, 0, 2), (8, 0, 2), (8, 6, 6), (2, 6, 6)]
    perim = get_polygon_perimeter_3d(poly)
    edges = get_polygon_edges_3d(poly)
    fig, ax = _make_3d_ax(f"3D Polygon Perimeter = {perim:.1f}", zlim=(0, 10))
    for (x1, y1, z1), (x2, y2, z2) in edges:
        ax.plot([x1, x2], [y1, y2], [z1, z2], color="steelblue", linewidth=2)
        d = math.sqrt((x2 - x1) ** 2 + (y2 - y1) ** 2 + (z2 - z1) ** 2)
        mx, my, mz = (x1 + x2) / 2, (y1 + y2) / 2, (z1 + z2) / 2
        ax.text(
            mx,
            my + 0.5,
            mz,
            f"{d:.1f}",
            ha="center",
            fontsize=11,
            bbox=dict(boxstyle="round,pad=0.2", facecolor="white", alpha=0.8),
        )
    return fig


def generate_bounds():
    """3D bounding box."""
    poly = [(0, 0, 2), (8, 0, 2), (10, 6, 6), (4, 8, 6), (-2, 4, 4)]
    x_min, y_min, x_max, y_max, z_min, z_max = get_polygon_bounds_3d(poly)
    fig, ax = _make_3d_ax("3D Polygon Bounding Box", zlim=(0, 10))
    _draw_polygon3d(ax, poly, "steelblue", "Polygon", alpha=0.15)
    corners = [
        (x_min, y_min, z_min),
        (x_max, y_min, z_min),
        (x_max, y_max, z_min),
        (x_min, y_max, z_min),
        (x_min, y_min, z_max),
        (x_max, y_min, z_max),
        (x_max, y_max, z_max),
        (x_min, y_max, z_max),
    ]
    for i, j in [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ]:
        ax.plot(
            [corners[i][0], corners[j][0]],
            [corners[i][1], corners[j][1]],
            [corners[i][2], corners[j][2]],
            color="tomato",
            linewidth=1,
            linestyle="--",
        )
    ax.legend(loc="upper left")
    return fig


def generate_centroid():
    """3D centroid."""
    n_seg_hull = 32
    poly = _lift(_make_circle(8, n_seg_hull), 5)
    cx, cy, cz = get_polygon_centroid_3d(poly)
    fig, ax = _make_3d_ax(
        f"3D Polygon Centroid ({cx:.1f}, {cy:.1f}, {cz:.1f})"
    )
    _draw_polygon3d(ax, poly, "steelblue", "Polygon", alpha=0.15)
    ax.plot(
        [cx],
        [cy],
        [cz],
        "o",
        color="limegreen",
        markersize=10,
        label="Centroid",
    )
    ax.legend(loc="upper left")
    return fig


def generate_edges():
    """3D polygon edges."""
    poly = _lift([(0, 0), (8, 0), (10, 5), (4, 8)], 5)
    edges = get_polygon_edges_3d(poly)
    fig, ax = _make_3d_ax("3D Polygon Edges (numbered)")
    _draw_polygon3d(ax, poly, "steelblue", "Polygon", alpha=0.1)
    for i, ((x1, y1, z1), (x2, y2, z2)) in enumerate(edges):
        mx, my, mz = (x1 + x2) / 2, (y1 + y2) / 2, (z1 + z2) / 2
        ax.text(
            mx,
            my,
            mz,
            str(i),
            color="k",
            fontsize=12,
            ha="center",
            va="center",
        )
    ax.legend(loc="upper left")
    return fig


def generate_convex_hull():
    """3D convex hull."""
    bowtie = [(0, 0, 3), (10, 0, 3), (0, 10, 7), (10, 10, 7)]
    hull = get_polygon_convex_hull_3d(bowtie)
    fig, ax = _make_3d_ax("3D Polygon Convex Hull", zlim=(0, 10))
    xs_b = [p[0] for p in bowtie]
    ys_b = [p[1] for p in bowtie]
    zs_b = [p[2] for p in bowtie]
    ax.plot(
        xs_b + [xs_b[0]],
        ys_b + [ys_b[0]],
        zs_b + [zs_b[0]],
        color="steelblue",
        linewidth=1.5,
        label="Original (bow-tie)",
    )
    ax.plot(xs_b, ys_b, zs_b, "o", color="steelblue", markersize=6)
    _draw_polygon3d(
        ax, hull, "limegreen", "Convex Hull", linewidth=3, alpha=0.25
    )
    ax.legend(loc="upper left")
    return fig


def generate_translate():
    """3D polygon translate."""
    poly = _lift(_make_square(6), 5)
    translated = translate_polygon_3d(poly, 5.0, 3.0, 2.0)
    fig, ax = _make_3d_ax("3D Polygon Translate (dx=5, dy=3, dz=2)")
    _draw_polygon3d(ax, poly, "steelblue", "Original", alpha=0.2)
    _draw_polygon3d(
        ax,
        translated,
        "tomato",
        "Translated",
        linewidth=3,
        alpha=0.4,
    )
    ax.legend(loc="upper left")
    return fig


def generate_scale():
    """3D polygon scale."""
    poly = _lift(_make_square(6), 5)
    scaled = scale_polygon_3d(poly, 1.5)
    fig, ax = _make_3d_ax("3D Polygon Scale (uniform)")
    _draw_polygon3d(ax, poly, "steelblue", "Original", alpha=0.2)
    _draw_polygon3d(
        ax, scaled, "tomato", "Scaled ×1.5", linewidth=3, alpha=0.4
    )
    ax.legend(loc="upper left")
    return fig


def generate_flip():
    """3D polygon flip."""
    poly = _lift([(2, 0), (8, 0), (8, 6), (4, 6)], 5)
    flipped = flip_polygon_3d(poly, flip_h=True, flip_z=True)
    fig, ax = _make_3d_ax("3D Polygon Flip (X + Z)")
    _draw_polygon3d(ax, poly, "steelblue", "Original", alpha=0.2)
    _draw_polygon3d(
        ax,
        flipped,
        "tomato",
        "flip_h + flip_z",
        linewidth=3,
        alpha=0.4,
    )
    ax.legend(loc="upper left")
    return fig


def generate_rotate():
    """3D polygon rotate."""
    poly = _lift(_make_square(6), 5)
    rotated = rotate_polygon_3d(poly, 45.0)
    fig, ax = _make_3d_ax("3D Polygon Rotate (Z-axis, Z preserved)")
    _draw_polygon3d(ax, poly, "steelblue", "Original (Z=5)", alpha=0.2)
    _draw_polygon3d(
        ax, rotated, "tomato", "Rotated 45°", linewidth=3, alpha=0.4
    )
    ax.legend(loc="upper left")
    return fig


def generate_true_offset():
    """True 3D polyline offset."""
    n = 16
    curve = []
    for i in range(n + 1):
        t = i / n
        a = t * math.pi / 2
        x = 6 * math.cos(a)
        y = 6 * math.sin(a)
        z = 6 - x / 3 + y / 3
        curve.append((x, y, z))
    off = offset_polyline_3d(curve, 1.2)
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title("True 3D Polyline Offset (edge-plane miter)", fontsize=14)
    ax.set_xlim(-2, 8)
    ax.set_ylim(-2, 8)
    ax.set_zlim(0, 10)
    ax.view_init(elev=25, azim=-55)
    fig.tight_layout()
    xs = [p[0] for p in curve]
    ys = [p[1] for p in curve]
    zs = [p[2] for p in curve]
    ax.plot(xs, ys, zs, "o-", color="steelblue", linewidth=2, label="Original")
    xs_o = [p[0] for p in off]
    ys_o = [p[1] for p in off]
    zs_o = [p[2] for p in off]
    ax.plot(
        xs_o,
        ys_o,
        zs_o,
        "o-",
        color="tomato",
        linewidth=3,
        label="Offset d=1.2",
        alpha=0.8,
    )
    for i in range(0, len(curve), 3):
        ax.plot(
            [curve[i][0], off[i][0]],
            [curve[i][1], off[i][1]],
            [curve[i][2], off[i][2]],
            color="gray",
            linewidth=1,
            linestyle=":",
        )
    ax.legend(loc="upper left")
    return fig


def generate_end_tangent():
    """End tangent."""
    poly = [
        (2.0, 2.0, 0.0),
        (5.0, 1.0, 0.0),
        (8.0, 3.0, 0.0),
        (10.0, 8.0, 0.0),
    ]
    dx, dy = get_polyline_end_tangent_3d(poly)

    fig, ax = plt.subplots(figsize=(7, 7))
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    ax.plot(
        xs,
        ys,
        "o-",
        color="steelblue",
        linewidth=2,
        markersize=6,
        label="Polyline",
    )
    # Arrow at the last point showing the tangent direction
    last = poly[-1]
    arrow_len = 2.0
    ax.arrow(
        last[0],
        last[1],
        dx * arrow_len,
        dy * arrow_len,
        head_width=0.4,
        head_length=0.4,
        fc="tomato",
        ec="tomato",
        linewidth=2,
        label=f"End tangent ({dx:.3f}, {dy:.3f})",
    )
    # Label points
    for i, (x, y, _) in enumerate(poly):
        ax.annotate(
            str(i),
            (x, y),
            textcoords="offset points",
            xytext=(4, 4),
            fontsize=10,
            color="k",
        )
    ax.set_aspect("equal")
    ax.set_xlim(0, 12)
    ax.set_ylim(0, 12)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    ax.set_title("Polyline end tangent direction", fontsize=12)
    fig.tight_layout()
    return fig


def generate_fillet_polyline_3d():
    """Fillet corners on a 3D polyline."""
    poly = [
        (0.0, 0.0, 0.0),
        (8.0, 0.0, 0.0),
        (8.0, 6.0, 3.0),
        (2.0, 6.0, 3.0),
        (2.0, 0.0, 6.0),
        (10.0, 0.0, 6.0),
    ]
    radius = 1.5
    result = fillet_polyline_3d(poly, radius)

    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title(f"3D Polyline Fillet (radius = {radius})", fontsize=14)
    ax.set_xlim(-1, 12)
    ax.set_ylim(-1, 8)
    ax.set_zlim(-1, 8)
    ax.view_init(elev=25, azim=-65)
    fig.tight_layout()

    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    zs = [p[2] for p in poly]
    ax.plot(xs, ys, zs, "o-", color="steelblue", linewidth=2, label="Original")

    xs_r = [p[0] for p in result]
    ys_r = [p[1] for p in result]
    zs_r = [p[2] for p in result]
    ax.plot(
        xs_r,
        ys_r,
        zs_r,
        "o-",
        color="tomato",
        linewidth=3,
        alpha=0.85,
        label=f"Filleted (r={radius})",
    )

    # Highlight fillet arcs with a different marker
    if len(result) > len(poly):
        ax.plot(
            xs_r,
            ys_r,
            zs_r,
            "o",
            color="tomato",
            markersize=4,
            alpha=0.85,
        )

    ax.legend(loc="upper left")
    return fig


def generate_area():
    """XY-projected area of a 3D polygon."""
    poly = [(0, 0, 2), (6, 0, 2), (6, 6, 5), (0, 6, 5)]
    area = get_polygon_area_3d(poly)
    signed = get_polygon_signed_area_3d(poly)
    fig, ax = _make_3d_ax(
        f"3D Polygon Area (XY-projected) = {area:.1f}, signed = {signed:.1f}",
        zlim=(0, 8),
    )
    _draw_polygon3d(ax, poly, "steelblue", "Polygon (CCW)", alpha=0.25)
    return fig


def generate_signed_area():
    """Signed XY-projected area shows winding direction."""
    cw = [(0, 0, 2), (0, 6, 2), (6, 6, 5), (6, 0, 5)]
    sa_cw = get_polygon_signed_area_3d(cw)
    sa_ccw = get_polygon_signed_area_3d(cw[::-1])  # reversed = CCW
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title("Signed Area: CW = red, CCW = green", fontsize=14)
    ax.set_zlim(0, 8)
    ax.view_init(elev=25, azim=-65)
    fig.tight_layout()
    _draw_polygon3d(ax, cw, "tomato", f"CW (signed = {sa_cw:.1f})", alpha=0.2)
    _draw_polygon3d(
        ax,
        cw[::-1],
        "limegreen",
        f"CCW (signed = {sa_ccw:.1f})",
        alpha=0.2,
    )
    ax.legend(loc="upper left")
    return fig


def generate_walk_along():
    """Walk along a 3D arc-like polyline."""
    n = 10
    poly = [
        (10 * math.cos(t), 10 * math.sin(t), 5.0)
        for t in [i / n * math.pi for i in range(n + 1)]
    ]
    # cumul distance at each vertex
    cumul = [0.0]
    for i in range(len(poly) - 1):
        cumul.append(cumul[-1] + math.dist(poly[i], poly[i + 1]))
    # start on the polyline between two interior vertices
    i0, i1 = 4, 5
    start = (
        (poly[i0][0] + poly[i1][0]) / 2,
        (poly[i0][1] + poly[i1][1]) / 2,
        (poly[i0][2] + poly[i1][2]) / 2,
    )
    target = walk_along_polyline_3d(poly, start, forward=True, distance=8.0)
    fig, ax = _make_3d_ax(
        "walk_along_polyline_3d: start (black circle)"
        " → 8 mm forward (red star)"
    )
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    zs = [p[2] for p in poly]
    ax.plot(
        xs,
        ys,
        zs,
        "-o",
        color="steelblue",
        linewidth=2,
        markerfacecolor="lightblue",
        markeredgecolor="steelblue",
        markersize=6,
        label="Arc polyline (Z=5)",
    )
    for i, (x, y, z) in enumerate(poly):
        ax.text(
            x,
            y,
            z,
            f"{cumul[i]:.1f}",
            fontsize=8,
            ha="left",
            va="top",
            color="steelblue",
        )
    ax.plot(
        [start[0]],
        [start[1]],
        [start[2]],
        "o",
        color="k",
        markersize=10,
        label="Start point",
    )
    ax.plot(
        [target[0]],
        [target[1]],
        [target[2]],
        "*",
        color="tomato",
        markersize=14,
        label="Target (8 mm)",
    )
    ax.text(
        target[0],
        target[1],
        target[2],
        "8 mm",
        fontsize=10,
        ha="center",
        va="bottom",
    )
    ax.legend(loc="upper left")
    return fig


def generate_walk_along_polygon():
    """Walk along a closed 3D polygon (pentagon)."""
    n = 5
    poly = [
        (
            8 * math.cos(2 * math.pi * i / n),
            8 * math.sin(2 * math.pi * i / n),
            5.0,
        )
        for i in range(n)
    ]
    start = walk_along_polygon_3d(poly, poly[0], forward=True, distance=5.0)
    targets = {
        d: walk_along_polygon_3d(poly, start, forward=True, distance=d)
        for d in [5, 10, 20]
    }
    fig, ax = _make_3d_ax(
        "walk_along_polygon_3d: start (black circle)"
        " → targets at +5, +10, +20 mm"
    )
    xs = [p[0] for p in poly] + [poly[0][0]]
    ys = [p[1] for p in poly] + [poly[0][1]]
    zs = [p[2] for p in poly] + [poly[0][2]]
    ax.plot(
        xs,
        ys,
        zs,
        "-o",
        color="steelblue",
        linewidth=2,
        markerfacecolor="lightblue",
        markeredgecolor="steelblue",
        markersize=6,
        label="Pentagon (Z=5)",
    )
    ax.plot(
        [start[0]],
        [start[1]],
        [start[2]],
        "o",
        color="k",
        markersize=10,
        label="Start (5 mm from vertex 0)",
    )
    colors = {5: "gold", 10: "tomato", 20: "limegreen"}
    for d, pt in targets.items():
        ax.plot(
            [pt[0]],
            [pt[1]],
            [pt[2]],
            "*",
            color=colors[d],
            markersize=14,
            label=f"+{d} mm",
        )
        ax.text(
            pt[0],
            pt[1],
            pt[2],
            f"+{d} mm",
            fontsize=10,
            ha="center",
            va="bottom",
        )
    ax.legend(loc="upper left")
    return fig


def generate_closest_point_3d():
    """Closest point on multiple 3D polygons."""
    polys = [
        _lift([(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)], 3.0),
        _lift([(12.0, 2.0), (18.0, 2.0), (18.0, 8.0), (12.0, 8.0)], 7.0),
    ]
    query = (15.0, 15.0, 8.0)

    fig, ax = _make_3d_ax("get_polygons_closest_point_3d", zlim=(0, 10))
    colors = ["steelblue", "tomato"]
    for poly, color in zip(polys, colors):
        _draw_polygon3d(ax, poly, color, "", alpha=0.15)

    ax.plot(
        [query[0]],
        [query[1]],
        [query[2]],
        "o",
        color="k",
        markersize=10,
        label="Query",
    )

    result = get_polygons_closest_point_3d(polys, query[0], query[1], query[2])
    if result is not None:
        pi, t, pt, d2 = result
        ax.plot(
            [pt[0]],
            [pt[1]],
            [pt[2]],
            "*",
            color="gold",
            markersize=16,
            label=f"Closest (poly {pi})",
        )
        ax.plot(
            [query[0], pt[0]],
            [query[1], pt[1]],
            [query[2], pt[2]],
            color="gray",
            lw=1.5,
            ls="--",
        )
        ax.set_title(
            f"Closest point on polygon {pi}, d²={d2:.2f}", fontsize=13
        )
    else:
        ax.set_title("No closest point found", fontsize=13)

    ax.legend(loc="upper left")
    ax.view_init(elev=20, azim=-40)
    return fig


__docs_target__ = ["raygeo.geo.shape.polygon3d.md"]
__images__ = [
    {
        "heading": "get_polygons_union_3d",
        "caption": "3D polygon union — Z from first polygon",
        "function": generate_boolean_union,
    },
    {
        "heading": "get_polygons_intersection_3d",
        "caption": "3D polygon intersection — Z from first polygon",
        "function": generate_boolean_intersection,
    },
    {
        "heading": "get_polygons_difference_3d",
        "caption": "3D polygon difference (A - B) — Z from A",
        "function": generate_boolean_difference,
    },
    {
        "heading": "offset_polygon_3d",
        "caption": "3D polygon offset — Z preserved from input",
        "function": generate_offset,
    },
    {
        "heading": "get_polygon_perimeter_3d",
        "caption": "3D polygon perimeter using full 3D edge lengths",
        "function": generate_perimeter,
    },
    {
        "heading": "get_polygon_bounds_3d",
        "caption": "3D bounding box (Rect3D)",
        "function": generate_bounds,
    },
    {
        "heading": "get_polygon_centroid_3d",
        "caption": "3D centroid - XY via shoelace, Z as average",
        "function": generate_centroid,
    },
    {
        "heading": "get_polygon_edges_3d",
        "caption": "3D polygon edges as (start, end) pairs",
        "function": generate_edges,
    },
    {
        "heading": "get_polygon_convex_hull_3d",
        "caption": "3D convex hull (XY-plane, Z from first hull vertex)",
        "function": generate_convex_hull,
    },
    {
        "heading": "translate_polygon_3d",
        "caption": "3D polygon translated by dx, dy, dz",
        "function": generate_translate,
    },
    {
        "heading": "scale_polygon_3d",
        "caption": "3D polygon scaled uniformly",
        "function": generate_scale,
    },
    {
        "heading": "flip_polygon_3d",
        "caption": "3D polygon flipped horizontally and along Z",
        "function": generate_flip,
    },
    {
        "heading": "rotate_polygon_3d",
        "caption": "3D polygon rotated around Z axis (Z preserved)",
        "function": generate_rotate,
    },
    {
        "heading": "offset_polyline_3d",
        "caption": "True 3D polyline offset (edge-plane miter)",
        "function": generate_true_offset,
    },
    {
        "heading": "get_polyline_end_tangent_3d",
        "caption": "Normalised end tangent direction of a 3D polyline",
        "function": generate_end_tangent,
    },
    {
        "heading": "fillet_polyline_3d",
        "caption": "Fillet corners of a 3D polyline with circular arcs",
        "function": generate_fillet_polyline_3d,
    },
    {
        "heading": "get_polygon_area_3d",
        "caption": "XY-projected (unsigned) area of a 3D polygon",
        "function": generate_area,
    },
    {
        "heading": "get_polygon_signed_area_3d",
        "caption": "Signed XY-projected area — positive = CCW, negative = CW",
        "function": generate_signed_area,
    },
    {
        "heading": "walk_along_polygon_3d",
        "caption": "Walk along a closed 3D polygon (wraps around)",
        "function": generate_walk_along_polygon,
    },
    {
        "heading": "walk_along_polyline_3d",
        "caption": "Walk along a 3D polyline by a given arc length",
        "function": generate_walk_along,
    },
    {
        "heading": "get_polygons_closest_point_3d",
        "caption": "Closest point on multiple 3D polygons",
        "function": generate_closest_point_3d,
    },
]
