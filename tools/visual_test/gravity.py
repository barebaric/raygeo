import matplotlib.pyplot as plt
import numpy as np
import streamlit as st
from matplotlib.colors import to_hex

from raygeo.geo.algo.nest2d.gravity import apply_gravity


def page_gravity():
    st.header("Gravity Tightening")
    st.write("Apply gravity sliding to tighten a nesting layout.")

    c1, c2, c3 = st.columns(3)
    with c1:
        n_parts = st.slider("Number of parts", 2, 20, 8, key="grav_n")
    with c2:
        size = st.slider("Part size", 10, 80, 25, key="grav_size")
    with c3:
        spacing = st.slider("Spacing", 0.0, 10.0, 2.0, 0.5, key="grav_spc")

    sheet_w = st.number_input("Sheet width", 50, 500, 160, key="grav_sw")
    sheet_h = st.number_input("Sheet height", 50, 500, 120, key="grav_sh")

    if st.button("Run Gravity", type="primary", key="grav_run"):
        rng = np.random.default_rng(42)

        def _make_part(i):
            if i % 2 == 0:
                w = size * (0.5 + 0.5 * rng.random())
                h = size * (0.5 + 0.5 * rng.random())
                return [(0, 0), (w, 0), (w, h), (0, h)]
            else:
                leg_w = size * (0.3 + 0.3 * rng.random())
                leg_h = size * (0.3 + 0.3 * rng.random())
                body_w = size * (0.5 + 0.3 * rng.random())
                body_h = size * (0.5 + 0.3 * rng.random())
                return [
                    (0, 0),
                    (body_w, 0),
                    (body_w, leg_h),
                    (leg_w, leg_h),
                    (leg_w, body_h),
                    (0, body_h),
                ]

        parts = [_make_part(i) for i in range(n_parts)]

        cols = 4
        placed_groups = []
        for i, poly in enumerate(parts):
            bx = min(p[0] for p in poly)
            by = min(p[1] for p in poly)
            col = i % cols
            row = i // cols
            ox = col * (size * 1.5) + 10 + rng.uniform(0, size * 0.3)
            oy = row * (size * 1.5) + 10 + rng.uniform(0, size * 0.3)
            shifted = [(p[0] - bx + ox, p[1] - by + oy) for p in poly]
            placed_groups.append([shifted])

        sheet_poly = [
            (0.0, 0.0),
            (sheet_w, 0.0),
            (sheet_w, sheet_h),
            (0.0, sheet_h),
        ]

        with st.spinner("Applying gravity..."):
            adjustments = apply_gravity(placed_groups, sheet_poly, spacing)

        cmap = plt.get_cmap("tab10")
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 7))

        for ax in (ax1, ax2):
            ax.plot(
                [p[0] for p in sheet_poly] + [sheet_poly[0][0]],
                [p[1] for p in sheet_poly] + [sheet_poly[0][1]],
                color="black",
                linewidth=2,
            )

        for pi, polys in enumerate(placed_groups):
            for poly in polys:
                px = [p[0] for p in poly] + [poly[0][0]]
                py = [p[1] for p in poly] + [poly[0][1]]
                color = to_hex(cmap(pi % 10))
                ax1.fill(px, py, alpha=0.25, color=color)
                ax1.plot(px, py, color=color, linewidth=1.5)

        for pi, (polys, adj) in enumerate(zip(placed_groups, adjustments)):
            for poly in polys:
                shifted = [(p[0] + adj[0], p[1] + adj[1]) for p in poly]
                px = [p[0] for p in shifted] + [shifted[0][0]]
                py = [p[1] for p in shifted] + [shifted[0][1]]
                color = to_hex(cmap(pi % 10))
                ax2.fill(px, py, alpha=0.25, color=color)
                ax2.plot(px, py, color=color, linewidth=1.5)

        for ax, title in zip(
            (ax1, ax2),
            ("Before gravity (loose placement)", "After gravity (tightened)"),
        ):
            ax.set_aspect("equal")
            ax.grid(True, alpha=0.3)
            ax.set_title(title, fontsize=14)

        fig.tight_layout()
        st.pyplot(fig)
        st.success(f"Applied {len(adjustments)} adjustments")

        c1, c2, c3 = st.columns(3)
        c1.metric("Parts", len(placed_groups))
        c2.metric("Adjustments", len(adjustments))
        if adjustments:
            total_dx = sum(abs(a[0]) for a in adjustments)
            total_dy = sum(abs(a[1]) for a in adjustments)
            c3.metric("Total movement", f"{total_dx + total_dy:.1f}")
    else:
        st.info("Configure parameters and click **Run Gravity**.")
