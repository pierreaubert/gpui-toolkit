#!/usr/bin/env python3
"""Check aggregate code-coverage against a minimum threshold.

Usage:
  python3 scripts/qa_cov_check.py \
      --summary target/qa/cov/summary.json \
      --threshold 90.00 \
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


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary", type=Path, default=DEFAULT_SUMMARY)
    parser.add_argument("--threshold", type=float, default=90.0)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
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

    primary = pct.get("lines", pct.get("functions", 0.0))
    passed = primary >= args.threshold
    status = "PASSED" if passed else "FAILED"

    lines = [
        "# Coverage Gate Report",
        "",
        f"- Threshold: **{args.threshold:.2f}%**",
        f"- Primary metric (lines): **{primary:.2f}%**",
        f"- Result: **{status}**",
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
