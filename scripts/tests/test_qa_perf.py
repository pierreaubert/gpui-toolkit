from __future__ import annotations

import unittest
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest import mock

import qa_perf_baseline
import qa_perf_check
import qa_perf_gate


def fixture(environment: dict[str, object]) -> dict[str, object]:
    return {
        "version": 2,
        "metadata": {
            "criterion_flags": "--noplot",
            "generator": "test",
            "environment": environment,
        },
        "records": [],
    }


class PerformanceEnvironmentTests(unittest.TestCase):
    def test_mesh_plot_benchmarks_are_registered_with_the_release_runner(self) -> None:
        specs = {
            spec.key: spec
            for spec in qa_perf_baseline.BENCHMARKS
        }
        self.assertIn("gpui-d3rs:mesh_prep", specs)
        self.assertIn("gpui-px:mesh_plot_frames", specs)
        self.assertEqual(
            qa_perf_baseline.spec_for_group("mesh_prep").key,
            "gpui-d3rs:mesh_prep",
        )
        self.assertEqual(
            qa_perf_baseline.spec_for_group("mesh_plot_retained_frames").key,
            "gpui-px:mesh_plot_frames",
        )
        self.assertEqual(
            qa_perf_baseline.spec_for_group("mesh_plot_build_200000_triangles").key,
            "gpui-px:mesh_plot_frames",
        )
        self.assertEqual(
            qa_perf_baseline.spec_for_group("mesh_plot_fit_200000_triangles").key,
            "gpui-px:mesh_plot_frames",
        )
        self.assertEqual(
            qa_perf_baseline.spec_for_group("mesh_plot_retained_picking").key,
            "gpui-px:mesh_plot_frames",
        )
        self.assertIn(
            "gpu-3d",
            specs["gpui-px:mesh_plot_frames"].features,
        )

    def test_single_benchmark_run_does_not_collect_stale_other_benchmarks(self) -> None:
        requested = next(
            spec
            for spec in qa_perf_baseline.BENCHMARKS
            if spec.key == "gpui-d3rs:mesh_prep"
        )
        records = [
            {
                "crate": "gpui-d3rs",
                "bench": "mesh_prep",
                "group": "mesh_prep",
                "function": "prepare_100000_triangles",
                "median_ns": 10.0,
                "mean_ns": 11.0,
                "unit": "ns",
            },
            {
                "crate": "gpui-px",
                "bench": "mesh_plot_frames",
                "group": "mesh_plot_retained_frames",
                "function": "field_replace_100000_values",
                "median_ns": 20.0,
                "mean_ns": 21.0,
                "unit": "ns",
            },
        ]
        with (
            mock.patch.object(qa_perf_baseline, "run_bench") as run_bench,
            mock.patch.object(qa_perf_baseline, "discover_records", return_value=records),
        ):
            selected = qa_perf_baseline.run_one(requested)

        run_bench.assert_called_once()
        self.assertEqual(run_bench.call_args.args[0], requested)
        self.assertEqual(
            {(record["crate"], record["bench"]) for record in selected},
            {("gpui-d3rs", "mesh_prep")},
        )

    def test_macos_arm_cpu_model_is_stable_when_sysctl_is_unavailable(self) -> None:
        with (
            mock.patch.object(qa_perf_baseline.sys, "platform", "darwin"),
            mock.patch.object(
                qa_perf_baseline.platform, "machine", return_value="arm64"
            ),
            mock.patch.object(qa_perf_baseline, "command_output", return_value="unknown"),
        ):
            self.assertEqual(qa_perf_baseline.cpu_model(), "arm64")

    def test_macos_arm_cpu_model_distinguishes_hardware(self) -> None:
        with (
            mock.patch.object(qa_perf_baseline.sys, "platform", "darwin"),
            mock.patch.object(qa_perf_baseline, "command_output", side_effect=["Apple M4 Pro", "Mac16,8"]),
        ):
            self.assertEqual(qa_perf_baseline.cpu_model(), "Apple M4 Pro | Mac16,8")

    def test_generated_baseline_records_comparable_environment(self) -> None:
        baseline = qa_perf_baseline.build_baseline([])
        self.assertEqual(baseline["version"], 2)
        environment = baseline["metadata"]["environment"]
        self.assertTrue(environment["system"])
        self.assertTrue(environment["machine"])
        self.assertTrue(environment["cpu_model"])
        self.assertNotEqual(environment["rustc"]["release"], "unknown")
        self.assertNotEqual(environment["rustc"]["host"], "unknown")

    def test_mismatched_cpu_is_rejected(self) -> None:
        common = {
            "system": "TestOS",
            "machine": "arm64",
            "cpu_model": "fast",
            "rustc": {"release": "1.90.0", "host": "arm64-test"},
        }
        current = {**common, "cpu_model": "slow"}
        with self.assertRaisesRegex(
            qa_perf_check.InputError, "environments are not comparable"
        ):
            qa_perf_check.require_comparable_environments(
                fixture(common), fixture(current), "baseline", "current"
            )

    def test_source_revision_does_not_make_hosts_incomparable(self) -> None:
        environment = {
            "system": "TestOS",
            "machine": "arm64",
            "cpu_model": "same",
            "rustc": {"release": "1.90.0", "host": "arm64-test"},
        }
        baseline = fixture({**environment, "source_revision": "old"})
        current = fixture({**environment, "source_revision": "new"})
        qa_perf_check.require_comparable_environments(
            baseline, current, "baseline", "current"
        )

    def test_legacy_baseline_is_rejected_explicitly(self) -> None:
        with self.assertRaisesRegex(qa_perf_check.InputError, "expected version 2"):
            qa_perf_check.validate_baseline(
                {"version": 1, "metadata": {}, "records": []}, "legacy"
            )


