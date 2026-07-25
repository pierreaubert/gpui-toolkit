from __future__ import annotations

import unittest

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


if __name__ == "__main__":
    unittest.main()
