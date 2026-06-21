#!/usr/bin/env python3
"""Compare a current benchmark run against a committed performance baseline.

Usage:
  python3 scripts/qa_perf_check.py \
      --baseline qa/perf/baseline.json \
      --current target/qa/perf/current.json \
      --threshold 10 \
      --output target/qa/perf/report.md

Exit codes:
  0  No regression detected.
  1  One or more benchmarks regressed beyond the threshold.
  2  Input error (missing file, malformed JSON, version mismatch, etc.).
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
BASELINE_VERSION = 1

DEFAULT_BASELINE = "qa/perf/baseline.json"
DEFAULT_CURRENT = "target/qa/perf/current.json"
DEFAULT_OUTPUT = "target/qa/perf/report.md"


class InputError(Exception):
    """Raised for malformed or incompatible input files."""


@dataclass(frozen=True, slots=True, order=True)
class RecordKey:
    crate: str
    bench: str
    group: str
    function: str


@dataclass(frozen=True, slots=True)
class Record:
    key: RecordKey
    median_ns: float
    mean_ns: float
    unit: str
    raw: dict[str, Any]


def load_json(path: Path) -> Any:
    if not path.exists():
        raise InputError(f"file not found: {path}")
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise InputError(f"invalid JSON in {path}: {exc}") from exc
    except OSError as exc:
        raise InputError(f"failed to read {path}: {exc}") from exc


def validate_baseline(data: Any, path: Path) -> dict[str, Any]:
    if not isinstance(data, dict):
        raise InputError(f"{path}: expected top-level JSON object")

    version = data.get("version")
    if version != BASELINE_VERSION:
        raise InputError(f"{path}: expected version {BASELINE_VERSION}, got {version!r}")

    records = data.get("records")
    if not isinstance(records, list):
        raise InputError(f"{path}: expected 'records' to be a list")

    return data


def parse_record(raw: dict[str, Any], path: Path) -> Record:
    required = ("crate", "bench", "group", "function", "median_ns", "mean_ns", "unit")
    missing = [key for key in required if key not in raw]
    if missing:
        raise InputError(f"{path}: record missing required fields: {missing}")

    try:
        median_ns = float(raw["median_ns"])
        mean_ns = float(raw["mean_ns"])
    except (TypeError, ValueError) as exc:
        raise InputError(f"{path}: record has non-numeric latency values: {exc}") from exc

    if median_ns < 0 or mean_ns < 0:
        raise InputError(f"{path}: record has negative latency values")

    return Record(
        key=RecordKey(
            crate=raw["crate"],
            bench=raw["bench"],
            group=raw["group"],
            function=raw["function"],
        ),
        median_ns=median_ns,
        mean_ns=mean_ns,
        unit=raw["unit"],
        raw=raw,
    )


def load_records(path: Path) -> dict[RecordKey, Record]:
    data = load_json(path)
    validate_baseline(data, path)
    records: dict[RecordKey, Record] = {}
    for idx, raw in enumerate(data.get("records", [])):
        if not isinstance(raw, dict):
            raise InputError(f"{path}: record at index {idx} is not an object")
        record = parse_record(raw, path)
        if record.key in records:
            raise InputError(f"{path}: duplicate record for {record.key}")
        records[record.key] = record
    return records


def compute_slowdown(baseline: Record, current: Record) -> float:
    return (current.median_ns - baseline.median_ns) / baseline.median_ns * 100.0


def format_slowdown(value: float) -> str:
    return f"{value:+.2f}%"


def compare(
    baseline: dict[RecordKey, Record],
    current: dict[RecordKey, Record],
    threshold: float,
    noise_floor_ns: float,
) -> tuple[list[tuple[RecordKey, Record, Record, float]], list[tuple[RecordKey, Record, Record, float]], list[RecordKey], list[RecordKey], list[tuple[RecordKey, float]]]:
    matched: list[tuple[RecordKey, Record, Record, float]] = []
    skipped: list[tuple[RecordKey, Record, Record, float]] = []
    missing: list[RecordKey] = []
    regressed: list[tuple[RecordKey, float]] = []

    for key, base_rec in baseline.items():
        cur_rec = current.get(key)
        if cur_rec is None:
            missing.append(key)
            continue

        slowdown = compute_slowdown(base_rec, cur_rec)
        if base_rec.median_ns < noise_floor_ns:
            skipped.append((key, base_rec, cur_rec, slowdown))
        else:
            matched.append((key, base_rec, cur_rec, slowdown))
            if slowdown > threshold:
                regressed.append((key, slowdown))

    newly_added = [key for key in current if key not in baseline]

    matched.sort(key=lambda item: item[0])
    skipped.sort(key=lambda item: item[0])
    missing.sort()
    newly_added.sort()
    regressed.sort(key=lambda item: item[1], reverse=True)

    return matched, skipped, missing, newly_added, regressed


def build_report(
    matched: list[tuple[RecordKey, Record, Record, float]],
    skipped: list[tuple[RecordKey, Record, Record, float]],
    missing: list[RecordKey],
    newly_added: list[RecordKey],
    regressed: list[tuple[RecordKey, float]],
    threshold: float,
    noise_floor_ns: float,
    warn_only: bool,
    baseline_path: Path,
    current_path: Path,
) -> str:
    failed = bool(regressed) and not warn_only
    status = "FAILED" if failed else "PASSED"
    warn_note = " (WARN-ONLY: regressions are reported but do not fail the suite)" if warn_only else ""

    lines = [
        "# Performance Regression Report",
        "",
        f"- Baseline: `{baseline_path}`",
        f"- Current: `{current_path}`",
        f"- Threshold: {threshold}%",
        f"- Noise floor: {noise_floor_ns} ns (benchmarks with baseline median below this are excluded from regression checks)",
        f"- Result: **{status}**{warn_note}",
        "",
        "## Summary",
        "",
        "| Metric | Count |",
        "|--------|-------|",
        f"| Matched benchmarks | {len(matched)} |",
        f"| Skipped (noisy) benchmarks | {len(skipped)} |",
        f"| Regressed benchmarks | {len(regressed)} |",
        f"| Missing benchmarks | {len(missing)} |",
        f"| New benchmarks | {len(newly_added)} |",
        "",
    ]

    if matched:
        lines.extend(
            [
                "## Matched Benchmarks",
                "",
                "| Crate | Bench | Group | Function | Baseline | Current | Slowdown |",
                "|-------|-------|-------|----------|----------|---------|----------|",
            ]
        )
        for key, base_rec, cur_rec, slowdown in matched:
            marker = " ⚠️" if slowdown > threshold else ""
            lines.append(
                f"| {key.crate} | {key.bench} | {key.group} | {key.function} | "
                f"{base_rec.median_ns:.3f} {base_rec.unit} | "
                f"{cur_rec.median_ns:.3f} {cur_rec.unit} | "
                f"{format_slowdown(slowdown)}{marker} |"
            )
        lines.append("")

    if skipped:
        lines.extend(
            [
                "## Skipped Benchmarks (below noise floor)",
                "",
                "| Crate | Bench | Group | Function | Baseline | Current | Slowdown |",
                "|-------|-------|-------|----------|----------|---------|----------|",
            ]
        )
        for key, base_rec, cur_rec, slowdown in skipped:
            lines.append(
                f"| {key.crate} | {key.bench} | {key.group} | {key.function} | "
                f"{base_rec.median_ns:.3f} {base_rec.unit} | "
                f"{cur_rec.median_ns:.3f} {cur_rec.unit} | "
                f"{format_slowdown(slowdown)} |"
            )
        lines.append("")

    if regressed:
        lines.extend(
            [
                "## Regressed Benchmarks",
                "",
                "| Crate | Bench | Group | Function | Slowdown |",
                "|-------|-------|-------|----------|----------|",
            ]
        )
        for key, slowdown in regressed:
            lines.append(
                f"| {key.crate} | {key.bench} | {key.group} | {key.function} | {format_slowdown(slowdown)} |"
            )
        lines.append("")

    if missing:
        lines.extend(
            [
                "## Missing Benchmarks (present in baseline, absent in current)",
                "",
                "| Crate | Bench | Group | Function |",
                "|-------|-------|-------|----------|",
            ]
        )
        for key in missing:
            lines.append(f"| {key.crate} | {key.bench} | {key.group} | {key.function} |")
        lines.append("")

    if newly_added:
        lines.extend(
            [
                "## New Benchmarks (present in current, absent in baseline)",
                "",
                "| Crate | Bench | Group | Function |",
                "|-------|-------|-------|----------|",
            ]
        )
        for key in newly_added:
            lines.append(f"| {key.crate} | {key.bench} | {key.group} | {key.function} |")
        lines.append("")

    return "\n".join(lines)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, default=ROOT / DEFAULT_BASELINE)
    parser.add_argument("--current", type=Path, default=ROOT / DEFAULT_CURRENT)
    parser.add_argument("--threshold", type=float, default=10.0)
    parser.add_argument(
        "--noise-floor-ns",
        type=float,
        default=150.0,
        help="Benchmarks whose baseline median is below this value (in nanoseconds) are excluded from regression checks because they are dominated by measurement noise.",
    )
    parser.add_argument(
        "--warn-only",
        action="store_true",
        help="Report regressions but exit with status 0. Useful while the baseline is being stabilized.",
    )
    parser.add_argument("--output", type=Path, default=ROOT / DEFAULT_OUTPUT)
    args = parser.parse_args(argv)

    if args.threshold < 0:
        print("ERROR: --threshold must be non-negative", file=sys.stderr)
        return 2
    if args.noise_floor_ns < 0:
        print("ERROR: --noise-floor-ns must be non-negative", file=sys.stderr)
        return 2

    try:
        baseline = load_records(args.baseline)
        current = load_records(args.current)
    except InputError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    matched, skipped, missing, newly_added, regressed = compare(
        baseline, current, args.threshold, args.noise_floor_ns,
    )

    report = build_report(
        matched, skipped, missing, newly_added, regressed,
        args.threshold, args.noise_floor_ns, args.warn_only, args.baseline, args.current,
    )

    print("## Performance Regression Summary")
    print("")
    print("| Crate | Bench | Group | Function | Baseline | Current | Slowdown |")
    print("|-------|-------|-------|----------|----------|---------|----------|")
    for key, base_rec, cur_rec, slowdown in matched:
        marker = " ⚠️" if slowdown > args.threshold else ""
        print(
            f"| {key.crate} | {key.bench} | {key.group} | {key.function} | "
            f"{base_rec.median_ns:.3f} {base_rec.unit} | "
            f"{cur_rec.median_ns:.3f} {cur_rec.unit} | "
            f"{format_slowdown(slowdown)}{marker} |"
        )

    if skipped:
        print("")
        print(f"Skipped (baseline median < {args.noise_floor_ns} ns, not counted as regression):")
        for key, base_rec, cur_rec, slowdown in skipped:
            print(
                f"- {key.crate}/{key.bench}/{key.group}/{key.function}: "
                f"{base_rec.median_ns:.3f} -> {cur_rec.median_ns:.3f} ns "
                f"({format_slowdown(slowdown)})"
            )

    if missing:
        print("")
        print("Missing in current (warning):")
        for key in missing:
            print(f"- {key.crate}/{key.bench}/{key.group}/{key.function}")

    if newly_added:
        print("")
        print("New benchmarks:")
        for key in newly_added:
            print(f"- {key.crate}/{key.bench}/{key.group}/{key.function}")

    if regressed:
        print("")
        print(f"Regressed beyond {args.threshold}% threshold:")
        for key, slowdown in regressed:
            print(f"- {key.crate}/{key.bench}/{key.group}/{key.function}: {format_slowdown(slowdown)}")

    try:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report + "\n")
    except OSError as exc:
        print(f"ERROR: failed to write report to {args.output}: {exc}", file=sys.stderr)
        return 2

    if regressed and args.warn_only:
        print("\nNOTE: --warn-only is set; regressions are reported but the suite exits successfully.")
        return 0
    return 1 if regressed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
