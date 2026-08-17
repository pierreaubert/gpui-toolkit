#!/usr/bin/env python3
"""Run an automated CVD screen over persisted MeshPlot product captures.

This is deliberately a lightweight rendered-stimulus regression screen. It
uses the same simple sRGB approximations as the Rust color-scale tests and
does not claim to replace a calibrated human review.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
from pathlib import Path

from mesh_plot_visual_compare import VisualCompareError, _decode_png
from qa_release_evidence import (
    MESH_PLOT_PRODUCT_VISUAL_ARTIFACT,
    _safe_repo_artifact_path,
)


OUTPUT_ARTIFACT = "target/qa/visual/mesh-plot-cvd.json"
REPORT_TYPE = "gpui-mesh-plot-cvd-screen"
DEFICIENCIES = {
    "protan": (
        (0.56667, 0.43333, 0.0),
        (0.55833, 0.44167, 0.0),
        (0.0, 0.24167, 0.75833),
    ),
    "deutan": (
        (0.625, 0.375, 0.0),
        (0.700, 0.300, 0.0),
        (0.0, 0.300, 0.700),
    ),
    "tritan": (
        (0.950, 0.050, 0.0),
        (0.0, 0.43333, 0.56667),
        (0.0, 0.475, 0.525),
    ),
}


def _read_object(path: Path, description: str) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RuntimeError(f"invalid {description}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"invalid {description}: expected an object")
    return value


def _transform(rgba: bytes, matrix: tuple[tuple[float, float, float], ...]) -> bytes:
    result = bytearray(rgba)
    for offset in range(0, len(rgba), 4):
        red, green, blue = (channel / 255.0 for channel in rgba[offset : offset + 3])
        for channel, row in enumerate(matrix):
            value = row[0] * red + row[1] * green + row[2] * blue
            result[offset + channel] = round(max(0.0, min(1.0, value)) * 255.0)
    return bytes(result)


def _changed_pixels(left: bytes, right: bytes) -> int:
    if len(left) != len(right):
        raise RuntimeError("CVD images have different byte lengths")
    return sum(
        1
        for offset in range(0, len(left), 4)
        if left[offset : offset + 4] != right[offset : offset + 4]
    )


def _unique_rgb(rgba: bytes) -> int:
    return len({rgba[offset : offset + 3] for offset in range(0, len(rgba), 4)})


def _skip_manifest(reason: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "report_type": REPORT_TYPE,
        "status": "skipped",
        "reason": reason,
        "source_revision": os.environ.get("QA_CVD_SOURCE_REVISION"),
        "source_dirty": os.environ.get("QA_CVD_SOURCE_DIRTY") == "1",
    }


def build_report(root: Path, product_path: Path) -> dict[str, object]:
    if not product_path.is_file():
        return _skip_manifest("high-level MeshPlot product manifest is unavailable")

    product = _read_object(product_path, "MeshPlot product manifest")
    if product.get("status") == "skipped":
        return _skip_manifest(str(product.get("reason") or "product capture was skipped"))
    if product.get("status") != "captured":
        raise RuntimeError("MeshPlot product manifest is not a captured report")

    cases = product.get("cases")
    if not isinstance(cases, list) or len(cases) != 4:
        raise RuntimeError("MeshPlot product manifest must contain four cases")

    decoded: dict[str, tuple[int, int, bytes]] = {}
    case_rows: list[dict[str, object]] = []
    for case in cases:
        if not isinstance(case, dict):
            raise RuntimeError("MeshPlot product manifest contains a malformed case")
        case_id = case.get("id")
        path_text = case.get("path")
        if not isinstance(case_id, str) or not isinstance(path_text, str):
            raise RuntimeError("MeshPlot product case must contain id and path")
        image = _safe_repo_artifact_path(root, path_text, f"CVD product case {case_id}", base=product_path.parent)
        try:
            decoded[case_id] = _decode_png(image)
        except VisualCompareError as error:
            raise RuntimeError(f"cannot decode CVD product case {case_id}: {error}") from error
        case_rows.append(
            {
                "id": case_id,
                "renderer": case.get("renderer"),
                "state": case.get("state"),
                "path": path_text,
                "width": decoded[case_id][0],
                "height": decoded[case_id][1],
            }
        )

    renderer_metrics: dict[str, dict[str, object]] = {}
    for deficiency, matrix in DEFICIENCIES.items():
        transformed = {
            case_id: _transform(image[2], matrix) for case_id, image in decoded.items()
        }
        renderer_metrics[deficiency] = {
            "cases": {
                case_id: {
                    "unique_rgb_colors": _unique_rgb(pixels),
                    "finite": all(math.isfinite(value) for value in pixels),
                }
                for case_id, pixels in transformed.items()
            },
            "selection_changed_pixels": {
                renderer: _changed_pixels(
                    transformed[f"{renderer}-plain"],
                    transformed[f"{renderer}-selected"],
                )
                for renderer in ("metal", "wgpu")
            },
        }

    return {
        "schema_version": 1,
        "report_type": REPORT_TYPE,
        "status": "captured",
        "source_revision": product.get("source_revision"),
        "source_dirty": product.get("source_dirty"),
        "product_manifest": product_path.as_posix(),
        "cases": case_rows,
        "deficiencies": renderer_metrics,
        "manual_review_required": True,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--product-manifest", type=Path)
    parser.add_argument("--output", type=Path, default=Path(OUTPUT_ARTIFACT))
    parser.add_argument("--required", action="store_true")
    args = parser.parse_args(argv)

    root = args.repo_root.resolve()
    product_path = (args.product_manifest or root / MESH_PLOT_PRODUCT_VISUAL_ARTIFACT).resolve()
    output = (args.output if args.output.is_absolute() else root / args.output).resolve()
    try:
        product_path.relative_to(root)
        output.relative_to(root)
        report = build_report(root, product_path)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    except (OSError, RuntimeError, ValueError) as error:
        print(f"MeshPlot CVD screen failed: {error}", file=sys.stderr)
        return 1

    if report.get("status") == "skipped" and args.required:
        print("MeshPlot CVD screen was skipped but is required", file=sys.stderr)
        return 1
    print(f"MeshPlot CVD screen: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
