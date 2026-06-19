#!/usr/bin/env python3
"""CLI for generating raygeo docs (API reference + visual examples)."""

import argparse
import importlib
import pkgutil
import shutil
import sys
from pathlib import Path

import matplotlib
import matplotlib.image as mpimg
import matplotlib.pyplot as plt
import numpy as np

import tools.examples
from tools import api_docs


def _module_to_doc(mod_name: str) -> list[str]:
    known_compound = {"cleared_area", "spatial_grid2d"}
    parts = mod_name.split("_")
    result_parts = []
    i = 0
    while i < len(parts):
        for n in range(len(parts), i, -1):
            candidate = "_".join(parts[i:n])
            if candidate in known_compound or n == i + 1:
                result_parts.append(candidate)
                i = n
                break
    base = "raygeo." + ".".join(result_parts) + ".md"
    if mod_name == "geo":
        return ["raygeo.md", base]
    return [base]


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


def _images_are_visually_identical(path_a: Path, path_b: Path) -> bool:
    a = mpimg.imread(path_a)
    b = mpimg.imread(path_b)
    if a.shape != b.shape:
        return False
    h, w = a.shape[:2]
    block = max(h, w) // 64
    if block < 2:
        return np.allclose(a, b, atol=1.0 / 255)
    bh, bw = h // block * block, w // block * block
    a_blocks = (
        a[:bh, :bw]
        .reshape(bh // block, block, bw // block, block, -1)
        .mean(axis=(1, 3))
    )
    b_blocks = (
        b[:bh, :bw]
        .reshape(bh // block, block, bw // block, block, -1)
        .mean(axis=(1, 3))
    )
    return np.allclose(a_blocks, b_blocks, atol=0.2)


def _generate_images(images_dir: Path) -> dict[str, list]:
    matplotlib.use("Agg")
    images_dir.mkdir(parents=True, exist_ok=True)
    modules = _collect_example_modules()

    inline_map: dict[str, list] = {}

    for mod in modules:
        images = getattr(mod, "__images__", None) or []
        if not images:
            continue
        mod_name = mod.__name__
        if not mod_name.startswith("tools.examples."):
            continue
        short = mod_name[len("tools.examples.") :]
        docs = _module_to_doc(short)
        stem_base = short.replace("_", "-")

        print(f"  Generating {mod.__name__}...")
        for img in images:
            func = img.get("function")
            if not func:
                continue
            name = func.__name__
            sub = (
                name[len("generate_") :]
                if name.startswith("generate_")
                else name
            )
            stem = f"{stem_base}-{sub.replace('_', '-')}"
            result = func()
            if hasattr(result, "savefig"):
                fig = result
                fig.savefig(images_dir / f"{stem}.png", dpi=150)
                plt.close(fig)
            else:
                continue
            for doc in docs:
                inline_map.setdefault(doc, []).append(
                    (img.get("heading"), stem, img.get("caption"))
                )

    return inline_map


def _inject_images_into_api(api_dir: Path, images_dir: Path, inline_map: dict):
    for md_file, image_list in inline_map.items():
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

        i = 0
        while i < len(insertions):
            j = i + 1
            while j < len(insertions) and insertions[j][0] == insertions[i][0]:
                j += 1
            insertions[i:j] = reversed(insertions[i:j])
            i = j

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
    inline_map = _generate_images(images_dir)

    print("Injecting images into API docs...")
    _inject_images_into_api(api_dir, images_dir, inline_map)


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
