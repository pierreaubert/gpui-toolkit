from __future__ import annotations

import unittest

import qa_perf_baseline
import qa_perf_check


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


if __name__ == "__main__":
    unittest.main()
