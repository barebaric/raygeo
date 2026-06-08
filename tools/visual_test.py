"""Visual test playground for raygeo.

Run with: make visual  (or: streamlit run tools/visual_test.py)
"""

import math

import matplotlib.pyplot as plt
import numpy as np
import streamlit as st
from raygeo.geo.shape.polygon import (
    get_polygons_difference,
    get_polygons_intersection,
    get_polygons_union,
    offset_polygon,
)

from raygeo import Geometry


def _plot_geometry(
    axes,
    geom,
    color="steelblue",
    label=None,
    show_points=False,
    linewidth=1.5,
):
    data = geom.data
    if data is None:
        return
    x = data[:, Geometry.COL_X]
    y = data[:, Geometry.COL_Y]
    types = data[:, Geometry.COL_TYPE].astype(int)
    move_mask = types == Geometry.CMD_TYPE_MOVE
    line_mask = types == Geometry.CMD_TYPE_LINE
    arc_mask = types == Geometry.CMD_TYPE_ARC
    bez_mask = types == Geometry.CMD_TYPE_BEZIER

    for i in range(len(data)):
        px, py = x[i], y[i]
        if i == 0:
            if show_points:
                axes.plot(px, py, "o", color=color, markersize=3)
            continue
        ppx, ppy = x[i - 1], y[i - 1]
        if show_points:
            axes.plot(px, py, "o", color=color, markersize=3)
        if move_mask[i]:
            continue
        if line_mask[i]:
            axes.plot(
                [ppx, px],
                [ppy, py],
                color=color,
                linewidth=linewidth,
                label=label,
            )
            label = None
        elif arc_mask[i]:
            ci = data[i, Geometry.COL_I]
            cj = data[i, Geometry.COL_J]
            cx = ppx + ci
            cy = ppy + cj
            cw = bool(data[i, Geometry.COL_CW])
            r = math.sqrt(ci**2 + cj**2)
            a_start = math.atan2(ppy - cy, ppx - cx)
            a_end = math.atan2(py - cy, px - cx)
            angles = _arc_angles(a_start, a_end, cw)
            ax_pts = [cx + r * math.cos(a) for a in angles]
            ay_pts = [cy + r * math.sin(a) for a in angles]
            axes.plot(ax_pts, ay_pts, color=color, linewidth=linewidth, label=label)
            label = None
        elif bez_mask[i]:
            c1x = data[i, Geometry.COL_I]
            c1y = data[i, Geometry.COL_J]
            c2x = data[i, Geometry.COL_CW]
            c2y = data[i, Geometry.COL_C2Y]
            ts = np.linspace(0, 1, 64)
            bx = (
                (1 - ts) ** 3 * ppx
                + 3 * (1 - ts) ** 2 * ts * c1x
                + 3 * (1 - ts) * ts**2 * c2x
                + ts**3 * px
            )
            by = (
                (1 - ts) ** 3 * ppy
                + 3 * (1 - ts) ** 2 * ts * c1y
                + 3 * (1 - ts) * ts**2 * c2y
                + ts**3 * py
            )
            axes.plot(bx, by, color=color, linewidth=linewidth, label=label)
            label = None


def _arc_angles(a_start, a_end, clockwise):
    diff = a_end - a_start
    if clockwise:
        if diff >= 0:
            diff -= 2 * math.pi
    else:
        if diff <= 0:
            diff += 2 * math.pi
    n = max(32, int(abs(diff) * 16))
    return [a_start + diff * i / n for i in range(n + 1)]


def _auto_limits(geoms):
    xs, ys = [], []
    for g in geoms:
        r = g.rect()
        if r:
            xs += [r[0], r[2]]
            ys += [r[1], r[3]]
        else:
            d = g.data
            if d is not None:
                xs += [d[:, Geometry.COL_X].min(), d[:, Geometry.COL_X].max()]
                ys += [d[:, Geometry.COL_Y].min(), d[:, Geometry.COL_Y].max()]
    if not xs:
        return -10, 10, -10, 10
    pad = max((max(xs) - min(xs)), (max(ys) - min(ys))) * 0.1 + 1
    return min(xs) - pad, max(xs) + pad, min(ys) - pad, max(ys) + pad


