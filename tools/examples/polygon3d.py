"""Generate 3D polygon operation example images."""

import math

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import art3d

from raygeo.geo.shape.polygon3d import (
    flip_polygon_3d,
    get_polygon_bounds_3d,
    get_polygon_centroid_3d,
    get_polygon_convex_hull_3d,
    get_polygon_edges_3d,
    get_polygon_perimeter_3d,
    get_polygons_difference_3d,
    get_polygons_intersection_3d,
    get_polygons_union_3d,
    offset_polygon_3d,
    rotate_polygon_3d,
    scale_polygon_3d,
    translate_polygon_3d,
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
    return [(x, y, z) for x, y in poly]


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


def _plot_3d_boolean(a, b, result, title, output_path):
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
    fig.savefig(output_path, dpi=150)
    plt.close(fig)


def generate_examples(output_dir):
    images = []
    n_seg = 64
    r = 12

    # --- 3D Union ---
    a_xy = _make_circle(r, n_seg, ox=-2, oy=0)
    b_xy = _make_square(r, ox=2, oy=0)
    a = _lift(a_xy, 3)
    b = _lift(b_xy, 7)
    result = get_polygons_union_3d([a, b])
    path = output_dir / "polygon3d-boolean-union.png"
    _plot_3d_boolean(a, b, result, "3D Union (Z from A)", path)
    images.append(
        {
            "path": "polygon3d-boolean-union.png",
            "caption": "3D polygon union — Z from first polygon",
        }
    )

    # --- 3D Intersection ---
    a_xy = _make_circle(r, n_seg, ox=-2, oy=0)
    b_xy = _make_square(r, ox=2, oy=0)
    a = _lift(a_xy, 3)
    b = _lift(b_xy, 7)
    result = get_polygons_intersection_3d(a, b)
    path = output_dir / "polygon3d-boolean-intersection.png"
    _plot_3d_boolean(a, b, result, "3D Intersection (Z from A)", path)
    images.append(
        {
            "path": "polygon3d-boolean-intersection.png",
            "caption": "3D polygon intersection — Z from first polygon",
        }
    )

    # --- 3D Difference ---
    a_xy = _make_circle(r, n_seg, ox=-2, oy=0)
    b_xy = _make_square(r, ox=2, oy=0)
    a = _lift(a_xy, 3)
    b = _lift(b_xy, 7)
    result = get_polygons_difference_3d(a, b)
    path = output_dir / "polygon3d-boolean-difference.png"
    _plot_3d_boolean(a, b, result, "3D Difference (Z from A)", path)
    images.append(
        {
            "path": "polygon3d-boolean-difference.png",
            "caption": "3D polygon difference (A − B) — Z from A",
        }
    )

    # --- 3D Offset ---
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
    path = output_dir / "polygon3d-offset.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-offset.png",
            "caption": "3D polygon offset — Z preserved from input",
        }
    )

    # --- Perimeter ---
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
    path = output_dir / "polygon3d-perimeter.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-perimeter.png",
            "caption": f"Perimeter = {perim:.1f} (labeled 3D edge lengths)",
        }
    )

    # --- Bounds ---
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
    path = output_dir / "polygon3d-bounds.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-bounds.png",
            "caption": "3D bounding box (Rect3D): min/max X, Y, Z",
        }
    )

    # --- Centroid ---
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
    path = output_dir / "polygon3d-centroid.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-centroid.png",
            "caption": f"Centroid ({cx:.1f}, {cy:.1f}, {cz:.1f})",
        }
    )

    # --- Edges ---
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
    path = output_dir / "polygon3d-edges.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-edges.png",
            "caption": "3D polygon edges as (start, end) pairs",
        }
    )

    # --- Convex Hull ---
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
    path = output_dir / "polygon3d-convex-hull.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-convex-hull.png",
            "caption": "3D convex hull (XY-plane, Z from first hull vertex)",
        }
    )

    # --- Translate ---
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
    path = output_dir / "polygon3d-translate.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-translate.png",
            "caption": "3D polygon translated by dx=5, dy=3, dz=2",
        }
    )

    # --- Scale ---
    poly = _lift(_make_square(6), 5)
    scaled = scale_polygon_3d(poly, 1.5)
    fig, ax = _make_3d_ax("3D Polygon Scale (uniform)")
    _draw_polygon3d(ax, poly, "steelblue", "Original", alpha=0.2)
    _draw_polygon3d(
        ax, scaled, "tomato", "Scaled ×1.5", linewidth=3, alpha=0.4
    )
    ax.legend(loc="upper left")
    path = output_dir / "polygon3d-scale.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-scale.png",
            "caption": "3D polygon scaled uniformly",
        }
    )

    # --- Flip ---
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
    path = output_dir / "polygon3d-flip.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-flip.png",
            "caption": "3D polygon flipped in X (flip_h) and Z (flip_z)",
        }
    )

    # --- Rotate ---
    poly = _lift(_make_square(6), 5)
    rotated = rotate_polygon_3d(poly, 45.0)
    fig, ax = _make_3d_ax("3D Polygon Rotate (Z-axis, Z preserved)")
    _draw_polygon3d(ax, poly, "steelblue", "Original (Z=5)", alpha=0.2)
    _draw_polygon3d(
        ax, rotated, "tomato", "Rotated 45°", linewidth=3, alpha=0.4
    )
    ax.legend(loc="upper left")
    path = output_dir / "polygon3d-rotate.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-rotate.png",
            "caption": "3D polygon rotated around Z axis (Z preserved)",
        }
    )

    return {
        "title": "3D Polygon Operations",
        "description": (
            "Boolean, offset, analytical, and transform operations on 3D "
            "polygons. Boolean/offset operations project to XY, run the 2D "
            "algorithm, then lift the result back to the input Z. "
            "Analytical functions (perimeter, bounds, centroid, edges, convex "
            "hull) compute meaningful 3D values. Transform functions "
            "(translate, scale, flip, rotate) operate on all three axes."
        ),
        "images": images,
    }
