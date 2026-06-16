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
from raygeo.geo.algo.analysis import get_area, get_path_winding_order
from raygeo.geo.algo.minkowski2d import get_polygon_minkowski_sum_convex
from raygeo.geo.algo.nest2d.genetic import GeneticAlgorithm
from raygeo.geo.algo.nest2d.gravity import apply_gravity
from raygeo.geo.algo.nest2d.ifp import inner_fit_polygon
from raygeo.geo.algo.nest2d.placement import place_parts
from raygeo.geo.shape.arc import linearize_arc
from raygeo.geo.shape.bezier import (
    flatten_bezier,
    get_bezier_point_at,
    split_bezier,
)
from raygeo.geo.shape.circle import (
    get_circle_circle_intersections,
    get_line_circle_intersections,
)
from raygeo.geo.shape.line import (
    get_line_closest_point,
    get_line_line_intersection,
    get_line_segment_intersection,
    get_point_line_distance,
)
from raygeo.geo.shape.polygon import (
    get_polygon_convex_hull,
    get_polygons_difference,
    get_polygons_intersection,
    get_polygons_union,
    offset_polygon,
)
from raygeo.geo.shape.polygon3d import (
    get_polygons_difference_3d,
    get_polygons_intersection_3d,
    get_polygons_union_3d,
    offset_polygon_3d,
)
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


def page_polygon3d():
    st.header("3D Polygon Boolean & Offset")
    st.markdown(
        "All operations run in the XY plane and **preserve Z** on output."
    )

    op = st.selectbox(
        "Operation",
        ["Union", "Intersection", "Difference", "Offset"],
        key="p3d_op",
    )

    c1, c2 = st.columns(2)
    with c1:
        st.subheader("Shape A")
        a_type = st.selectbox("Type", ["Square", "Circle"], key="p3d_a_type")
        a_r = st.number_input("Size", 0.5, 100.0, 10.0, key="p3d_a_r")
        a_z = st.number_input("Z", -20.0, 20.0, 5.0, key="p3d_a_z")
    with c2:
        st.subheader("Shape B")
        b_type = st.selectbox("Type", ["Square", "Circle"], key="p3d_b_type")
        b_r = st.number_input("Size", 0.5, 100.0, 8.0, key="p3d_b_r")
        b_z = st.number_input("Z", -20.0, 20.0, 5.0, key="p3d_b_z")

    dx = st.number_input("B offset X", -50.0, 50.0, 6.0, key="p3d_dx")
    dy = st.number_input("B offset Y", -50.0, 50.0, 0.0, key="p3d_dy")

    n_seg = st.number_input("Circle segments", 3, 360, 64, step=1, key="p3d_n")

    offset_amt = 0.0
    if op == "Offset":
        offset_amt = st.number_input(
            "Offset amount", -50.0, 50.0, 2.0, key="p3d_offset"
        )

    def _make_square(r, ox=0.0, oy=0.0):
        return [
            (ox - r, oy - r),
            (ox + r, oy - r),
            (ox + r, oy + r),
            (ox - r, oy + r),
        ]

    def _make_circle(r, n, ox=0.0, oy=0.0):
        return [
            (
                ox + r * math.cos(2 * math.pi * i / n),
                oy + r * math.sin(2 * math.pi * i / n),
            )
            for i in range(n)
        ]

    def _lift(poly, z):
        return [(x, y, z) for x, y in poly]

    if a_type == "Circle":
        a_xy = _make_circle(a_r, n_seg)
    else:
        a_xy = _make_square(a_r)
    a = _lift(a_xy, a_z)

    if b_type == "Circle":
        b_raw = _make_circle(b_r, n_seg)
    else:
        b_raw = _make_square(b_r)
    b_xy = [(x + dx, y + dy) for x, y in b_raw]
    b = _lift(b_xy, b_z)

    if op == "Union":
        result = get_polygons_union_3d([a, b])
        result_label = "Union"
    elif op == "Intersection":
        result = get_polygons_intersection_3d(a, b)
        result_label = "Intersection"
    elif op == "Difference":
        result = get_polygons_difference_3d(a, b)
        result_label = "Difference"
    else:
        result = offset_polygon_3d(a, offset_amt)
        result_label = f"Offset ({offset_amt:+.1f})"

    fig, ax = plt.subplots()
    plot_polygon(ax, [(p[0], p[1]) for p in a], "steelblue", f"A (Z={a_z})")
    if op != "Offset":
        plot_polygon(ax, [(p[0], p[1]) for p in b], "tomato", f"B (Z={b_z})")

    if result:
        for i, poly in enumerate(result):
            r_z = poly[0][2]
            if len(result) > 1:
                label = f"{result_label} {i} (Z={r_z})"
            else:
                label = f"{result_label} (Z={r_z})"
            poly_xy = [(p[0], p[1]) for p in poly]
            plot_polygon(ax, poly_xy, "limegreen", label, linewidth=2.5)

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9)
    st.pyplot(fig)

    if result:
        info_cols = st.columns(4)
        info_cols[0].metric("Result count", len(result))
        info_cols[1].metric(
            "Output Z",
            f"{result[0][0][2]:.1f}" if result else "—",
        )


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