def page_geometry():
    st.header("Geometry Playground")

    tab_build, tab_ops, tab_analyze, tab_fit = st.tabs(
        ["Build", "Transform", "Analyze", "Curve Fitting"]
    )

    with tab_build:
        shape = st.selectbox(
            "Shape preset",
            [
                "Rectangle",
                "Circle (linearized)",
                "Polygon (regular)",
                "Star",
                "Custom path",
            ],
        )

        geom = Geometry()

        if shape == "Rectangle":
            c1, c2 = st.columns(2)
            w = c1.number_input("Width", 0.1, 1000.0, 10.0)
            h = c2.number_input("Height", 0.1, 1000.0, 10.0)
            geom = Geometry.from_points([(0, 0), (w, 0), (w, h), (0, h)])

        elif shape == "Circle (linearized)":
            c1, c2 = st.columns(2)
            r = c1.number_input("Radius", 0.1, 500.0, 10.0)
            n = c2.number_input("Segments", 3, 360, 64)
            geom.move_to(r, 0)
            for i in range(1, n + 1):
                a = 2 * math.pi * i / n
                geom.line_to(r * math.cos(a), r * math.sin(a))

        elif shape == "Polygon (regular)":
            c1, c2 = st.columns(2)
            r = c1.number_input("Radius", 0.1, 500.0, 10.0)
            n = c2.number_input("Sides", 3, 64, 6, step=1)
            geom = Geometry.from_points(
                [
                    (
                        r * math.cos(2 * math.pi * i / n),
                        r * math.sin(2 * math.pi * i / n),
                    )
                    for i in range(n)
                ]
            )

        elif shape == "Star":
            c1, c2 = st.columns(2)
            r = c1.number_input("Outer radius", 0.1, 500.0, 10.0)
            ri = c2.number_input("Inner radius", 0.1, 500.0, 4.0)
            points = c1.number_input("Points", 3, 64, 5, step=1)
            coords = []
            for i in range(points * 2):
                a = math.pi / 2 + math.pi * i / points
                rd = r if i % 2 == 0 else ri
                coords.append((rd * math.cos(a), rd * math.sin(a)))
            geom = Geometry.from_points(coords)

        elif shape == "Custom path":
            pts_text = st.text_area(
                "Points (one per line: x,y)",
                "0,0\n10,0\n10,10\n0,10",
            )
            close = st.checkbox("Close path", value=True)
            pts = []
            for line in pts_text.strip().splitlines():
                line = line.strip()
                if not line:
                    continue
                parts = line.split(",")
                pts.append((float(parts[0]), float(parts[1])))
            if pts:
                geom = Geometry.from_points(pts, close=close)

    with tab_ops:
        op = st.selectbox(
            "Operation",
            [
                "None",
                "Grow (offset outward)",
                "Shrink (offset inward)",
                "Flip X",
                "Flip Y",
                "Simplify",
                "Linearize",
                "Split contours",
                "Remove inner edges",
            ],
        )
        if op == "Grow (offset outward)":
            amount = st.number_input("Offset amount", 0.01, 100.0, 1.0)
            geom = geom.grow(amount)
        elif op == "Shrink (offset inward)":
            amount = st.number_input("Offset amount", 0.01, 100.0, 1.0)
            geom = geom.grow(-amount)
        elif op == "Flip X":
            geom.flip_x()
        elif op == "Flip Y":
            geom.flip_y()
        elif op == "Simplify":
            tol = st.number_input("Tolerance", 0.001, 100.0, 0.1)
            geom.simplify(tol)
        elif op == "Linearize":
            tol = st.number_input("Tolerance", 0.001, 100.0, 0.1)
            geom.linearize(tol)
        elif op == "Split contours":
            contours = geom.split_into_contours()
            st.info(f"Split into {len(contours)} contour(s)")
            fig, ax = plt.subplots()
            colors = plt.cm.tab10.colors
            for i, c in enumerate(contours):
                lbl = f"Contour {i}"
                _plot_geometry(
                    ax,
                    c,
                    color=colors[i % len(colors)],
                    label=lbl,
                )
            xmin, xmax, ymin, ymax = _auto_limits(contours)
            ax.set_xlim(xmin, xmax)
            ax.set_ylim(ymin, ymax)
            ax.set_aspect("equal")
            ax.grid(True, alpha=0.3)
            ax.legend()
            st.pyplot(fig)
            return

    with tab_analyze:
        st.subheader("Properties")
        cols = st.columns(4)
        cols[0].metric("Commands", len(geom))
        cols[1].metric("Area", f"{geom.area():.4f}")
        cols[2].metric("Distance", f"{geom.distance():.4f}")
        r = geom.rect()
        if r:
            bounds = f"({r[0]:.1f}, {r[1]:.1f}) - ({r[2]:.1f}, {r[3]:.1f})"
            cols[3].metric("Bounds", bounds)
        st.checkbox("Closed", value=geom.is_closed(), disabled=True, key="closed")

    with tab_fit:
        fit_op = st.selectbox(
            "Fit operation",
            [
                "Fit arcs",
                "Fit curves (arcs + beziers)",
            ],
        )
        tol = st.number_input("Tolerance", 0.001, 100.0, 0.5)
        if fit_op == "Fit arcs":
            geom.fit_arcs(tol)
        else:
            geom.fit_curves(tol, beziers=True, arcs=True)

    fig, ax = plt.subplots()
    show_pts = st.checkbox("Show control points", value=False, key="show_pts_build")
    _plot_geometry(ax, geom, show_points=show_pts)
    xmin, xmax, ymin, ymax = _auto_limits([geom])
    ax.set_xlim(xmin, xmax)
    ax.set_ylim(ymin, ymax)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    st.pyplot(fig)

    with st.expander("Raw data"):
        d = geom.data
        if d is not None:
            st.dataframe(d, use_container_width=True)


