#!/usr/bin/env python3
"""Run all Criterion benches in the workspace and emit qa/perf/baseline.json.

Usage:
  python3 scripts/qa_perf_baseline.py --output qa/perf/baseline.json
  python3 scripts/qa_perf_baseline.py --output target/qa/perf/current.json
  python3 scripts/qa_perf_baseline.py --run gpui-builder:solved_tree
  python3 scripts/qa_perf_baseline.py --collect-only --output qa/perf/baseline.json

Baseline JSON schema (version 1):
{
  "version": 1,
  "generated_at": "<ISO-8601 UTC timestamp>",
  "metadata": {"criterion_flags": "...", "generator": "scripts/qa_perf_baseline.py"},
  "records": [
    {"crate": "...", "bench": "...", "group": "...", "function": "...",
     "median_ns": float, "mean_ns": float, "unit": "ns"}
  ]
}
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import platform
import re
import subprocess
import sys
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
TARGET_CRITERION = ROOT / "target" / "criterion"
BASELINE_VERSION = 2

# Stable-but-reasonable Criterion settings for the non-regression suite.
# Default settings are too slow for `just qa`; these still produce usable medians.
CRITERION_FLAGS = [
    "--noplot",
    "--sample-size",
    "100",
    "--measurement-time",
    "5",
    "--warm-up-time",
    "2",
]
BENCH_TIMEOUT_SECS = 1800


@dataclass(frozen=True)
class BenchSpec:
    crate: str
    bench: str
    features: list[str]
    # Criterion group_id glob patterns that belong to this benchmark binary.
    groups: list[str] = field(default_factory=list)
    no_default_features: bool = False

    @property
    def key(self) -> str:
        return f"{self.crate}:{self.bench}"


# Benchmarks included in the non-regression suite.
# The `groups` patterns map Criterion group_id values to crate:bench.
BENCHMARKS: list[BenchSpec] = [
    BenchSpec(
        "gpui-builder",
        "solved_tree",
        [],
        groups=["balanced_tree_*", "wide_tree_*", "text_cache_hit"],
    ),
    BenchSpec("gpui-d3rs", "force_many_body", ["gpui", "gpu-2d", "gpu-3d"]),
    BenchSpec(
        "gpui-d3rs",
        "path_strings",
        ["gpui", "gpu-2d", "gpu-3d"],
        groups=["path/*", "geo_path/*"],
    ),
    BenchSpec(
        "gpui-keybinding",
        "discovery",
        [],
        groups=["search_command_palette_cached", "format_key_label"],
    ),
    BenchSpec(
        "gpui-pretext",
        "layout_temporaries",
        [],
        groups=["measurement/*", "layout/*"],
    ),
    BenchSpec(
        "gpui-px",
        "streaming_prepare",
        [],
        groups=["streaming_prepare"],
    ),
    BenchSpec(
        "gpui-ui-kit",
        "edit_state",
        ["bench"],
        groups=["insert_char", "backspace", "delete_selection", "kill_word_backward"],
    ),
    BenchSpec("gpui-ios", "accessibility_diff", [], groups=["accessibility_diff"]),
]


def existing_criterion_dirs() -> set[str]:
    if not TARGET_CRITERION.exists():
        return set()
    return {str(p.relative_to(TARGET_CRITERION)) for p in TARGET_CRITERION.rglob("new") if p.is_dir()}


def run_bench(spec: BenchSpec) -> None:
    print(f"Running {spec.key} ...")
    cmd = ["cargo", "bench", "-p", spec.crate, "--bench", spec.bench]
    if spec.no_default_features:
        cmd.append("--no-default-features")
    if spec.features:
        cmd.extend(["--features", ",".join(spec.features)])
    cmd.extend(["--", *CRITERION_FLAGS])

    before = existing_criterion_dirs()
    try:
        subprocess.run(cmd, cwd=ROOT, check=True, timeout=BENCH_TIMEOUT_SECS)
    except FileNotFoundError:
        print("ERROR: cargo executable not found; is Rust installed?", file=sys.stderr)
        sys.exit(2)
    except subprocess.CalledProcessError as exc:
        print(f"ERROR: benchmark {spec.key} failed: {exc}", file=sys.stderr)
        sys.exit(2)
    except subprocess.TimeoutExpired:
        print(f"ERROR: benchmark {spec.key} timed out after {BENCH_TIMEOUT_SECS}s", file=sys.stderr)
        sys.exit(2)

    after = existing_criterion_dirs()
    if not (after - before):
        print(
            f"NOTE: {spec.key} did not create any new target/criterion/*/new directories "
            "(it may have reused existing ones).",
            file=sys.stderr,
        )


def parse_estimates(new_dir: Path) -> dict[str, Any] | None:
    estimates = new_dir / "estimates.json"
    try:
        return json.loads(estimates.read_text())  # type: ignore[no-any-return]
    except (json.JSONDecodeError, OSError) as exc:
        print(f"WARNING: could not read {estimates}: {exc}", file=sys.stderr)
        return None


def _meta_str(value: Any) -> str:
    return "" if value is None else str(value)


def read_benchmark_meta(new_dir: Path) -> tuple[str, str, str]:
    """Return (group_id, function_id, value_str) from the nearest benchmark.json."""
    benchmark_json = new_dir / "benchmark.json"
    try:
        meta = json.loads(benchmark_json.read_text())
        return (
            _meta_str(meta.get("group_id", "")),
            _meta_str(meta.get("function_id", "")),
            _meta_str(meta.get("value_str", "")),
        )
    except (json.JSONDecodeError, OSError):
        pass

    # Fallback: derive from directory layout target/criterion/<group>/<function>/<value>/new
    parts = new_dir.relative_to(TARGET_CRITERION).parts
    if len(parts) >= 4 and parts[-1] == "new":
        return parts[-4], parts[-3], parts[-2]
    if len(parts) == 3 and parts[-1] == "new":
        return parts[-3], parts[-2], ""
    if len(parts) == 2 and parts[-1] == "new":
        return parts[-2], parts[-2], ""
    return "", "", ""


def discover_records() -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    if not TARGET_CRITERION.exists():
        return records

    for new_dir in sorted(TARGET_CRITERION.rglob("new")):
        if not new_dir.is_dir():
            continue
        estimates = parse_estimates(new_dir)
        if estimates is None:
            continue
        try:
            median_ns = float(estimates["median"]["point_estimate"])
            mean_ns = float(estimates["mean"]["point_estimate"])
        except (KeyError, TypeError, ValueError) as exc:
            print(f"WARNING: unexpected estimates JSON in {new_dir}: {exc}", file=sys.stderr)
            continue

        group_name, function_name, value_str = read_benchmark_meta(new_dir)
        if not group_name:
            group_name = new_dir.parent.name
        if not function_name:
            function_name = new_dir.parent.name
        if value_str:
            function_name = f"{function_name}/{value_str}" if function_name else value_str

        records.append(
            {
                "crate": "",  # filled in later
                "bench": "",
                "group": group_name,
                "function": function_name,
                "median_ns": median_ns,
                "mean_ns": mean_ns,
                "unit": "ns",
            }
        )
    return records


def spec_for_group(group: str) -> BenchSpec | None:
    """Map a Criterion group_id to the benchmark spec that owns it."""
    for spec in BENCHMARKS:
        if spec.groups:
            for pattern in spec.groups:
                if fnmatch.fnmatch(group, pattern):
                    return spec
        # Fallback: some groups are named exactly like the bench binary.
        if group == spec.bench:
            return spec
    return None


def assign_crates(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Map Criterion records to crate:bench pairs by group_id."""
    assigned: list[dict[str, Any]] = []
    for rec in records:
        spec = spec_for_group(rec["group"])
        if spec is None:
            print(
                f"WARNING: could not map record group={rec['group']} function={rec['function']} to a crate",
                file=sys.stderr,
            )
            continue
        rec["crate"] = spec.crate
        rec["bench"] = spec.bench
        assigned.append(rec)
    return assigned


