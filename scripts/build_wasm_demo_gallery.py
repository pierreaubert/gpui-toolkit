#!/usr/bin/env python3
"""Generate the static WASM demo gallery and optionally capture its snapshots.

The Rust applications own their section inventories. This script merges those
inventories with the small docs-level catalog, drives deterministic query URLs,
and writes a self-contained static site suitable for Pages or another static
host.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from html import escape
from pathlib import Path
from urllib.parse import urlencode

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CATALOG = REPO_ROOT / "docs/demos/catalog.json"
DEFAULT_OUTPUT = REPO_ROOT / "target/demo-site"
DEFAULT_CAPTURE_ROOT = REPO_ROOT / "target/qa/wasm-gallery"

# Must match `web_initial_theme` in crates/gpui-miniapp/src/misc.rs.
THEME_VALUES = [
    "dark",
    "light",
    "midnight",
    "forest",
    "black-and-white",
    "onyx",
    "carbon-white",
    "carbon-gray-10",
    "carbon-gray-90",
    "carbon-gray-100",
]

# Subset of `DesignLanguage::from_id` shown in the featured matrix.
STYLE_VALUES = ["apple", "material3", "fluent", "neutral"]


def seeded_pick(entry_id: str, values: list[str]) -> str:
    """Deterministically pick one value per entry id (stable rebuilds)."""
    digest = hashlib.md5(entry_id.encode("utf-8")).hexdigest()
    return values[int(digest, 16) % len(values)]


def load_catalog(path: Path) -> dict:
    catalog = json.loads(path.read_text(encoding="utf-8"))
    if catalog.get("schema_version") != 1:
        raise ValueError(f"unsupported demo catalog schema: {catalog.get('schema_version')}")
    if not catalog.get("apps"):
        raise ValueError("demo catalog has no applications")
    return catalog


def run_manifest(command: list[str]) -> dict:
    result = subprocess.run(
        command,
        cwd=REPO_ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ValueError(
            f"manifest command did not emit JSON: {' '.join(command)}\n{result.stdout}\n{result.stderr}"
        ) from exc


def app_manifests(catalog: dict) -> dict[str, dict]:
    manifests = {}
    for app in catalog["apps"]:
        command = app.get("manifest_command")
        if command:
            manifest = run_manifest(command)
            if not isinstance(manifest.get("captures"), list):
                raise ValueError(f"manifest for {app['id']} has no captures array")
            manifests[app["id"]] = manifest
    return manifests


def entries_for(catalog: dict, manifests: dict[str, dict]) -> list[dict]:
    entries: list[dict] = []
    for app in catalog["apps"]:
        app_id = app["id"]
        manifest = manifests.get(app_id)
        if manifest:
            for capture in manifest["captures"]:
                section = capture["section"]
                viewport = capture["viewport_id"]
                renderer_query = capture.get("renderer_query", "")
                entry_id = f"{app_id}-{capture['id']}"
                live_theme = seeded_pick(entry_id, THEME_VALUES)
                live_style = seeded_pick(f"{entry_id}/style", STYLE_VALUES)
                live_params = {"section": section, "theme": live_theme, "style": live_style}
                if renderer_query:
                    live_params["renderer"] = renderer_query
                entries.append(
                    {
                        "id": entry_id,
                        "live_theme": live_theme,
                        "live_style": live_style,
                        "app_id": app_id,
                        "app_title": app["title"],
                        "app_description": app["description"],
                        "title": capture["section_label"],
                        "section": section,
                        "section_label": capture["section_label"],
                        "group": capture.get("group", app["title"]),
                        "viewport_id": viewport,
                        "viewport_label": capture["viewport_label"],
                        "width": capture["width"],
                        "height": capture["height"],
                        "scale_factor": capture["scale_factor"],
                        "renderer": capture.get("renderer", ""),
                        "renderer_query": renderer_query,
                        "renderer_qa_queries": capture.get("renderer_qa_queries", []),
                        "image": f"snapshots/{app_id}/{viewport}/{section}.png",
                        "thumbnail": f"thumbnails/{app_id}/{viewport}/{section}.webp",
                        "live_url": f"{app['route']}?{urlencode(live_params)}",
                        "source_url": app.get("source_url", ""),
                        "source": capture.get("source_path", ""),
                    }
                )
        elif app.get("static_image"):
            entries.append(
                {
                    "id": app_id,
                    "app_id": app_id,
                    "app_title": app["title"],
                    "app_description": app["description"],
                    "title": app["title"],
                    "section": "",
                    "section_label": app["title"],
                    "group": app["title"],
                    "viewport_id": "desktop",
                    "viewport_label": "Desktop",
                    "width": 0,
                    "height": 0,
                    "scale_factor": 1,
                    "image": f"snapshots/{app_id}/{Path(app['static_image']).name}",
                    "thumbnail": f"thumbnails/{app_id}/{Path(app['static_image']).stem}.webp",
                    "live_url": app.get("route", ""),
                    "source_url": app.get("source_url", ""),
                    "source_image": app["static_image"],
                    "source": app["static_image"],
                }
            )
    return entries


def featured_variants(catalog: dict, entries: list[dict]) -> list[dict]:
    """Build the themed top section from one featured example.

    Uses the first featured catalog id resolving to a captured section
    (falling back to the first captured section), rendered once per
    theme at neutral style plus once per style at dark theme. Variant
    captures carry their own theme/style so tiles match their links.
    """
    by_id = {entry["id"]: entry for entry in entries}
    base = next(
        (by_id[entry_id] for entry_id in catalog.get("featured", []) if entry_id in by_id and by_id[entry_id].get("section")),
        None,
    )
    if base is None:
        base = next((entry for entry in entries if entry.get("section")), None)
    if base is None:
        return []
    variants = []
    themed = [(theme, "neutral", "theme") for theme in THEME_VALUES]
    styled = [("dark", style, "style") for style in STYLE_VALUES]
    for theme, style, kind in themed + styled:
        variant_id = f"{base['id']}--{kind}--{theme if kind == 'theme' else style}"
        slug = f"{base['section']}--{theme}--{style}"
        params = {"section": base["section"], "theme": theme, "style": style}
        if base.get("renderer_query"):
            params["renderer"] = base["renderer_query"]
        variants.append(
            {
                **base,
                "id": variant_id,
                "variant_kind": kind,
                "variant_label": theme if kind == "theme" else style,
                "live_theme": theme,
                "live_style": style,
                "capture_theme": theme,
                "capture_style": style,
                "image": f"snapshots/{base['app_id']}/{base['viewport_id']}/{slug}.png",
                "thumbnail": f"thumbnails/{base['app_id']}/{base['viewport_id']}/{slug}.webp",
                "live_url": f"{base['live_url'].split('?')[0]}?{urlencode(params)}",
            }
        )
    return variants


def parse_urls(values: list[str]) -> dict[str, str]:
    urls = {}
    for value in values:
        app_id, separator, url = value.partition("=")
        if not separator or not app_id or not url:
            raise ValueError(f"--url must be APP=URL, got {value!r}")
        urls[app_id] = url.rstrip("/")
    return urls


def capture_entries(
    entries: list[dict],
    urls: dict[str, str],
    output_root: Path,
    settle_ms: int,
    only: set[str] | None = None,
) -> list[str]:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from qa_wasm_screenshot import capture

    failures = []
    for entry in entries:
        if only and entry["id"] not in only:
            continue
        app_id = entry.get("app_id")
        if not app_id or not entry.get("section"):
            continue
        base_url = urls.get(app_id)
        if not base_url:
            failures.append(f"{entry['id']}: no URL supplied for {app_id}")
            continue
        query_params = {"section": entry["section"], "theme": entry.get("capture_theme", "dark")}
        if entry.get("capture_style"):
            query_params["style"] = entry["capture_style"]
        if entry.get("renderer_query"):
            query_params["renderer"] = entry["renderer_query"]
        query = urlencode(query_params)
        output = output_root / Path(entry["image"]).relative_to("snapshots")
        output.parent.mkdir(parents=True, exist_ok=True)
        report = capture(
            f"{base_url}/?{query}",
            output,
            wait_ms=800,
            click_texts=[],
            click_xys=[],
            settle_ms=settle_ms,
            viewport=(entry["width"], entry["height"]),
            device_scale_factor=entry["scale_factor"],
            ready_selector="html[data-gpui-ready='true']",
            screenshot_timeout_ms=10_000,
        )
        if report["canvas_width"] <= 0 or not report["ready"] or not report["screenshot"]:
            failures.append(
                f"{entry['id']}: canvas={report['canvas_width']} ready={report['ready']} "
                f"screenshot={report['screenshot']} console={report['console'][-2:]}"
            )
        print(
            f"captured {entry['id']} {entry['width']}x{entry['height']}@{entry['scale_factor']} "
            f"canvas={report['canvas_width']}x{report['canvas_height']}"
        )
    return failures


def copy_images(entries: list[dict], capture_root: Path, output: Path) -> int:
    copied = 0
    for entry in entries:
        destination = output / entry["image"]
        destination.parent.mkdir(parents=True, exist_ok=True)
        if entry.get("source_image"):
            source = REPO_ROOT / entry["source_image"]
        else:
            source = capture_root / Path(entry["image"]).relative_to("snapshots")
        if source.exists():
            shutil.copy2(source, destination)
            entry["available"] = True
            copied += 1
        else:
            entry["available"] = False
    return copied


def build_thumbnails(entries: list[dict], output: Path) -> None:
    try:
        from PIL import Image
    except ImportError:
        print("warning: Pillow unavailable; skipping thumbnails", file=sys.stderr)
        return

    for entry in entries:
        source = output / entry["image"]
        if not source.exists():
            continue
        destination = output / entry["thumbnail"]
        destination.parent.mkdir(parents=True, exist_ok=True)
        with Image.open(source) as image:
            image.thumbnail((720, 480), Image.Resampling.LANCZOS)
            image.convert("RGB").save(destination, "WEBP", quality=84, method=6)


def write_site(catalog: dict, entries: list[dict], output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    (output / "manifest.json").write_text(
        json.dumps({"catalog": catalog, "entries": entries}, indent=2) + "\n",
        encoding="utf-8",
    )
    (output / "_headers").write_text(
        "/*\n"
        "  Cross-Origin-Embedder-Policy: require-corp\n"
        "  Cross-Origin-Opener-Policy: same-origin\n",
        encoding="utf-8",
    )

    base_entries = [entry for entry in entries if "variant_kind" not in entry]
    variants = [entry for entry in entries if "variant_kind" in entry]
    featured_ids = set(catalog.get("featured", []))
    featured = [entry for entry in base_entries if entry["id"] in featured_ids]
    cards = featured + [entry for entry in base_entries if entry not in featured]

    def media_for(entry: dict) -> str:
        image_path = output / entry["thumbnail"]
        if not image_path.exists():
            image_path = output / entry["image"]
        if not image_path.exists():
            return '<div class="placeholder">Capture pending</div>'
        return (
            f'<img loading="lazy" src="{escape(str(image_path.relative_to(output)))}" '
            f'alt="{escape(entry["app_title"] + " — " + entry["section_label"])}">'
        )

    rendered_cards = []
    for entry in cards:
        live = (
            f'<a class="live" href="{escape(entry["live_url"])}">Open live demo ↗</a>'
            if entry.get("live_url")
            else ""
        )
        source = (
            f'<a class="source" href="{escape(entry["source_url"])}">View source ↗</a>'
            if entry.get("source_url")
            else ""
        )
        variant = (
            f' <span class="variant" title="Live demo opens with this theme and design style">'
            f'{escape(entry["live_theme"])} · {escape(entry["live_style"])}</span>'
            if entry.get("live_theme")
            else ""
        )
        availability = "" if entry.get("available") else " missing"
        rendered_cards.append(
            f'<article class="card{availability}" data-search="{escape((entry["app_title"] + " " + entry["section_label"] + " " + entry["group"]).lower())}">'
            f'<a class="image" href="{escape(entry["image"])}">{media_for(entry)}</a>'
            f'<div class="meta"><p class="eyebrow">{escape(entry["app_title"])} · {escape(entry["viewport_label"])}</p>'
            f'<h2>{escape(entry["section_label"])}</h2><p>{escape(entry["app_description"])}</p>{live}{variant} {source}</div></article>'
        )

    def variant_tile(entry: dict) -> str:
        return (
            f'<a class="tile" href="{escape(entry["live_url"])}">'
            f'<span class="tile-image">{media_for(entry)}</span>'
            f'<span class="tile-label">{escape(entry["variant_label"])}</span></a>'
        )

    theme_tiles = "".join(variant_tile(entry) for entry in variants if entry["variant_kind"] == "theme")
    style_tiles = "".join(variant_tile(entry) for entry in variants if entry["variant_kind"] == "style")
    if variants:
        base_title = f'{escape(variants[0]["app_title"])} — {escape(variants[0]["section_label"])}'
        featured_html = (
            f'<section class="featured"><h2>One example, every theme and style</h2>'
            f'<p class="count">{base_title}, captured once per theme at neutral style, '
            f"then once per design style at dark theme. Click a tile to open it live.</p>"
            f'<h3>Themes</h3><div class="tiles">{theme_tiles}</div>'
            f'<h3>Design styles</h3><div class="tiles">{style_tiles}</div></section>'
        )
    else:
        featured_html = ""
    html = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{escape(catalog['title'])}</title>
  <style>
    :root {{ color-scheme: dark; font-family: Inter, ui-sans-serif, system-ui, sans-serif; background: #101018; color: #f4f2f7; }}
    body {{ max-width: 1480px; margin: 0 auto; padding: 48px 28px 80px; background: radial-gradient(circle at top right, #2a2044, transparent 40%), #101018; }}
    header {{ max-width: 820px; margin-bottom: 34px; }} h1 {{ font-size: clamp(2.2rem, 5vw, 4.5rem); line-height: .98; margin: 0 0 18px; }}
    header p {{ color: #c8c1d4; font-size: 1.1rem; line-height: 1.5; }}
    .toolbar {{ display:flex; gap:12px; flex-wrap:wrap; margin:28px 0; }} input {{ background:#1b1a27; border:1px solid #464052; border-radius:10px; color:inherit; padding:12px 14px; min-width:280px; }}
    .grid {{ display:grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap:18px; }}
    .card {{ overflow:hidden; background:rgba(28,27,40,.9); border:1px solid #393448; border-radius:16px; box-shadow:0 12px 36px rgba(0,0,0,.2); }} .card.missing {{ opacity:.5; }}
    .image {{ display:block; aspect-ratio: 4 / 3; background:#0a0a0f; }} .image img {{ width:100%; height:100%; object-fit:contain; display:block; }} .placeholder {{ height:100%; display:grid; place-items:center; color:#8e879c; font-size:.9rem; }}
    .meta {{ padding:16px 18px 18px; }} .eyebrow {{ color:#b6a4ef; font-size:.75rem; text-transform:uppercase; letter-spacing:.08em; }} h2 {{ margin:5px 0 8px; font-size:1.3rem; }} .meta p {{ color:#c8c1d4; line-height:1.4; }}
    .live, .source {{ color:#d7c9ff; font-weight:600; margin-right:12px; }} .variant {{ color:#8e879c; font-size:.8rem; }}
    .featured {{ margin:8px 0 42px; }} .featured h2 {{ font-size:1.6rem; margin:0 0 6px; }} .featured h3 {{ font-size:.8rem; text-transform:uppercase; letter-spacing:.08em; color:#b6a4ef; margin:20px 0 10px; }}
    .tiles {{ display:grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap:12px; }}
    .tile {{ display:block; background:rgba(28,27,40,.9); border:1px solid #393448; border-radius:12px; overflow:hidden; text-decoration:none; }} .tile:hover {{ border-color:#6f6391; }}
    .tile-image {{ display:block; aspect-ratio: 8 / 5; background:#0a0a0f; }} .tile-image img {{ width:100%; height:100%; object-fit:cover; display:block; }} .tile-image .placeholder {{ height:100%; }}
    .tile-label {{ display:block; padding:8px 12px 10px; color:#d7c9ff; font-size:.85rem; font-weight:600; }}
    .count {{ color:#aaa2b9; }}
  </style>
</head>
<body>
  <header><p class="eyebrow">GPUI Toolkit · generated gallery</p><h1>{escape(catalog['title'])}</h1><p>{escape(catalog['description'])}</p><p class="count">{len(base_entries)} catalog entries · generated from Rust showcase inventories</p></header>
  {featured_html}
  <div class="toolbar"><input id="search" type="search" placeholder="Filter demos…" aria-label="Filter demos"><span id="count" class="count"></span></div>
  <main id="gallery" class="grid">{"".join(rendered_cards)}</main>
  <script>
    const input = document.querySelector('#search'); const cards = [...document.querySelectorAll('.card')]; const count = document.querySelector('#count');
    function update() {{ const q = input.value.toLowerCase().trim(); let shown = 0; cards.forEach(card => {{ const visible = !q || card.dataset.search.includes(q); card.hidden = !visible; if (visible) shown++; }}); count.textContent = `${{shown}} demos`; }}
    input.addEventListener('input', update); update();
  </script>
</body>
</html>
"""
    (output / "index.html").write_text(html, encoding="utf-8")


