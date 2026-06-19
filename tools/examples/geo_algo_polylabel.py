"""Visualise pole-of-inaccessibility (Polylabel) detection."""

import math

import matplotlib.pyplot as plt
from matplotlib.patches import Circle, Rectangle

from raygeo.geo.algo.offset import offset_contour_group
from raygeo.geo.algo.polylabel import find_largest_circle, polylabel
from raygeo.geo.shape.polygon import (
    get_polygon_closest_point,
    get_polygon_signed_area,
    is_point_inside_polygon,
    point_line_distance,
)


def _signed_dist(pt, poly):
    inside = is_point_inside_polygon(pt, poly)
    n = len(poly)
    md = float("inf")
    for i in range(n):
        d = point_line_distance(pt, poly[i], poly[(i + 1) % n])
        md = min(md, d)
    return md if inside else -md


def _cell_grid(boundary):
    """Yield (x, y, half_size) for debugging — shows the cell hierarchy."""
    x_min = min(p[0] for p in boundary)
    x_max = max(p[0] for p in boundary)
    y_min = min(p[1] for p in boundary)
    y_max = max(p[1] for p in boundary)
    w = x_max - x_min
    h = y_max - y_min
    cell_size = max(w, h) / 16.0
    cell_radius = cell_size / math.sqrt(2)

    cells = []
    y = y_min + cell_size * 0.5
    while y < y_max:
        x = x_min + cell_size * 0.5
        while x < x_max:
            d = _signed_dist((x, y), boundary)
            if d >= 0:
                cells.append((x, y, cell_radius, d))
            x += cell_size
        y += cell_size

    yield cells  # initial grid

    # Subdivide some layers for visualisation
    for _layer in range(2):
        next_cells = []
        for cx, cy, hr, dist in cells:
            off = hr * 0.5
            h2 = hr * 0.5
            for dx, dy in [(-off, -off), (-off, off), (off, -off), (off, off)]:
                nx, ny = cx + dx, cy + dy
                d = _signed_dist((nx, ny), boundary)
                if d + h2 >= 0:
                    next_cells.append((nx, ny, h2, d))
        yield next_cells
        cells = next_cells


def generate_rect_lshape():
    """Polylabel rect L-shape."""
    rect = [(0, 0), (100, 0), (100, 80), (0, 80)]
    l_shape = [(0, 0), (100, 0), (100, 40), (40, 40), (40, 80), (0, 80)]

    r_pole = polylabel(rect, holes=[], precision=0.1)
    l_pole = polylabel(l_shape, holes=[], precision=0.5)

    fig1, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Left: rectangle
    arr = list(rect) + [rect[0]]
    ax1.plot(*zip(*arr), "k-", linewidth=2, label="Boundary")
    for layer in _cell_grid(rect):
        for cx, cy, hr, _d in layer:
            r = hr * 1.414
            rect_patch = Rectangle(
                (cx - r / 2, cy - r / 2),
                r,
                r,
                fill=False,
                edgecolor="steelblue",
                alpha=0.25,
                linewidth=0.5,
            )
            ax1.add_patch(rect_patch)
    if r_pole:
        ax1.plot(r_pole[0], r_pole[1], "r*", markersize=18, label="Pole")
    ax1.set_title("Rectangle — Pole of Inaccessibility")
    ax1.set_xlabel("X")
    ax1.set_ylabel("Y")
    ax1.set_aspect("equal")
    ax1.legend(fontsize=8)
    ax1.grid(True, alpha=0.3)

    # Right: L-shape
    arr2 = list(l_shape) + [l_shape[0]]
    ax2.plot(*zip(*arr2), "k-", linewidth=2, label="Boundary")
    for layer in _cell_grid(l_shape):
        for cx, cy, hr, _d in layer:
            r = hr * 1.414
            rect_patch = Rectangle(
                (cx - r / 2, cy - r / 2),
                r,
                r,
                fill=False,
                edgecolor="steelblue",
                alpha=0.25,
                linewidth=0.5,
            )
            ax2.add_patch(rect_patch)
    if l_pole:
        ax2.plot(l_pole[0], l_pole[1], "r*", markersize=18, label="Pole")
    ax2.set_title("L-Shaped Pocket — Pole of Inaccessibility")
    ax2.set_xlabel("X")
    ax2.set_ylabel("Y")
    ax2.set_aspect("equal")
    ax2.legend(fontsize=8)
    ax2.grid(True, alpha=0.3)

    fig1.tight_layout()
    return fig1


