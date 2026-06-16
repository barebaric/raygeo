"""Generate 3D polygon operation example images."""

import math

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import art3d

from raygeo.geo.shape.polygon3d import (
    get_polygons_difference_3d,
    get_polygons_intersection_3d,
    get_polygons_union_3d,
    offset_polygon_3d,
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
    """Draw a 3D polygon as a line on the given 3D axes."""
    xs = [p[0] for p in poly3d] + [poly3d[0][0]]
    ys = [p[1] for p in poly3d] + [poly3d[0][1]]
    zs = [p[2] for p in poly3d] + [poly3d[0][2]]
    ax.plot(xs, ys, zs, color=color, linewidth=linewidth, label=label)
    verts = [list(zip(xs, ys))]
    poly = mpatches.Polygon(verts[0], color=color, alpha=alpha)
    ax.add_patch(poly)
    art3d.pathpatch_2d_to_3d(poly, z=poly3d[0][2], zdir="z")


def _plot_3d_boolean(a, b, result, title, output_path):
    """Generate a 3D boolean operation visualization."""
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")
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
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title(title, fontsize=14)
    ax.legend(loc="upper left")
    ax.set_zlim(0, 15)
    ax.view_init(elev=20, azim=-90)
    fig.tight_layout()
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
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")
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
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title("3D Polygon Offset (Z preserved)", fontsize=14)
    ax.legend(loc="upper left")
    ax.set_zlim(0, 15)
    ax.view_init(elev=20, azim=-90)
    fig.tight_layout()
    path = output_dir / "polygon3d-offset.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "polygon3d-offset.png",
            "caption": "3D polygon offset — Z preserved from input",
        }
    )

    return {
        "title": "3D Polygon Operations",
        "description": (
            "Boolean and offset operations on 3D polygons. "
            "Operations project to XY, run the 2D algorithm, "
            "then lift the result back to the input Z."
        ),
        "images": images,
    }