def run_all() -> list[dict[str, Any]]:
    for spec in BENCHMARKS:
        run_bench(spec)
    records = discover_records()
    return assign_crates(records)


def collect_all() -> list[dict[str, Any]]:
    records = discover_records()
    assigned = assign_crates(records)
    found_keys = {(r["crate"], r["bench"]) for r in assigned}
    for spec in BENCHMARKS:
        if (spec.crate, spec.bench) not in found_keys:
            print(
                f"WARNING: no Criterion results found for {spec.key}",
                file=sys.stderr,
            )
    return assigned


def build_baseline(records: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "version": BASELINE_VERSION,
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "metadata": {
            "criterion_flags": " ".join(CRITERION_FLAGS),
            "generator": "scripts/qa_perf_baseline.py",
            "environment": environment_metadata(),
        },
        "records": records,
    }


def command_output(command: list[str]) -> str:
    """Return a stable one-line command result without making QA fragile."""
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (FileNotFoundError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return "unknown"
    return " ".join(result.stdout.strip().split()) or "unknown"


def cpu_model() -> str:
    """Return the most useful stable CPU identity available on this host."""
    if sys.platform == "darwin":
        brand = command_output(["sysctl", "-n", "machdep.cpu.brand_string"])
        hardware = command_output(["sysctl", "-n", "hw.model"])
        identity = " | ".join(value for value in (brand, hardware) if value != "unknown")
        if identity:
            return identity
    if sys.platform.startswith("linux"):
        try:
            for line in Path("/proc/cpuinfo").read_text().splitlines():
                if line.startswith(("model name", "Hardware")):
                    return line.split(":", 1)[-1].strip()
        except OSError:
            pass
    return platform.processor() or "unknown"


def rustc_metadata() -> dict[str, str]:
    output = command_output(["rustc", "-Vv"])
    release = re.search(r"(?:^| )release: ([^ ]+)", output)
    host = re.search(r"(?:^| )host: ([^ ]+)", output)
    # `command_output` flattens lines, so retain the full value for diagnostics
    # even when a future rustc changes the verbose layout.
    return {
        "release": release.group(1) if release else "unknown",
        "host": host.group(1) if host else "unknown",
        "verbose": output,
    }


def environment_metadata() -> dict[str, Any]:
    """Describe the benchmark environment and source used for this run."""
    source_status = command_output(["git", "status", "--porcelain"])
    return {
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "cpu_model": cpu_model(),
        "rustc": rustc_metadata(),
        "cargo": command_output(["cargo", "-V"]),
        "source_revision": command_output(["git", "rev-parse", "HEAD"]),
        "source_dirty": source_status not in ("", "unknown"),
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        help="Path to write the baseline JSON (default: print to stdout)",
    )
    parser.add_argument(
        "--run",
        type=str,
        help="Run a single benchmark by key (crate:bench) and update the output file",
    )
    parser.add_argument(
        "--no-write",
        action="store_true",
        help="Run benchmarks but do not write the output file",
    )
    parser.add_argument(
        "--collect-only",
        action="store_true",
        help="Re-collect results already present in target/criterion without running benches",
    )
    args = parser.parse_args(argv)

    if args.collect_only:
        records = collect_all()
    elif args.run:
        spec = next((s for s in BENCHMARKS if s.key == args.run), None)
        if spec is None:
            print(f"ERROR: unknown benchmark {args.run!r}", file=sys.stderr)
            return 2
        run_bench(spec)
        records = assign_crates(discover_records())
    else:
        records = run_all()

    baseline = build_baseline(records)

    if args.no_write or args.output is None:
        print(json.dumps(baseline, indent=2))
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(baseline, indent=2) + "\n")
        print(f"Wrote {len(records)} records to {args.output}")

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
