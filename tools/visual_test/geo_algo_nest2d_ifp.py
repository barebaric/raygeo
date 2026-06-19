import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.algo.nest2d.ifp import inner_fit_polygon
from tools.plot import plot_polygon


def page_inner_fit_polygon():
    st.header("Inner Fit Polygon (IFP)")
    st.write("Compute the valid placement region for a part inside a bin.")

    c1, c2 = st.columns(2)
    with c1:
        bin_w = st.number_input("Bin width", 20.0, 300.0, 100.0, key="ifp_bw")
        bin_h = st.number_input("Bin height", 20.0, 300.0, 80.0, key="ifp_bh")
    with c2:
        part_w = st.number_input("Part width", 5.0, 100.0, 30.0, key="ifp_pw")
        part_h = st.number_input("Part height", 5.0, 100.0, 25.0, key="ifp_ph")

    bin_poly = [(0.0, 0.0), (bin_w, 0.0), (bin_w, bin_h), (0.0, bin_h)]
    part = [(0.0, 0.0), (part_w, 0.0), (part_w, part_h), (0.0, part_h)]

    ifp_result = inner_fit_polygon(bin_poly, part)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_polygon(ax, bin_poly, "black", "Bin", linewidth=2.5)
    if ifp_result:
        plot_polygon(
            ax, ifp_result[0], "limegreen", "IFP (valid region)", linewidth=2.5
        )
        xs = [p[0] for p in ifp_result[0]] + [ifp_result[0][0][0]]
        ys = [p[1] for p in ifp_result[0]] + [ifp_result[0][0][1]]
        ax.fill(xs, ys, alpha=0.08, color="limegreen")

    sample_x = st.slider(
        "Sample part X", 0.0, max(bin_w - part_w, 1.0), 15.0, key="ifp_sx"
    )
    sample_y = st.slider(
        "Sample part Y", 0.0, max(bin_h - part_h, 1.0), 12.0, key="ifp_sy"
    )
    shifted = [(p[0] + sample_x, p[1] + sample_y) for p in part]
    plot_polygon(ax, shifted, "tomato", "Part (placed example)", linewidth=2.5)
    xs_s = [p[0] for p in shifted] + [shifted[0][0]]
    ys_s = [p[1] for p in shifted] + [shifted[0][1]]
    ax.fill(xs_s, ys_s, alpha=0.15, color="tomato")

    ax.set_aspect("equal")
    ax.set_xlim(-bin_w * 0.1, bin_w * 1.1)
    ax.set_ylim(-bin_h * 0.1, bin_h * 1.1)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2 = st.columns(2)
    c1.metric("IFP polygons", len(ifp_result))
    if ifp_result:
        poly = ifp_result[0]
        ifp_area = abs(
            sum(
                poly[i][0] * poly[(i + 1) % len(poly)][1]
                - poly[(i + 1) % len(poly)][0] * poly[i][1]
                for i in range(len(poly))
            )
            * 0.5
        )
        c2.metric("IFP area", f"{ifp_area:.1f}")