def generate_multi_island():
    """Polylabel multi-island."""
    mb = [(0, 0), (160, 0), (160, 100), (0, 100)]
    isl1 = [(30, 20), (50, 20), (50, 40), (30, 40)]
    isl2 = [(110, 60), (130, 60), (130, 80), (110, 80)]
    m_area = offset_contour_group(mb, [isl1, isl2], -5.0, join_style="round")

    # Union all valid-area fragments so holes are properly subtracted
    # Take the largest valid region by absolute area
    m_largest = (
        max(m_area, key=lambda p: abs(get_polygon_signed_area(p)))
        if m_area
        else None
    )
    m_pole = (
        polylabel(m_largest, holes=[], precision=0.5) if m_largest else None
    )

    fig2, ax = plt.subplots(figsize=(7, 6))

    mb_arr = list(mb) + [mb[0]]
    ax.plot(*zip(*mb_arr), "k-", linewidth=2, label="Boundary")
    for i, isl in enumerate([isl1, isl2]):
        arr = list(isl) + [isl[0]]
        ax.fill(
            *zip(*arr),
            facecolor="#ddd",
            edgecolor="#999",
            linewidth=1.5,
            label="Island" if i == 0 else None,
        )
    for i, poly in enumerate(m_area):
        arr = list(poly) + [poly[0]]
        ax.plot(
            *zip(*arr),
            "--",
            color="steelblue",
            alpha=0.7,
            linewidth=1.5,
            label="Valid area" if i == 0 else None,
        )
    if m_pole:
        ax.plot(m_pole[0], m_pole[1], "r*", markersize=18, label="Pole")
    ax.set_title("Multi-Island — Pole of Inaccessibility")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_aspect("equal")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)

    fig2.tight_layout()
    return fig2


def generate_central_island():
    """Polylabel central island."""
    cb = [(0, 0), (100, 0), (100, 100), (0, 100)]
    cisl = [(35, 35), (65, 35), (65, 65), (35, 65)]
    c_pole = polylabel(cb, holes=[cisl], precision=0.5)

    fig3, ax = plt.subplots(figsize=(7, 6))

    cb_arr = list(cb) + [cb[0]]
    ax.plot(*zip(*cb_arr), "k-", linewidth=2, label="Boundary")
    cisl_arr = list(cisl) + [cisl[0]]
    ax.fill(
        *zip(*cisl_arr),
        facecolor="#ddd",
        edgecolor="#999",
        linewidth=1.5,
        label="Island",
    )
    if c_pole:
        ax.plot(c_pole[0], c_pole[1], "r*", markersize=18, label="Pole")
    ax.set_title("Central Island — Pole of Inaccessibility")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_aspect("equal")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)

    fig3.tight_layout()
    return fig3