def page_polygon_boolean():
    st.header("Polygon Boolean Operations")

    c1, c2 = st.columns(2)
    with c1:
        st.subheader("Shape A (polygon)")
        a_type = st.selectbox("Shape A type", ["Square", "Circle"])
        a_r = st.number_input("A: radius / half-size", 0.5, 100.0, 10.0, key="a_r")
    with c2:
        st.subheader("Shape B (polygon)")
        b_type = st.selectbox(
            "Shape B type",
            ["Square", "Circle"],
            key="b_type",
        )
        b_r = st.number_input("B: radius / half-size", 0.5, 100.0, 8.0, key="b_r")

    n_seg = st.number_input("Circle segments", 3, 360, 64, step=1, key="bool_n")

    dx = st.number_input("B offset X", -50.0, 50.0, 6.0)
    dy = st.number_input("B offset Y", -50.0, 50.0, 0.0)

    def _make_circle(r, n, ox=0.0, oy=0.0):
        return [
            (
                ox + r * math.cos(2 * math.pi * i / n),
                oy + r * math.sin(2 * math.pi * i / n),
            )
            for i in range(n)
        ]

    def _make_square(r, ox=0.0, oy=0.0):
        return [(ox - r, oy - r), (ox + r, oy - r), (ox + r, oy + r), (ox - r, oy + r)]

    a = _make_circle(a_r, n_seg) if a_type == "Circle" else _make_square(a_r)
    b_raw = _make_circle(b_r, n_seg) if b_type == "Circle" else _make_square(b_r)
    b = [(x + dx, y + dy) for x, y in b_raw]

    op = st.selectbox("Operation", ["Union", "Intersection", "Difference"])

    if op == "Union":
        result = get_polygons_union([a, b])
    elif op == "Intersection":
        result = get_polygons_intersection(a, b)
    else:
        result = get_polygons_difference(a, b)

    fig, ax = plt.subplots()
    _plot_polygon(ax, a, "steelblue", "A")
    _plot_polygon(ax, b, "tomato", "B")
    if result:
        _plot_polygon(ax, result[0], "limegreen", "Result", linewidth=2.5)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend()
    st.pyplot(fig)


def _plot_polygon(ax, pts, color, label, linewidth=1.5):
    if not pts:
        return
    xs = [p[0] for p in pts] + [pts[0][0]]
    ys = [p[1] for p in pts] + [pts[0][1]]
    ax.plot(xs, ys, color=color, linewidth=linewidth, label=label)


def page_offset():
    st.header("Polygon Offset")

    n_seg = st.number_input("Circle segments", 3, 360, 64, step=1, key="off_n")
    r = st.number_input("Shape radius", 0.5, 100.0, 10.0, key="off_r")
    amount = st.number_input("Offset amount", -50.0, 50.0, 2.0)

    pts = [
        (r * math.cos(2 * math.pi * i / n_seg), r * math.sin(2 * math.pi * i / n_seg))
        for i in range(n_seg)
    ]

    result = offset_polygon(pts, amount)

    fig, ax = plt.subplots()
    _plot_polygon(ax, pts, "steelblue", "Original")
    for i, poly in enumerate(result):
        _plot_polygon(ax, poly, "limegreen", f"Offset {i}", linewidth=2.5)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend()
    st.pyplot(fig)


