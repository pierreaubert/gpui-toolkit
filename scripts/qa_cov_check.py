#!/usr/bin/env python3
"""Check aggregate code-coverage against a minimum threshold.

Usage:
  python3 scripts/qa_cov_check.py \
      --summary target/qa/cov/summary.json \
      --threshold 73.50 \
      --output target/qa/cov/report.md

Reads a cargo-llvm-cov JSON summary and exits non-zero if the aggregate
line or function coverage is below the threshold.  Writes a Markdown report.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]

DEFAULT_SUMMARY = ROOT / "target" / "qa" / "cov" / "summary.json"
DEFAULT_OUTPUT = ROOT / "target" / "qa" / "cov" / "report.md"
DEFAULT_CRATE_THRESHOLDS = ROOT / "qa" / "cov" / "crate-thresholds.json"
# Keep the command-line default aligned with qa/cov/config.toml and the CI
# ratchet. The 90% value in that file is the release target, not the gate.
DEFAULT_THRESHOLD = 73.5


def load_summary(path: Path) -> Any:
    if not path.exists():
        raise RuntimeError(f"coverage summary not found: {path}")
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"invalid JSON in {path}: {exc}") from exc
    except OSError as exc:
        raise RuntimeError(f"failed to read {path}: {exc}") from exc


def coverage_pct(summary: dict[str, Any]) -> dict[str, float]:
    data = summary.get("data", [])
    if not data:
        raise RuntimeError("coverage summary 'data' array is empty")
    totals = data[0].get("totals", {})
    result: dict[str, float] = {}
    for kind in ("lines", "functions", "regions", "branches"):
        section = totals.get(kind)
        if isinstance(section, dict):
            count = section.get("count", 0)
            covered = section.get("covered", 0)
            if count > 0:
                result[kind] = covered / count * 100.0
            else:
                result[kind] = 100.0
    return result


def per_crate_coverage(summary: dict[str, Any]) -> list[dict[str, Any]]:
    """Aggregate file coverage under crates/<crate>/ without changing totals."""
    data = summary.get("data", [])
    if not data:
        raise RuntimeError("coverage summary 'data' array is empty")

    crates: dict[str, dict[str, int]] = {}
    for item in data[0].get("files", []):
        filename = Path(item.get("filename", ""))
        try:
            relative = filename.resolve().relative_to(ROOT)
        except (OSError, ValueError):
            continue
        if len(relative.parts) < 3 or relative.parts[0] != "crates":
            continue
        name = relative.parts[1]
        row = crates.setdefault(
            name,
            {"lines_covered": 0, "lines_total": 0, "functions_covered": 0, "functions_total": 0},
        )
        file_summary = item.get("summary", {})
        for metric, prefix in (("lines", "lines"), ("functions", "functions")):
            values = file_summary.get(metric, {})
            row[f"{prefix}_covered"] += int(values.get("covered", 0))
            row[f"{prefix}_total"] += int(values.get("count", 0))

    result = []
    for name, values in sorted(crates.items()):
        result.append(
            {
                "crate": name,
                **values,
                "lines_percent": 100.0 * values["lines_covered"] / values["lines_total"]
                if values["lines_total"]
                else 100.0,
                "functions_percent": 100.0
                * values["functions_covered"]
                / values["functions_total"]
                if values["functions_total"]
                else 100.0,
            }
        )
    return result


def crate_ratchet_failures(
    rows: list[dict[str, Any]], thresholds: dict[str, float]
) -> list[tuple[str, float, float]]:
    measured = {row["crate"]: float(row["lines_percent"]) for row in rows}
    failures = []
    for crate, threshold in sorted(thresholds.items()):
        actual = measured.get(crate, 0.0)
        if actual < threshold:
            failures.append((crate, actual, threshold))
    return failures


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary", type=Path, default=DEFAULT_SUMMARY)
    parser.add_argument(
        "--threshold",
        type=float,
        default=DEFAULT_THRESHOLD,
        help="enforced aggregate line-coverage floor (default: 73.5; release target: 90.0)",
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--crate-thresholds",
        type=Path,
        default=DEFAULT_CRATE_THRESHOLDS,
        help="JSON object mapping crate names to minimum line percentages",
    )
    parser.add_argument(
        "--ignore-regex",
        default="not supplied",
        help="Exact llvm-cov exclusion expression recorded in the report",
    )
    args = parser.parse_args(argv)

    try:
        summary = load_summary(args.summary)
    except RuntimeError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    try:
        pct = coverage_pct(summary)
    except RuntimeError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    try:
        crate_thresholds = json.loads(args.crate_thresholds.read_text())
        if not isinstance(crate_thresholds, dict):
            raise ValueError("expected a JSON object")
        crate_thresholds = {
            str(crate): float(threshold) for crate, threshold in crate_thresholds.items()
        }
    except (OSError, ValueError, TypeError, json.JSONDecodeError) as exc:
        print(f"ERROR: invalid crate threshold file {args.crate_thresholds}: {exc}", file=sys.stderr)
        return 2

    crate_rows = per_crate_coverage(summary)
    crate_failures = crate_ratchet_failures(crate_rows, crate_thresholds)
    primary = pct.get("lines", pct.get("functions", 0.0))
    passed = primary >= args.threshold and not crate_failures
    status = "PASSED" if passed else "FAILED"

    lines = [
        "# Coverage Gate Report",
        "",
        f"- Threshold: **{args.threshold:.2f}%**",
        f"- Primary metric (lines): **{primary:.2f}%**",
        f"- Result: **{status}**",
        f"- Scope: portable production library code under `crates/`",
        f"- Exclusion expression: `{args.ignore_regex}`",
        f"- Per-crate ratchets: `{args.crate_thresholds}`",
        "",
        "## Coverage Breakdown",
        "",
        "| Metric | Covered | Total | Percent |",
        "|--------|---------|-------|----------|",
    ]
    for kind, value in pct.items():
        data = summary["data"][0]["totals"].get(kind, {})
        covered = data.get("covered", 0)
        total = data.get("count", 0)
        lines.append(f"| {kind.capitalize()} | {covered} | {total} | {value:.2f}% |")
    lines.append("")

    lines.extend(
        [
            "## Per-crate Coverage",
            "",
            "Each configured crate floor is enforced in addition to the portable-core aggregate.",
            "",
            "| Crate | Lines | Line percent | Functions | Function percent |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for row in crate_rows:
        threshold = crate_thresholds.get(row["crate"])
        marker = (
            f" (minimum {threshold:.2f}%)" if threshold is not None else " (not configured)"
        )
        lines.append(
            f"| {row['crate']} | {row['lines_covered']}/{row['lines_total']} | "
            f"{row['lines_percent']:.2f}%{marker} | {row['functions_covered']}/{row['functions_total']} | "
            f"{row['functions_percent']:.2f}% |"
        )
    lines.append("")
    if crate_failures:
        lines.extend(["## Failed Per-crate Ratchets", ""])
        for crate, actual, threshold in crate_failures:
            lines.append(f"- `{crate}`: {actual:.2f}% < {threshold:.2f}%")
        lines.append("")

    report = "\n".join(lines)
    print(f"Coverage {primary:.2f}% (threshold {args.threshold:.2f}%) - {status}")

    try:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report + "\n")
    except OSError as exc:
        print(f"ERROR: failed to write report to {args.output}: {exc}", file=sys.stderr)
        return 2

    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