def page_arc_linearize():
    st.header("Arc Linearization")
    st.write("Convert arcs into line segments at adjustable resolution.")

    c1, c2 = st.columns(2)
    r = c1.number_input("Arc radius", 1.0, 50.0, 10.0, key="al_r")
    arc_deg = c2.slider("Arc sweep (degrees)", 10, 360, 180, key="al_sweep")

    resolution = st.slider(
        "Linearization resolution", 0.1, 5.0, 1.0, key="al_res"
    )

    geom = Geometry()
    sweep_rad = math.radians(arc_deg)
    end_x = r * math.cos(sweep_rad)
    end_y = r * math.sin(sweep_rad)
    geom.move_to(r, 0, 0)
    geom.arc_to(end_x, end_y, -r, 0, False, 0)

    cmds = geom.iter_typed_commands()
    first_arc = None
    for cmd in cmds:
        if isinstance(cmd, Arc):
            first_arc = cmd
            break

    fig, axes = st.columns(2)
    fig_mpl, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    plot_geometry(ax1, geom, color="steelblue", linewidth=2.5)
    ax1.set_aspect("equal")
    ax1.grid(True, alpha=0.3)
    ax1.set_title(f"Original arc ({arc_deg}°)", fontsize=13)
    margin = r * 0.3
    ax1.set_xlim(-margin, r * 1.2 + margin)
    ax1.set_ylim(-r * 1.2 - margin, r * 1.2 + margin)

    segments = linearize_arc(first_arc, (r, 0.0, 0.0), resolution)
    for (sx, sy, _), (ex, ey, _) in segments:
        ax2.plot([sx, ex], [sy, ey], color="tomato", linewidth=2.5)
    ax2.scatter(
        [pt for seg in segments for pt in (seg[0], seg[1])],
        [pt for seg in segments for pt in (seg[0], seg[1])],
        color="tomato",
        s=20,
        zorder=3,
    )
    ax2.set_aspect("equal")
    ax2.grid(True, alpha=0.3)
    ax2.set_title(f"Linearized ({len(segments)} segments)", fontsize=13)
    ax2.set_xlim(ax1.get_xlim())
    ax2.set_ylim(ax1.get_ylim())

    fig_mpl.tight_layout()
    st.pyplot(fig_mpl)


def page_bezier_curves():
    st.header("Bezier Curve Operations")
    st.write("Split, evaluate, and flatten cubic bezier curves.")

    tab_split, tab_eval, tab_flatten = st.tabs(
        ["Split", "Point at t", "Flatten"]
    )

    p0, p1, p2, p3 = (0.0, 0.0), (0.0, 10.0), (12.0, 10.0), (12.0, 0.0)

    def eval_bezier(pts, n=100):
        ts = np.linspace(0, 1, n)
        result = []
        for t in ts:
            u = 1 - t
            x = (
                u**3 * pts[0][0]
                + 3 * u**2 * t * pts[1][0]
                + 3 * u * t**2 * pts[2][0]
                + t**3 * pts[3][0]
            )
            y = (
                u**3 * pts[0][1]
                + 3 * u**2 * t * pts[1][1]
                + 3 * u * t**2 * pts[2][1]
                + t**3 * pts[3][1]
            )
            result.append((x, y))
        return result

    def plot_curve(ax, pts, color, lw=3, label=None, ls="-"):
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        ax.plot(xs, ys, color=color, lw=lw, label=label, ls=ls)

    with tab_split:
        t_split = st.slider("Split parameter t", 0.01, 0.99, 0.4, 0.01)
        left, right = split_bezier(p0, p1, p2, p3, t_split)
        split_pt = get_bezier_point_at(p0, p1, p2, p3, t_split)
        curve = eval_bezier((p0, p1, p2, p3))
        left_curve = eval_bezier(left)
        right_curve = eval_bezier(right)

        fig, ax = plt.subplots(figsize=(8, 6))
        plot_curve(ax, curve, "gray", lw=2, ls="--", label="Original")
        plot_curve(ax, left_curve, "tomato", lw=3, label="Left half")
        plot_curve(ax, right_curve, "forestgreen", lw=3, label="Right half")
        ax.plot(
            split_pt[0],
            split_pt[1],
            "*",
            color="gold",
            markersize=15,
            zorder=5,
        )
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=11)
        ax.set_xlim(-1, 14)
        ax.set_ylim(-1, 12)
        st.pyplot(fig)

    with tab_eval:
        t_eval = st.slider("Parameter t", 0.0, 1.0, 0.6, 0.01, key="bez_eval")
        pt = get_bezier_point_at(p0, p1, p2, p3, t_eval)
        cp = [(0.0, 0.0), (0.0, 10.0), (12.0, 10.0), (12.0, 0.0)]
        pt = get_bezier_point_at(cp[0], cp[1], cp[2], cp[3], t_eval)
        curve = eval_bezier(cp)
        fig, ax = plt.subplots(figsize=(8, 6))
        plot_curve(ax, curve, "steelblue", lw=3)
        for cpi, cpv in enumerate(cp):
            ax.plot(cpv[0], cpv[1], "o", color="gray", markersize=6, zorder=4)
            ax.plot(pt[0], pt[1], "o", color="tomato", markersize=12, zorder=5)
            ax.annotate(
                f"t={t_eval}",
                pt,
                xytext=(8, 8),
                textcoords="offset points",
                fontsize=12,
                color="tomato",
                fontweight="bold",
            )
            ax.set_aspect("equal")
            ax.grid(True, alpha=0.3)
            ax.set_xlim(-1, 14)
            ax.set_ylim(-1, 12)
            st.pyplot(fig)

    with tab_flatten:
        c_tol, c_nsub = st.columns(2)
        tolerance = c_tol.slider("Tolerance", 0.1, 10.0, 2.0, 0.1)
        max_sub = c_nsub.number_input("Max subdivisions", 1, 64, 20, 1)
        pts = []
        p0_3d = (p0[0], p0[1], 0.0)
        p1_3d = (p1[0], p1[1], 0.0)
        p2_3d = (p2[0], p2[1], 0.0)
        p3_3d = (p3[0], p3[1], 0.0)
        flatten_bezier(p0_3d, p1_3d, p2_3d, p3_3d, tolerance, max_sub, pts)

        fig, ax = plt.subplots(figsize=(8, 6))
        curve = eval_bezier((p0, p1, p2, p3))
        plot_curve(ax, curve, "steelblue", lw=2, ls="--", label="Original")
        xs_f = [p[0] for p in pts]
        ys_f = [p[1] for p in pts]
        ax.plot(
            xs_f,
            ys_f,
            "o-",
            color="tomato",
            lw=2,
            markersize=4,
            label=f"Flattened ({len(pts)} pts)",
        )
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_xlim(-1, 14)
        ax.set_ylim(-1, 12)
        ax.legend(fontsize=11)
        st.pyplot(fig)


