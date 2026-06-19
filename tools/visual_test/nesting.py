import math

import matplotlib.pyplot as plt
import numpy as np
import streamlit as st
from matplotlib.colors import to_hex

from raygeo.geo.algo.nest2d.genetic import GeneticAlgorithm
from raygeo.geo.algo.nest2d.gravity import apply_gravity
from raygeo.geo.algo.nest2d.placement import place_parts
from raygeo.geo.shape.polygon import get_polygon_convex_hull


def page_nesting():
    st.header("Nesting")

    c1, c2, c3 = st.columns(3)
    with c1:
        shape = st.selectbox(
            "Part shape",
            ["Rectangle", "Circle", "L-shape", "Mixed"],
            key="nest_shape",
        )
    with c2:
        n_parts = st.slider("Number of parts", 1, 50, 10, key="nest_n")
    with c3:
        size = st.slider("Part size", 5, 100, 30, key="nest_size")

    c4, c5, c6 = st.columns(3)
    with c4:
        sheet_w = st.number_input("Sheet width", 50, 2000, 200, key="nest_sw")
    with c5:
        sheet_h = st.number_input("Sheet height", 50, 2000, 200, key="nest_sh")
    with c6:
        n_sheets = st.number_input("Number of sheets", 1, 10, 1, key="nest_ns")

    c7, c8, c9 = st.columns(3)
    with c7:
        spacing = st.slider("Spacing", 0.0, 20.0, 2.0, 0.5, key="nest_spc")
    with c8:
        rot_max = st.slider(
            "Max rotation (deg)", 0, 360, 180, 45, key="nest_rot"
        )
    with c9:
        flip_h = st.checkbox("Allow X flip", value=True, key="nest_fh")
        flip_v = st.checkbox("Allow Y flip", value=False, key="nest_fv")

    rng = np.random.default_rng()

    def _make_part(i):
        cx, cy = size / 2, size / 2
        if shape == "Rectangle" or (shape == "Mixed" and i % 2 == 0):
            w = size * (0.5 + 0.5 * rng.random())
            h = size * (0.5 + 0.5 * rng.random())
            return [
                (0, 0),
                (w, 0),
                (w, h),
                (0, h),
            ]
        elif shape == "Circle" or (shape == "Mixed" and i % 3 == 0):
            r = size * (0.3 + 0.3 * rng.random())
            n = 32
            return [
                (
                    cx + r * math.cos(2 * math.pi * j / n),
                    cy + r * math.sin(2 * math.pi * j / n),
                )
                for j in range(n)
            ]
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

    spf_0 = (0.0, 0.0)
    spf_1 = (sheet_w, 0.0)
    spf_2 = (sheet_w, sheet_h)
    spf_3 = (0.0, sheet_h)
    sheet_poly_flat = [spf_0, spf_1, spf_2, spf_3]

    c_ga = st.columns(1)[0]
    with c_ga:
        use_ga = st.checkbox(
            "Use genetic algorithm", value=True, key="nest_ga"
        )
        n_gen = st.slider("Generations", 1, 20, 5, key="nest_ngen")

    if st.button("Run Nesting", type="primary", key="nest_run"):
        part_polys = [[_make_part(i)] for i in range(n_parts)]
        part_hulls = [
            [get_polygon_convex_hull(part_polys[i][0])] for i in range(n_parts)
        ]
        sheet_poly = [sheet_poly_flat]
        sheet_offsets = [(0.0, 0.0)] * n_sheets

        if use_ga and rot_max > 0:
            ga_config = {
                "rotation_count": max(1, rot_max // 45) + 1,
                "flip_h": flip_h,
                "flip_v": flip_v,
                "population_size": 10,
                "mutation_rate": 30.0,
            }
            ga = GeneticAlgorithm(n_parts, ga_config)
            best_fitness = float("inf")
            best_result = None

            progress = st.progress(0, text="Evolving...")
            for gen in range(n_gen):
                for idx in range(len(ga)):
                    rots, fh_arr, fv_arr, _ = ga.get_individual(idx)
                    result = place_parts(
                        part_polys,
                        part_hulls,
                        sheet_poly,
                        sheet_offsets,
                        rots,
                        fh_arr,
                        fv_arr,
                        spacing=spacing,
                    )
                    fit = result[0]["fitness"] if result else float("inf")
                    ga.set_fitness(idx, fit)
                    if fit < best_fitness:
                        best_fitness = fit
                        best_result = result
                ga.generation()
                progress.progress(
                    (gen + 1) / n_gen, text=f"Gen {gen + 1}/{n_gen}"
                )

            result = best_result
        else:
            rotations = [
                rng.uniform(0.0, float(rot_max)) for _ in range(n_parts)
            ]
            fh = [flip_h] * n_parts
            fv = [flip_v] * n_parts
            with st.spinner("Nesting..."):
                result = place_parts(
                    part_polys,
                    part_hulls,
                    sheet_poly,
                    sheet_offsets,
                    rotations,
                    fh,
                    fv,
                    spacing=spacing,
                )

        if not result:
            st.warning("No placements found.")
            return

        total_placed = sum(len(sheet["placements"]) for sheet in result)
        fitness = result[0].get("fitness", float("inf"))
        sheet_label = "sheet" if len(result) == 1 else "sheets"
        st.success(
            f"Placed {total_placed} / {n_parts} parts across "
            f"{len(result)} {sheet_label}"
            + (f" | fitness: {fitness:.4f}" if fitness != float("inf") else "")
        )

        cmap = plt.get_cmap("tab10")
        for si, sheet_result in enumerate(result):
            st.subheader(f"Sheet {si + 1}")
            placements = sheet_result["placements"]
            fig, ax = plt.subplots(figsize=(10, 8))
            ax.plot(
                [p[0] for p in sheet_poly_flat] + [sheet_poly_flat[0][0]],
                [p[1] for p in sheet_poly_flat] + [sheet_poly_flat[0][1]],
                color="black",
                linewidth=2,
                label="Sheet",
            )
            for pi, pl in enumerate(placements):
                for poly in pl["polygons"]:
                    px = [p[0] for p in poly] + [poly[0][0]]
                    py = [p[1] for p in poly] + [poly[0][1]]
                    color = to_hex(cmap(pi % 10))
                    ax.fill(px, py, alpha=0.25, color=color)
                    ax.plot(px, py, color=color, linewidth=1.5)
            ax.set_aspect("equal")
            ax.set_xlim(-spacing * 2, sheet_w + spacing * 2)
            ax.set_ylim(-spacing * 2, sheet_h + spacing * 2)
            ax.grid(True, alpha=0.3)
            ax.legend(fontsize=9, loc="upper right")
            fig.tight_layout()
            st.pyplot(fig)

        if n_sheets == 1 and total_placed > 0:
            st.subheader("After gravity")
            placed_groups = [pl["polygons"] for pl in result[0]["placements"]]
            adjustments = apply_gravity(
                placed_groups, sheet_poly_flat, spacing
            )
            fig2, ax2 = plt.subplots(figsize=(10, 8))
            ax2.plot(
                [p[0] for p in sheet_poly_flat] + [sheet_poly_flat[0][0]],
                [p[1] for p in sheet_poly_flat] + [sheet_poly_flat[0][1]],
                color="black",
                linewidth=2,
            )
            for pi, (pl, adj) in enumerate(
                zip(result[0]["placements"], adjustments)
            ):
                for poly in pl["polygons"]:
                    shifted = [(p[0] + adj[0], p[1] + adj[1]) for p in poly]
                    px = [p[0] for p in shifted] + [shifted[0][0]]
                    py = [p[1] for p in shifted] + [shifted[0][1]]
                    color = to_hex(cmap(pi % 10))
                    ax2.fill(px, py, alpha=0.25, color=color)
                    ax2.plot(px, py, color=color, linewidth=1.5)
            ax2.set_aspect("equal")
            ax2.set_xlim(-spacing * 2, sheet_w + spacing * 2)
            ax2.set_ylim(-spacing * 2, sheet_h + spacing * 2)
            ax2.grid(True, alpha=0.3)
            fig2.tight_layout()
            st.pyplot(fig2)
    else:
        st.info("Configure parts and sheet above, then click **Run Nesting**.")
