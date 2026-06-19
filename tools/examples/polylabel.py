"""Visualise pole-of-inaccessibility (Polylabel) detection."""

__images__ = [
    {
        "stem": "polylabel-rect-lshape",
        "caption": (
            "Polylabel: priority-queue cell refinement finds the point"
            " farthest from the boundary — the pole of inaccessibility"
        ),
        "doc": "raygeo.geo.algo.polylabel.md",
        "heading": "polylabel",
    },
    {
        "stem": "polylabel-multi-island",
        "caption": (
            "Multi-island pocket: the pole of inaccessibility sits in"
            " the largest valid region, farthest from all boundaries"
        ),
        "doc": "raygeo.geo.algo.polylabel.md",
        "heading": "polylabel",
    },
    {
        "stem": "polylabel-central-island",
        "caption": (
            "Central-island pocket (annular): the pole of inaccessibility"
            " sits at the centre of the ring"
        ),
        "doc": "raygeo.geo.algo.polylabel.md",
        "heading": "polylabel",
    },
]

import math

import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

from raygeo.geo.algo.offset import offset_contour_group
from raygeo.geo.algo.polylabel import polylabel
from raygeo.geo.shape.polygon import (
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


def generate_examples(output_dir):
    images = []

    # ----------------------------------------------------------------
    # Figure 1: rectangle (left) + L-shape (right)
    # ----------------------------------------------------------------
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
    path1 = output_dir / "polylabel-rect-lshape.png"
    fig1.savefig(path1, dpi=150)
    plt.close(fig1)
    images.append(
        {
            "path": "polylabel-rect-lshape.png",
            "caption": (
                "The pole of inaccessibility (red star) for a simple"
                " rectangle is its centre; for an L-shaped pocket it"
                " lands in the overlap region farthest from all edges."
            ),
        }
    )

    # ----------------------------------------------------------------
    # Figure 2: multi-island pocket
    # ----------------------------------------------------------------
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
    path2 = output_dir / "polylabel-multi-island.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "polylabel-multi-island.png",
            "caption": (
                "Of the three disconnected valid regions the largest"
                " (the middle band) contains the pole of inaccessibility"
                " — the safest helical-entry point for the whole pocket."
            ),
        }
    )

    # ----------------------------------------------------------------
    # Figure 3: central-island pocket (annular)
    # ----------------------------------------------------------------
    cb = [(0, 0), (100, 0), (100, 100), (0, 100)]
    cisl = [(35, 35), (65, 35), (65, 65), (35, 65)]
    c_area = offset_contour_group(cb, [cisl], -5.0, join_style="round")

    # Separate shell (CCW, positive area) from holes (CW, negative area)
    c_shell = None
    c_holes = []
    for p in c_area:
        sa = get_polygon_signed_area(p)
        if sa >= 0:
            c_shell = p
        else:
            c_holes.append(p)
    c_pole = (
        polylabel(c_shell, holes=c_holes, precision=0.5)
        if c_shell is not None
        else None
    )

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
    for poly in c_area:
        arr = list(poly) + [poly[0]]
        ax.plot(
            *zip(*arr),
            "--",
            color="steelblue",
            alpha=0.7,
            linewidth=1.5,
            label="Valid area" if poly is c_area[0] else None,
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
    path3 = output_dir / "polylabel-central-island.png"
    fig3.savefig(path3, dpi=150)
    plt.close(fig3)
    images.append(
        {
            "path": "polylabel-central-island.png",
            "caption": (
                "A central island creates an annular valid tool area."
                " The pole of inaccessibility sits at the centre of the"
                " ring — the single deepest accessible point."
            ),
        }
    )

    return {
        "title": "Pole of Inaccessibility (Polylabel)",
        "description": (
            "The Polylabel algorithm uses a priority-queue of grid cells"
            " to find the point inside a polygon that is farthest from"
            " its boundary — the ideal location for labels, helical-entry"
            " plunges, or starting-point selection in adaptive clearing."
        ),
        "images": images,
    }