def page_circle_intersections():
    st.header("Circle Intersections")
    st.write("Find intersection points between circles and lines.")

    tab_cc, tab_lc = st.tabs(["Circle-Circle", "Line-Circle"])

    with tab_cc:
        c1, c2 = st.columns(2)
        with c1:
            cx1 = st.number_input(
                "Circle 1 X", -20.0, 20.0, -3.0, key="cc_cx1"
            )
            cy1 = st.number_input("Circle 1 Y", -20.0, 20.0, 0.0, key="cc_cy1")
            r1 = st.number_input(
                "Circle 1 radius", 0.1, 20.0, 5.0, key="cc_r1"
            )
        with c2:
            cx2 = st.number_input("Circle 2 X", -20.0, 20.0, 5.0, key="cc_cx2")
            cy2 = st.number_input("Circle 2 Y", -20.0, 20.0, 0.0, key="cc_cy2")
            r2 = st.number_input(
                "Circle 2 radius", 0.1, 20.0, 4.0, key="cc_r2"
            )

        fig, ax = plt.subplots(figsize=(8, 7))
        circ1 = mpatches.Circle(
            (cx1, cy1), r1, fill=False, edgecolor="steelblue", lw=2.5
        )
        circ2 = mpatches.Circle(
            (cx2, cy2), r2, fill=False, edgecolor="tomato", lw=2.5
        )
        ax.add_patch(circ1)
        ax.add_patch(circ2)
        ax.plot(cx1, cy1, "o", color="steelblue", markersize=6)
        ax.plot(cx2, cy2, "o", color="tomato", markersize=6)

        pts = get_circle_circle_intersections((cx1, cy1), r1, (cx2, cy2), r2)
        for pt in pts:
            ax.plot(pt[0], pt[1], "*", color="gold", markersize=18, zorder=5)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        margin = max(r1, r2) * 0.3
        all_xs = [cx1 - r1, cx1 + r1, cx2 - r2, cx2 + r2]
        all_ys = [cy1 - r1, cy1 + r1, cy2 - r2, cy2 + r2]
        ax.set_xlim(min(all_xs) - margin, max(all_xs) + margin)
        ax.set_ylim(min(all_ys) - margin, max(all_ys) + margin)
        ax.set_title(f"Circle-Circle: {len(pts)} intersection(s)", fontsize=13)
        if pts:
            st.success(f"Found {len(pts)} intersection point(s)")
        else:
            st.info(
                "No intersections (circles separated or one inside the other)"
            )
        st.pyplot(fig)

    with tab_lc:
        c1, c2 = st.columns(2)
        with c1:
            lx1 = st.number_input(
                "Line start X", -20.0, 20.0, -5.0, key="lc_lx1"
            )
            ly1 = st.number_input(
                "Line start Y", -20.0, 20.0, 3.0, key="lc_ly1"
            )
        with c2:
            lx2 = st.number_input(
                "Line end X", -20.0, 20.0, 10.0, key="lc_lx2"
            )
            ly2 = st.number_input(
                "Line end Y", -20.0, 20.0, -2.0, key="lc_ly2"
            )
        ccx = st.number_input("Circle center X", -20.0, 20.0, 3.0, key="lc_cx")
        ccy = st.number_input("Circle center Y", -20.0, 20.0, 0.0, key="lc_cy")
        cr = st.number_input("Circle radius", 0.1, 20.0, 5.0, key="lc_r")

        fig, ax = plt.subplots(figsize=(8, 7))
        circ = mpatches.Circle(
            (ccx, ccy), cr, fill=False, edgecolor="steelblue", lw=2.5
        )
        ax.add_patch(circ)
        ax.plot(
            [lx1, lx2],
            [ly1, ly2],
            color="tomato",
            lw=2.5,
            label="Line segment",
        )

        pts = get_line_circle_intersections(
            (lx1, ly1), (lx2, ly2), (ccx, ccy), cr
        )
        for pt in pts:
            ax.plot(pt[0], pt[1], "*", color="gold", markersize=18, zorder=5)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        margin = cr * 0.3
        all_xs = [lx1, lx2, ccx - cr, ccx + cr]
        all_ys = [ly1, ly2, ccy - cr, ccy + cr]
        ax.set_xlim(min(all_xs) - margin, max(all_xs) + margin)
        ax.set_ylim(min(all_ys) - margin, max(all_ys) + margin)
        ax.set_title(f"Line-Circle: {len(pts)} intersection(s)", fontsize=13)
        ax.legend(fontsize=11)
        if pts:
            st.success(f"Found {len(pts)} intersection point(s)")
        else:
            st.info("No intersections")
        st.pyplot(fig)


