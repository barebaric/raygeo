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


def _plot_ops(
    axes,
    ops,
    color="steelblue",
    label=None,
    show_points=False,
    linewidth=1.5,
    show_travel=False,
    show_power=False,
):
    """Plot an Ops sequence, drawing lines/arcs/beziers and optionally
    showing travel moves and power state."""
    from raygeo.ops.types import CommandType

    ops.preload_state()
    last_pt = (0.0, 0.0, 0.0)
    seg_label = label
    draw_color = color
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.SET_POWER:
            continue
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if show_travel and last_pt != ep:
                axes.plot(
                    [last_pt[0], ep[0]],
                    [last_pt[1], ep[1]],
                    color="gray",
                    linewidth=0.5,
                    linestyle=":",
                )
            last_pt = ep
            if show_points:
                axes.plot(ep[0], ep[1], "o", color=draw_color, markersize=3)
            continue
        if ct not in (
            CommandType.LINE_TO,
            CommandType.BEZIER_TO,
            CommandType.ARC_TO,
        ):
            continue
        if show_power:
            st = ops.preloaded_state(i)
            if st is not None and st.power is not None:
                draw_color = plt.cm.RdYlGn(st.power)
            else:
                draw_color = color
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            axes.plot(
                [last_pt[0], ep[0]],
                [last_pt[1], ep[1]],
                color=draw_color,
                linewidth=linewidth,
                label=seg_label,
            )
            seg_label = None
            last_pt = ep
            if show_points:
                axes.plot(ep[0], ep[1], "o", color=draw_color, markersize=3)
            continue
        if ct == CommandType.BEZIER_TO:
            ep = ops.endpoint(i)
            info = ops.inspect(i)
            c1 = info.control1
            c2 = info.control2
            if c1 and c2:
                ts = np.linspace(0, 1, 64)
                bx = (
                    (1 - ts) ** 3 * last_pt[0]
                    + 3 * (1 - ts) ** 2 * ts * c1[0]
                    + 3 * (1 - ts) * ts**2 * c2[0]
                    + ts**3 * ep[0]
                )
                by = (
                    (1 - ts) ** 3 * last_pt[1]
                    + 3 * (1 - ts) ** 2 * ts * c1[1]
                    + 3 * (1 - ts) * ts**2 * c2[1]
                    + ts**3 * ep[1]
                )
                axes.plot(
                    bx,
                    by,
                    color=draw_color,
                    linewidth=linewidth,
                    label=seg_label,
                )
                seg_label = None
            last_pt = ep
            continue
        if ct == CommandType.ARC_TO:
            ep = ops.endpoint(i)
            info = ops.inspect(i)
            co = info.center_offset
            cw = info.clockwise
            if co:
                cx = last_pt[0] + co[0]
                cy = last_pt[1] + co[1]
                r = math.sqrt(co[0] ** 2 + co[1] ** 2)
                a_start = math.atan2(last_pt[1] - cy, last_pt[0] - cx)
                a_end = math.atan2(ep[1] - cy, ep[0] - cx)
                angles = _arc_angles(a_start, a_end, cw)
                ax_pts = [cx + r * math.cos(a) for a in angles]
                ay_pts = [cy + r * math.sin(a) for a in angles]
                axes.plot(
                    ax_pts,
                    ay_pts,
                    color=draw_color,
                    linewidth=linewidth,
                    label=seg_label,
                )
                seg_label = None
            last_pt = ep
            continue


