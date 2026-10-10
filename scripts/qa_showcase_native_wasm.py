#!/usr/bin/env python3
"""Cross-check native headless showcase captures against wasm renders.

Option B lane: the native `--visual-capture` harness (Metal headless) writes
one PNG per manifest case; this script screenshots the same sections from a
served wasm showcase build (`trunk serve`) and diffs each pair with
cross-renderer-tolerant metrics:

- `px`: fraction of pixels whose RGB channel-sum differs by more than 90
  (catches recolors and missing/extra fills, blind to antialiasing noise).
- `edge_iou2`: edge-map IoU after 2px dilation (geometry gate; exact-pixel
  edge IoU is meaningless across text backends — CoreText vs browser canvas
  shift glyph outlines by a pixel even for identical layout).
- `corr`: 20px-block luminance correlation (layout gate).

Usage (server already running via `trunk serve --port 8081`):
    python3 scripts/qa_showcase_native_wasm.py --port 8081 \
        --manifest-json /tmp/showcase-manifest.json \
        --native-dir /tmp/natcap/metal/actual

Requires: pip install playwright pillow && playwright install chromium
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from urllib.parse import urlencode

sys.path.insert(0, str(Path(__file__).resolve().parent))
from qa_wasm_screenshot import BrowserSession  # noqa: E402

PX_CHANNEL_SUM_TOL = 90
PX_GATE = 0.08
EDGE_IOU_GATE = 0.50
CORR_GATE = 0.93
EDGE_THRESHOLD = 24

# Gates calibrated on the 76-section desktop matrix (cosmic-text natives):
# worst legit px 0.049, worst legit edge-iou 0.543 (text-heavy alerts), worst
# legit corr 0.952. The iou floor is glyph-rasterization noise (Metal 2x
# downscale vs browser 1x canvas: identical ink, different antialiasing
# curves); layout-shift detection is carried by the corr gate, which a real
# wrap divergence fails decisively (dialog/CoreText scored corr 0.74).

# Sections whose pixels are time-dependent: a single screenshot pair cannot
# compare animation phases, so they are skipped (reported, never silent).
DYNAMIC_CASES = {
    # Nine live orbs; ThinkingOrb ignores freeze_visual_animations, so the
    # native t=0 frame and the wasm mid-animation frame never align.
    "thinking-orbs-desktop": "live animation phases differ by construction",
}

# Sections whose wasm console errors are expected and identical-by-design on
# both sides (rendered placeholders match): errors are printed but do not fail.
EXPECTED_CONSOLE_ERRORS = {
    # Demos the missing-asset fallback with assets/missing.png (no such
    # file) plus a cwd-relative gallery path that only resolves from the
    # repo root; both sides log and render the placeholder identically.
    "lightbox-desktop": "missing-asset fallback demo logs by design",
}


def section_url(port: int, slug: str, theme: str, design: str) -> str:
    query = urlencode({"section": slug, "theme": theme, "style": design})
    return f"http://127.0.0.1:{port}/?{query}"


def px_diff_ratio(a, b) -> float:
    """Fraction of pixels with RGB channel-sum difference above tolerance."""
    import numpy as np

    x = np.asarray(a.convert("RGB")).astype(int)
    y = np.asarray(b.convert("RGB")).astype(int)
    return float((np.abs(x - y).sum(axis=2) > PX_CHANNEL_SUM_TOL).mean())


def edge_map(img):
    from PIL import ImageFilter

    return img.convert("L").filter(ImageFilter.FIND_EDGES).point(
        lambda v: 255 if v > EDGE_THRESHOLD else 0
    )


def edge_iou(a, b, dilate: int = 2) -> float:
    """Edge-map IoU after MaxFilter dilation (tolerant geometry match)."""
    import numpy as np
    from PIL import ImageFilter

    ea = edge_map(a)
    eb = edge_map(b)
    if dilate:
        ea = ea.filter(ImageFilter.MaxFilter(2 * dilate + 1))
        eb = eb.filter(ImageFilter.MaxFilter(2 * dilate + 1))
    na = np.asarray(ea) > 0
    nb = np.asarray(eb) > 0
    union = (na | nb).sum()
    return float((na & nb).sum() / union) if union else 1.0


def block_corr(a, b, block: int = 20) -> float:
    """Pearson correlation of mean-luminance over block x block cells."""
    import numpy as np

    w, h = a.size
    cw, ch = w // block, h // block
    if not cw or not ch:
        return 0.0
    fa = np.asarray(a.convert("L").crop((0, 0, cw * block, ch * block))).astype(float)
    fb = np.asarray(b.convert("L").crop((0, 0, cw * block, ch * block))).astype(float)
    ba = fa.reshape(ch, block, cw, block).mean(axis=(1, 3)).ravel()
    bb = fb.reshape(ch, block, cw, block).mean(axis=(1, 3)).ravel()
    if ba.std() == 0 or bb.std() == 0:
        return 1.0 if np.allclose(ba, bb) else 0.0
    return float(np.corrcoef(ba, bb)[0, 1])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--manifest-json", type=Path, required=True)
    parser.add_argument("--native-dir", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, default=Path("target/qa/wasm-native"))
    parser.add_argument("--theme", default="dark")
    parser.add_argument("--design", default="neutral")
    parser.add_argument("--cases", nargs="*", default=[], help="capture id subset (default: all desktop)")
    parser.add_argument("--wait-ms", type=int, default=3000)
    parser.add_argument("--settle-ms", type=int, default=1500)
    parser.add_argument("--px-gate", type=float, default=PX_GATE)
    parser.add_argument("--iou-gate", type=float, default=EDGE_IOU_GATE)
    parser.add_argument("--corr-gate", type=float, default=CORR_GATE)
    parser.add_argument(
        "--session-cases",
        type=int,
        default=20,
        help="captures per browser launch (recycle to avoid wedged GPU contexts)",
    )
    args = parser.parse_args()

    try:
        import playwright  # noqa: F401
        from PIL import Image
    except ImportError:
        print("error: pip install playwright pillow && playwright install chromium", file=sys.stderr)
        return 2

    manifest = json.loads(args.manifest_json.read_text())
    selected = [
        c
        for c in manifest["captures"]
        if c["viewport_id"] == "desktop"
        and (not args.cases or c["id"] in args.cases)
    ]
    if not selected:
        print("error: no desktop captures selected", file=sys.stderr)
        return 2

    args.out_dir.mkdir(parents=True, exist_ok=True)
    # Browsers are recycled every --session-cases captures: one launch per
    # section costs ~30s each, but a single browser for all 76 sections
    # wedges its GPU contexts (~case 60). Desktop cases share one viewport,
    # so chunking only relaunches the browser, never the geometry.
    viewports = {(c["width"], c["height"], c.get("scale_factor", 1)) for c in selected}
    if len(viewports) != 1:
        print(f"error: matrix needs one viewport, got {sorted(viewports)}", file=sys.stderr)
        return 2
    width, height, scale = viewports.pop()
    # Dynamic cases skip before any browser launches: nothing to compare,
    # and live-animation pages spin the renderer (a hang source) for no
    # signal.
    runnable = []
    for case in selected:
        if case["id"] in DYNAMIC_CASES:
            print(f"{case['id']}: SKIP ({DYNAMIC_CASES[case['id']]})", flush=True)
        else:
            runnable.append(case)
    chunk = max(args.session_cases, 1)
    rows: list[tuple[str, float, float, float, str]] = []
    failed = 0
    for offset in range(0, len(runnable), chunk):
        with BrowserSession(viewport=(width, height), device_scale_factor=scale) as session:
            chunk_cases = runnable[offset : offset + chunk]
            print(f"--- browser session: cases {offset + 1}-{offset + len(chunk_cases)}", flush=True)
            for case in chunk_cases:
                native_path = args.native_dir / f"{case['id']}.png"
                if not native_path.exists():
                    print(f"error: missing native capture {native_path}", file=sys.stderr)
                    return 2
                out = args.out_dir / f"{case['id']}-wasm.png"
                report = session.capture(
                    section_url(args.port, case["section"], args.theme, args.design),
                    out,
                    wait_ms=args.wait_ms,
                    click_texts=[],
                    click_xys=[],
                    settle_ms=args.settle_ms,
                )
                errors = [line for line in report["console"] if "[pageerror]" in line or "[ERROR]" in line]
                if errors:
                    print(f"[{case['id']}] console errors: {len(errors)}", file=sys.stderr)
                    for line in errors[:3]:
                        print(f"[{case['id']}]   {line[:300]}", file=sys.stderr)
                native = Image.open(native_path).convert("RGB")
                wasm = Image.open(out).convert("RGB")
                if native.size != wasm.size:
                    native = native.resize(wasm.size, Image.LANCZOS)
                px = px_diff_ratio(native, wasm)
                iou = edge_iou(native, wasm)
                corr = block_corr(native, wasm)
                verdict = "pass"
                unexpected_errors = errors and case["id"] not in EXPECTED_CONSOLE_ERRORS
                if unexpected_errors or px > args.px_gate or iou < args.iou_gate or corr < args.corr_gate:
                    verdict = "FAIL"
                    failed += 1
                rows.append((case["id"], px, iou, corr, verdict))
                print(f"{case['id']}: px={px:.4f} iou2={iou:.3f} corr={corr:.4f} {verdict}", flush=True)

    print(f"\n{len(rows) - failed}/{len(rows)} sections pass "
          f"(px<={args.px_gate} iou2>={args.iou_gate} corr>={args.corr_gate})")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