def page_line_intersections():
    st.header("Line Intersections")
    st.write(
        "Find intersections between lines and segments, and measure distances."
    )

    tab_ll, tab_ss, tab_dist = st.tabs(
        ["Line-Line", "Segment-Segment", "Point-Line Distance"]
    )

    with tab_ll:
        c1, c2 = st.columns(2)
        with c1:
            l1x1 = st.number_input(
                "Line 1 start X", -10.0, 20.0, 2.0, key="ll_l1x1"
            )
            l1y1 = st.number_input(
                "Line 1 start Y", -10.0, 20.0, 1.0, key="ll_l1y1"
            )
            l1x2 = st.number_input(
                "Line 1 end X", -10.0, 20.0, 10.0, key="ll_l1x2"
            )
            l1y2 = st.number_input(
                "Line 1 end Y", -10.0, 20.0, 8.0, key="ll_l1y2"
            )
        with c2:
            l2x1 = st.number_input(
                "Line 2 start X", -10.0, 20.0, 2.0, key="ll_l2x1"
            )
            l2y1 = st.number_input(
                "Line 2 start Y", -10.0, 20.0, 8.0, key="ll_l2y1"
            )
            l2x2 = st.number_input(
                "Line 2 end X", -10.0, 20.0, 10.0, key="ll_l2x2"
            )
            l2y2 = st.number_input(
                "Line 2 end Y", -10.0, 20.0, 1.0, key="ll_l2y2"
            )

        fig, ax = plt.subplots(figsize=(8, 7))
        ax.plot(
            [l1x1, l1x2],
            [l1y1, l1y2],
            "o-",
            color="steelblue",
            lw=2.5,
            label="Line 1",
        )
        ax.plot(
            [l2x1, l2x2],
            [l2y1, l2y2],
            "o-",
            color="tomato",
            lw=2.5,
            label="Line 2",
        )
        inter = get_line_line_intersection(
            (l1x1, l1y1), (l1x2, l1y2), (l2x1, l2y1), (l2x2, l2y2)
        )
        if inter:
            ax.plot(
                inter[0], inter[1], "*", color="gold", markersize=18, zorder=5
            )
            ax.annotate(
                f"({inter[0]:.2f}, {inter[1]:.2f})",
                inter,
                xytext=(5, 8),
                textcoords="offset points",
                fontsize=11,
                color="gold",
                fontweight="bold",
            )
            st.success(f"Intersection at ({inter[0]:.3f}, {inter[1]:.3f})")
        else:
            st.info("Lines are parallel — no intersection")
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=11)
        all_xs = [l1x1, l1x2, l2x1, l2x2]
        all_ys = [l1y1, l1y2, l2y1, l2y2]
        margin = 1
        ax.set_xlim(min(all_xs) - margin, max(all_xs) + margin)
        ax.set_ylim(min(all_ys) - margin, max(all_ys) + margin)
        st.pyplot(fig)

    with tab_ss:
        c1, c2 = st.columns(2)
        with c1:
            s1x1 = st.number_input(
                "Seg 1 start X", -10.0, 20.0, 2.0, key="ss_s1x1"
            )
            s1y1 = st.number_input(
                "Seg 1 start Y", -10.0, 20.0, 1.0, key="ss_s1y1"
            )
            s1x2 = st.number_input(
                "Seg 1 end X", -10.0, 20.0, 8.0, key="ss_s1x2"
            )
            s1y2 = st.number_input(
                "Seg 1 end Y", -10.0, 20.0, 7.0, key="ss_s1y2"
            )
        with c2:
            s2x1 = st.number_input(
                "Seg 2 start X", -10.0, 20.0, 2.0, key="ss_s2x1"
            )
            s2y1 = st.number_input(
                "Seg 2 start Y", -10.0, 20.0, 6.0, key="ss_s2y1"
            )
            s2x2 = st.number_input(
                "Seg 2 end X", -10.0, 20.0, 9.0, key="ss_s2x2"
            )
            s2y2 = st.number_input(
                "Seg 2 end Y", -10.0, 20.0, 2.0, key="ss_s2y2"
            )

        fig, ax = plt.subplots(figsize=(8, 7))
        ax.plot(
            [s1x1, s1x2],
            [s1y1, s1y2],
            "o-",
            color="steelblue",
            lw=3,
            label="Segment 1",
        )
        ax.plot(
            [s2x1, s2x2],
            [s2y1, s2y2],
            "o-",
            color="tomato",
            lw=3,
            label="Segment 2",
        )
        inter = get_line_segment_intersection(
            (s1x1, s1y1), (s1x2, s1y2), (s2x1, s2y1), (s2x2, s2y2)
        )
        if inter:
            ax.plot(
                inter[0], inter[1], "*", color="gold", markersize=18, zorder=5
            )
            ax.annotate(
                f"({inter[0]:.2f}, {inter[1]:.2f})",
                inter,
                xytext=(5, 8),
                textcoords="offset points",
                fontsize=11,
                color="gold",
                fontweight="bold",
            )
            st.success(
                f"Segments intersect at ({inter[0]:.3f}, {inter[1]:.3f})"
            )
        else:
            st.info("Segments do not intersect")
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=11)
        all_xs = [s1x1, s1x2, s2x1, s2x2]
        all_ys = [s1y1, s1y2, s2y1, s2y2]
        margin = 1
        ax.set_xlim(min(all_xs) - margin, max(all_xs) + margin)
        ax.set_ylim(min(all_ys) - margin, max(all_ys) + margin)
        st.pyplot(fig)

    with tab_dist:
        c1, c2 = st.columns(2)
        with c1:
            dlx1 = st.number_input(
                "Line point 1 X", -10.0, 20.0, 2.0, key="dist_lx1"
            )
            dly1 = st.number_input(
                "Line point 1 Y", -10.0, 20.0, 1.0, key="dist_ly1"
            )
            dlx2 = st.number_input(
                "Line point 2 X", -10.0, 20.0, 10.0, key="dist_lx2"
            )
            dly2 = st.number_input(
                "Line point 2 Y", -10.0, 20.0, 7.0, key="dist_ly2"
            )
        with c2:
            dpx = st.number_input("Point X", -10.0, 20.0, 4.0, key="dist_px")
            dpy = st.number_input("Point Y", -10.0, 20.0, 6.0, key="dist_py")

        point = (dpx, dpy)
        line_p1 = (dlx1, dly1)
        line_p2 = (dlx2, dly2)
        dist = get_point_line_distance(point, line_p1, line_p2)
        closest = get_line_closest_point(line_p1, line_p2, point[0], point[1])

        fig, ax = plt.subplots(figsize=(8, 7))
        ax.plot(
            [line_p1[0], line_p2[0]],
            [line_p1[1], line_p2[1]],
            "o-",
            color="steelblue",
            lw=2.5,
            label="Line",
        )
        ax.plot(
            point[0],
            point[1],
            "o",
            color="tomato",
            markersize=10,
            zorder=5,
            label="Point",
        )
        ax.plot(
            closest[0],
            closest[1],
            "o",
            color="forestgreen",
            markersize=8,
            zorder=5,
            label="Closest",
        )
        ax.plot(
            [point[0], closest[0]],
            [point[1], closest[1]],
            "--",
            color="forestgreen",
            lw=2,
            label=f"Distance = {dist:.3f}",
        )
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=11)
        all_xs = [dlx1, dlx2, dpx]
        all_ys = [dly1, dly2, dpy]
        margin = 1
        ax.set_xlim(min(all_xs) - margin, max(all_xs) + margin)
        ax.set_ylim(min(all_ys) - margin, max(all_ys) + margin)
        ax.set_title(f"Perpendicular distance: {dist:.3f}", fontsize=13)
        st.pyplot(fig)