def page_tabs():
    st.header("Tab Operations")

    from raygeo.ops import Ops
    from raygeo.ops.types import SectionType

    c1, c2 = st.columns(2)
    with c1:
        shape = st.selectbox(
            "Shape", ["Rectangle", "Circle", "Rounded Rect"], key="tab_shape"
        )
    with c2:
        mode = st.selectbox("Mode", ["Gap", "Power"], key="tab_mode")

    cx, cy = 10, 10
    if shape == "Rectangle":
        w = st.number_input("Width", 2.0, 100.0, 20.0, key="tab_w")
        h = st.number_input("Height", 2.0, 100.0, 20.0, key="tab_h")
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.set_power(1.0)
        ops.move_to(cx - w / 2, cy - h / 2, 0)
        ops.line_to(cx + w / 2, cy - h / 2, 0)
        ops.line_to(cx + w / 2, cy + h / 2, 0)
        ops.line_to(cx - w / 2, cy + h / 2, 0)
        ops.close_path()
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif shape == "Circle":
        r = st.number_input("Radius", 1.0, 50.0, 10.0, key="tab_r")
        n = st.number_input("Segments", 8, 128, 64, key="tab_n")
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.set_power(1.0)
        ops.move_to(cx + r, cy, 0)
        for i in range(1, n + 1):
            a = 2 * math.pi * i / n
            ops.line_to(cx + r * math.cos(a), cy + r * math.sin(a), 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    else:
        w = st.number_input("Width", 2.0, 100.0, 20.0, key="tab_w2")
        h = st.number_input("Height", 2.0, 100.0, 20.0, key="tab_h2")
        d = min(w, h) * 0.2
        k = 0.5522847498
        kd = k * d
        x0, y0 = cx - w / 2, cy - h / 2
        ops = Ops()
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.set_power(1.0)
        ops.move_to(x0 + d, y0, 0)
        ops.line_to(x0 + w - d, y0, 0)
        ops.bezier_to(
            (x0 + w - d + kd, y0, 0),
            (x0 + w, y0 + d - kd, 0),
            (x0 + w, y0 + d, 0),
        )
        ops.line_to(x0 + w, y0 + h - d, 0)
        ops.bezier_to(
            (x0 + w, y0 + h - d + kd, 0),
            (x0 + w - d + kd, y0 + h, 0),
            (x0 + w - d, y0 + h, 0),
        )
        ops.line_to(x0 + d, y0 + h, 0)
        ops.bezier_to(
            (x0 + d - kd, y0 + h, 0),
            (x0, y0 + h - d + kd, 0),
            (x0, y0 + h - d, 0),
        )
        ops.line_to(x0, y0 + d, 0)
        ops.bezier_to(
            (x0, y0 + d - kd, 0),
            (x0 + d - kd, y0, 0),
            (x0 + d, y0, 0),
        )
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig_ops = ops

    st.subheader("Tab Positions")
    n_tabs = st.number_input("Number of tabs", 0, 10, 2, key="tab_count")
    tab_power = st.slider("Tab power", 0.0, 1.0, 0.1, key="tab_pwr")
    tab_width = st.number_input("Tab width (mm)", 0.1, 20.0, 2.0, key="tab_tw")

    geo = orig_ops.to_geometry()
    geo_data = geo.data
    total_pts = len(geo_data) if geo_data is not None else 0

    clips = []
    if total_pts > 1:
        seg_dists = []
        for j in range(1, total_pts):
            dx = geo_data[j, 1] - geo_data[j - 1, 1]
            dy = geo_data[j, 2] - geo_data[j - 1, 2]
            seg_dists.append(math.sqrt(dx * dx + dy * dy))
        total_dist = sum(seg_dists)

        for t in range(n_tabs):
            target = total_dist * (t + 1) / (n_tabs + 1)
            accum = 0.0
            for seg_i, sd in enumerate(seg_dists):
                if accum + sd >= target - 1e-9:
                    frac = (target - accum) / sd if sd > 1e-9 else 0.0
                    px = geo_data[seg_i, 1] + frac * (
                        geo_data[seg_i + 1, 1] - geo_data[seg_i, 1]
                    )
                    py = geo_data[seg_i, 2] + frac * (
                        geo_data[seg_i + 1, 2] - geo_data[seg_i, 2]
                    )
                    clips.append((px, py, tab_width))
                    break
                accum += sd

    result_ops = orig_ops.copy()
    if clips:
        if mode == "Gap":
            result_ops.apply_tab_gaps(clips)
        else:
            result_ops.apply_tab_power(clips, tab_power, 1.0)

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    axes[0].set_title("Original")
    _plot_ops(axes[0], orig_ops, color="steelblue")
    for cx_, cy_, tw_ in clips:
        axes[0].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
        axes[0].add_patch(
            plt.Circle(
                (cx_, cy_),
                tw_ / 2,
                fill=False,
                color="red",
                linestyle="--",
                linewidth=1,
            )
        )
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)

    axes[1].set_title(f"After {mode} Tabs")
    show_pwr = mode == "Power"
    _plot_ops(
        axes[1],
        result_ops,
        color="steelblue",
        show_power=show_pwr,
    )
    for cx_, cy_, tw_ in clips:
        axes[1].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Original commands", orig_ops.len())
    c2.metric("Result commands", result_ops.len())
    c3.metric("Original cut dist", f"{orig_ops.cut_distance():.2f} mm")


