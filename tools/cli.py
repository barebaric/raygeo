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

_INLINE_IMAGE_MAP = {
    "raygeo.md": [
        ("geometry-playground", "Various geometry shapes and operations"),
    ],
    "raygeo.geo.md": [
        ("geometry-playground", "Various geometry shapes and operations"),
    ],
    "raygeo.geo.shape.polygon.md": [
        ("polygon-boolean", "Polygon boolean operations"),
        ("polygon-offset", "Polygon offset (outward)"),
    ],
    "raygeo.image.md": [
        ("image-processing-srgb", "sRGB to linear round-trip"),
        (
            "image-processing-dither",
            "Dithering: Floyd-Steinberg and Bayer 4x4",
        ),
    ],
    "raygeo.svg.md": [
        ("svg-parsing", "SVG path data parsed into geometries"),
    ],
    "raygeo.ops.md": [
        ("tab-operations", "Tab operations on a rectangle"),
        ("merge-lines", "Line merging before and after"),
        ("overscan", "Overscan applied to raster lines"),
        ("lead-in-out", "Lead-in and lead-out paths"),
    ],
    "raygeo.ops.raster.md": [
        (
            "rasterization-power-modulation",
            "Rasterization: Power Modulation",
        ),
        ("rasterization-mask-scan", "Rasterization: Mask Scan"),
        ("rasterization-mask-lines", "Rasterization: Mask Lines"),
        ("rasterization-multi-pass", "Rasterization: Multi-Pass"),
    ],
    "raygeo.geo.algo.hull.md": [
        ("concave-hull", "Concave vs convex hull"),
    ],
    "raygeo.geo.algo.clipping.md": [
        ("clipping-rect", "Line clipped to rectangle"),
        ("clipping-polygon", "Line clipped to polygon"),
        ("clipping-subtract", "Subtract polygon from line"),
    ],
    "raygeo.geo.algo.fitting.md": [
        ("fitting-circle", "Circle fitted to points"),
        ("fitting-primitives", "Fitted primitives"),
    ],
    "raygeo.geo.algo.overcut.md": [
        ("overcut", "Overcut on closed contour"),
    ],
    "raygeo.geo.algo.simplify.md": [
        ("simplify", "Simplify and linearize"),
    ],
    "raygeo.geo.algo.smooth.md": [
        ("smooth", "Gaussian smoothing"),
    ],
    "raygeo.nest.md": [
        ("nesting", "Part nesting on a sheet"),
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

        image_block_parts = []
        for stem, caption in image_list:
            if not (images_dir / f"{stem}.png").exists():
                continue
            image_block_parts.append(f"![{caption}](images/{stem}.png)")
            image_block_parts.append("")
            image_block_parts.append(f"*{caption}*")
            image_block_parts.append("")

        if not image_block_parts:
            continue

        image_block = "\n".join(image_block_parts) + "\n"

        frontmatter_end = content.find("---\n", 3)
        if frontmatter_end == -1:
            continue

        body_start = frontmatter_end + 4
        lines = content[body_start:].split("\n")
        skip_empty = True
        insert_pos = body_start
        for i, line in enumerate(lines):
            if skip_empty and line.strip() == "":
                continue
            skip_empty = False
            insert_pos = body_start + sum(len(ln) + 1 for ln in lines[:i])
            break

        content = (
            content[:insert_pos] + "\n" + image_block + content[insert_pos:]
        )
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
