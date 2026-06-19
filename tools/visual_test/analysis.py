import math

import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo import Geometry
from raygeo.geo.algo.analysis import get_area, get_path_winding_order
from tools.plot import auto_limits, plot_geometry


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