class PerformanceRetryTests(unittest.TestCase):
    def test_retry_keys_are_sorted_and_deduplicated_by_benchmark_binary(self) -> None:
        regressed = [
            (
                qa_perf_check.RecordKey("crate-b", "bench-z", "group", "one"),
                80.0,
            ),
            (
                qa_perf_check.RecordKey("crate-a", "bench-a", "group", "two"),
                40.0,
            ),
            (
                qa_perf_check.RecordKey("crate-b", "bench-z", "group", "three"),
                20.1,
            ),
        ]

        self.assertEqual(
            qa_perf_gate.retry_bench_keys(regressed),
            ["crate-a:bench-a", "crate-b:bench-z"],
        )

    def test_merge_keeps_best_measurement_only_for_retried_benchmark(self) -> None:
        initial = fixture(
            {
                "system": "TestOS",
                "machine": "arm64",
                "cpu_model": "same",
                "rustc": {"release": "1.90.0", "host": "arm64-test"},
            }
        )
        initial["records"] = [
            {
                "crate": "crate-a",
                "bench": "bench-a",
                "group": "group",
                "function": "function",
                "median_ns": 200.0,
                "mean_ns": 210.0,
                "unit": "ns",
            },
            {
                "crate": "crate-b",
                "bench": "bench-b",
                "group": "group",
                "function": "function",
                "median_ns": 300.0,
                "mean_ns": 310.0,
                "unit": "ns",
            },
        ]
        retry = {**initial, "records": [dict(record) for record in initial["records"]]}
        retry["records"][0].update(median_ns=150.0, mean_ns=160.0)
        retry["records"][1].update(median_ns=100.0, mean_ns=110.0)

        merged = qa_perf_gate.merge_best_records(
            initial, retry, {"crate-a:bench-a"}
        )

        self.assertEqual(merged["records"][0]["median_ns"], 150.0)
        self.assertEqual(merged["records"][0]["mean_ns"], 160.0)
        self.assertEqual(merged["records"][1]["median_ns"], 300.0)


def timing_record(crate: str = "crate", bench: str = "bench") -> dict[str, object]:
    return {
        "crate": crate, "bench": bench, "group": "group", "function": "function",
        "median_ns": 1000.0, "mean_ns": 1100.0, "unit": "ns",
    }


