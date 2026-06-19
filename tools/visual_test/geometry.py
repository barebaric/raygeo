import math

import matplotlib.pyplot as plt
import streamlit as st
from matplotlib.colors import to_hex

from raygeo.geo import Arc, Bezier, Geometry, Line, Move
from tools.plot import auto_limits, plot_geometry


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
