#!/usr/bin/env python3
"""CLI for generating raygeo docs (API reference + visual examples)."""

import argparse
import importlib
import pkgutil
import shutil
import sys
import tempfile
from pathlib import Path

import matplotlib
import matplotlib.image as mpimg
import numpy as np

import tools.examples
from tools import api_docs


def _build_inline_image_map():
    mapping = {}
    for mod in _collect_example_modules():
        images = getattr(mod, "__images__", None)
        if not images:
            continue
        for img in images:
            doc_files = img.get("doc")
            if not doc_files:
                continue
            if isinstance(doc_files, str):
                doc_files = [doc_files]
            for doc in doc_files:
                mapping.setdefault(doc, []).append(
                    (img.get("heading"), img["stem"], img["caption"])
                )
    return mapping


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


def _module_output_stems(mod):
    images = getattr(mod, "__images__", None)
    if images:
        return [img["stem"] for img in images]
    return getattr(mod, "__outputs__", None) or []


def _module_is_up_to_date(mod, images_dir: Path) -> bool:
    stems = _module_output_stems(mod)
    if not stems:
        return False
    if not images_dir.is_dir():
        return False
    try:
        mod_mtime = Path(mod.__file__).stat().st_mtime
    except (OSError, TypeError):
        return False
    for stem in stems:
        img = images_dir / f"{stem}.png"
        if not img.exists():
            return False
        try:
            if img.stat().st_mtime < mod_mtime:
                return False
        except OSError:
            return False
    return True


def _generate_images(images_dir: Path):
    matplotlib.use("Agg")

    images_dir.mkdir(parents=True, exist_ok=True)
    modules = _collect_example_modules()

    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        for mod in modules:
            if not hasattr(mod, "generate_examples"):
                continue
            if _module_is_up_to_date(mod, images_dir):
                print(f"  Skipping {mod.__name__} (up to date)")
                continue
            print(f"  Generating {mod.__name__}...")
            mod.generate_examples(tmp_dir)

        for new_path in sorted(tmp_dir.glob("*.png")):
            dest = images_dir / new_path.name
            if dest.exists() and _images_are_visually_identical(
                new_path, dest
            ):
                continue
            shutil.copy2(new_path, dest)
            print(f"    Updated {new_path.name}")


def _inject_images_into_api(api_dir: Path, images_dir: Path):
    inline_map = _build_inline_image_map()
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

        # Reverse same-position groups so first entry in the list
        # ends up first in the rendered output.
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
