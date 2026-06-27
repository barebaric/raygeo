#!/usr/bin/env python3
"""CLI for generating raygeo docs (API reference + visual examples)."""

import argparse
import ast
import importlib
import pkgutil
import shutil
import sys
from pathlib import Path

import matplotlib
import matplotlib.pyplot as plt
import mdformat

import tools.examples
from tools import api_docs


def _format_md(content: str) -> str:
    return mdformat.text(
        content,
        extensions=["frontmatter", "tables"],
        options={"wrap": 100},
    )


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


def _generate_images(
    images_dir: Path,
    doc_filter: str | None = None,
    func_filter: str | None = None,
) -> tuple[dict[str, list], set[Path]]:
    matplotlib.use("Agg")
    images_dir.mkdir(parents=True, exist_ok=True)
    modules = _collect_example_modules()

    inline_map: dict[str, list] = {}
    produced: set[Path] = set()

    for mod in modules:
        docs: list[str] = getattr(mod, "__docs_target__", None) or []
        if not docs:
            continue
        if doc_filter is not None and doc_filter not in docs:
            continue

        images = getattr(mod, "__images__", None) or []
        if not images:
            continue
        stem_base = mod.__name__.removeprefix("tools.examples.").replace(
            "_", "-"
        )

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
            if func_filter is not None and sub != func_filter:
                continue
            stem = f"{stem_base}-{sub.replace('_', '-')}"
            img_path = images_dir / f"{stem}.png"
            result = func()
            if hasattr(result, "savefig"):
                fig = result
                fig.savefig(img_path, dpi=150)
                plt.close(fig)
                produced.add(img_path)
            else:
                continue
            for doc in docs:
                inline_map.setdefault(doc, []).append(
                    (img.get("heading"), stem, img.get("caption"))
                )

    return inline_map, produced


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
                    raise ValueError(
                        f"Heading '{heading}()' not found in {md_file}. "
                        "Check that the heading matches a function or "
                        "property name in the API docs."
                    )
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

        formatted = _format_md(content)
        path.write_text(formatted)
        print(f"  Injected images into {md_file}")


def cmd_api(args):
    stubs_dir = Path(args.stubs)
    output_dir = Path(args.output) / "api"
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Generating API docs from {stubs_dir} -> {output_dir}")
    api_docs.generate(stubs_dir, output_dir, "raygeo")

    for md_file in sorted(output_dir.glob("*.md")):
        content = md_file.read_text()
        formatted = _format_md(content)
        md_file.write_text(formatted)


def cmd_examples(args):
    images_dir = Path(args.output) / "api" / "images"
    images_dir.mkdir(parents=True, exist_ok=True)
    print(f"Generating visual examples -> {images_dir}")
    inline_map, _ = _generate_images(images_dir)


