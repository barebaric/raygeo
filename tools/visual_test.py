"""Visual test playground for raygeo.

Run with: make visual  (or: streamlit run tools/visual_test.py)
"""

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
import streamlit as st
from matplotlib.colors import to_hex

import raygeo.image as img
from raygeo.geo import Arc, Bezier, Geometry, Line, Move
from raygeo.geo.algo import hull
from raygeo.geo.shape.polygon import (
    get_polygon_convex_hull,
    get_polygons_difference,
    get_polygons_intersection,
    get_polygons_union,
    offset_polygon,
)
from raygeo.nest.genetic import GeneticAlgorithm
from raygeo.nest.gravity import apply_gravity
from raygeo.nest.placement import place_parts
from raygeo.ops import Ops
from raygeo.ops.raster import (
    ScanMode,
    rasterize_mask_lines,
    rasterize_mask_scan,
    rasterize_multi_pass,
    rasterize_power_modulation,
)
from raygeo.ops.types import CommandType, SectionType
from raygeo.svg import parse_svg_path_data, svg_string_to_geometries
from tools.plot import (
    auto_limits,
    fill_rounded_rect,
    make_pattern,
    plot_geometry,
    plot_ops,
    plot_polygon,
    rasterize_geometries_to_mask,
)

EXAMPLE_SVG = (
    '<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">\n'
    '  <rect x="10" y="10" width="30" height="30" />\n'
    '  <circle cx="70" cy="70" r="20" />\n'
    '  <path d="M 10 70 L 40 90 L 30 50 Z" />\n'
    "</svg>"
)


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
                "Circle",
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

        elif shape == "Circle":
            c1, c2 = st.columns(2)
            r = c1.number_input("Radius", 0.1, 500.0, 10.0)
            cw = c2.checkbox("Clockwise", value=True)
            geom.move_to(r, 0, 0)
            geom.arc_to(-r, 0, -r, 0, cw, 0)
            geom.arc_to(r, 0, r, 0, cw, 0)

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
            geom = geom.flip_x()
        elif op == "Flip Y":
            geom = geom.flip_y()
        elif op == "Simplify":
            tol = st.number_input("Tolerance", 0.001, 100.0, 0.1)
            geom = geom.simplify(tol)
        elif op == "Linearize":
            tol = st.number_input("Tolerance", 0.001, 100.0, 0.1)
            geom = geom.linearize(tol)
        elif op == "Split contours":
            contours = geom.split_into_contours()
            st.info(f"Split into {len(contours)} contour(s)")
            fig, ax = plt.subplots()
            cmap = plt.get_cmap("tab10")
            colors = [to_hex(cmap(i / 10)) for i in range(10)]
            for i, c in enumerate(contours):
                lbl = f"Contour {i}"
                plot_geometry(
                    ax,
                    c,
                    color=colors[i % len(colors)],
                    label=lbl,
                )
            xmin, xmax, ymin, ymax = auto_limits(contours)
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
        if not geom.is_empty():
            bounds = f"({r[0]:.1f}, {r[1]:.1f}) - ({r[2]:.1f}, {r[3]:.1f})"
            cols[3].metric("Bounds", bounds)
        st.checkbox(
            "Closed", value=geom.is_closed(), disabled=True, key="closed"
        )

    with tab_fit:
        fit_op = st.selectbox(
            "Fit operation",
            [
                "None",
                "Fit arcs",
                "Fit curves (arcs + beziers)",
            ],
        )
        tol = st.number_input("Tolerance", 0.001, 100.0, 0.5)
        if fit_op == "Fit arcs":
            geom = geom.fit_arcs(tol)
        elif fit_op == "Fit curves (arcs + beziers)":
            geom = geom.fit_curves(tol, beziers=True, arcs=True)

    fig, ax = plt.subplots()
    show_pts = st.checkbox(
        "Show control points", value=False, key="show_pts_build"
    )
    plot_geometry(ax, geom, show_points=show_pts)
    xmin, xmax, ymin, ymax = auto_limits([geom])
    ax.set_xlim(xmin, xmax)
    ax.set_ylim(ymin, ymax)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    st.pyplot(fig)

    with st.expander("Raw data"):
        cmds = geom.iter_typed_commands()
        if cmds:
            rows = []
            for cmd in cmds:
                if isinstance(cmd, Move):
                    rows.append(
                        {"type": "Move", "x": cmd.end[0], "y": cmd.end[1]}
                    )
                elif isinstance(cmd, Line):
                    rows.append(
                        {"type": "Line", "x": cmd.end[0], "y": cmd.end[1]}
                    )
                elif isinstance(cmd, Arc):
                    rows.append(
                        {
                            "type": "Arc",
                            "x": cmd.end[0],
                            "y": cmd.end[1],
                            "i": cmd.center_offset[0],
                            "j": cmd.center_offset[1],
                            "cw": cmd.clockwise,
                        }
                    )
                elif isinstance(cmd, Bezier):
                    rows.append(
                        {
                            "type": "Bezier",
                            "x": cmd.end[0],
                            "y": cmd.end[1],
                            "c1x": cmd.control1[0],
                            "c1y": cmd.control1[1],
                            "c2x": cmd.control2[0],
                            "c2y": cmd.control2[1],
                        }
                    )
            st.dataframe(rows)


