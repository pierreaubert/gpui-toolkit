#!/usr/bin/env python3
"""Register a freshly collected performance baseline for this machine."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import qa_perf_baseline
import qa_perf_check


def register(current_path: Path, directory: Path) -> Path:
    data = qa_perf_check.validate_baseline(qa_perf_check.load_json(current_path), current_path)
    qa_perf_check.records_from_data(data, current_path)
    if data["metadata"].get("collection") != "fresh-run":
        raise qa_perf_check.InputError("baseline registration requires a fresh full benchmark run")
    expected = {spec.key for spec in qa_perf_baseline.host_benchmarks()}
    found = {f"{record['crate']}:{record['bench']}" for record in data["records"]}
    if missing := sorted(expected - found):
        raise qa_perf_check.InputError(f"baseline is missing benchmark families: {', '.join(missing)}")
    identity = qa_perf_baseline.environment_metadata()
    if qa_perf_check.host_identity(data) != qa_perf_check.host_identity({"metadata": {"environment": identity}}):
        raise qa_perf_check.InputError("benchmark samples were not captured on this machine")
    destination = qa_perf_check.find_host_baseline(directory, data)
    if destination is None:
        destination = qa_perf_check.new_host_baseline_path(directory, data)
        if destination.exists():
            raise qa_perf_check.InputError(f"baseline path already exists: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return destination


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--current", type=Path, required=True)
    parser.add_argument("--baseline-dir", type=Path, default=Path("qa/perf"))
    args = parser.parse_args(argv)
    try:
        path = register(args.current, args.baseline_dir)
    except (qa_perf_check.InputError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    print(f"Registered performance baseline: {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