def write_readme_snippet(catalog: dict, entries: list[dict], path: Path) -> None:
    base = catalog["public_base_url"].rstrip("/") + "/"
    base_entries = [entry for entry in entries if "variant_kind" not in entry]
    featured = {entry["id"]: entry for entry in base_entries}
    lines = [
        "<!-- BEGIN GENERATED DEMO GALLERY -->",
        "## Explore the toolkit",
        "",
        "The showcases are rendered by the same GPUI WASM builds used for browser QA.",
        "",
    ]
    for entry_id in catalog.get("featured", []):
        entry = featured.get(entry_id)
        if not entry:
            continue
        image = base + entry["thumbnail"]
        link = base + entry.get("live_url", "")
        lines.extend(
            [
                f'[![{entry["app_title"]} — {entry["section_label"]}]({image})]({link})',
                f'*{entry["app_title"]} · {entry["section_label"]}*',
                "",
            ]
        )
    lines.extend(
        [
            f"[Browse all {len(base_entries)} generated snapshots and live demos →]({base})",
            "<!-- END GENERATED DEMO GALLERY -->",
            "",
        ]
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--capture-root", type=Path, default=DEFAULT_CAPTURE_ROOT)
    parser.add_argument("--capture", action="store_true", help="capture all query-driven WASM entries")
    parser.add_argument("--url", action="append", default=[], metavar="APP=URL")
    parser.add_argument("--settle-ms", type=int, default=900)
    parser.add_argument(
        "--only",
        action="append",
        default=[],
        metavar="ENTRY_ID",
        help="capture only these entry IDs while still generating the complete catalog page (repeatable)",
    )
    parser.add_argument("--readme-snippet", type=Path, help="also write a generated README markdown block")
    args = parser.parse_args()

    catalog = load_catalog(args.catalog.resolve())
    manifests = app_manifests(catalog)
    entries = entries_for(catalog, manifests)
    entries.extend(featured_variants(catalog, entries))

    if args.capture:
        urls = parse_urls(args.url)
        failures = capture_entries(
            entries,
            urls,
            args.capture_root.resolve(),
            args.settle_ms,
            only=set(args.only),
        )
        if failures:
            for failure in failures:
                print(f"error: {failure}", file=sys.stderr)
            return 1

    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    copied = copy_images(entries, args.capture_root.resolve(), output)
    build_thumbnails(entries, output)
    write_site(catalog, entries, output)
    if args.readme_snippet:
        write_readme_snippet(catalog, entries, args.readme_snippet.resolve())
    base_count = sum(1 for entry in entries if "variant_kind" not in entry)
    print(f"wrote {output} ({base_count} entries, {copied} images)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