def page_polygon_boolean():
    st.header("Polygon Boolean Operations")

    c1, c2 = st.columns(2)
    with c1:
        st.subheader("Shape A (polygon)")
        a_type = st.selectbox("Shape A type", ["Square", "Circle"])
        a_r = st.number_input(
            "A: radius / half-size", 0.5, 100.0, 10.0, key="a_r"
        )
    with c2:
        st.subheader("Shape B (polygon)")
        b_type = st.selectbox(
            "Shape B type",
            ["Square", "Circle"],
            key="b_type",
        )
        b_r = st.number_input(
            "B: radius / half-size", 0.5, 100.0, 8.0, key="b_r"
        )

    n_seg = st.number_input(
        "Circle segments", 3, 360, 64, step=1, key="bool_n"
    )

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
        return [
            (ox - r, oy - r),
            (ox + r, oy - r),
            (ox + r, oy + r),
            (ox - r, oy + r),
        ]

    a = _make_circle(a_r, n_seg) if a_type == "Circle" else _make_square(a_r)
    b_raw = (
        _make_circle(b_r, n_seg) if b_type == "Circle" else _make_square(b_r)
    )
    b = [(x + dx, y + dy) for x, y in b_raw]

    op = st.selectbox("Operation", ["Union", "Intersection", "Difference"])

    if op == "Union":
        result = get_polygons_union([a, b])
    elif op == "Intersection":
        result = get_polygons_intersection(a, b)
    else:
        result = get_polygons_difference(a, b)

    fig, ax = plt.subplots()
    plot_polygon(ax, a, "steelblue", "A")
    plot_polygon(ax, b, "tomato", "B")
    if result:
        plot_polygon(ax, result[0], "limegreen", "Result", linewidth=2.5)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend()
    st.pyplot(fig)


def page_offset():
    st.header("Polygon Offset")

    n_seg = st.number_input("Circle segments", 3, 360, 64, step=1, key="off_n")
    r = st.number_input("Shape radius", 0.5, 100.0, 10.0, key="off_r")
    amount = st.number_input("Offset amount", -50.0, 50.0, 2.0)

    pts = [
        (
            r * math.cos(2 * math.pi * i / n_seg),
            r * math.sin(2 * math.pi * i / n_seg),
        )
        for i in range(n_seg)
    ]

    result = offset_polygon(pts, amount)

    fig, ax = plt.subplots()
    plot_polygon(ax, pts, "steelblue", "Original")
    for i, poly in enumerate(result):
        plot_polygon(ax, poly, "limegreen", f"Offset {i}", linewidth=2.5)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend()
    st.pyplot(fig)


