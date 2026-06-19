import math

import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.shape.polygon3d import (
    get_polygons_difference_3d,
    get_polygons_intersection_3d,
    get_polygons_union_3d,
    offset_polygon_3d,
)
from tools.plot import plot_polygon


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