def page_analysis():
    st.header("Geometry Analysis")
    st.write("Compute polygon area and determine winding order.")

    tab_area, tab_winding = st.tabs(["Area", "Winding Order"])

    with tab_area:
        preset = st.selectbox(
            "Shape preset",
            ["Rectangle", "Circle", "Star", "Custom"],
            key="area_preset",
        )
        if preset == "Rectangle":
            c1, c2 = st.columns(2)
            w = c1.number_input("Width", 0.1, 50.0, 10.0, key="area_rw")
            h = c2.number_input("Height", 0.1, 50.0, 8.0, key="area_rh")
            geom = Geometry.from_points([(0, 0), (w, 0), (w, h), (0, h)])
        elif preset == "Circle":
            r = st.number_input("Radius", 0.1, 50.0, 8.0, key="area_cr")
            n = st.number_input("Segments", 8, 128, 64, key="area_cn")
            pts = [
                (
                    r * math.cos(2 * math.pi * i / n),
                    r * math.sin(2 * math.pi * i / n),
                )
                for i in range(n)
            ]
            geom = Geometry.from_points(pts, close=True)
        elif preset == "Star":
            c1, c2, c3 = st.columns(3)
            outer_r = c1.number_input(
                "Outer radius", 0.1, 50.0, 10.0, key="area_or"
            )
            inner_r = c2.number_input(
                "Inner radius", 0.1, 50.0, 4.0, key="area_ir"
            )
            n_pts = c3.number_input("Points", 3, 64, 5, step=1, key="area_np")
            pts = []
            for i in range(n_pts * 2):
                a = -math.pi / 2 + math.pi * i / n_pts
                rd = outer_r if i % 2 == 0 else inner_r
                pts.append((rd * math.cos(a), rd * math.sin(a)))
            geom = Geometry.from_points(pts, close=True)
        else:
            pts_text = st.text_area(
                "Points (one per line: x,y)",
                "0,0\n10,0\n10,10\n0,10",
                key="area_pts",
            )
            close = st.checkbox("Close path", value=True, key="area_close")
            pts = []
            for line in pts_text.strip().splitlines():
                line = line.strip()
                if not line:
                    continue
                parts = line.split(",")
                pts.append((float(parts[0]), float(parts[1])))
            geom = (
                Geometry.from_points(pts, close=close) if pts else Geometry()
            )

        if not geom.is_empty():
            area = get_area(geom)
            try:
                winding = get_path_winding_order(geom, 0)
            except Exception:
                winding = "unknown"

            fig, ax = plt.subplots(figsize=(8, 7))
            plot_geometry(ax, geom, color="steelblue", linewidth=2.5)
            xmin, xmax, ymin, ymax = auto_limits([geom])
            ax.set_xlim(xmin, xmax)
            ax.set_ylim(ymin, ymax)
            ax.set_aspect("equal")
            ax.grid(True, alpha=0.3)
            ax.set_title(
                f"Area = {area:.4f}  |  Winding = {winding}", fontsize=13
            )
            st.pyplot(fig)

            cols = st.columns(3)
            cols[0].metric("Area", f"{area:.4f}")
            cols[1].metric("Winding", winding)
            cols[2].metric("Commands", len(geom))
        else:
            st.warning("Empty geometry — enter valid points")

    with tab_winding:
        st.write(
            "**CW vs CCW** — Polygons wound clockwise or counter-clockwise."
        )
        c1, c2 = st.columns(2)
        with c1:
            st.subheader("Counter-Clockwise (CCW)")
            ccw = Geometry.from_points(
                [(2, 2), (10, 2), (10, 10), (2, 10)], close=True
            )
            fig1, ax1 = plt.subplots(figsize=(5, 5))
            plot_geometry(ax1, ccw, color="steelblue", linewidth=2.5)
            w1 = get_path_winding_order(ccw, 0)
            ax1.set_title(f"CCW — {w1}", fontsize=12)
            ax1.set_aspect("equal")
            ax1.grid(True, alpha=0.3)
            ax1.set_xlim(0, 12)
            ax1.set_ylim(0, 12)
            st.pyplot(fig1)
        with c2:
            st.subheader("Clockwise (CW)")
            cw = Geometry()
            cw.move_to(2, 2, 0)
            cw.line_to(2, 10, 0)
            cw.line_to(10, 10, 0)
            cw.line_to(10, 2, 0)
            fig2, ax2 = plt.subplots(figsize=(5, 5))
            plot_geometry(ax2, cw, color="tomato", linewidth=2.5)
            w2 = get_path_winding_order(cw, 0)
            ax2.set_title(f"CW — {w2}", fontsize=12)
            ax2.set_aspect("equal")
            ax2.grid(True, alpha=0.3)
            ax2.set_xlim(0, 12)
            ax2.set_ylim(0, 12)
            st.pyplot(fig2)


