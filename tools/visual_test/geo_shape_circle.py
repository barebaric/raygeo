import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.shape.circle import (
    find_tangent_circle_centers,
    get_circle_circle_intersections,
    get_line_circle_intersections,
    nearest_tangent_circle_on_polyline,
)


def page_circle_intersections():
    st.header("Circle Intersections")
    st.write("Find intersection points between circles and lines.")

    tab_cc, tab_lc, tab_tc, tab_np = st.tabs(
        [
            "Circle-Circle",
            "Line-Circle",
            "Tangent Circles",
            "Nearest on Polyline",
        ]
    )

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

    with tab_tc:
        st.header("Tangent Circles")
        st.write(
            "Find circles of a given radius that pass through a point "
            "and are tangent to a segment."
        )

        c1, c2 = st.columns(2)
        with c1:
            pt_x = st.number_input(
                "Pass-through X", -100.0, 100.0, 6.0, key="tc_ptx"
            )
            pt_y = st.number_input(
                "Pass-through Y", -100.0, 100.0, 5.0, key="tc_pty"
            )
        with c2:
            radius = st.number_input("Radius", 0.01, 100.0, 3.0, key="tc_r")

        st.subheader("Segment")
        cc1, cc2 = st.columns(2)
        with cc1:
            ax1 = st.number_input("A X", -100.0, 100.0, 2.0, key="tc_ax")
            ay1 = st.number_input("A Y", -100.0, 100.0, 0.0, key="tc_ay")
        with cc2:
            bx1 = st.number_input("B X", -100.0, 100.0, 10.0, key="tc_bx")
            by1 = st.number_input("B Y", -100.0, 100.0, 0.0, key="tc_by")

        results = find_tangent_circle_centers(
            (pt_x, pt_y), (ax1, ay1), (bx1, by1), radius
        )

        fig, ax = plt.subplots(figsize=(9, 8))
        ax.plot(
            [ax1, bx1],
            [ay1, by1],
            color="steelblue",
            lw=3,
            label="Segment",
        )
        ax.plot(
            pt_x,
            pt_y,
            "o",
            color="k",
            markersize=10,
            label="Pass-through",
        )

        colors = ["tomato", "limegreen", "gold", "mediumpurple"]
        for i, (center, tangent) in enumerate(results):
            c = colors[i % len(colors)]
            circ = mpatches.Circle(
                center,
                radius,
                fill=False,
                edgecolor=c,
                lw=2,
                linestyle="--",
            )
            ax.add_patch(circ)
            ax.plot(
                center[0],
                center[1],
                "s",
                color=c,
                markersize=8,
            )
            ax.plot(
                tangent[0],
                tangent[1],
                "*",
                color=c,
                markersize=12,
            )

        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        all_xs = (
            [pt_x, ax1, bx1]
            + [c[0] - radius for c, _ in results]
            + [c[0] + radius for c, _ in results]
        )
        all_ys = (
            [pt_y, ay1, by1]
            + [c[1] - radius for c, _ in results]
            + [c[1] + radius for c, _ in results]
        )
        if all_xs:
            margin = (
                max(
                    max(all_xs) - min(all_xs),
                    max(all_ys) - min(all_ys),
                    radius,
                )
                * 0.3
                + 1
            )
            ax.set_xlim(min(all_xs) - margin, max(all_xs) + margin)
            ax.set_ylim(min(all_ys) - margin, max(all_ys) + margin)
        ax.set_title(
            f"Tangent Circles: {len(results)} result(s), r={radius}",
            fontsize=13,
        )
        ax.legend(fontsize=10)
        if results:
            st.success(f"Found {len(results)} tangent circle(s)")
        else:
            st.info(
                "No tangent circles — radius too small or point/segment "
                "geometry doesn't allow it"
            )
        st.pyplot(fig)

    with tab_np:
        st.header("Nearest Tangent Circle on Polyline")
        st.write(
            "Find a circle tangent to a polyline, passing through a point, "
            "with centre inside a containment polygon."
        )

        c1, c2 = st.columns(2)
        with c1:
            pt_x = st.number_input("Point X", -100.0, 100.0, 6.0, key="np_ptx")
            pt_y = st.number_input("Point Y", -100.0, 100.0, 6.0, key="np_pty")
            radius = st.number_input("Radius", 0.01, 100.0, 3.0, key="np_r")
            from_end = st.checkbox("Search from end", value=False, key="np_fe")
        with c2:
            st.subheader("Polyline vertices")
            n_verts = st.number_input("Vertex count", 2, 10, 3, key="np_nv")
            poly_pts = []
            for vi in range(n_verts):
                cc = st.columns(2)
                x = cc[0].number_input(
                    f"V{vi} X",
                    -100.0,
                    100.0,
                    2.0 + 8.0 * vi / max(n_verts - 1, 1),
                    key=f"np_vx{vi}",
                )
                y = cc[1].number_input(
                    f"V{vi} Y",
                    -100.0,
                    100.0,
                    2.0 if vi % 2 == 0 else -2.0,
                    key=f"np_vy{vi}",
                )
                poly_pts.append((x, y))

        st.subheader("Containment polygon")
        c_pts_input = st.text_input(
            "Vertices as x1,y1 x2,y2 ...",
            value="0,-5 14,-5 14,12 0,12",
            key="np_cont",
        )
        cont_pts = []
        for token in c_pts_input.strip().split():
            parts = token.split(",")
            if len(parts) == 2:
                try:
                    cont_pts.append((float(parts[0]), float(parts[1])))
                except ValueError:
                    pass

        result = nearest_tangent_circle_on_polyline(
            (pt_x, pt_y), poly_pts, radius, from_end, cont_pts
        )

        fig, ax = plt.subplots(figsize=(9, 8))
        xs = [p[0] for p in poly_pts]
        ys = [p[1] for p in poly_pts]
        ax.plot(
            xs,
            ys,
            "-o",
            color="steelblue",
            lw=2.5,
            markerfacecolor="lightblue",
            markeredgecolor="steelblue",
            markersize=7,
            label="Polyline",
        )

        if len(cont_pts) >= 3:
            cxs = [p[0] for p in cont_pts] + [cont_pts[0][0]]
            cys = [p[1] for p in cont_pts] + [cont_pts[0][1]]
            ax.plot(
                cxs, cys, color="gray", lw=1.5, ls="--", label="Containment"
            )
            ax.fill(cxs, cys, color="gray", alpha=0.08)

        ax.plot(pt_x, pt_y, "o", color="k", markersize=10, label="Point")

        if result is not None:
            center, tangent, idx = result
            circ = mpatches.Circle(
                center,
                radius,
                fill=False,
                edgecolor="tomato",
                lw=2,
                linestyle="--",
            )
            ax.add_patch(circ)
            ax.plot(
                center[0],
                center[1],
                "s",
                color="tomato",
                markersize=9,
                label="Centre",
            )
            ax.plot(
                tangent[0],
                tangent[1],
                "*",
                color="gold",
                markersize=16,
                label="Tangent",
            )
            ax.plot(
                [center[0], pt_x],
                [center[1], pt_y],
                color="gray",
                lw=1,
                ls=":",
            )
            ax.plot(
                [center[0], tangent[0]],
                [center[1], tangent[1]],
                color="gray",
                lw=1,
                ls=":",
            )
            dir_label = "from end" if from_end else "from start"
            st.success(
                f"Found: centre ({center[0]:.3f}, {center[1]:.3f}), "
                f"tangent ({tangent[0]:.3f}, {tangent[1]:.3f}), "
                f"segment {idx} ({dir_label})"
            )
        else:
            st.info("No valid tangent circle found")

        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        cx_min = cx_max = pt_x
        cy_min = cy_max = pt_y
        if result is not None:
            center, _tangent, _idx = result
            cx_min = cx_max = center[0]
            cy_min = cy_max = center[1]
        margin = max(radius, 5)
        all_xs = xs + [pt_x, cx_min - radius, cx_max + radius]
        for cp in cont_pts:
            all_xs.append(cp[0])
        all_ys = ys + [pt_y, cy_min - radius, cy_max + radius]
        for cp in cont_pts:
            all_ys.append(cp[1])
        x_range = max(all_xs) - min(all_xs) if all_xs else 20
        y_range = max(all_ys) - min(all_ys) if all_ys else 20
        pad = max(x_range, y_range, 10) * 0.2 + 1
        ax.set_xlim(min(all_xs) - pad, max(all_xs) + pad)
        ax.set_ylim(min(all_ys) - pad, max(all_ys) + pad)
        ax.set_title(
            f"Nearest tangent circle (r={radius}) — "
            f"{'from end' if from_end else 'from start'}",
            fontsize=13,
        )
        ax.legend(fontsize=10)
        st.pyplot(fig)
