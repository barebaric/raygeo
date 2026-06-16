#!/usr/bin/env python3
"""CLI for generating raygeo docs (API reference + visual examples)."""

import argparse
import importlib
import pkgutil
import shutil
import sys
from pathlib import Path

import matplotlib

import tools.examples
from tools import api_docs

# Each entry maps a doc file to a list of (section_heading, stem, caption).
# section_heading is the function name to find the `### \`func()\`` heading,
# or None to place the image at the top of the page content.
_INLINE_IMAGE_MAP = {
    "raygeo.md": [
        (
            None,
            "geometry-playground",
            "Various geometry shapes and operations",
        ),
    ],
    "raygeo.geo.md": [
        (
            None,
            "geometry-playground",
            "Various geometry shapes and operations",
        ),
        (
            "convert_arcs_to_beziers",
            "arc-to-bezier",
            "Arc commands converted to Bezier curve approximations",
        ),
        (
            "convert_arcs_to_beziers",
            "arc-to-bezier-overlay",
            "Overlay showing Bezier curves closely matching the original arcs",
        ),
    ],
    "raygeo.geo.shape.polygon.md": [
        (
            "get_polygons_union",
            "polygon-boolean-union",
            "Polygon union",
        ),
        (
            "get_polygons_intersection",
            "polygon-boolean-intersection",
            "Polygon intersection",
        ),
        (
            "get_polygons_difference",
            "polygon-boolean-difference",
            "Polygon difference",
        ),
        ("offset_polygon", "polygon-offset", "Polygon offset (outward)"),
    ],
    "raygeo.geo.shape.polygon3d.md": [
        (
            "get_polygons_union_3d",
            "polygon3d-boolean-union",
            "3D polygon union — Z from first polygon",
        ),
        (
            "get_polygons_intersection_3d",
            "polygon3d-boolean-intersection",
            "3D polygon intersection — Z from first polygon",
        ),
        (
            "get_polygons_difference_3d",
            "polygon3d-boolean-difference",
            "3D polygon difference (A − B) — Z from A",
        ),
        (
            "offset_polygon_3d",
            "polygon3d-offset",
            "3D polygon offset — Z preserved from input",
        ),
    ],
    "raygeo.image.md": [
        (
            "rasterize_scanlines",
            "rasterize-scanlines",
            "Scanline ops rasterized into a 2D power-map buffer",
        ),
        (
            "srgb_to_linear",
            "image-processing-srgb",
            "sRGB to linear round-trip",
        ),
        (
            "apply_floyd_steinberg_dither",
            "image-processing-dither-floyd",
            "Floyd-Steinberg dithering",
        ),
        (
            "apply_bayer_dither",
            "image-processing-dither-bayer",
            "Bayer 4x4 ordered dithering",
        ),
        (
            "grayscale_to_binary",
            "image-processing-otsu",
            "Grayscale to binary via Otsu and fixed threshold",
        ),
        (
            "get_component_areas",
            "image-processing-component-areas",
            "Connected component areas sorted ascending",
        ),
        (
            "filter_components",
            "image-processing-filter-components",
            "Component filtering by minimum area",
        ),
        (
            "denoise_binary",
            "image-processing-denoise-binary",
            "Binary image denoised via adaptive thresholding",
        ),
        (
            "compute_adaptive_threshold",
            "image-processing-adaptive-threshold",
            "Adaptive threshold from component area distribution",
        ),
        (
            "apply_minimum_run_length",
            "image-processing-min-run-len",
            "Minimum run length applied to binary image",
        ),
    ],
    "raygeo.svg.md": [
        (None, "svg-parsing", "SVG path data parsed into geometries"),
    ],
    "raygeo.ops.md": [
        ("apply_tab_gaps", "tab-operations", "Tab operations on a rectangle"),
        (
            "merge_overlapping_lines",
            "merge-lines",
            "Line merging before and after",
        ),
        ("apply_overscan", "overscan", "Overscan applied to raster lines"),
        ("apply_lead_in_out", "lead-in-out", "Lead-in and lead-out paths"),
        (
            "optimize_travel",
            "ops-optimize-travel",
            "Travel path before and after optimization",
        ),
        (
            "clip_rect",
            "ops-clip-rect",
            "Ops paths clipped to a rectangle",
        ),
    ],
    "raygeo.ops.raster.md": [
        (
            "rasterize_power_modulation",
            "rasterization-power-modulation",
            "Rasterization: Power Modulation",
        ),
        (
            "rasterize_mask_scan",
            "rasterization-mask-scan",
            "Rasterization: Mask Scan",
        ),
        (
            "rasterize_mask_lines",
            "rasterization-mask-lines",
            "Rasterization: Mask Lines",
        ),
        (
            "rasterize_multi_pass",
            "rasterization-multi-pass",
            "Rasterization: Multi-Pass",
        ),
        (
            "extract_zero_power_segments",
            "zero-power-segments",
            "Zero-power segment extraction",
        ),
    ],
    "raygeo.geo.algo.hull.md": [
        ("get_concave_hull", "concave-hull", "Concave vs convex hull"),
    ],
    "raygeo.geo.algo.clipping.md": [
        (
            "clip_line_segment_with_rect",
            "clipping-rect",
            "Line clipped to rectangle",
        ),
        (
            "clip_line_segment_with_polygons",
            "clipping-polygon",
            "Line clipped to polygon",
        ),
        (
            "subtract_polygons_from_line_segment",
            "clipping-subtract",
            "Subtract polygon from line",
        ),
    ],
    "raygeo.geo.algo.fitting.md": [
        ("fit_circle_to_points", "fitting-circle", "Circle fitted to points"),
        (
            "fit_points_with_primitives",
            "fitting-primitives",
            "Fitted primitives",
        ),
        (
            "fit_circle_to_3_points",
            "fitting-3-points",
            "Circle fitted to three points",
        ),
        (
            "flatten_to_points",
            "fitting-flatten",
            "Arc curve flattened to dense line segments",
        ),
        (
            "linearize_geometry",
            "fitting-linearize",
            "Arc curve linearized with RDP simplification",
        ),
        (
            "get_polyline_arc_deviation",
            "fitting-arc-deviation",
            "Maximum deviation from a reference arc",
        ),
        (
            "get_polyline_line_deviation",
            "fitting-line-deviation",
            "Maximum deviation from a chord",
        ),
        (
            "project_circle_center_to_bisector",
            "fitting-project-bisector",
            "Circle center projected onto the perpendicular bisector",
        ),
    ],
    "raygeo.geo.algo.overcut.md": [
        ("apply_overcut", "overcut", "Overcut on closed contour"),
    ],
    "raygeo.geo.algo.simplify.md": [
        ("simplify_polyline", "simplify", "Simplify and linearize"),
        (
            "simplify_polyline_3d",
            "simplify-3d",
            "3D polyline simplification preserving Z coordinates",
        ),
    ],
    "raygeo.geo.algo.smooth.md": [
        ("smooth_polyline", "smooth", "Gaussian smoothing"),
        (
            "compute_gaussian_kernel",
            "smooth-gaussian-kernel",
            "Gaussian kernel weights",
        ),
        (
            "resample_polyline",
            "smooth-resample",
            "Polyline resampling",
        ),
        (
            "smooth_circularly",
            "smooth-circular",
            "Circular smoothing",
        ),
        (
            "smooth_sub_segment",
            "smooth-sub-segment",
            "Sub-segment smoothing",
        ),
    ],
    "raygeo.geo.shape.arc.md": [
        (
            "linearize_arc",
            "arc-linearize",
            "Arc linearization: coarse and fine resolution",
        ),
    ],
    "raygeo.geo.shape.bezier.md": [
        (
            "split_bezier",
            "bezier-split",
            "Bezier split at parameter t",
        ),
        (
            "get_bezier_point_at",
            "bezier-point-at",
            "Bezier point evaluation at parameter t",
        ),
        (
            "flatten_bezier",
            "bezier-flatten",
            "Bezier flattening via adaptive subdivision",
        ),
    ],
    "raygeo.geo.shape.circle.md": [
        (
            "get_circle_circle_intersections",
            "circle-intersections",
            "Circle-circle and line-circle intersection points",
        ),
    ],
    "raygeo.geo.shape.line.md": [
        (
            "get_line_line_intersection",
            "line-intersections",
            "Line-line and segment intersection",
        ),
        (
            "get_point_line_distance",
            "line-point-distance",
            "Perpendicular distance from a point to a line",
        ),
    ],
    "raygeo.geo.algo.analysis.md": [
        (
            "get_area",
            "analysis-area-winding",
            "Polygon area and winding order analysis",
        ),
    ],
    "raygeo.geo.algo.cylindrical.md": [
        (
            "transform_to_cylinder",
            "cylindrical-transform",
            "Flat vertex pairs wrapped onto a cylinder surface",
        ),
    ],
    "raygeo.geo.algo.minkowski2d.md": [
        (
            "get_polygon_minkowski_sum_convex",
            "minkowski-sum",
            "Minkowski sum of two convex polygons",
        ),
    ],
    "raygeo.geo.algo.nest2d.ifp.md": [
        (
            "inner_fit_polygon",
            "inner-fit-polygon",
            "Inner Fit Polygon showing valid placement region",
        ),
    ],
    "raygeo.geo.algo.nest2d.gravity.md": [
        (
            "apply_gravity",
            "gravity",
            "Gravity tightening: before vs after",
        ),
    ],
    "raygeo.geo.algo.nest2d.md": [
        (None, "nesting", "Part nesting on a sheet"),
    ],
}