def cmd_all(args):
    output_dir = Path(args.output)
    api_dir = output_dir / "api"
    images_dir = api_dir / "images"
    stubs_dir = Path(args.stubs)

    api_dir.mkdir(parents=True, exist_ok=True)
    images_dir.mkdir(parents=True, exist_ok=True)

    files = api_docs.find_stub_files(stubs_dir)
    if not files:
        print("No .pyi stub files found.", file=sys.stderr)
        sys.exit(1)

    root_module = stubs_dir.name
    all_mods = [
        api_docs.module_name_from_path(rel, root_module) for rel, _ in files
    ]

    def has_children(mod: str) -> bool:
        prefix = mod + "."
        return any(m.startswith(prefix) for m in all_mods)

    produced_docs: set[Path] = set()
    produced_images: set[Path] = set()

    all_doc_targets: set[str] = set()
    for mod in _collect_example_modules():
        for t in getattr(mod, "__docs_target__", None) or []:
            all_doc_targets.add(t)

    for rel_path, filepath in files:
        mod = api_docs.module_name_from_path(rel_path, root_module)
        if api_docs.is_reexport_only(
            ast.parse(filepath.read_text())
        ) and not has_children(mod):
            print(f"  {mod} -> skipped (re-export only)")
            continue

        out_path = api_docs.output_path_from_rel(
            rel_path, api_dir, root_module
        )
        out_path.parent.mkdir(parents=True, exist_ok=True)
        page = api_docs.process_file(rel_path, filepath, root_module)
        if not page.strip():
            continue
        page = _format_md(page)
        out_path.write_text(page)
        produced_docs.add(out_path)
        print(f"  {mod} -> {out_path}")

        doc_target = f"{mod}.md"
        inline_map, imgs = _generate_images(images_dir, doc_filter=doc_target)
        produced_images.update(imgs)
        _inject_images_into_api(api_dir, images_dir, inline_map)

    orphan_targets = all_doc_targets - {p.name for p in produced_docs}
    for target in orphan_targets:
        inline_map, imgs = _generate_images(images_dir, doc_filter=target)
        produced_images.update(imgs)
        _inject_images_into_api(api_dir, images_dir, inline_map)

    for old in api_dir.glob("*.md"):
        if old not in produced_docs:
            old.unlink()
            print(f"  Removed stale {old.name}")

    for old in images_dir.glob("*.png"):
        if old not in produced_images:
            old.unlink()
            print(f"  Removed stale {old.name}")

    total = len(list(api_dir.glob("*.md")))
    print(f"\nGenerated {total} API doc pages in {api_dir}")


def cmd_clean(args):
    output_dir = Path(args.output)
    if output_dir.exists():
        print(f"Removing {output_dir}")
        shutil.rmtree(output_dir)
    else:
        print(f"{output_dir} does not exist, nothing to clean.")


def cmd_doc(args):
    output_dir = Path(args.output)
    api_dir = output_dir / "api"
    images_dir = api_dir / "images"
    module = args.module

    stubs_dir = Path(args.stubs)
    files = api_docs.find_stub_files(stubs_dir)
    root_module = stubs_dir.name

    # If module does not resolve, try splitting the last component as a
    # generator function name (e.g. "ops.assembly.adaptive.target_area_curves"
    # → module="ops.assembly.adaptive", func="target_area_curves").
    candidates = [module, f"{root_module}.{module}"]
    func_filter = None

    matched_mod = None
    matched_rel = None
    matched_file = None
    for rel_path, filepath in files:
        mod = api_docs.module_name_from_path(rel_path, root_module)
        if mod in candidates:
            matched_mod = mod
            matched_rel = rel_path
            matched_file = filepath
            break

    if matched_mod is None:
        *parts, func_filter = module.split(".")
        module_base = ".".join(parts)
        candidates = [module_base, f"{root_module}.{module_base}"]
        for rel_path, filepath in files:
            mod = api_docs.module_name_from_path(rel_path, root_module)
            if mod in candidates:
                matched_mod = mod
                matched_rel = rel_path
                matched_file = filepath
                break

    if matched_mod is None:
        print(
            f"Error: module '{module}' not found in stubs directory",
            file=sys.stderr,
        )
        sys.exit(1)

    assert matched_rel is not None and matched_file is not None

    api_dir.mkdir(parents=True, exist_ok=True)
    out_path = api_docs.output_path_from_rel(matched_rel, api_dir, root_module)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    page = api_docs.process_file(matched_rel, matched_file, root_module)
    if page.strip():
        page = _format_md(page)
        out_path.write_text(page)
        print(f"  {matched_mod} -> {out_path}")

    doc_target = f"{matched_mod}.md"
    print(f"Generating visual examples for {doc_target}...")
    inline_map, _ = _generate_images(
        images_dir,
        doc_filter=doc_target,
        func_filter=func_filter,
    )

    print("Injecting images into API docs...")
    _inject_images_into_api(api_dir, images_dir, inline_map)


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

    p_doc = subparsers.add_parser(
        "doc",
        help="Generate API doc + images for a single module",
    )
    p_doc.add_argument(
        "module",
        type=str,
        help="Dotted module name (e.g. raygeo.geo.shape.circle)",
    )
    p_doc.set_defaults(func=cmd_doc)

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    args.func(args)


if __name__ == "__main__":
    main()