def page_minkowski():
    st.header("Minkowski Sum")
    st.write("Compute the Minkowski sum of two convex polygons.")

    c1, c2 = st.columns(2)
    with c1:
        st.subheader("Polygon A (triangle)")
        a_size = st.number_input("Triangle size", 5.0, 100.0, 40.0, key="mk_a")
        a_off = st.number_input(
            "Triangle offset", -50.0, 50.0, 0.0, key="mk_ao"
        )
    with c2:
        st.subheader("Polygon B (square)")
        b_size = st.number_input("Square size", 5.0, 100.0, 20.0, key="mk_b")
        b_off = st.number_input("Square offset", -50.0, 50.0, 0.0, key="mk_bo")

    tri = [
        (a_off, a_off),
        (a_off + a_size, a_off),
        (a_off + a_size / 2, a_off + a_size * 0.875),
    ]
    sq = [
        (b_off, b_off),
        (b_off + b_size, b_off),
        (b_off + b_size, b_off + b_size),
        (b_off, b_off + b_size),
    ]

    result = get_polygon_minkowski_sum_convex(tri, sq)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_polygon(ax, tri, "steelblue", "Triangle (A)", linewidth=2.5)
    plot_polygon(ax, sq, "tomato", "Square (B)", linewidth=2.5)
    for poly in result:
        plot_polygon(
            ax, poly, "limegreen", "A ⊕ B (Minkowski sum)", linewidth=2.5
        )
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=11)

    all_pts = tri + sq
    for p in result:
        all_pts.extend(p)
    xs = [p[0] for p in all_pts]
    ys = [p[1] for p in all_pts]
    if xs:
        xmin, xmax = min(xs), max(xs)
        ymin, ymax = min(ys), max(ys)
        margin = max(xmax - xmin, ymax - ymin) * 0.25 + 20
        ax.set_xlim(xmin - margin, xmax + margin)
        ax.set_ylim(ymin - margin, ymax + margin)

    fig.tight_layout()
    st.pyplot(fig)

    if result:
        st.success(f"Minkowski sum produced {len(result)} polygon(s)")
    else:
        st.warning("No result — polygons may not be convex")


def page_inner_fit_polygon():
    st.header("Inner Fit Polygon (IFP)")
    st.write("Compute the valid placement region for a part inside a bin.")

    c1, c2 = st.columns(2)
    with c1:
        bin_w = st.number_input("Bin width", 20.0, 300.0, 100.0, key="ifp_bw")
        bin_h = st.number_input("Bin height", 20.0, 300.0, 80.0, key="ifp_bh")
    with c2:
        part_w = st.number_input("Part width", 5.0, 100.0, 30.0, key="ifp_pw")
        part_h = st.number_input("Part height", 5.0, 100.0, 25.0, key="ifp_ph")

    bin_poly = [(0.0, 0.0), (bin_w, 0.0), (bin_w, bin_h), (0.0, bin_h)]
    part = [(0.0, 0.0), (part_w, 0.0), (part_w, part_h), (0.0, part_h)]

    ifp_result = inner_fit_polygon(bin_poly, part)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_polygon(ax, bin_poly, "black", "Bin", linewidth=2.5)
    if ifp_result:
        plot_polygon(
            ax, ifp_result[0], "limegreen", "IFP (valid region)", linewidth=2.5
        )
        xs = [p[0] for p in ifp_result[0]] + [ifp_result[0][0][0]]
        ys = [p[1] for p in ifp_result[0]] + [ifp_result[0][0][1]]
        ax.fill(xs, ys, alpha=0.08, color="limegreen")

    sample_x = st.slider(
        "Sample part X", 0.0, max(bin_w - part_w, 1.0), 15.0, key="ifp_sx"
    )
    sample_y = st.slider(
        "Sample part Y", 0.0, max(bin_h - part_h, 1.0), 12.0, key="ifp_sy"
    )
    shifted = [(p[0] + sample_x, p[1] + sample_y) for p in part]
    plot_polygon(ax, shifted, "tomato", "Part (placed example)", linewidth=2.5)
    xs_s = [p[0] for p in shifted] + [shifted[0][0]]
    ys_s = [p[1] for p in shifted] + [shifted[0][1]]
    ax.fill(xs_s, ys_s, alpha=0.15, color="tomato")

    ax.set_aspect("equal")
    ax.set_xlim(-bin_w * 0.1, bin_w * 1.1)
    ax.set_ylim(-bin_h * 0.1, bin_h * 1.1)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2 = st.columns(2)
    c1.metric("IFP polygons", len(ifp_result))
    if ifp_result:
        poly = ifp_result[0]
        ifp_area = abs(
            sum(
                poly[i][0] * poly[(i + 1) % len(poly)][1]
                - poly[(i + 1) % len(poly)][0] * poly[i][1]
                for i in range(len(poly))
            )
            * 0.5
        )
        c2.metric("IFP area", f"{ifp_area:.1f}")


