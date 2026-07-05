import math

import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo import Arc, Geometry
from raygeo.geo.shape.arc import get_arc_through_point, linearize_arc
from tools.plot import plot_geometry


def page_arc_linearize():
    st.header("Arc Operations")
    st.write("Construct and linearize arcs.")

    tab_lin, tab_atp = st.tabs(["Linearize", "Arc Through Point"])

    with tab_lin:
        c1, c2 = st.columns(2)
        r = c1.number_input("Arc radius", 1.0, 50.0, 10.0, key="al_r")
        arc_deg = c2.slider(
            "Arc sweep (degrees)", 10, 360, 180, key="al_sweep"
        )

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

        fig_mpl, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

        plot_geometry(ax1, geom, color="steelblue", linewidth=2.5)
        ax1.set_aspect("equal")
        ax1.grid(True, alpha=0.3)
        ax1.set_title(f"Original arc ({arc_deg}°)", fontsize=13)
        margin = r * 0.3
        ax1.set_xlim(-margin, r * 1.2 + margin)
        ax1.set_ylim(-r * 1.2 - margin, r * 1.2 + margin)

        segments = linearize_arc(first_arc, (r, 0.0, 0.0), resolution)
        pts_x, pts_y = [], []
        for (sx, sy, _), (ex, ey, _) in segments:
            ax2.plot([sx, ex], [sy, ey], color="tomato", linewidth=2.5)
            pts_x.extend([sx, ex])
            pts_y.extend([sy, ey])
        ax2.scatter(pts_x, pts_y, color="tomato", s=20, zorder=3)
        ax2.set_aspect("equal")
        ax2.grid(True, alpha=0.3)
        ax2.set_title(f"Linearized ({len(segments)} segments)", fontsize=13)
        ax2.set_xlim(ax1.get_xlim())
        ax2.set_ylim(ax1.get_ylim())

        fig_mpl.tight_layout()
        st.pyplot(fig_mpl)

    with tab_atp:
        st.header("Arc Through Point")
        st.write("Build a circular arc through three points around a centre.")

        c1, c2 = st.columns(2)
        with c1:
            cx = st.number_input("Centre X", -50.0, 50.0, 0.0, key="atp_cx")
            cy = st.number_input("Centre Y", -50.0, 50.0, 0.0, key="atp_cy")
        with c2:
            r = st.number_input("Radius", 0.1, 50.0, 5.0, key="atp_r")

        st.subheader("Start point")
        c1, c2 = st.columns(2)
        sx = c1.number_input("X", -50.0, 50.0, 5.0, key="atp_sx")
        sy = c2.number_input("Y", -50.0, 50.0, 0.0, key="atp_sy")

        st.subheader("End point")
        c1, c2 = st.columns(2)
        ex = c1.number_input("X", -50.0, 50.0, 0.0, key="atp_ex")
        ey = c2.number_input("Y", -50.0, 50.0, 5.0, key="atp_ey")

        st.subheader("Mid (pass-through) point")
        c1, c2 = st.columns(2)
        mx = c1.number_input("X", -50.0, 50.0, 3.5355, key="atp_mx")
        my = c2.number_input("Y", -50.0, 50.0, 3.5355, key="atp_my")

        arc = get_arc_through_point((sx, sy), (ex, ey), (mx, my), (cx, cy), r)

        fig, ax = plt.subplots(figsize=(8, 8))
        xs = [p[0] for p in arc]
        ys = [p[1] for p in arc]
        ax.plot(
            xs,
            ys,
            "-o",
            color="steelblue",
            lw=2.5,
            markerfacecolor="lightblue",
            markeredgecolor="steelblue",
            markersize=4,
            label="Arc",
        )

        ax.plot(cx, cy, "x", color="gray", markersize=12, label="Centre")
        ax.plot(sx, sy, "o", color="k", markersize=10, label="Start")
        ax.plot(ex, ey, "s", color="tomato", markersize=10, label="End")
        ax.plot(
            mx,
            my,
            "*",
            color="gold",
            markersize=14,
            label="Mid (pass-through)",
        )

        if arc:
            st.success(
                f"Arc generated: {len(arc)} points, "
                f"first ({arc[0][0]:.2f}, {arc[0][1]:.2f}), "
                f"last ({arc[-1][0]:.2f}, {arc[-1][1]:.2f})"
            )
        else:
            st.info("No arc generated")

        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        pad = max(r * 1.5, 5)
        all_xs = [sx, ex, mx, cx] + [p[0] for p in arc]
        all_ys = [sy, ey, my, cy] + [p[1] for p in arc]
        ax.set_xlim(min(all_xs) - pad, max(all_xs) + pad)
        ax.set_ylim(min(all_ys) - pad, max(all_ys) + pad)
        ax.set_title(f"Arc through point (r={r})", fontsize=14)
        ax.legend(fontsize=11)
        fig.tight_layout()
        st.pyplot(fig)