class PerformanceEvidenceTests(unittest.TestCase):
    def test_empty_cli_comparison_fails_instead_of_passing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("base", "current"):
                (root / f"{name}.json").write_text(json.dumps(fixture({})))
            result = subprocess.run([
                sys.executable, str(Path(qa_perf_check.__file__)),
                "--baseline", str(root / "base.json"),
                "--current", str(root / "current.json"),
                "--output", str(root / "report.md"),
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertIn("no benchmark records", result.stderr)
            self.assertFalse((root / "report.md").exists())

    def test_missing_or_unmatched_samples_fail(self) -> None:
        base_data = fixture({})
        base_data["records"] = [timing_record()]
        baseline = qa_perf_check.records_from_data(base_data, Path("base"))
        for current in ({}, {qa_perf_check.RecordKey("other", "bench", "g", "f"): next(iter(baseline.values()))}):
            with self.assertRaises(qa_perf_check.InputError):
                qa_perf_check.compare(baseline, current, 20.0, 150.0)

    def test_only_noise_floor_samples_cannot_pass(self) -> None:
        data = fixture({})
        data["records"] = [{**timing_record(), "median_ns": 1.0}]
        records = qa_perf_check.records_from_data(data, Path("samples"))
        with self.assertRaisesRegex(qa_perf_check.InputError, "no matched benchmarks"):
            qa_perf_check.compare(records, records, 20, 150)

    def test_explicit_binary_removal_still_requires_a_real_match(self) -> None:
        data = fixture({})
        data["records"] = [timing_record(), timing_record(bench="retired")]
        base = qa_perf_check.records_from_data(data, Path("base"))
        data["records"] = [timing_record()]
        current = qa_perf_check.records_from_data(data, Path("current"))
        result = qa_perf_check.compare(base, current, 20, 150, frozenset({"crate:retired"}))
        self.assertEqual(len(result[0]), 1)
        self.assertEqual(len(result[2]), 1)

    def test_nonfinite_zero_or_invalid_unit_samples_are_rejected(self) -> None:
        for value in (float("nan"), float("inf"), -1.0, 0.0):
            with self.subTest(value=value), self.assertRaises(qa_perf_check.InputError):
                qa_perf_check.parse_record({**timing_record(), "median_ns": value}, Path("bad"))
        with self.assertRaises(qa_perf_check.InputError):
            qa_perf_check.parse_record({**timing_record(), "unit": "ms"}, Path("bad"))

    def test_stale_collection_and_feature_mismatch_are_rejected(self) -> None:
        baseline, current = fixture({}), fixture({})
        baseline["metadata"]["collection"] = "existing-samples"
        with self.assertRaisesRegex(qa_perf_check.InputError, "fresh benchmark runs"):
            qa_perf_check.require_comparable_environments(baseline, current, Path("b"), Path("c"))
        baseline["metadata"].pop("collection")
        baseline["metadata"]["benchmarks"] = {"crate:bench": {"features": []}}
        current["metadata"]["benchmarks"] = {"crate:bench": {"features": ["gpu"]}}
        with self.assertRaisesRegex(qa_perf_check.InputError, "configuration differs"):
            qa_perf_check.require_comparable_environments(baseline, current, Path("b"), Path("c"))

    def test_cargo_metadata_resolves_separate_base_and_candidate_targets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[package]\nname="perf-evidence-test"\nversion="0.0.0"\nedition="2024"\n')
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("")
            for name in ("base-target", "candidate-target"):
                with mock.patch.object(qa_perf_baseline, "ROOT", root), mock.patch.dict(os.environ, {"CARGO_TARGET_DIR": str(root / name)}):
                    self.assertEqual(qa_perf_baseline.resolve_criterion_directory(), root.resolve() / name / "criterion")
            (root / ".cargo").mkdir()
            (root / ".cargo/config.toml").write_text('[build]\ntarget-dir="configured-target"\n')
            environment = {key: value for key, value in os.environ.items() if key != "CARGO_TARGET_DIR"}
            with mock.patch.object(qa_perf_baseline, "ROOT", root), mock.patch.dict(os.environ, environment, clear=True):
                self.assertEqual(qa_perf_baseline.resolve_criterion_directory(), root.resolve() / "configured-target/criterion")

    def test_successful_binary_without_fresh_samples_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with (
                mock.patch.object(qa_perf_baseline, "TARGET_CRITERION", Path(directory) / "criterion"),
                mock.patch.object(qa_perf_baseline, "run_bench"),
            ):
                with self.assertRaisesRegex(qa_perf_baseline.CollectionError, "no fresh"):
                    qa_perf_baseline.run_one(qa_perf_baseline.BENCHMARKS[0])

    def test_real_subprocess_samples_are_isolated_from_old_results(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = root / "emit.py"
            script.write_text('''import os, json
from pathlib import Path
root = Path(os.environ["CRITERION_HOME"]) / "prepare_chunks_test" / "new"
root.mkdir(parents=True)
(root / "benchmark.json").write_text(json.dumps({"group_id":"prepare_chunks_test","function_id":"test"}))
(root / "estimates.json").write_text(json.dumps({"median":{"point_estimate":1000},"mean":{"point_estimate":1100}}))
''')
            with (
                mock.patch.object(qa_perf_baseline, "TARGET_CRITERION", root / "criterion"),
                mock.patch.object(qa_perf_baseline, "resolve_command", return_value=[sys.executable, str(script)]),
            ):
                first = qa_perf_baseline.run_one(qa_perf_baseline.BENCHMARKS[0])
                second = qa_perf_baseline.run_one(qa_perf_baseline.BENCHMARKS[0])
            self.assertEqual(len(first), 1)
            self.assertEqual(len(second), 1)
            self.assertEqual(first[0]["crate"], "gpui-scene3d")

    def test_scene_and_flow_families_are_required(self) -> None:
        keys = {spec.key for spec in qa_perf_baseline.BENCHMARKS}
        self.assertTrue({"gpui-scene3d:render_bench", "gpui-dataset-transformation:flow_bench"} <= keys)
        with mock.patch.object(qa_perf_baseline, "discover_records", return_value=[]):
            with self.assertRaisesRegex(qa_perf_baseline.CollectionError, "missing required"):
                qa_perf_baseline.collect_all()


if __name__ == "__main__":
    unittest.main()
