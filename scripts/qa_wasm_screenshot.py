#!/usr/bin/env python3
"""Screenshot a wasm GPUI page and diff against a stored baseline.

Usage:
    python3 scripts/qa_wasm_screenshot.py --url http://127.0.0.1:8081 --name showcase [--record]
    python3 scripts/qa_wasm_screenshot.py --url http://127.0.0.1:8082 --name px-scatter \
        --click-xy 80 137 [--click-text "Scatter"] [--settle-ms 4000]

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


def capture(
    url: str,
    out: Path,
    wait_ms: int,
    click_texts: list[str],
    click_xys: list[tuple[int, int]],
    settle_ms: int,
    viewport: tuple[int, int] = (1280, 900),
    device_scale_factor: int = 1,
    ready_selector: str | None = None,
    screenshot_timeout_ms: int = 30_000,
) -> dict:
    """Screenshot the page; return a liveness report (canvas size, console log)."""
    from playwright.sync_api import sync_playwright

    console: list[str] = []
    with sync_playwright() as p:
        # Prefer a full browser: the headless shell exposes no WebGPU adapter
        # ("No available adapters" from wgpu, canvas never appears). CI
        # installs Playwright's Chromium; developer machines often have
        # Google Chrome instead.
        browser = None
        for channel in ("chromium", "chrome"):
            try:
                browser = p.chromium.launch(channel=channel)
                break
            except Exception as exc:
                console.append(f"[harness] browser channel {channel!r} unavailable: {exc.__class__.__name__}")
        if browser is None:
            browser = p.chromium.launch()
            console.append("[harness] fell back to the Playwright headless shell; WebGPU may be unavailable")
        page = browser.new_page(
            viewport={"width": viewport[0], "height": viewport[1]},
            device_scale_factor=device_scale_factor,
        )
        page.on("console", lambda msg: console.append(f"[{msg.type}] {msg.text}"))
        page.on("pageerror", lambda exc: console.append(f"[pageerror] {exc}"))
        page.goto(url, wait_until="domcontentloaded", timeout=30_000)
        canvas_found = True
        try:
            page.wait_for_selector("canvas", timeout=30_000)
        except Exception as exc:
            canvas_found = False
            console.append(f"[harness] wait_for_selector('canvas') failed: {exc.__class__.__name__}")
        ready = True
        if ready_selector:
            try:
                page.wait_for_selector(ready_selector, timeout=30_000)
            except Exception as exc:
                ready = False
                console.append(
                    f"[harness] wait_for_selector({ready_selector!r}) failed: {exc.__class__.__name__}"
                )
        page.wait_for_timeout(wait_ms)  # let GPUI render a few frames
        for text in click_texts:
            try:
                page.get_by_text(text, exact=True).first.click(timeout=10_000)
                console.append(f"[harness] clicked '{text}'")
                page.wait_for_timeout(settle_ms)  # let the target section render
            except Exception as exc:
                console.append(f"[harness] click '{text}' failed: {exc.__class__.__name__}")
        for x, y in click_xys:
            # Canvas-rendered apps (GPUI) have no DOM text nodes to locate;
            # click at viewport coordinates instead.
            page.mouse.click(x, y)
            console.append(f"[harness] clicked at ({x}, {y})")
            page.wait_for_timeout(settle_ms)  # let the target section render
        if canvas_found:
            canvas_width = page.evaluate("document.querySelector('canvas').width")
            canvas_height = page.evaluate("document.querySelector('canvas').height")
        else:
            canvas_width = canvas_height = 0
        screenshot_ok = True
        try:
            page.screenshot(path=str(out), timeout=screenshot_timeout_ms)
        except Exception as exc:
            screenshot_ok = False
            console.append(f"[harness] screenshot failed: {exc.__class__.__name__}")
        try:
            browser.close()
        except Exception as exc:
            screenshot_ok = False
            console.append(f"[harness] browser close failed: {exc.__class__.__name__}")
    return {
        "canvas_width": canvas_width,
        "canvas_height": canvas_height,
        "console": console,
        "ready": ready,
        "screenshot": screenshot_ok,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--output", type=Path, help="write the capture to this path instead of target/qa/wasm")
    parser.add_argument("--record", action="store_true", help="write the baseline instead of comparing")
    parser.add_argument("--wait-ms", type=int, default=3000)
    parser.add_argument("--viewport", nargs=2, type=int, default=(1280, 900), metavar=("WIDTH", "HEIGHT"))
    parser.add_argument("--device-scale-factor", type=int, default=1)
    parser.add_argument(
        "--ready-selector",
        help="wait for this selector after canvas boot (for example html[data-gpui-ready='true'])",
    )
    parser.add_argument(
        "--click-text",
        action="append",
        default=[],
        metavar="TEXT",
        help="click an element with this exact visible text after boot, before capture (repeatable)",
    )
    parser.add_argument(
        "--click-xy",
        action="append",
        default=[],
        nargs=2,
        type=int,
        metavar=("X", "Y"),
        help="click at these viewport coordinates after boot, before capture (repeatable);"
        " needed for canvas-rendered (GPUI) UI with no DOM text nodes",
    )
    parser.add_argument(
        "--settle-ms",
        type=int,
        default=1500,
        help="delay after each --click-text/--click-xy to let the target section render",
    )
    args = parser.parse_args()

    try:
        import playwright  # noqa: F401
        from PIL import Image
    except ImportError:
        print("error: pip install playwright pillow && playwright install chromium", file=sys.stderr)
        return 2

    BASELINES.mkdir(parents=True, exist_ok=True)
    baseline = BASELINES / f"{args.name}.png"
    actual = args.output or Path("target/qa/wasm") / f"{args.name}.png"
    actual.parent.mkdir(parents=True, exist_ok=True)
    click_xys = [(x, y) for x, y in args.click_xy]
    report = capture(
        args.url,
        actual,
        args.wait_ms,
        args.click_text,
        click_xys,
        args.settle_ms,
        viewport=tuple(args.viewport),
        device_scale_factor=args.device_scale_factor,
        ready_selector=args.ready_selector,
    )

    print(f"canvas: {report['canvas_width']}x{report['canvas_height']}")
    if report["canvas_width"] <= 0 or not report["ready"] or not report["screenshot"]:
        print("error: page did not render a ready screenshot", file=sys.stderr)
    if report["console"]:
        print("console output:")
        for line in report["console"]:
            print(f"  {line}")

    if args.record:
        if report["canvas_width"] <= 0 or not report["ready"] or not report["screenshot"]:
            print("error: refusing to record a baseline from an incomplete render", file=sys.stderr)
            return 1
        baseline.write_bytes(actual.read_bytes())
        print(f"recorded {baseline}")
        return 0
    if not baseline.exists():
        print(f"error: no baseline {baseline}; run with --record first", file=sys.stderr)
        return 2
    ratio = diff_ratio(Image.open(actual), Image.open(baseline))
    print(f"diff ratio: {ratio:.4f} (threshold {THRESHOLD})")
    if report["canvas_width"] <= 0 or not report["ready"] or not report["screenshot"]:
        return 1
    return 0 if ratio <= THRESHOLD else 1


if __name__ == "__main__":
    sys.exit(main())