def page_merge_lines():
    st.header("Merge Lines")

    from raygeo.ops import Ops
    from raygeo.ops.types import CommandType

    preset = st.selectbox(
        "Preset",
        [
            "Near-duplicate lines (tolerance-sensitive)",
            "Identical duplicates",
            "Overlapping collinear",
            "Adjacent rectangles",
            "Triangle shared edge",
            "Custom",
        ],
        key="ml_preset",
    )

    ops = Ops()
    ops.set_power(1.0)

    if preset == "Near-duplicate lines (tolerance-sensitive)":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.move_to(0, 1.5)
        ops.line_to(100, 1.5)
        ops.move_to(0, 5)
        ops.line_to(100, 5)
    elif preset == "Identical duplicates":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.move_to(0, 0)
        ops.line_to(100, 0)
    elif preset == "Overlapping collinear":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.move_to(50, 0)
        ops.line_to(150, 0)
    elif preset == "Adjacent rectangles":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.line_to(100, 100)
        ops.line_to(0, 100)
        ops.line_to(0, 0)
        ops.move_to(100, 0)
        ops.line_to(200, 0)
        ops.line_to(200, 100)
        ops.line_to(100, 100)
        ops.line_to(100, 0)
    elif preset == "Triangle shared edge":
        ops.move_to(0, 0)
        ops.line_to(100, 0)
        ops.line_to(50, 100)
        ops.line_to(0, 0)
        ops.move_to(100, 0)
        ops.line_to(0, 0)
        ops.line_to(50, -100)
        ops.line_to(100, 0)
    else:
        pts_text = st.text_area(
            "Segments (one per line: x1,y1 -> x2,y2)",
            "0,0 -> 100,0\n0,0 -> 100,0",
            key="ml_custom",
        )
        for line in pts_text.strip().splitlines():
            parts = line.strip().split("->")
            if len(parts) == 2:
                start = [float(v) for v in parts[0].strip().split(",")]
                end = [float(v) for v in parts[1].strip().split(",")]
                ops.move_to(start[0], start[1])
                ops.line_to(end[0], end[1])

    tol = st.slider("Tolerance", 0.0, 5.0, 1.0, 0.1, key="ml_tol")

    orig = ops.copy()
    orig_lines = len(ops.indices_of(CommandType.LINE_TO))
    orig_moves = len(ops.indices_of(CommandType.MOVE_TO))
    orig_cut = ops.cut_distance()

    ops.merge_overlapping_lines(tol)

    result_lines = len(ops.indices_of(CommandType.LINE_TO))
    result_moves = len(ops.indices_of(CommandType.MOVE_TO))
    result_cut = ops.cut_distance()

    fig, ax = plt.subplots(figsize=(12, 8))

    ax.set_title(
        f"Tolerance={tol:.1f}  |  "
        f"Original: {orig_lines} lines, {orig_moves} moves, "
        f"cut={orig_cut:.1f}  ->  "
        f"Merged: {result_lines} lines, {result_moves} moves, "
        f"cut={result_cut:.1f}"
    )

    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if pos != ep:
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
                label="Merged result" if i == 0 else None,
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.set_aspect("equal")
    xl = ax.get_xlim()
    yl = ax.get_ylim()
    pad = max(xl[1] - xl[0], yl[1] - yl[0]) * 0.05 + 5
    ax.set_xlim(xl[0] - pad, xl[1] + pad)
    ax.set_ylim(yl[0] - pad, yl[1] + pad)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)


