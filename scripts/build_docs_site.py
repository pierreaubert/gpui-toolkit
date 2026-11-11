#!/usr/bin/env python3
"""Render the /docs/ developer pages from Markdown into the demo site.

Sources live in docs/developer/ (index.md, python.md, rust.md, plus
template.html and docs.css). Output defaults to target/demo-site/docs/ so
`just wasm-upload` publishes it with the rest of gpui-toolkit.spinorama.org.
build_wasm_demo_gallery.py calls build() after writing the landing page;
run this script directly (`just docs-site`) to iterate without WASM builds.
Requires the `markdown` package (`uv run --with markdown ...`).
"""

from __future__ import annotations

import argparse
import html
import shutil
from pathlib import Path
from string import Template

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE = REPO_ROOT / "docs" / "developer"
DEFAULT_OUTPUT = REPO_ROOT / "target" / "demo-site" / "docs"

# (source file, route directory under /docs/, nav label, meta description)
PAGES = (
    (
        "index.md",
        "",
        "Overview",
        "Developer documentation for GPUI Toolkit: build native GPU apps, charts, and 3D scenes from Python or Rust.",
    ),
    (
        "python.md",
        "python",
        "Python guide",
        "Install gpui-toolkit from PyPI and build native desktop apps, charts, and 3D scenes in pure Python.",
    ),
    (
        "rust.md",
        "rust",
        "Rust guide",
        "Scaffold a native GPUI Toolkit app in Rust: prerequisites, the scaffolder, your first interactive view, and distribution.",
    ),
)


def page_title(markdown_text: str, fallback: str) -> str:
    for line in markdown_text.splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return fallback


def route_href(route: str) -> str:
    return f"/docs/{route}/" if route else "/docs/"


def build(output: Path = DEFAULT_OUTPUT, source: Path = DEFAULT_SOURCE) -> list[Path]:
    try:
        import markdown
    except ImportError as error:
        raise SystemExit(
            "error: the `markdown` package is required to build /docs/ "
            "(run via `uv run --with markdown python3 scripts/build_docs_site.py`)"
        ) from error

    template = Template((source / "template.html").read_text(encoding="utf-8"))
    output.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source / "docs.css", output / "docs.css")

    written: list[Path] = []
    for index, (filename, route, label, description) in enumerate(PAGES):
        text = (source / filename).read_text(encoding="utf-8")
        converter = markdown.Markdown(
            extensions=["fenced_code", "tables", "sane_lists", "toc"],
            extension_configs={"toc": {"toc_depth": "2-3"}},
        )
        content = converter.convert(text)
        toc = str(getattr(converter, "toc", ""))  # set by the toc extension
        pages = "".join(
            f'<li><a href="{route_href(other_route)}"'
            f'{" aria-current=\"page\"" if other_route == route else ""}>{html.escape(other_label)}</a></li>'
            for _, other_route, other_label, _ in PAGES
        )
        pager_parts: list[str] = []
        if index > 0:
            _, prev_route, prev_label, _ = PAGES[index - 1]
            pager_parts.append(f'<a href="{route_href(prev_route)}">← {html.escape(prev_label)}</a>')
        else:
            pager_parts.append("<span></span>")
        if index < len(PAGES) - 1:
            _, next_route, next_label, _ = PAGES[index + 1]
            pager_parts.append(f'<a href="{route_href(next_route)}">{html.escape(next_label)} →</a>')
        rendered = template.substitute(
            title=html.escape(page_title(text, label)),
            description=html.escape(description, quote=True),
            pages=pages,
            toc=toc,
            content=content,
            pager="".join(pager_parts),
        )
        destination = (output / route / "index.html") if route else (output / "index.html")
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(rendered, encoding="utf-8")
        written.append(destination)
    return written


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    written = build(args.output.resolve(), args.source.resolve())
    for path in written:
        print(f"wrote {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
