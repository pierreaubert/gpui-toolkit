#!/usr/bin/env python3
"""Compare two release-built Python host test binaries on the same machine.

Both binaries must include `python_ir_perf_tests.rs` and use the `perf-test`
feature. For the baseline, export the original revision and copy in only the
candidate's benchmark module, its module declaration, and the `perf-test`
manifest additions. Build each with `cargo test --release -p gpui-python-runtime
--features perf-test --bin gpui-python-showcase --no-run`. Use separate target
directories or copy each executable immediately: Cargo can reuse an output path.
The script alternates the two binaries and records every successful run.
For `--renderer metal`, build both with `--features perf-test,native-qa` and
run on macOS with access to a Metal device.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
from datetime import datetime, timezone


CASES = (
    ("table_refresh_10000", "table", "refresh", 10_000),
    ("overview_refresh_10000", "overview", "refresh", 10_000),
    ("table_scroll_10000", "table", "scroll", 10_000),
    ("table_selection_patch_10000", "table", "patch", 10_000),
    ("table_refresh_100", "table", "refresh", 100),
)
METRICS = (
    "cold_ns",
    "median_ns",
    "min_ns",
    "max_ns",
    "frames",
    "views_built",
    "alloc_bytes_per_iter",
    "alloc_calls_per_iter",
    "image_hash",
)
TEST_NAME = "python_ir_showcase::python_ir_perf_tests::python_table_frames"


def digest(path):
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def measure(binary, environment, section, mode, renderer, rows, frames):
    command = [str(binary), "--exact", TEST_NAME, "--ignored", "--nocapture", "--test-threads=1"]
    result = subprocess.run(command, env=environment, capture_output=True, text=True, check=True)
    line = next(
        (line for line in result.stdout.splitlines() if "section=" in line and "median_ns=" in line),
        None,
    )
    if line is None:
        raise RuntimeError(f"benchmark produced no measurement: {result.stdout}\n{result.stderr}")
    fields = dict(field.split("=", 1) for field in line.split() if "=" in field)
    if (fields["section"], fields["mode"], fields["renderer"], int(fields["rows"]), int(fields["iterations"])) != (
        section,
        mode,
        renderer,
        rows,
        frames,
    ):
        raise RuntimeError(f"unexpected benchmark workload: {line}")
    sample = {key: int(fields[key]) for key in METRICS}
    if sample["frames"] < frames or sample["views_built"] < frames:
        raise RuntimeError(f"benchmark did not rebuild every requested view: {line}")
    return sample


def summarize(samples):
    medians = [sample["median_ns"] for sample in samples]
    cold = [sample["cold_ns"] for sample in samples]
    return {
        "median_ns": statistics.median(medians),
        "median_range_ns": [min(medians), max(medians)],
        "cold_median_ns": statistics.median(cold),
        "cold_range_ns": [min(cold), max(cold)],
        "alloc_bytes_per_iteration": statistics.median(
            sample["alloc_bytes_per_iter"] for sample in samples
        ),
        "alloc_calls_per_iteration": statistics.median(
            sample["alloc_calls_per_iter"] for sample in samples
        ),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--baseline-revision", required=True)
    parser.add_argument("--build-features", default="perf-test")
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--frames", type=int, default=60)
    parser.add_argument("--renderer", choices=("headless", "metal"), default="headless")
    parser.add_argument("--case", action="append", choices=[case[0] for case in CASES])
    args = parser.parse_args()
    binaries = {"baseline": args.baseline.resolve(), "candidate": args.candidate.resolve()}
    hashes = {name: digest(path) for name, path in binaries.items()}
    if hashes["baseline"] == hashes["candidate"]:
        raise ValueError("baseline and candidate executables are identical")

    workspace = Path(__file__).resolve().parent.parent
    sources = (
        "Cargo.lock",
        "crates/gpui-python-runtime/Cargo.toml",
        "crates/gpui-python-runtime/bin/showcase/python_ir_showcase.rs",
        "crates/gpui-python-runtime/bin/showcase/python_table_rows.rs",
        "crates/gpui-python-runtime/bin/showcase/python_ir_lifecycle_tests.rs",
        "crates/gpui-python-runtime/bin/showcase/python_ir_perf_tests.rs",
    )
    report = {
        "baseline_revision": args.baseline_revision,
        "build_features": args.build_features,
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "platform": platform.platform(),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "logical_cpus": os.cpu_count(),
        "load_before": os.getloadavg(),
        "binary_sha256": hashes,
        "comparison_script_sha256": digest(Path(__file__).resolve()),
        "candidate_source_sha256": {name: digest(workspace / name) for name in sources},
        "runs_per_side": args.runs,
        "iterations_per_run": args.frames,
        "text_system": "CosmicTextSystem with bundled IBM Plex Sans fonts",
        "renderer": (
            "Metal offscreen rendering with synchronous pixel readback"
            if args.renderer == "metal"
            else "headless; no GPU rasterization or presentation"
        ),
        "baseline_preparation": "original revision with the same perf-test feature and benchmark module added",
        "allocations": "process-wide allocation counters, averaged per timed iteration",
        "cold_scope": "window open and first draw; excludes snapshot loading and font registration",
        "iteration_scope": "notified update and draw; scroll moves the list offset, patch commits a selection update",
        "cases": {},
    }
    with tempfile.TemporaryDirectory(prefix="gpui-table-perf-") as tmp:
        state = Path(tmp) / "gpui-python-runtime" / "host-presentation.json"
        state.parent.mkdir(parents=True)
        state.write_text(
            json.dumps({"version": 2, "width": 1240.0, "height": 820.0, "section": "table", "scroll_y": 0.0})
        )
        for name, section, mode, rows in CASES:
            if args.case and name not in args.case:
                continue
            case = {"section": section, "mode": mode, "rows": rows, "runs": {"baseline": [], "candidate": []}}
            for _ in range(args.runs):
                for side in ("baseline", "candidate"):
                    environment = os.environ.copy()
                    environment.update({
                        "GPUI_TOOLKIT_DATA_DIR": tmp,
                        "GPUI_TOOLKIT_APP_ID": "gpui-python-runtime",
                        "GPUI_VIEW_RETENTION": "1",
                        "GPUI_SCROLL_LAYERS": "1",
                        "GPUI_PERF_ROWS": str(rows),
                        "GPUI_PERF_FRAMES": str(args.frames),
                        "GPUI_PERF_SECTION": section,
                        "GPUI_PERF_MODE": mode,
                        "GPUI_PERF_RENDERER": args.renderer,
                    })
                    case["runs"][side].append(
                        measure(binaries[side], environment, section, mode, args.renderer, rows, args.frames)
                    )
            for side in ("baseline", "candidate"):
                case[side] = summarize(case["runs"][side])
            if args.renderer == "metal":
                image_hashes = {
                    sample["image_hash"]
                    for samples in case["runs"].values()
                    for sample in samples
                }
                if len(image_hashes) != 1:
                    raise RuntimeError(f"rendered images differ for {name}: {image_hashes}")
            case["change_percent"] = 100 * (
                case["candidate"]["median_ns"] / case["baseline"]["median_ns"] - 1
            )
            report["cases"][name] = case
            print(
                f"{name}: {case['baseline']['median_ns']} -> {case['candidate']['median_ns']} ns "
                f"({case['change_percent']:+.1f}%)",
                flush=True,
            )
    report["load_after"] = os.getloadavg()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