def page_gravity():
    st.header("Gravity Tightening")
    st.write("Apply gravity sliding to tighten a nesting layout.")

    c1, c2, c3 = st.columns(3)
    with c1:
        n_parts = st.slider("Number of parts", 2, 20, 8, key="grav_n")
    with c2:
        size = st.slider("Part size", 10, 80, 25, key="grav_size")
    with c3:
        spacing = st.slider("Spacing", 0.0, 10.0, 2.0, 0.5, key="grav_spc")

    sheet_w = st.number_input("Sheet width", 50, 500, 160, key="grav_sw")
    sheet_h = st.number_input("Sheet height", 50, 500, 120, key="grav_sh")

    if st.button("Run Gravity", type="primary", key="grav_run"):
        rng = np.random.default_rng(42)

        def _make_part(i):
            if i % 2 == 0:
                w = size * (0.5 + 0.5 * rng.random())
                h = size * (0.5 + 0.5 * rng.random())
                return [(0, 0), (w, 0), (w, h), (0, h)]
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

        parts = [_make_part(i) for i in range(n_parts)]

        cols = 4
        placed_groups = []
        for i, poly in enumerate(parts):
            bx = min(p[0] for p in poly)
            by = min(p[1] for p in poly)
            col = i % cols
            row = i // cols
            ox = col * (size * 1.5) + 10 + rng.uniform(0, size * 0.3)
            oy = row * (size * 1.5) + 10 + rng.uniform(0, size * 0.3)
            shifted = [(p[0] - bx + ox, p[1] - by + oy) for p in poly]
            placed_groups.append([shifted])

        sheet_poly = [
            (0.0, 0.0),
            (sheet_w, 0.0),
            (sheet_w, sheet_h),
            (0.0, sheet_h),
        ]

        with st.spinner("Applying gravity..."):
            adjustments = apply_gravity(placed_groups, sheet_poly, spacing)

        cmap = plt.get_cmap("tab10")
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))

        for ax in (ax1, ax2):
            ax.plot(
                [p[0] for p in sheet_poly] + [sheet_poly[0][0]],
                [p[1] for p in sheet_poly] + [sheet_poly[0][1]],
                color="black",
                linewidth=2,
            )

        for pi, polys in enumerate(placed_groups):
            for poly in polys:
                px = [p[0] for p in poly] + [poly[0][0]]
                py = [p[1] for p in poly] + [poly[0][1]]
                color = to_hex(cmap(pi % 10))
                ax1.fill(px, py, alpha=0.25, color=color)
                ax1.plot(px, py, color=color, linewidth=1.5)

        for pi, (polys, adj) in enumerate(zip(placed_groups, adjustments)):
            for poly in polys:
                shifted = [(p[0] + adj[0], p[1] + adj[1]) for p in poly]
                px = [p[0] for p in shifted] + [shifted[0][0]]
                py = [p[1] for p in shifted] + [shifted[0][1]]
                color = to_hex(cmap(pi % 10))
                ax2.fill(px, py, alpha=0.25, color=color)
                ax2.plot(px, py, color=color, linewidth=1.5)

        for ax, title in zip(
            (ax1, ax2),
            ("Before gravity (loose placement)", "After gravity (tightened)"),
        ):
            ax.set_aspect("equal")
            ax.grid(True, alpha=0.3)
            ax.set_title(title, fontsize=14)

        fig.tight_layout()
        st.pyplot(fig)
        st.success(f"Applied {len(adjustments)} adjustments")

        c1, c2, c3 = st.columns(3)
        c1.metric("Parts", len(placed_groups))
        c2.metric("Adjustments", len(adjustments))
        if adjustments:
            total_dx = sum(abs(a[0]) for a in adjustments)
            total_dy = sum(abs(a[1]) for a in adjustments)
            c3.metric("Total movement", f"{total_dx + total_dy:.1f}")
    else:
        st.info("Configure parameters and click **Run Gravity**.")


def page_ops_optimize_travel():
    st.header("Travel Optimization")
    st.write("Optimize the travel (non-cutting) path by reordering segments.")

    preset = st.selectbox(
        "Preset",
        ["Multiple rectangles", "Triangle + squares", "Scattered segments"],
        key="opt_preset",
    )

    ops = Ops()
    ops.set_power(1.0)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")

    if preset == "Multiple rectangles":
        ops.move_to(10, 10, 0)
        ops.line_to(30, 10, 0)
        ops.line_to(30, 30, 0)
        ops.line_to(10, 30, 0)
        ops.line_to(10, 10, 0)
        ops.move_to(50, 50, 0)
        ops.line_to(70, 50, 0)
        ops.line_to(70, 70, 0)
        ops.line_to(50, 70, 0)
        ops.line_to(50, 50, 0)
        ops.move_to(10, 60, 0)
        ops.line_to(30, 60, 0)
        ops.line_to(30, 80, 0)
        ops.line_to(10, 80, 0)
        ops.line_to(10, 60, 0)
        ops.move_to(60, 10, 0)
        ops.line_to(80, 10, 0)
        ops.line_to(80, 30, 0)
        ops.line_to(60, 30, 0)
        ops.line_to(60, 10, 0)
    elif preset == "Triangle + squares":
        ops.move_to(5, 75, 0)
        ops.line_to(20, 75, 0)
        ops.line_to(20, 90, 0)
        ops.line_to(5, 90, 0)
        ops.line_to(5, 75, 0)
        ops.move_to(80, 5, 0)
        ops.line_to(95, 5, 0)
        ops.line_to(95, 20, 0)
        ops.line_to(80, 20, 0)
        ops.line_to(80, 5, 0)
        ops.move_to(50, 40, 0)
        ops.line_to(70, 80, 0)
        ops.line_to(30, 80, 0)
        ops.line_to(50, 40, 0)
    else:
        ops.move_to(10, 10, 0)
        ops.line_to(40, 10, 0)
        ops.move_to(60, 70, 0)
        ops.line_to(80, 50, 0)
        ops.move_to(30, 80, 0)
        ops.line_to(50, 80, 0)
        ops.line_to(50, 60, 0)
        ops.move_to(70, 20, 0)
        ops.line_to(90, 20, 0)
        ops.line_to(90, 40, 0)

    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    orig = ops.copy()
    ops_noflip = ops.copy()
    ops_flip = ops.copy()
    ops_noflip.optimize_travel(allow_flip=False)
    ops_flip.optimize_travel(allow_flip=True)

    before_travel = orig.distance() - orig.cut_distance()
    travel_noflip = ops_noflip.distance() - ops_noflip.cut_distance()
    travel_flip = ops_flip.distance() - ops_flip.cut_distance()

    fig, (ax1, ax2, ax3) = plt.subplots(1, 3, figsize=(22, 7))

    def _plot(ax, seq, title, travel_d):
        seq.preload_state()
        pos = (0.0, 0.0, 0.0)
        for i in range(seq.len()):
            ct = seq.command_type(i)
            if ct == CommandType.MOVE_TO:
                ep = seq.endpoint(i)
                if pos != ep:
                    ax.annotate(
                        "",
                        xy=(ep[0], ep[1]),
                        xytext=(pos[0], pos[1]),
                        arrowprops=dict(
                            arrowstyle="->",
                            color="gray",
                            lw=1.5,
                            linestyle=":",
                        ),
                    )
                pos = ep
                continue
            if ct == CommandType.LINE_TO:
                ep = seq.endpoint(i)
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="steelblue",
                    linewidth=3,
                    solid_capstyle="round",
                )
                pos = ep
        ax.plot([], [], color="steelblue", linewidth=3, label="Cut")
        ax.plot(
            [], [], color="gray", linewidth=1.5, linestyle=":", label="Travel"
        )
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=10)
        ax.set_title(f"{title}\nTravel: {travel_d:.1f}", fontsize=12)

    _plot(ax1, orig, "Before optimization", before_travel)
    _plot(ax2, ops_noflip, "Optimized (no flip)", travel_noflip)
    _plot(ax3, ops_flip, "Optimized (with flip)", travel_flip)

    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Cut distance", f"{orig.cut_distance():.1f}")
    c2.metric("Travel (no flip)", f"{travel_noflip:.1f}")
    c3.metric("Travel (with flip)", f"{travel_flip:.1f}")