def page_image():
    st.header("Image Processing")

    import raygeo.image as img

    c1, c2 = st.columns(2)
    with c1:
        w = st.number_input("Width", 8, 1024, 128, step=8, key="img_w")
        h = st.number_input("Height", 8, 1024, 128, step=8, key="img_h")
    with c2:
        pattern = st.selectbox(
            "Test pattern",
            [
                "Gradient",
                "Checkered",
                "Circle",
                "Random noise",
            ],
        )

    arr = _make_pattern(w, h, pattern)

    fig, axes = plt.subplots(1, 2, figsize=(10, 4))

    axes[0].imshow(arr, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original (uint8)")

    linear = img.srgb_to_linear(arr)
    back = img.linear_to_srgb(linear)
    axes[1].imshow(back, cmap="gray", vmin=0, vmax=255)
    axes[1].set_title("Round-trip (sRGB -> linear -> sRGB)")

    st.pyplot(fig)

    st.subheader("Dithering")
    dither = st.selectbox("Dither method", ["Floyd-Steinberg", "Bayer 4x4"])
    invert = st.checkbox("Invert", value=False)

    gray = img.normalize_grayscale(arr).astype(np.uint8)
    if dither == "Floyd-Steinberg":
        dithered = img.apply_floyd_steinberg_dither(gray, invert)
    else:
        bayer = np.array(
            [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
            dtype=np.float64,
        )
        dithered = img.apply_bayer_dither(gray, bayer, invert, cell_size=1)

    fig2, axes2 = plt.subplots(1, 2, figsize=(10, 4))
    axes2[0].imshow(arr, cmap="gray", vmin=0, vmax=255)
    axes2[0].set_title("Original")
    axes2[1].imshow(dithered, cmap="gray", vmin=0, vmax=1)
    axes2[1].set_title(f"Dithered ({dither})")
    st.pyplot(fig2)


def _make_pattern(w, h, pattern):
    x = np.arange(w, dtype=np.float64)
    y = np.arange(h, dtype=np.float64)
    xx, yy = np.meshgrid(x, y)

    if pattern == "Gradient":
        arr = ((xx / w) * 255).astype(np.uint8)
    elif pattern == "Checkered":
        block = 16
        checker = ((xx // block) + (yy // block)) % 2 == 0
        arr = np.where(checker, 255, 0).astype(np.uint8)
    elif pattern == "Circle":
        cx, cy = w / 2, h / 2
        dist = np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2)
        r = min(w, h) / 2 * 0.8
        arr = np.where(dist < r, 255, 0).astype(np.uint8)
    else:
        rng = np.random.default_rng(42)
        arr = rng.integers(0, 256, (h, w), dtype=np.uint8)

    return arr


def page_svg():
    st.header("SVG Parsing")

    sample = st.selectbox(
        "Sample SVG path",
        [
            "Rectangle + Circle",
            "Star path",
            "Custom (paste below)",
        ],
    )

    n_circle = 48
    circle_pts = " ".join(
        f"L {50 + 20 * math.cos(2 * math.pi * i / n_circle):.1f}"
        f" {50 + 20 * math.sin(2 * math.pi * i / n_circle):.1f}"
        for i in range(1, n_circle + 1)
    )
    rect_circle = f"M 10 10 L 90 10 L 90 90 L 10 90 Z M 50 50 {circle_pts} Z"
    star = (
        "M 50 5 L 61 35 L 95 35 L 68 57 L 79 91"
        " L 50 70 L 21 91 L 32 57 L 5 35 L 39 35 Z"
    )

    if sample == "Rectangle + Circle":
        path_data = rect_circle
    elif sample == "Star path":
        path_data = star
    else:
        path_data = st.text_area(
            "SVG path data",
            "M 0 0 L 100 0 L 100 100 Z",
        )

    from raygeo.svg import parse_svg_path_data

    geoms = parse_svg_path_data(path_data)

    fig, ax = plt.subplots()
    colors = plt.cm.tab10.colors
    for i, g in enumerate(geoms):
        _plot_geometry(ax, g, color=colors[i % len(colors)], label=f"Path {i}")
    xmin, xmax, ymin, ymax = _auto_limits(geoms) if geoms else (0, 100, 0, 100)
    ax.set_xlim(xmin, xmax)
    ax.set_ylim(ymin, ymax)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    if geoms:
        ax.legend()
    st.pyplot(fig)


st.set_page_config(layout="wide", page_title="raygeo visual test")
st.title("raygeo Visual Test")

page = st.sidebar.radio(
    "Page",
    [
        "Geometry",
        "Polygon Boolean",
        "Polygon Offset",
        "Image Processing",
        "SVG Parsing",
    ],
)

if page == "Geometry":
    page_geometry()
elif page == "Polygon Boolean":
    page_polygon_boolean()
elif page == "Polygon Offset":
    page_offset()
elif page == "Image Processing":
    page_image()
elif page == "SVG Parsing":
    page_svg()
