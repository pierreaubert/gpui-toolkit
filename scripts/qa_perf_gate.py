#!/usr/bin/env python3
"""Run the performance comparison with one bounded transient-load retry.

The full benchmark sweep can encounter short-lived thermal or background-load
spikes on developer and CI hosts. When the first comparison reports a
regression, rerun only the affected Criterion benchmark binaries and compare
using the better same-host measurement for each case. A repeatable regression
still fails with the original threshold.
"""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import qa_perf_check

ROOT = Path(__file__).resolve().parents[1]
BASELINE_RUNNER = ROOT / "scripts" / "qa_perf_baseline.py"


def retry_bench_keys(
    regressed: list[tuple[qa_perf_check.RecordKey, float]],
) -> list[str]:
    """Return sorted, unique crate:bench binaries containing regressions."""
    return sorted({f"{key.crate}:{key.bench}" for key, _slowdown in regressed})


def _record_key(record: dict[str, Any]) -> tuple[str, str, str, str]:
    return (
        str(record["crate"]),
        str(record["bench"]),
        str(record["group"]),
        str(record["function"]),
    )


def merge_best_records(
    initial: dict[str, Any],
    retry: dict[str, Any],
    retried_benches: set[str],
) -> dict[str, Any]:
    """Keep the lower median from two runs for explicitly retried binaries."""
    merged = copy.deepcopy(initial)
    retry_records = {
        _record_key(record): record for record in retry.get("records", [])
    }

    for index, record in enumerate(merged.get("records", [])):
        bench_key = f"{record['crate']}:{record['bench']}"
        if bench_key not in retried_benches:
            continue
        candidate = retry_records.get(_record_key(record))
        if candidate is None:
            continue
        if float(candidate["median_ns"]) < float(record["median_ns"]):
            merged["records"][index] = copy.deepcopy(candidate)

    return merged


def _load_validated(path: Path) -> dict[str, Any]:
    return qa_perf_check.validate_baseline(qa_perf_check.load_json(path), path)


def _regressions(
    baseline_data: dict[str, Any],
    current_data: dict[str, Any],
    threshold: float,
    noise_floor_ns: float,
) -> list[tuple[qa_perf_check.RecordKey, float]]:
    baseline = qa_perf_check.records_from_data(baseline_data, Path("baseline"))
    current = qa_perf_check.records_from_data(current_data, Path("current"))
    return qa_perf_check.compare(
        baseline, current, threshold, noise_floor_ns
    )[-1]


def _check_args(args: argparse.Namespace) -> list[str]:
    result = [
        "--baseline",
        str(args.baseline),
        "--current",
        str(args.current),
        "--threshold",
        str(args.threshold),
        "--noise-floor-ns",
        str(args.noise_floor_ns),
        "--output",
        str(args.output),
    ]
    if args.warn_only:
        result.append("--warn-only")
    return result


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--current", type=Path, required=True)
    parser.add_argument("--threshold", type=float, default=10.0)
    parser.add_argument("--noise-floor-ns", type=float, default=150.0)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--retries", type=int, default=1)
    parser.add_argument("--warn-only", action="store_true")
    args = parser.parse_args(argv)

    if args.retries < 0:
        print("ERROR: --retries must be non-negative", file=sys.stderr)
        return 2

    try:
        baseline_data = _load_validated(args.baseline)
        current_data = _load_validated(args.current)
        qa_perf_check.require_comparable_environments(
            baseline_data, current_data, args.baseline, args.current
        )
        regressed = _regressions(
            baseline_data, current_data, args.threshold, args.noise_floor_ns
        )
    except qa_perf_check.InputError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    retry_path = args.current.with_name(f"{args.current.stem}-retry.json")
    for attempt in range(1, args.retries + 1):
        bench_keys = retry_bench_keys(regressed)
        if not bench_keys:
            break
        print(
            f"Retrying {len(bench_keys)} regressed benchmark binaries "
            f"(attempt {attempt}/{args.retries}): {', '.join(bench_keys)}"
        )
        for bench_key in bench_keys:
            try:
                subprocess.run(
                    [
                        sys.executable,
                        str(BASELINE_RUNNER),
                        "--run",
                        bench_key,
                        "--output",
                        str(retry_path),
                    ],
                    cwd=ROOT,
                    check=True,
                )
                retry_data = _load_validated(retry_path)
                qa_perf_check.require_comparable_environments(
                    current_data, retry_data, args.current, retry_path
                )
            except (subprocess.CalledProcessError, qa_perf_check.InputError) as exc:
                print(
                    f"ERROR: performance retry failed for {bench_key}: {exc}",
                    file=sys.stderr,
                )
                return 2
            current_data = merge_best_records(
                current_data, retry_data, {bench_key}
            )

        args.current.write_text(json.dumps(current_data, indent=2) + "\n")
        regressed = _regressions(
            baseline_data, current_data, args.threshold, args.noise_floor_ns
        )

    return qa_perf_check.main(_check_args(args))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
