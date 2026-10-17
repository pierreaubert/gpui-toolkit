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
from string import Template
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
                extra_query = capture.get("extra_query") or {}
                live_params.update(extra_query)
                entries.append(
                    {
                        "id": entry_id,
                        "live_theme": live_theme,
                        "live_style": live_style,
                        "extra_query": extra_query,
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
    # Pin each app's preview picture to its `picture_theme`, so the six
    # landing cards each show a different theme and the capture matches its
    # live link. Resolution mirrors `write_site`: first featured entry for
    # the app, otherwise its first captured section.
    featured_ids = set(catalog.get("featured", []))
    for app in catalog["apps"]:
        theme = app.get("picture_theme")
        if not theme:
            continue
        candidates = [entry for entry in entries if entry["app_id"] == app["id"] and entry.get("section")]
        preview = next((entry for entry in candidates if entry["id"] in featured_ids), None)
        if preview is None and candidates:
            preview = candidates[0]
        if preview is None:
            continue
        preview["live_theme"] = theme
        preview["capture_theme"] = theme
        live_params = {"section": preview["section"], "theme": theme, "style": preview["live_style"]}
        if preview.get("renderer_query"):
            live_params["renderer"] = preview["renderer_query"]
        live_params.update(preview.get("extra_query") or {})
        preview["live_url"] = f"{app['route']}?{urlencode(live_params)}"
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


# Hard cap per capture, enforced by the OS on a subprocess. Healthy
# captures take ~3s; a page that wedges its renderer (observed on 390px
# showcase sections) otherwise blocks the whole matrix forever, and
# in-process watchdogs corrupt Playwright's driver. One browser process
# per capture keeps wedges isolated.
CAPTURE_TIMEOUT_SECONDS = 90


def capture_entries(
    entries: list[dict],
    urls: dict[str, str],
    output_root: Path,
    settle_ms: int,
    only: set[str] | None = None,
) -> list[str]:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from qa_chess_content import chess_content_errors

    pending = []
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
        if entry.get("extra_query"):
            query_params.update(entry["extra_query"])
        pending.append((entry, base_url, query_params))

    if not pending:
        return failures

    screenshot_script = Path(__file__).resolve().parent / "qa_wasm_screenshot.py"
    for entry, base_url, query_params in pending:
        query = urlencode(query_params)
        output = output_root / Path(entry["image"]).relative_to("snapshots")
        output.parent.mkdir(parents=True, exist_ok=True)
        ready_selector = ("html[data-spinorama-state='loaded']" if entry["app_id"] == "spinorama"
                          else "html[data-gpui-ready='true']")
        command = [
            sys.executable,
            str(screenshot_script),
            "--name", entry["id"],
            "--url", f"{base_url}/?{query}",
            "--output", str(output),
            "--wait-ms", "800",
            "--settle-ms", str(settle_ms),
            "--viewport", str(entry["width"]), str(entry["height"]),
            "--device-scale-factor", str(entry["scale_factor"]),
            "--ready-selector", ready_selector,
            "--report-json",
        ]
        try:
            completed = subprocess.run(
                command, capture_output=True, text=True, timeout=CAPTURE_TIMEOUT_SECONDS,
            )
        except subprocess.TimeoutExpired:
            failures.append(
                f"{entry['id']}: capture exceeded {CAPTURE_TIMEOUT_SECONDS}s; page unresponsive, skipped"
            )
            print(f"timeout {entry['id']}; continuing", flush=True)
            continue
        report = None
        for line in reversed(completed.stdout.splitlines()):
            line = line.strip()
            if line.startswith("{"):
                try:
                    report = json.loads(line)
                except json.JSONDecodeError:
                    report = None
                break
        if report is None:
            failures.append(
                f"{entry['id']}: capture crashed (exit {completed.returncode}): "
                f"{completed.stderr.strip().splitlines()[-1:]}"
            )
            continue
        if report["canvas_width"] <= 0 or not report["ready"] or not report["screenshot"]:
            failures.append(
                f"{entry['id']}: canvas={report['canvas_width']} ready={report['ready']} "
                f"screenshot={report['screenshot']} console={report['console'][-2:]}"
            )
        if entry["section"] == "chess" and report["screenshot"]:
            from PIL import Image
            with Image.open(output) as image:
                failures.extend(f"{entry['id']}: {error}" for error in chess_content_errors(image))
        print(
            f"captured {entry['id']} {entry['width']}x{entry['height']}@{entry['scale_factor']} "
            f"canvas={report['canvas_width']}x{report['canvas_height']}",
            flush=True,
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
            return '<div class="placeholder">Explore live ↗</div>'
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
            f'<a class="image" href="{escape(entry.get("live_url") or entry["image"])}">{media_for(entry)}</a>'
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
    app_cards = []
    for index, app in enumerate(catalog["apps"], 1):
        preview = next((entry for entry in featured if entry["app_id"] == app["id"]), None)
        if preview is None:
            preview = next((entry for entry in base_entries if entry["app_id"] == app["id"]), None)
        media = media_for(preview) if preview else '<div class="placeholder">↗</div>'
        route = escape(app["route"])
        source = escape(app.get("source_url", "https://github.com/pierreaubert/gpui-toolkit"))
        app_cards.append(
            f'<article class="app"><a class="app-preview" href="{route}" aria-label="Open {escape(app["title"])}">{media}</a>'
            f'<div class="app-meta"><span class="eyebrow">{index:02d} / LIVE DEMO</span>'
            f'<h3>{escape(app["title"])}</h3><p>{escape(app["description"])}</p>'
            f'<a href="{route}">Launch demo ↗</a><a class="source" href="{source}">Source ↗</a></div></article>'
        )
    assets = REPO_ROOT / "docs/demos"
    for name in ("landing.css", "landing.js"):
        shutil.copy2(assets / name, output / name)
    html = Template((assets / "landing.html").read_text(encoding="utf-8")).substitute(
        apps="".join(app_cards), cards="".join(rendered_cards),
        variants=featured_html, count=len(base_entries),
    )
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
    shutil.copyfile(REPO_ROOT / "docs/demos/spinorama-nginx.conf", output / "spinorama-nginx.conf")
    if args.readme_snippet:
        write_readme_snippet(catalog, entries, args.readme_snippet.resolve())
    base_count = sum(1 for entry in entries if "variant_kind" not in entry)
    print(f"wrote {output} ({base_count} entries, {copied} images)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