def _collect_example_modules():
    modules = []
    for importer, modname, is_pkg in pkgutil.iter_modules(
        tools.examples.__path__
    ):
        if not is_pkg and not modname.startswith("_"):
            modules.append(
                importlib.import_module(f"tools.examples.{modname}")
            )
    return modules


def _generate_images(images_dir: Path):
    matplotlib.use("Agg")

    images_dir.mkdir(parents=True, exist_ok=True)
    for old in images_dir.glob("*.png"):
        old.unlink()
    modules = _collect_example_modules()

    for mod in modules:
        if not hasattr(mod, "generate_examples"):
            continue
        print(f"  Generating {mod.__name__}...")
        mod.generate_examples(images_dir)


def _inject_images_into_api(api_dir: Path, images_dir: Path):
    for md_file, image_list in _INLINE_IMAGE_MAP.items():
        path = api_dir / md_file
        if not path.exists():
            continue

        content = path.read_text()

        insertions = []
        for heading, stem, caption in image_list:
            if not (images_dir / f"{stem}.png").exists():
                continue
            image_block = f"![{caption}](images/{stem}.png)\n\n*{caption}*\n"

            if heading is None:
                frontmatter_end = content.find("---\n", 3)
                if frontmatter_end == -1:
                    continue
                body_start = frontmatter_end + 4
                body = content[body_start:].split("\n")
                insert_pos = body_start
                for i, line in enumerate(body):
                    if line.strip():
                        insert_pos = body_start + sum(
                            len(ln) + 1 for ln in body[:i]
                        )
                        break
                insertions.append((insert_pos, image_block))
            else:
                section_pattern = f"### `{heading}()`"
                pos = content.find(section_pattern)
                if pos == -1:
                    continue
                section_end = content.find(
                    "\n### ", pos + len(section_pattern)
                )
                if section_end == -1:
                    section_end = len(content)
                insertions.append((section_end, image_block))

        if not insertions:
            continue

        for pos, block in sorted(insertions, key=lambda x: -x[0]):
            content = content[:pos] + "\n" + block + content[pos:]

        path.write_text(content)
        print(f"  Injected images into {md_file}")