def generate_largest_circle():
    """Find largest circle."""
    rect = [(0, 0), (100, 0), (100, 80), (0, 80)]
    l_shape = [(0, 0), (100, 0), (100, 40), (40, 40), (40, 80), (0, 80)]
    mb = [(0, 0), (160, 0), (160, 100), (0, 100)]
    isl1 = [(30, 20), (50, 20), (50, 40), (30, 40)]
    isl2 = [(110, 60), (130, 60), (130, 80), (110, 80)]
    cb = [(0, 0), (100, 0), (100, 100), (0, 100)]
    cisl = [(35, 35), (65, 35), (65, 65), (35, 65)]

    r_circle = find_largest_circle(rect, holes=[], precision=0.1)
    l_circle = find_largest_circle(l_shape, holes=[], precision=0.5)
    m_circle = find_largest_circle(mb, holes=[isl1, isl2], precision=0.5)
    c_circle = find_largest_circle(cb, holes=[cisl], precision=0.5)

    fig4, ((ax7, ax8), (ax9, ax10)) = plt.subplots(2, 2, figsize=(14, 12))

    def draw_circle(ax, poly, circle_data, title, holes=None, islands=None):
        if (
            isinstance(poly, list)
            and len(poly) > 0
            and isinstance(poly[0], tuple)
        ):
            arr = list(poly) + [poly[0]]
            ax.plot(*zip(*arr), "k-", linewidth=2, label="Boundary")
        if islands:
            for isl in islands:
                iarr = list(isl) + [isl[0]]
                ax.fill(
                    *zip(*iarr),
                    facecolor="#ddd",
                    edgecolor="#999",
                    linewidth=1.5,
                )
        if holes:
            for h in holes:
                harr = list(h) + [h[0]]
                ax.fill(
                    *zip(*harr),
                    facecolor="#ddd",
                    edgecolor="#999",
                    linewidth=1.5,
                )
        if circle_data:
            (cx, cy), rad = circle_data
            c = Circle(
                (cx, cy),
                rad,
                fill=False,
                edgecolor="crimson",
                linewidth=2,
                linestyle="-",
                label="Inscribed circle",
            )
            ax.add_patch(c)
            ax.plot(cx, cy, "r*", markersize=18, label="Centre")
        ax.set_title(title)
        ax.set_xlabel("X")
        ax.set_ylabel("Y")
        ax.set_aspect("equal")
        ax.legend(fontsize=8)
        ax.grid(True, alpha=0.3)

    draw_circle(ax7, rect, r_circle, "Rectangle")
    draw_circle(ax8, l_shape, l_circle, "L-Shape")
    draw_circle(
        ax9,
        mb,
        m_circle,
        "Multi-Island Pocket",
        islands=[isl1, isl2],
    )
    draw_circle(ax10, cb, c_circle, "Central-Island Pocket", holes=[cisl])

    fig4.tight_layout()
    return fig4


def generate_closest_point():
    """Polygon closest point."""
    poly = [(10, 10), (90, 10), (90, 70), (10, 70)]
    test_points = [(50, 60), (30, 20), (120, 40), (50, 40), (120, 80)]

    fig5, ax9 = plt.subplots(figsize=(7, 6))

    arr = list(poly) + [poly[0]]
    ax9.plot(*zip(*arr), "k-", linewidth=2, label="Polygon")
    ax9.fill(*zip(*arr), facecolor="#eef", alpha=0.3)

    for pt in test_points:
        res = get_polygon_closest_point(poly, pt[0], pt[1])
        ax9.plot(pt[0], pt[1], "o", color="steelblue", markersize=8)
        if res:
            _t, (cx, cy), _d2 = res
            ax9.plot(cx, cy, "r*", markersize=10)
            ax9.plot(
                [pt[0], cx],
                [pt[1], cy],
                "-",
                color="crimson",
                alpha=0.5,
                linewidth=1,
            )

    ax9.plot([], [], "o", color="steelblue", label="Query point")
    ax9.plot([], [], "r*", markersize=10, label="Closest boundary point")
    ax9.plot([], [], "-", color="crimson", alpha=0.5, label="Distance")
    ax9.set_title("get_polygon_closest_point — Boundary Distance")
    ax9.set_xlabel("X")
    ax9.set_ylabel("Y")
    ax9.set_aspect("equal")
    ax9.legend(fontsize=8)
    ax9.grid(True, alpha=0.3)

    fig5.tight_layout()
    return fig5


__images__ = [
    {
        "heading": "polylabel",
        "caption": (
            "Polylabel: priority-queue cell refinement finds the point"
            " farthest from the boundary — the pole of inaccessibility"
        ),
        "function": generate_rect_lshape,
    },
    {
        "heading": "polylabel",
        "caption": (
            "Multi-island pocket: the pole of inaccessibility sits in"
            " the largest valid region, farthest from all boundaries"
        ),
        "function": generate_multi_island,
    },
    {
        "heading": "polylabel",
        "caption": (
            "Central-island pocket (annular): the pole of inaccessibility"
            " sits at the centre of the ring"
        ),
        "function": generate_central_island,
    },
    {
        "heading": "find_largest_circle",
        "caption": (
            "find_largest_circle returns the centre and radius of the"
            " largest inscribed circle — the entry point and its"
            " clearance for helical versus ramp decisions"
        ),
        "function": generate_largest_circle,
    },
    {
        "heading": "get_polygon_closest_point",
        "caption": (
            "get_polygon_closest_point finds the nearest boundary"
            " point to a given coordinate — used by find_largest_circle"
            " to compute the inscribed radius"
        ),
        "function": generate_closest_point,
    },
]