def page_overscan():
    st.header("Overscan")

    from raygeo.ops import Ops
    from raygeo.ops.types import CommandType, SectionType

    preset = st.selectbox(
        "Preset",
        [
            "Horizontal raster lines",
            "Bidirectional raster",
            "Diagonal line",
            "Variable power scanline",
            "Mixed raster + vector",
        ],
        key="os_preset",
    )

    dist = st.slider("Overscan distance (mm)", 0.0, 20.0, 5.0, 0.5, key="os_dist")

    ops = Ops()
    ops.set_power(1.0)

    if preset == "Horizontal raster lines":
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(90, 10, 0)
        ops.move_to(10, 20, 0)
        ops.line_to(90, 20, 0)
        ops.move_to(10, 30, 0)
        ops.line_to(90, 30, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
    elif preset == "Bidirectional raster":
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(90, 10, 0)
        ops.move_to(90, 20, 0)
        ops.line_to(10, 20, 0)
        ops.move_to(10, 30, 0)
        ops.line_to(90, 30, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
    elif preset == "Diagonal line":
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(70, 70, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)
    elif preset == "Variable power scanline":
        pv = bytearray(range(0, 256, 4))
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 50, 0)
        ops.scan_to(90, 50, 0, power_values=pv)
        ops.ops_section_end(SectionType.RASTER_FILL)
    elif preset == "Mixed raster + vector":
        ops.move_to(5, 5, 0)
        ops.line_to(95, 95, 0)
        ops.ops_section_start(SectionType.RASTER_FILL, "wp1")
        ops.move_to(10, 20, 0)
        ops.line_to(80, 20, 0)
        ops.move_to(10, 30, 0)
        ops.line_to(80, 30, 0)
        ops.ops_section_end(SectionType.RASTER_FILL)

    orig = ops.copy()
    orig_lines = len(ops.indices_of(CommandType.LINE_TO))
    orig_scans = len(
        [i for i in range(ops.len()) if ops.command_type(i) == CommandType.SCAN_LINE]
    )

    ops.apply_overscan(dist)

    result_lines = len(ops.indices_of(CommandType.LINE_TO))
    result_scans = len(
        [i for i in range(ops.len()) if ops.command_type(i) == CommandType.SCAN_LINE]
    )

    fig, ax = plt.subplots(figsize=(12, 8))

    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep
        elif ct == CommandType.SCAN_LINE:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if pos != ep:
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep
        elif ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot(
        [], [], color="forestgreen", linewidth=2.5, label="With overscan"
    )
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Lines", f"{orig_lines} -> {result_lines}")
    c2.metric("Scan lines", f"{orig_scans} -> {result_scans}")
    c3.metric("Overscan", f"{dist:.1f} mm")


def page_lead_in_out():
    st.header("Lead-In / Lead-Out")

    from raygeo.ops import Ops
    from raygeo.ops.types import CommandType, SectionType

    preset = st.selectbox(
        "Preset",
        [
            "Rectangle",
            "Triangle",
            "Diagonal line",
            "Circle (linearized)",
            "Multiple contours",
        ],
        key="lio_preset",
    )

    c1, c2 = st.columns(2)
    with c1:
        lead_in = st.slider(
            "Lead-in (mm)", 0.0, 20.0, 5.0, 0.5, key="lio_in"
        )
    with c2:
        lead_out = st.slider(
            "Lead-out (mm)", 0.0, 20.0, 5.0, 0.5, key="lio_out"
        )

    ops = Ops()
    ops.set_power(1.0)

    if preset == "Rectangle":
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(20, 20, 0)
        ops.line_to(80, 20, 0)
        ops.line_to(80, 80, 0)
        ops.line_to(20, 80, 0)
        ops.line_to(20, 20, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif preset == "Triangle":
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(50, 10, 0)
        ops.line_to(90, 80, 0)
        ops.line_to(10, 80, 0)
        ops.line_to(50, 10, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif preset == "Diagonal line":
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(90, 90, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif preset == "Circle (linearized)":
        n = 64
        r = 35
        cx, cy = 50, 50
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(cx + r, cy, 0)
        for i in range(1, n + 1):
            a = 2 * math.pi * i / n
            ops.line_to(cx + r * math.cos(a), cy + r * math.sin(a), 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    elif preset == "Multiple contours":
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(10, 10, 0)
        ops.line_to(40, 10, 0)
        ops.line_to(40, 40, 0)
        ops.line_to(10, 40, 0)
        ops.line_to(10, 10, 0)
        ops.move_to(60, 60, 0)
        ops.line_to(90, 60, 0)
        ops.line_to(90, 90, 0)
        ops.line_to(60, 90, 0)
        ops.line_to(60, 60, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig = ops.copy()
    orig_lines = len(ops.indices_of(CommandType.LINE_TO))

    ops.apply_lead_in_out(lead_in, lead_out)

    result_lines = len(ops.indices_of(CommandType.LINE_TO))

    fig, ax = plt.subplots(figsize=(10, 10))

    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if pos != ep:
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            state = ops.preloaded_state(i)
            color = "dodgerblue" if state and state.power < 0.01 else "forestgreen"
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=color,
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot([], [], color="forestgreen", linewidth=2.5, label="Cut (power > 0)")
    ax.plot([], [], color="dodgerblue", linewidth=2.5, label="Lead (power = 0)")
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Lines", f"{orig_lines} -> {result_lines}")
    c2.metric("Lead-in", f"{lead_in:.1f} mm")
    c3.metric("Lead-out", f"{lead_out:.1f} mm")


def page_concave_hull():
    st.header("Concave Hull (Shrink-Wrap)")

    from raygeo.geo.algo import hull

    preset = st.selectbox(
        "Shape",
        [
            "Two squares",
            "Hourglass",
            "L-shape",
            "Circle",
            "Three dots",
        ],
        key="ch_shape",
    )

    gravity = st.slider(
        "Gravity", 0.0, 1.0, 0.1, 0.05, key="ch_grav"
    )

    height, width = 200, 200
    img = np.zeros((height, width), dtype=bool)

    if preset == "Two squares":
        img[30:70, 30:70] = True
        img[130:170, 130:170] = True
    elif preset == "Hourglass":
        r = 8
        _fill_rounded_rect(img, (60, 30), (140, 70), r)
        _fill_rounded_rect(img, (80, 110), (120, 150), r)
        _fill_rounded_rect(img, (60, 110), (140, 170), r)
    elif preset == "L-shape":
        img[30:170, 30:70] = True
        img[30:100, 70:170] = True
    elif preset == "Circle":
        yy, xx = np.ogrid[:height, :width]
        mask = (xx - 100) ** 2 + (yy - 100) ** 2 <= 2500
        img[mask] = True
    elif preset == "Three dots":
        for cy, cx in [(50, 50), (50, 150), (150, 100)]:
            yy, xx = np.ogrid[:height, :width]
            mask = (xx - cx) ** 2 + (yy - cy) ** 2 <= 400
            img[mask] = True

    convex_geo = hull.get_enclosing_hull(img)
    concave_geo = hull.get_concave_hull(img, gravity=gravity)
    per_component = hull.get_hulls_from_image(img)

    fig, ax = plt.subplots(figsize=(8, 8))

    ax.imshow(
        img,
        origin="upper",
        cmap="Blues",
        alpha=0.3,
        extent=[0, width, height, 0],
    )

    if convex_geo is not None:
        _plot_geometry(
            ax,
            convex_geo,
            color="tomato",
            label="Convex hull",
            linewidth=1.5,
        )

    if concave_geo is not None:
        _plot_geometry(
            ax,
            concave_geo,
            color="forestgreen",
            label="Concave hull",
            linewidth=2,
        )

    for i, g in enumerate(per_component):
        _plot_geometry(
            ax,
            g,
            color="dodgerblue",
            label="Per-component" if i == 0 else None,
            linewidth=1,
            show_points=True,
        )

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Gravity", f"{gravity:.2f}")
    if convex_geo is not None and concave_geo is not None:
        c2.metric("Convex area", f"{convex_geo.area():.1f}")
        c3.metric("Concave area", f"{concave_geo.area():.1f}")
    c3.metric("Components", f"{len(per_component)}")


def _fill_rounded_rect(img, pt1, pt2, r):
    x1, y1 = pt1
    x2, y2 = pt2
    h, w = img.shape
    img[max(0, y1 + r) : min(h, y2 - r), max(0, x1) : min(w, x2)] = True
    img[max(0, y1) : min(h, y2), max(0, x1 + r) : min(w, x2 - r)] = True
    for cy, cx in [
        (y1 + r, x1 + r),
        (y1 + r, x2 - r),
        (y2 - r, x1 + r),
        (y2 - r, x2 - r),
    ]:
        yy, xx = np.ogrid[-r : r + 1, -r : r + 1]
        mask = xx**2 + yy**2 <= r**2
        ys = slice(max(0, cy - r), min(h, cy + r + 1))
        xs = slice(max(0, cx - r), min(w, cx + r + 1))
        img[ys, xs][mask[: min(h, cy + r + 1) - max(0, cy - r), : min(w, cx + r + 1) - max(0, cx - r)]] = True


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
        "Tab Operations",
        "Merge Lines",
        "Overscan",
        "Lead-In/Out",
        "Concave Hull",
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
elif page == "Tab Operations":
    page_tabs()
elif page == "Merge Lines":
    page_merge_lines()
elif page == "Overscan":
    page_overscan()
elif page == "Lead-In/Out":
    page_lead_in_out()
elif page == "Concave Hull":
    page_concave_hull()