def cmd_api(args):
    stubs_dir = Path(args.stubs)
    output_dir = Path(args.output) / "api"
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Generating API docs from {stubs_dir} -> {output_dir}")
    api_docs.generate(stubs_dir, output_dir, "raygeo")


def cmd_examples(args):
    images_dir = Path(args.output) / "api" / "images"
    images_dir.mkdir(parents=True, exist_ok=True)
    print(f"Generating visual examples -> {images_dir}")
    _generate_images(images_dir)


def cmd_all(args):
    output_dir = Path(args.output)
    api_dir = output_dir / "api"
    images_dir = api_dir / "images"

    cmd_api(args)

    print("Generating visual example images...")
    _generate_images(images_dir)

    print("Injecting images into API docs...")
    _inject_images_into_api(api_dir, images_dir)


def cmd_clean(args):
    output_dir = Path(args.output)
    if output_dir.exists():
        print(f"Removing {output_dir}")
        shutil.rmtree(output_dir)
    else:
        print(f"{output_dir} does not exist, nothing to clean.")


def main():
    parser = argparse.ArgumentParser(
        description="raygeo documentation generator"
    )
    parser.add_argument(
        "--output",
        "-o",
        type=str,
        default="docs",
        help="Output directory (default: docs)",
    )
    parser.add_argument(
        "--stubs",
        type=str,
        default="python/raygeo",
        help="Path to .pyi stub directory (default: python/raygeo)",
    )

    subparsers = parser.add_subparsers(dest="command")

    p_api = subparsers.add_parser(
        "api", help="Generate API reference from stubs"
    )
    p_api.set_defaults(func=cmd_api)

    p_ex = subparsers.add_parser(
        "examples",
        help="Generate visual example images (without API integration)",
    )
    p_ex.set_defaults(func=cmd_examples)

    p_all = subparsers.add_parser(
        "all", help="Generate API docs with inline images"
    )
    p_all.set_defaults(func=cmd_all)

    p_clean = subparsers.add_parser("clean", help="Remove generated docs")
    p_clean.set_defaults(func=cmd_clean)

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    args.func(args)


if __name__ == "__main__":
    main()
