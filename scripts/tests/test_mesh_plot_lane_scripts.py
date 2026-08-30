from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class MeshPlotLaneScriptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.repo = Path(__file__).resolve().parents[2]
        self.script = self.repo / "scripts/qa_mesh_cross_adapter_visual.sh"

    def write_skip_manifest(self, path: Path, reason: str) -> None:
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "renderer": "mesh-test",
                    "status": "skipped",
                    "reason": reason,
                    "cases": [],
                }
            ),
            encoding="utf-8",
        )

    def run_lane(
        self,
        directory: Path,
        *,
        required: bool,
        metal_status: str = "skipped",
    ) -> subprocess.CompletedProcess[str]:
        metal = directory / "metal.json"
        wgpu = directory / "wgpu.json"
        report = directory / "cross-adapter.json"
        if metal_status == "skipped":
            self.write_skip_manifest(metal, "no Metal adapter")
        else:
            metal.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "renderer": "mesh-test",
                        "status": metal_status,
                        "cases": [],
                    }
                ),
                encoding="utf-8",
            )
        self.write_skip_manifest(wgpu, "no WGPU adapter")
        report.write_text("stale report", encoding="utf-8")
        environment = {
            **os.environ,
            "MESH_PLOT_PYTHON_BIN": sys.executable,
            "QA_MESH_METAL_MANIFEST": str(metal),
            "QA_MESH_WGPU_MANIFEST": str(wgpu),
            "QA_MESH_CROSS_ADAPTER_REPORT": str(report),
        }
        if required:
            environment["QA_CROSS_ADAPTER_REQUIRED"] = "1"
        else:
            environment.pop("QA_CROSS_ADAPTER_REQUIRED", None)
        return subprocess.run(
            ["bash", str(self.script)],
            cwd=self.repo,
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )

    def test_developer_lane_skips_captured_pair_and_clears_stale_report(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            report = Path(temporary) / "cross-adapter.json"
            result = self.run_lane(Path(temporary), required=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(report.exists())
            self.assertIn("captures were skipped", result.stdout)

    def test_required_lane_rejects_skipped_pair(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = self.run_lane(Path(temporary), required=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn("requires captured", result.stderr)

    def test_developer_lane_rejects_unknown_manifest_status(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            result = self.run_lane(directory, required=False, metal_status="corrupt")
            self.assertEqual(result.returncode, 1)
            self.assertIn("unknown status", result.stderr)


if __name__ == "__main__":
    unittest.main()
