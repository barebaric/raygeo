import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.shape.circle import (
    get_circle_circle_intersections,
    get_line_circle_intersections,
)


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
