import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.shape.line import (
    get_line_closest_point,
    get_line_line_intersection,
    get_line_segment_intersection,
    get_point_line_distance,
    interpolated_segment_3d,
)


def page_line_intersections():
    st.header("Line Intersections")
    st.write(
        "Find intersections between lines and segments, and measure distances."
    )

    tab_ll, tab_ss, tab_dist, tab_interp = st.tabs(
        [
            "Line-Line",
            "Segment-Segment",
            "Point-Line Distance",
            "Interpolated Segment",
        ]
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

    with tab_interp:
        st.header("Interpolated Segment 3D")
        st.write(
            "Generate evenly-spaced 3D points along a 2D line segment "
            "at a fixed Z height."
        )

        c1, c2 = st.columns(2)
        with c1:
            fx = st.number_input("From X", -50.0, 50.0, 2.0, key="ip_fx")
            fy = st.number_input("From Y", -50.0, 50.0, 2.0, key="ip_fy")
        with c2:
            tx = st.number_input("To X", -50.0, 50.0, 10.0, key="ip_tx")
            ty = st.number_input("To Y", -50.0, 50.0, 8.0, key="ip_ty")
        z = st.number_input("Z height", -50.0, 50.0, 5.0, key="ip_z")
        n = st.slider("Number of points", 1, 50, 8, key="ip_n")

        pts = interpolated_segment_3d(fx, fy, tx, ty, z, n)

        fig, ax = plt.subplots(figsize=(8, 7))
        ax.plot(
            [fx, tx],
            [fy, ty],
            color="steelblue",
            lw=2,
            label="Segment (XY)",
        )
        ax.plot(
            [p[0] for p in pts],
            [p[1] for p in pts],
            "o",
            color="tomato",
            markersize=8,
            label=f"Interpolated ({n} pts, Z={z})",
        )
        ax.plot(fx, fy, "o", color="k", markersize=8, label="From")
        ax.plot(tx, ty, "s", color="k", markersize=8, label="To")

        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=11)
        margin = 2
        all_xs = [fx, tx] + [p[0] for p in pts]
        all_ys = [fy, ty] + [p[1] for p in pts]
        ax.set_xlim(min(all_xs) - margin, max(all_xs) + margin)
        ax.set_ylim(min(all_ys) - margin, max(all_ys) + margin)
        ax.set_title(f"Interpolated segment: {n} point(s), Z={z}", fontsize=13)
        if pts:
            st.success(
                f"Generated {len(pts)} point(s): "
                f"from ({pts[0][0]:.2f}, {pts[0][1]:.2f}, "
                f"{pts[0][2]:.2f}) "
                f"to ({pts[-1][0]:.2f}, {pts[-1][1]:.2f}, "
                f"{pts[-1][2]:.2f})"
            )
        else:
            st.info("No points generated")
        st.pyplot(fig)
