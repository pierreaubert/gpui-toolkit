#!/usr/bin/env python3
"""Detect wasm-vs-native class visual differences without a native harness.

Two modes:

1. Renderer parity (default): capture the same wasm URL with
   `?renderer=cpu` and `?renderer=auto` and diff them. Both renders run
   the same Rust scene through different paint backends, so any
   structural difference is a backend-divergence bug of the same class
   that separates wasm from native (e.g. a paint path ignoring corner
   radius, gradient, or stroke width).

2. Reference compare (`--reference native.png`): capture the wasm URL
   once and diff against a supplied native screenshot (for example a
   headless Metal capture or a simulator snapshot), resized to a common
   geometry. Framing must already match; use per-section URLs on both
   sides.

Usage:
    python3 scripts/qa_wasm_renderer_parity.py --url http://127.0.0.1:8082
    python3 scripts/qa_wasm_renderer_parity.py --url "http://127.0.0.1:8082?section=scatter" \
        --reference /tmp/native-scatter.png --threshold 0.02

Requires: pip install playwright pillow && playwright install chromium
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit

sys.path.insert(0, str(Path(__file__).resolve().parent))
from qa_wasm_screenshot import THRESHOLD, capture, diff_ratio  # noqa: E402

EDGE_THRESHOLD = 24  # grayscale edge strength counted as an edge pixel


def with_renderer(url: str, renderer: str) -> str:
    """Return url with the `renderer` query param set (replacing it)."""
    parts = urlsplit(url)
    query = [(k, v) for k, v in parse_qsl(parts.query) if k != "renderer"]
    query.append(("renderer", renderer))
    return urlunsplit((parts.scheme, parts.netloc, parts.path, urlencode(query), parts.fragment))


def normalize_pair(a, b):
    """Resize the larger image down to the smaller geometry (common pixels)."""
    if a.size == b.size:
        return a, b
    target = (min(a.size[0], b.size[0]), min(a.size[1], b.size[1]))
    from PIL import Image

    return (
        a.resize(target, Image.LANCZOS) if a.size != target else a,
        b.resize(target, Image.LANCZOS) if b.size != target else b,
    )


def edge_map(img):
    """Binarized edge map: geometry-only signal, blind to flat recolors."""
    from PIL import ImageFilter

    return img.convert("L").filter(ImageFilter.FIND_EDGES).point(lambda v: 255 if v > EDGE_THRESHOLD else 0)


def edge_diff_ratio(a, b) -> float:
    """Fraction of pixels whose edge maps differ (0.0-1.0)."""
    a, b = normalize_pair(edge_map(a), edge_map(b))
    return diff_ratio(a, b)


def changed_bbox(a, b):
    """Bounding box of changed pixels, or None when identical."""
    from PIL import ImageChops

    a, b = normalize_pair(a.convert("RGB"), b.convert("RGB"))
    return ImageChops.difference(a, b).getbbox()


def check_console(report: dict, label: str) -> bool:
    """Print console output; return False when the render is unusable."""
    print(f"[{label}] canvas: {report['canvas_width']}x{report['canvas_height']}")
    fatal = report["canvas_width"] <= 0 or not report["ready"] or not report["screenshot"]
    errors = [line for line in report["console"] if "[pageerror]" in line or "[ERROR]" in line]
    if report["console"]:
        print(f"[{label}] console output:")
        for line in report["console"]:
            print(f"  {line}")
    if errors:
        print(f"[{label}] error: {len(errors)} console/page errors during render", file=sys.stderr)
    return not fatal and not errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True, help="base wasm URL (section query params preserved)")
    parser.add_argument("--name", default="parity", help="capture file stem under target/qa/wasm")
    parser.add_argument(
        "--reference",
        type=Path,
        help="native reference PNG: single capture diffed against it instead of renderer A/B",
    )
    parser.add_argument("--threshold", type=float, default=THRESHOLD)
    parser.add_argument("--wait-ms", type=int, default=3000)
    parser.add_argument("--viewport", nargs=2, type=int, default=(1280, 900), metavar=("WIDTH", "HEIGHT"))
    parser.add_argument("--device-scale-factor", type=int, default=1)
    parser.add_argument("--ready-selector", default=None)
    parser.add_argument("--click-text", action="append", default=[], metavar="TEXT")
    parser.add_argument("--click-xy", action="append", default=[], nargs=2, type=int, metavar=("X", "Y"))
    parser.add_argument("--settle-ms", type=int, default=1500)
    args = parser.parse_args()

    try:
        import playwright  # noqa: F401
        from PIL import Image
    except ImportError:
        print("error: pip install playwright pillow && playwright install chromium", file=sys.stderr)
        return 2

    out_dir = Path("target/qa/wasm")
    out_dir.mkdir(parents=True, exist_ok=True)
    click_xys = [(x, y) for x, y in args.click_xy]
    common = dict(
        wait_ms=args.wait_ms,
        click_texts=args.click_text,
        click_xys=click_xys,
        settle_ms=args.settle_ms,
        viewport=tuple(args.viewport),
        device_scale_factor=args.device_scale_factor,
        ready_selector=args.ready_selector,
    )

    pairs: list[tuple[str, Path]]
    if args.reference:
        if not args.reference.exists():
            print(f"error: no reference {args.reference}", file=sys.stderr)
            return 2
        out = out_dir / f"{args.name}.png"
        report = capture(args.url, out, **common)
        ok = check_console(report, "wasm")
        pairs = [("wasm", out)]
        reference = Image.open(args.reference)
    else:
        pairs = []
        ok = True
        for renderer in ("cpu", "auto"):
            out = out_dir / f"{args.name}-{renderer}.png"
            report = capture(with_renderer(args.url, renderer), out, **common)
            ok = check_console(report, renderer) and ok
            pairs.append((renderer, out))
        reference = None

    if not ok:
        return 1
    first = Image.open(pairs[0][1])
    if reference is not None:
        second, label = reference, f"reference {args.reference}"
    else:
        second, label = Image.open(pairs[1][1]), "renderer=auto"
    first_n, second_n = normalize_pair(first.convert("RGB"), second.convert("RGB"))
    if first.size != second.size:
        print(f"note: normalized {first.size} vs {second.size} to {first_n.size}")
    ratio = diff_ratio(first_n, second_n)
    edge_ratio = edge_diff_ratio(first_n, second_n)
    bbox = changed_bbox(first_n, second_n)
    print(f"diff ratio ({pairs[0][0]} vs {label}): {ratio:.4f} (threshold {args.threshold})")
    print(f"edge diff ratio: {edge_ratio:.4f} bbox: {bbox}")
    print(f"captures: {', '.join(str(p) for _, p in pairs)}")
    return 0 if ratio <= args.threshold else 1


if __name__ == "__main__":
    sys.exit(main())
