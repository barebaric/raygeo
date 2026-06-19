import math

import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.shape.polygon import offset_polygon
from tools.plot import plot_polygon


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
