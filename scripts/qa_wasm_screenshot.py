#!/usr/bin/env python3
"""Screenshot a wasm GPUI page and diff against a stored baseline.

Usage:
    python3 scripts/qa_wasm_screenshot.py --url http://127.0.0.1:8081 --name showcase [--record]

Requires: pip install playwright pillow && playwright install chromium
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

BASELINES = Path("qa/visual/wasm/baselines")
THRESHOLD = 0.01  # fraction of pixels allowed to differ


def diff_ratio(a, b) -> float:
    """Fraction of pixels that differ between two PIL images (0.0-1.0)."""
    if a.size != b.size:
        return 1.0
    from PIL import ImageChops

    diff = ImageChops.difference(a.convert("RGB"), b.convert("RGB"))
    # histogram() on an RGB image stacks the three 256-bin channel histograms,
    # so bin index is not the channel value; histogram each channel instead.
    differing = 0
    for channel in diff.split():
        differing += sum(channel.histogram()[1:])
    total = a.size[0] * a.size[1] * 3
    return differing / total


def capture(url: str, out: Path, wait_ms: int) -> dict:
    """Screenshot the page; return a liveness report (canvas size, console log)."""
    from playwright.sync_api import sync_playwright

    console: list[str] = []
    with sync_playwright() as p:
        # Full Chromium, not the headless shell: the shell exposes no WebGPU
        # adapter ("No available adapters" from wgpu, canvas never appears).
        browser = p.chromium.launch(channel="chromium")
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        page.on("console", lambda msg: console.append(f"[{msg.type}] {msg.text}"))
        page.on("pageerror", lambda exc: console.append(f"[pageerror] {exc}"))
        page.goto(url)
        canvas_found = True
        try:
            page.wait_for_selector("canvas", timeout=30_000)
        except Exception as exc:
            canvas_found = False
            console.append(f"[harness] wait_for_selector('canvas') failed: {exc.__class__.__name__}")
        page.wait_for_timeout(wait_ms)  # let GPUI render a few frames
        if canvas_found:
            canvas_width = page.evaluate("document.querySelector('canvas').width")
            canvas_height = page.evaluate("document.querySelector('canvas').height")
        else:
            canvas_width = canvas_height = 0
        page.screenshot(path=str(out))
        browser.close()
    return {"canvas_width": canvas_width, "canvas_height": canvas_height, "console": console}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--record", action="store_true", help="write the baseline instead of comparing")
    parser.add_argument("--wait-ms", type=int, default=3000)
    args = parser.parse_args()

    try:
        import playwright  # noqa: F401
        from PIL import Image
    except ImportError:
        print("error: pip install playwright pillow && playwright install chromium", file=sys.stderr)
        return 2

    BASELINES.mkdir(parents=True, exist_ok=True)
    baseline = BASELINES / f"{args.name}.png"
    actual = Path("target/qa/wasm") / f"{args.name}.png"
    actual.parent.mkdir(parents=True, exist_ok=True)
    report = capture(args.url, actual, args.wait_ms)

    print(f"canvas: {report['canvas_width']}x{report['canvas_height']}")
    if report["canvas_width"] <= 0:
        print("error: canvas has zero width; page did not render", file=sys.stderr)
    if report["console"]:
        print("console output:")
        for line in report["console"]:
            print(f"  {line}")

    if args.record:
        baseline.write_bytes(actual.read_bytes())
        print(f"recorded {baseline}")
        return 0 if report["canvas_width"] > 0 else 1
    if not baseline.exists():
        print(f"error: no baseline {baseline}; run with --record first", file=sys.stderr)
        return 2
    ratio = diff_ratio(Image.open(actual), Image.open(baseline))
    print(f"diff ratio: {ratio:.4f} (threshold {THRESHOLD})")
    if report["canvas_width"] <= 0:
        return 1
    return 0 if ratio <= THRESHOLD else 1


if __name__ == "__main__":
    sys.exit(main())