def page_ops_clip():
    st.header("Ops Clipping")
    st.write("Clip Ops paths to rectangles and polygonal regions.")

    preset = st.selectbox(
        "Shape preset",
        ["Nested squares + triangle", "Overlapping lines", "Cross pattern"],
        key="clip_preset",
    )

    ops = Ops()
    ops.set_power(1.0)

    if preset == "Nested squares + triangle":
        ops.move_to(10, 10, 0)
        ops.line_to(90, 10, 0)
        ops.line_to(90, 90, 0)
        ops.line_to(10, 90, 0)
        ops.line_to(10, 10, 0)
        ops.move_to(30, 30, 0)
        ops.line_to(70, 30, 0)
        ops.line_to(70, 70, 0)
        ops.line_to(30, 70, 0)
        ops.line_to(30, 30, 0)
        ops.move_to(20, 40, 0)
        ops.line_to(80, 40, 0)
        ops.line_to(50, 80, 0)
        ops.line_to(20, 40, 0)
    elif preset == "Overlapping lines":
        ops.move_to(5, 50, 0)
        ops.line_to(95, 50, 0)
        ops.move_to(50, 5, 0)
        ops.line_to(50, 95, 0)
        ops.move_to(10, 10, 0)
        ops.line_to(90, 90, 0)
        ops.move_to(10, 90, 0)
        ops.line_to(90, 10, 0)
    else:
        ops.move_to(10, 50, 0)
        ops.line_to(90, 50, 0)
        ops.move_to(50, 10, 0)
        ops.line_to(50, 90, 0)
        ops.move_to(10, 10, 0)
        ops.line_to(90, 90, 0)
        ops.move_to(90, 10, 0)
        ops.line_to(10, 90, 0)

    c1, c2 = st.columns(2)
    with c1:
        rx1 = st.slider("Clip rect X1", 0.0, 100.0, 25.0, key="clip_rx1")
        ry1 = st.slider("Clip rect Y1", 0.0, 100.0, 25.0, key="clip_ry1")
    with c2:
        rx2 = st.slider("Clip rect X2", 0.0, 100.0, 75.0, key="clip_rx2")
        ry2 = st.slider("Clip rect Y2", 0.0, 100.0, 85.0, key="clip_ry2")

    clip_rect = (rx1, ry1, rx2, ry2)
    clipped = ops.clip_rect(clip_rect)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))

    def _plot(ax, seq, title):
        seq.preload_state()
        pos = (0.0, 0.0, 0.0)
        for i in range(seq.len()):
            ct = seq.command_type(i)
            if ct == CommandType.MOVE_TO:
                pos = seq.endpoint(i)
                continue
            if ct == CommandType.LINE_TO:
                ep = seq.endpoint(i)
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="steelblue",
                    linewidth=2.5,
                    solid_capstyle="round",
                )
                pos = ep
        rect = mpatches.Rectangle(
            (clip_rect[0], clip_rect[1]),
            clip_rect[2] - clip_rect[0],
            clip_rect[3] - clip_rect[1],
            fill=False,
            edgecolor="tomato",
            linewidth=2,
            linestyle="--",
            label="Clip rect",
        )
        ax.add_patch(rect)
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=10)
        ax.set_title(title, fontsize=13)

    _plot(ax1, ops, "Original paths")
    _plot(ax2, clipped, "After clip_rect")

    fig.tight_layout()
    st.pyplot(fig)

    c1, c2 = st.columns(2)
    c1.metric("Original commands", ops.len())
    c2.metric("Clipped commands", clipped.len())


st.set_page_config(layout="wide", page_title="raygeo visual test")
st.title("raygeo Visual Test")

page = st.sidebar.radio(
    "Page",
    [
        "Geometry",
        "Arc Linearization",
        "Bezier Curves",
        "Circle Intersections",
        "Line Intersections",
        "Geometry Analysis",
        "Polygon Boolean",
        "Polygon Offset",
        "Polygon 3D",
        "Image Processing",
        "SVG Parsing",
        "Tab Operations",
        "Merge Lines",
        "Overscan",
        "Lead-In/Out",
        "Rasterization",
        "Concave Hull",
        "Nesting",
        "Minkowski Sum",
        "Inner Fit Polygon",
        "Gravity",
        "Travel Optimization",
        "Ops Clipping",
    ],
)

if page == "Geometry":
    page_geometry()
elif page == "Arc Linearization":
    page_arc_linearize()
elif page == "Bezier Curves":
    page_bezier_curves()
elif page == "Circle Intersections":
    page_circle_intersections()
elif page == "Line Intersections":
    page_line_intersections()
elif page == "Geometry Analysis":
    page_analysis()
elif page == "Polygon Boolean":
    page_polygon_boolean()
elif page == "Polygon Offset":
    page_offset()
elif page == "Polygon 3D":
    page_polygon3d()
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
elif page == "Minkowski Sum":
    page_minkowski()
elif page == "Inner Fit Polygon":
    page_inner_fit_polygon()
elif page == "Gravity":
    page_gravity()
elif page == "Travel Optimization":
    page_ops_optimize_travel()
elif page == "Ops Clipping":
    page_ops_clip()
