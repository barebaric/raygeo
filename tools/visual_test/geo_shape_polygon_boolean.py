import math

import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.shape.polygon import (
    get_polygons_difference,
    get_polygons_intersection,
    get_polygons_union,
)
from tools.plot import plot_polygon


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