def page_image():
    st.header("Image Processing")

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

    arr = make_pattern(w, h, pattern)

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
            dtype=np.float32,
        )
        dithered = img.apply_bayer_dither(gray, bayer, invert, cell_size=1)

    fig2, axes2 = plt.subplots(1, 2, figsize=(10, 4))
    axes2[0].imshow(arr, cmap="gray", vmin=0, vmax=255)
    axes2[0].set_title("Original")
    axes2[1].imshow(dithered, cmap="gray", vmin=0, vmax=1)
    axes2[1].set_title(f"Dithered ({dither})")
    st.pyplot(fig2)


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

    geoms = parse_svg_path_data(path_data)

    fig, ax = plt.subplots()
    cmap = plt.get_cmap("tab10")
    colors = [to_hex(cmap(i / 10)) for i in range(10)]
    for i, g in enumerate(geoms):
        plot_geometry(ax, g, color=colors[i % len(colors)], label=f"Path {i}")
    xmin, xmax, ymin, ymax = auto_limits(geoms) if geoms else (0, 100, 0, 100)
    ax.set_xlim(xmin, xmax)
    ax.set_ylim(ymin, ymax)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    if geoms:
        ax.legend()
    st.pyplot(fig)


def page_tabs():
    st.header("Tab Operations")

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
    segments = geo.segments()
    seg_dists = []
    for seg in segments:
        for j in range(1, len(seg)):
            dx = seg[j][0] - seg[j - 1][0]
            dy = seg[j][1] - seg[j - 1][1]
            seg_dists.append(math.sqrt(dx * dx + dy * dy))
    total_dist = sum(seg_dists)

    clips = []
    if total_dist > 0 and n_tabs > 0:
        flat_pts = [p for seg in segments for p in seg]
        for t in range(n_tabs):
            target = total_dist * (t + 1) / (n_tabs + 1)
            accum = 0.0
            for seg_i, sd in enumerate(seg_dists):
                if accum + sd >= target - 1e-9:
                    frac = (target - accum) / sd if sd > 1e-9 else 0.0
                    px = flat_pts[seg_i][0] + frac * (
                        flat_pts[seg_i + 1][0] - flat_pts[seg_i][0]
                    )
                    py = flat_pts[seg_i][1] + frac * (
                        flat_pts[seg_i + 1][1] - flat_pts[seg_i][1]
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
    plot_ops(axes[0], orig_ops, color="steelblue")
    for cx_, cy_, tw_ in clips:
        axes[0].plot(cx_, cy_, "rx", markersize=10, markeredgewidth=2)
        axes[0].add_patch(
            mpatches.Circle(
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
    plot_ops(
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

    dist = st.slider(
        "Overscan distance (mm)", 0.0, 20.0, 5.0, 0.5, key="os_dist"
    )

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
        [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
    )

    ops.apply_overscan(dist)

    result_lines = len(ops.indices_of(CommandType.LINE_TO))
    result_scans = len(
        [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
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
    ax.plot([], [], color="forestgreen", linewidth=2.5, label="With overscan")
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
        lead_in = st.slider("Lead-in (mm)", 0.0, 20.0, 5.0, 0.5, key="lio_in")
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
            state = ops.state(i)
            color = (
                "dodgerblue" if state and state.power < 0.01 else "forestgreen"
            )
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=color,
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot(
        [], [], color="forestgreen", linewidth=2.5, label="Cut (power > 0)"
    )
    ax.plot(
        [], [], color="dodgerblue", linewidth=2.5, label="Lead (power = 0)"
    )
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

    preset = st.selectbox(
        "Shape",
        [
            "Two squares",
            "Hourglass",
            "L-shape",
            "Circle",
            "Three dots",
            "Upload SVG",
        ],
        key="ch_shape",
    )

    height = st.slider("Resolution", 200, 1000, 500, 50, key="ch_res")
    width = height

    gravity = st.slider("Gravity", 0.0, 1.0, 0.1, 0.05, key="ch_grav")

    img = np.zeros((height, width), dtype=bool)
    svg_geoms = []

    if preset == "Upload SVG":
        svg_source = st.radio(
            "SVG source", ["Upload file", "Paste SVG text"], key="ch_svg_src"
        )
        svg_str = ""
        if svg_source == "Upload file":
            uploaded = st.file_uploader(
                "Choose an SVG file", type=["svg"], key="ch_svg_file"
            )
            if uploaded is not None:
                svg_str = uploaded.read().decode("utf-8")
        else:
            svg_str = st.text_area(
                "SVG markup",
                EXAMPLE_SVG,
                height=200,
                key="ch_svg_text",
            )

        if svg_str.strip():
            try:
                svg_geoms = svg_string_to_geometries(svg_str)
                if svg_geoms:
                    img = rasterize_geometries_to_mask(
                        svg_geoms, width, height
                    )
                else:
                    st.warning("No geometries found in SVG")
            except Exception as e:
                st.error(f"Failed to parse SVG: {e}")
    elif preset == "Two squares":
        img[30:70, 30:70] = True
        img[130:170, 130:170] = True
    elif preset == "Hourglass":
        r = 8
        fill_rounded_rect(img, (60, 30), (140, 70), r)
        fill_rounded_rect(img, (80, 110), (120, 150), r)
        fill_rounded_rect(img, (60, 110), (140, 170), r)
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
        extent=(0, width, height, 0),
    )

    if convex_geo is not None:
        plot_geometry(
            ax,
            convex_geo,
            color="tomato",
            label="Convex hull",
            linewidth=1.5,
        )

    if concave_geo is not None:
        plot_geometry(
            ax,
            concave_geo,
            color="forestgreen",
            label="Concave hull",
            linewidth=2,
        )

    for i, g in enumerate(per_component):
        plot_geometry(
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

    if preset == "Upload SVG" and svg_geoms:
        st.subheader("Parsed SVG geometries")
        c = plt.get_cmap("tab10")
        fig2, ax2 = plt.subplots(figsize=(6, 6))
        for i, g in enumerate(svg_geoms):
            plot_geometry(
                ax2,
                g,
                color=to_hex(c(i / 10)),
                label=f"Path {i}",
                linewidth=1.5,
            )
        xmin, xmax, ymin, ymax = auto_limits(svg_geoms)
        ax2.set_xlim(xmin, xmax)
        ax2.set_ylim(ymin, ymax)
        ax2.set_aspect("equal")
        ax2.grid(True, alpha=0.3)
        ax2.legend(fontsize=8)
        fig2.tight_layout()
        st.pyplot(fig2)


def page_rasterization():
    st.header("Rasterization")

    c1, c2 = st.columns(2)
    with c1:
        mode = st.selectbox(
            "Rasterization mode",
            [
                "Power Modulation",
                "Mask Scan",
                "Mask Lines",
                "Multi-Pass",
            ],
            key="rast_mode",
        )
        scan_mode = st.selectbox(
            "Scan mode",
            ["Segmented", "FullSweep"],
            key="rast_scan_mode",
        )
    with c2:
        pattern = st.selectbox(
            "Test pattern",
            ["Gradient", "Checkered", "Circle", "Random noise"],
            key="rast_pattern",
        )
        img_size = st.number_input(
            "Image size", 16, 256, 64, step=16, key="rast_size"
        )

    c3, c4, c5 = st.columns(3)
    with c3:
        line_interval = st.slider(
            "Line interval (mm)", 0.05, 1.0, 0.1, 0.05, key="rast_li"
        )
        angle = st.slider("Angle (deg)", 0, 90, 0, 5, key="rast_angle")
    with c4:
        ppm_val = st.number_input(
            "Pixels per mm", 1.0, 50.0, 10.0, 0.5, key="rast_ppm"
        )
    with c5:
        sample_interval = 0.05
        min_power = 0.0
        max_power = 1.0
        num_depth = 3
        z_step = 0.5
        if mode == "Power Modulation":
            sample_interval = st.slider(
                "Sample interval (mm)",
                0.01,
                0.5,
                0.05,
                0.01,
                key="rast_si",
            )
            min_power = st.slider(
                "Min power", 0.0, 1.0, 0.0, 0.1, key="rast_minp"
            )
            max_power = st.slider(
                "Max power", 0.0, 1.0, 1.0, 0.1, key="rast_maxp"
            )
        elif mode == "Multi-Pass":
            num_depth = st.slider("Depth levels", 2, 10, 3, key="rast_depth")
            z_step = st.slider(
                "Z step down", 0.1, 2.0, 0.5, 0.1, key="rast_zstep"
            )

    sm = ScanMode.Segmented if scan_mode == "Segmented" else ScanMode.FullSweep

    gray = make_pattern(img_size, img_size, pattern)

    if mode == "Power Modulation":
        alpha = np.full((img_size, img_size), 255, dtype=np.uint8)
        ops = rasterize_power_modulation(
            gray,
            alpha,
            (ppm_val, ppm_val),
            0.0,
            0.0,
            line_interval,
            sample_interval,
            min_power=min_power,
            max_power=max_power,
            angle=float(angle),
            scan_mode=sm,
        )
    elif mode == "Mask Scan":
        mask = (gray > 128).astype(np.uint8)
        ops = rasterize_mask_scan(
            mask,
            (ppm_val, ppm_val),
            0.0,
            0.0,
            line_interval,
            angle=float(angle),
            scan_mode=sm,
        )
    elif mode == "Mask Lines":
        mask = (gray > 128).astype(np.uint8)
        ops = rasterize_mask_lines(
            mask,
            (ppm_val, ppm_val),
            0.0,
            0.0,
            line_interval,
            angle=float(angle),
            scan_mode=sm,
        )
    else:
        ops = rasterize_multi_pass(
            gray,
            (ppm_val, ppm_val),
            0.0,
            0.0,
            line_interval,
            num_depth,
            z_step,
            angle=float(angle),
            scan_mode=sm,
        )

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    axes[0].imshow(gray, cmap="gray", vmin=0, vmax=255, origin="lower")
    axes[0].set_title("Input image")
    axes[0].set_aspect("equal")

    cmap = plt.get_cmap("hot")

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if abs(pos[0] - ep[0]) > 1e-6 or abs(pos[1] - ep[1]) > 1e-6:
                axes[1].plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.5,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            sd = ops.scanline_data(i)
            n = len(sd)
            if n > 0:
                xs = np.linspace(pos[0], ep[0], n)
                ys = np.linspace(pos[1], ep[1], n)
                power_arr = np.frombuffer(sd, dtype=np.uint8).astype(
                    np.float64
                )
                power_norm = power_arr / 255.0
                colors = cmap(power_norm)
                colors[:, 3] = np.clip(power_norm * 2, 0.15, 1.0)
                axes[1].scatter(
                    xs, ys, c=colors, s=max(0.5, 800 / img_size), marker="s"
                )
            pos = ep
        elif ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            axes[1].plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=(0.9, 0.3, 0.3, 0.9),
                linewidth=1.0,
                solid_capstyle="round",
            )
            pos = ep

    line_count = len(ops.indices_of(CommandType.LINE_TO))
    scan_count = len(
        [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
    )
    total = ops.len()

    axes[1].plot(
        [], [], color="gray", linewidth=0.5, linestyle=":", label="Travel"
    )
    axes[1].plot([], [], color=cmap(1.0), linewidth=2, label="Scan (high pwr)")
    axes[1].plot([], [], color=cmap(0.3), linewidth=2, label="Scan (low pwr)")
    axes[1].plot(
        [], [], color=(0.9, 0.3, 0.3, 0.9), linewidth=2, label="Lines"
    )
    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)
    axes[1].legend(fontsize=9)
    axes[1].set_title(f"{scan_mode} | {mode} ({angle}\u00b0)")
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Commands", total)
    c2.metric("Scan lines", scan_count)
    c3.metric("Lines", line_count)


def page_nesting():
    st.header("Nesting")

    c1, c2, c3 = st.columns(3)
    with c1:
        shape = st.selectbox(
            "Part shape",
            ["Rectangle", "Circle", "L-shape", "Mixed"],
            key="nest_shape",
        )
    with c2:
        n_parts = st.slider("Number of parts", 1, 50, 10, key="nest_n")
    with c3:
        size = st.slider("Part size", 5, 100, 30, key="nest_size")

    c4, c5, c6 = st.columns(3)
    with c4:
        sheet_w = st.number_input("Sheet width", 50, 2000, 200, key="nest_sw")
    with c5:
        sheet_h = st.number_input("Sheet height", 50, 2000, 200, key="nest_sh")
    with c6:
        n_sheets = st.number_input("Number of sheets", 1, 10, 1, key="nest_ns")

    c7, c8, c9 = st.columns(3)
    with c7:
        spacing = st.slider("Spacing", 0.0, 20.0, 2.0, 0.5, key="nest_spc")
    with c8:
        rot_max = st.slider(
            "Max rotation (deg)", 0, 360, 180, 45, key="nest_rot"
        )
    with c9:
        flip_h = st.checkbox("Allow X flip", value=True, key="nest_fh")
        flip_v = st.checkbox("Allow Y flip", value=False, key="nest_fv")

    rng = np.random.default_rng()

    def _make_part(i):
        cx, cy = size / 2, size / 2
        if shape == "Rectangle" or (shape == "Mixed" and i % 2 == 0):
            w = size * (0.5 + 0.5 * rng.random())
            h = size * (0.5 + 0.5 * rng.random())
            return [
                (0, 0),
                (w, 0),
                (w, h),
                (0, h),
            ]
        elif shape == "Circle" or (shape == "Mixed" and i % 3 == 0):
            r = size * (0.3 + 0.3 * rng.random())
            n = 32
            return [
                (
                    cx + r * math.cos(2 * math.pi * j / n),
                    cy + r * math.sin(2 * math.pi * j / n),
                )
                for j in range(n)
            ]
        else:
            leg_w = size * (0.3 + 0.3 * rng.random())
            leg_h = size * (0.3 + 0.3 * rng.random())
            body_w = size * (0.5 + 0.3 * rng.random())
            body_h = size * (0.5 + 0.3 * rng.random())
            return [
                (0, 0),
                (body_w, 0),
                (body_w, leg_h),
                (leg_w, leg_h),
                (leg_w, body_h),
                (0, body_h),
            ]

    spf_0 = (0.0, 0.0)
    spf_1 = (sheet_w, 0.0)
    spf_2 = (sheet_w, sheet_h)
    spf_3 = (0.0, sheet_h)
    sheet_poly_flat = [spf_0, spf_1, spf_2, spf_3]

    c_ga = st.columns(1)[0]
    with c_ga:
        use_ga = st.checkbox(
            "Use genetic algorithm", value=True, key="nest_ga"
        )
        n_gen = st.slider("Generations", 1, 20, 5, key="nest_ngen")

    if st.button("Run Nesting", type="primary", key="nest_run"):
        part_polys = [[_make_part(i)] for i in range(n_parts)]
        part_hulls = [
            [get_polygon_convex_hull(part_polys[i][0])] for i in range(n_parts)
        ]
        sheet_poly = [sheet_poly_flat]
        sheet_offsets = [(0.0, 0.0)] * n_sheets

        if use_ga and rot_max > 0:
            ga_config = {
                "rotation_count": max(1, rot_max // 45) + 1,
                "flip_h": flip_h,
                "flip_v": flip_v,
                "population_size": 10,
                "mutation_rate": 30.0,
            }
            ga = GeneticAlgorithm(n_parts, ga_config)
            best_fitness = float("inf")
            best_result = None

            progress = st.progress(0, text="Evolving...")
            for gen in range(n_gen):
                for idx in range(len(ga)):
                    rots, fh_arr, fv_arr, _ = ga.get_individual(idx)
                    result = place_parts(
                        part_polys,
                        part_hulls,
                        sheet_poly,
                        sheet_offsets,
                        rots,
                        fh_arr,
                        fv_arr,
                        spacing=spacing,
                    )
                    fit = result[0]["fitness"] if result else float("inf")
                    ga.set_fitness(idx, fit)
                    if fit < best_fitness:
                        best_fitness = fit
                        best_result = result
                ga.generation()
                progress.progress(
                    (gen + 1) / n_gen, text=f"Gen {gen + 1}/{n_gen}"
                )

            result = best_result
        else:
            rotations = [
                rng.uniform(0.0, float(rot_max)) for _ in range(n_parts)
            ]
            fh = [flip_h] * n_parts
            fv = [flip_v] * n_parts
            with st.spinner("Nesting..."):
                result = place_parts(
                    part_polys,
                    part_hulls,
                    sheet_poly,
                    sheet_offsets,
                    rotations,
                    fh,
                    fv,
                    spacing=spacing,
                )

        if not result:
            st.warning("No placements found.")
            return

        total_placed = sum(len(sheet["placements"]) for sheet in result)
        fitness = result[0].get("fitness", float("inf"))
        sheet_label = "sheet" if len(result) == 1 else "sheets"
        st.success(
            f"Placed {total_placed} / {n_parts} parts across "
            f"{len(result)} {sheet_label}"
            + (f" | fitness: {fitness:.4f}" if fitness != float("inf") else "")
        )

        cmap = plt.get_cmap("tab10")
        for si, sheet_result in enumerate(result):
            st.subheader(f"Sheet {si + 1}")
            placements = sheet_result["placements"]
            fig, ax = plt.subplots(figsize=(10, 8))
            ax.plot(
                [p[0] for p in sheet_poly_flat] + [sheet_poly_flat[0][0]],
                [p[1] for p in sheet_poly_flat] + [sheet_poly_flat[0][1]],
                color="black",
                linewidth=2,
                label="Sheet",
            )
            for pi, pl in enumerate(placements):
                for poly in pl["polygons"]:
                    px = [p[0] for p in poly] + [poly[0][0]]
                    py = [p[1] for p in poly] + [poly[0][1]]
                    color = to_hex(cmap(pi % 10))
                    ax.fill(px, py, alpha=0.25, color=color)
                    ax.plot(px, py, color=color, linewidth=1.5)
            ax.set_aspect("equal")
            ax.set_xlim(-spacing * 2, sheet_w + spacing * 2)
            ax.set_ylim(-spacing * 2, sheet_h + spacing * 2)
            ax.grid(True, alpha=0.3)
            ax.legend(fontsize=9, loc="upper right")
            fig.tight_layout()
            st.pyplot(fig)

        if n_sheets == 1 and total_placed > 0:
            st.subheader("After gravity")
            placed_groups = [pl["polygons"] for pl in result[0]["placements"]]
            adjustments = apply_gravity(
                placed_groups, sheet_poly_flat, spacing
            )
            fig2, ax2 = plt.subplots(figsize=(10, 8))
            ax2.plot(
                [p[0] for p in sheet_poly_flat] + [sheet_poly_flat[0][0]],
                [p[1] for p in sheet_poly_flat] + [sheet_poly_flat[0][1]],
                color="black",
                linewidth=2,
            )
            for pi, (pl, adj) in enumerate(
                zip(result[0]["placements"], adjustments)
            ):
                for poly in pl["polygons"]:
                    shifted = [(p[0] + adj[0], p[1] + adj[1]) for p in poly]
                    px = [p[0] for p in shifted] + [shifted[0][0]]
                    py = [p[1] for p in shifted] + [shifted[0][1]]
                    color = to_hex(cmap(pi % 10))
                    ax2.fill(px, py, alpha=0.25, color=color)
                    ax2.plot(px, py, color=color, linewidth=1.5)
            ax2.set_aspect("equal")
            ax2.set_xlim(-spacing * 2, sheet_w + spacing * 2)
            ax2.set_ylim(-spacing * 2, sheet_h + spacing * 2)
            ax2.grid(True, alpha=0.3)
            fig2.tight_layout()
            st.pyplot(fig2)
    else:
        st.info("Configure parts and sheet above, then click **Run Nesting**.")


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
        "Rasterization",
        "Concave Hull",
        "Nesting",
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
elif page == "Rasterization":
    page_rasterization()
elif page == "Concave Hull":
    page_concave_hull()
elif page == "Nesting":
    page_nesting()
