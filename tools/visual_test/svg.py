import math

import matplotlib.pyplot as plt
import streamlit as st
from matplotlib.colors import to_hex

from raygeo.svg import parse_svg_path_data
from tools.plot import auto_limits, plot_geometry


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
