"""Generate point operation example images."""

import math

import matplotlib.pyplot as plt

from raygeo.geo.shape.point import circumcenter, midpoint


def _setup_3d_ax(title, elev=25, azim=-65):
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection="3d")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title(title, fontsize=14)
    ax.view_init(elev=elev, azim=azim)
    fig.tight_layout()
    return fig, ax


def _draw_circle_3d(ax, center, radius, normal, n=64, **kwargs):
    """Draw a 3D circle given its center, radius, and normal vector."""
    # Build a local 2D basis in the circle plane
    ref = (1.0, 0.0, 0.0)
    if abs(normal[0]) > 0.9:
        ref = (0.0, 1.0, 0.0)
    # First basis vector: cross(ref, normal)
    ex = (
        ref[1] * normal[2] - ref[2] * normal[1],
        ref[2] * normal[0] - ref[0] * normal[2],
        ref[0] * normal[1] - ref[1] * normal[0],
    )
    elen = math.sqrt(ex[0] ** 2 + ex[1] ** 2 + ex[2] ** 2)
    ex = (ex[0] / elen, ex[1] / elen, ex[2] / elen)
    # Second basis vector: cross(normal, ex)
    ey = (
        normal[1] * ex[2] - normal[2] * ex[1],
        normal[2] * ex[0] - normal[0] * ex[2],
        normal[0] * ex[1] - normal[1] * ex[0],
    )
    pts = []
    for i in range(n + 1):
        a = 2.0 * math.pi * i / n
        c, s = math.cos(a), math.sin(a)
        pts.append(
            (
                center[0] + radius * (c * ex[0] + s * ey[0]),
                center[1] + radius * (c * ex[1] + s * ey[1]),
                center[2] + radius * (c * ex[2] + s * ey[2]),
            )
        )
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    zs = [p[2] for p in pts]
    ax.plot(xs, ys, zs, **kwargs)


def generate_circumcenter():
    """Circumcenter of three 3D points."""
    a = (0.0, 0.0, 0.0)
    b = (6.0, 0.0, 1.0)
    c = (2.0, 5.0, 3.0)
    center = circumcenter(a, b, c)
    assert center is not None

    r = math.sqrt(
        (center[0] - a[0]) ** 2
        + (center[1] - a[1]) ** 2
        + (center[2] - a[2]) ** 2
    )
    # Plane normal
    ab = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
    ac = (c[0] - a[0], c[1] - a[1], c[2] - a[2])
    n = (
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    )
    n_len = math.sqrt(n[0] ** 2 + n[1] ** 2 + n[2] ** 2)
    n = (n[0] / n_len, n[1] / n_len, n[2] / n_len)

    fig, ax = _setup_3d_ax(
        f"3D Circumcenter ({center[0]:.2f}, {center[1]:.2f}, {center[2]:.2f})"
    )
    _draw_circle_3d(ax, center, r, n, color="tomato", linewidth=2, alpha=0.5)

    pts = [a, b, c]
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    zs = [p[2] for p in pts]
    ax.plot(
        xs + [xs[0]],
        ys + [ys[0]],
        zs + [zs[0]],
        "o-",
        color="steelblue",
        linewidth=2,
        markersize=8,
        label="Triangle",
    )
    ax.plot(
        [center[0]],
        [center[1]],
        [center[2]],
        "o",
        color="limegreen",
        markersize=10,
        label="Circumcenter",
    )
    # Radii lines
    for p in pts:
        ax.plot(
            [center[0], p[0]],
            [center[1], p[1]],
            [center[2], p[2]],
            color="gray",
            linewidth=1,
            linestyle="--",
        )
    ax.legend(loc="upper left")
    return fig


def generate_midpoint():
    """Midpoint between two 3D points."""
    a = (0.0, 0.0, 0.0)
    b = (8.0, 6.0, 4.0)
    m = midpoint(a, b)
    fig, ax = _setup_3d_ax(f"3D Midpoint ({m[0]:.1f}, {m[1]:.1f}, {m[2]:.1f})")
    ax.plot(
        [a[0], b[0]],
        [a[1], b[1]],
        [a[2], b[2]],
        "o-",
        color="steelblue",
        linewidth=2,
        markersize=8,
        label="Segment",
    )
    ax.plot(
        [m[0]],
        [m[1]],
        [m[2]],
        "o",
        color="limegreen",
        markersize=10,
        label="Midpoint",
    )
    ax.legend(loc="upper left")
    return fig


__docs_target__ = ["raygeo.geo.shape.point.md"]
__images__ = [
    {
        "heading": "circumcenter",
        "caption": "Circumcenter of three 3D points with circumcircle",
        "function": generate_circumcenter,
    },
    {
        "heading": "midpoint",
        "caption": "Midpoint of a 3D segment",
        "function": generate_midpoint,
    },
]
