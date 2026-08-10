import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from qa_release_evidence import (
    EvidenceError,
    MESH_PLOT_LOCAL_CAPTURE_COUNT,
    MESH_PLOT_VERSIONED_BASELINE_COUNT,
    MESH_PLOT_VISUAL_CAPTURE_ARTIFACT,
    MESH_PLOT_VISUAL_DIFF_ARTIFACT,
    PLATFORM_EVIDENCE,
    REQUIRED_ARTIFACTS,
    build_manifest,
    render_markdown,
)


PERF_ENVIRONMENT = {
    "system": "TestOS",
    "machine": "arm64",
    "cpu_model": "fixture",
    "rustc": {"release": "1.90.0", "host": "arm64-test"},
}


def perf_fixture() -> dict[str, object]:
    return {
        "version": 2,
        "metadata": {"environment": PERF_ENVIRONMENT},
        "records": [
            {"crate": crate, "bench": bench}
            for crate, bench in (
                ("gpui-d3rs", "mesh_prep"),
                ("gpui-px", "mesh_plot_frames"),
            )
        ],
    }


MESH_PLOT_BASELINE_IDS = tuple(
    f"px-mesh-plot__fixture-{index:02d}"
    for index in range(MESH_PLOT_VERSIONED_BASELINE_COUNT)
)


def mesh_plot_capture_fixture(root: Path) -> dict[str, object]:
    cases = []
    for index in range(MESH_PLOT_LOCAL_CAPTURE_COUNT):
        capture_id = f"px-mesh-plot__local-{index:03d}"
        actual_path = Path(
            "target/qa/visual/component-lab/metal/actual"
        ) / f"{capture_id}.png"
        path = root / actual_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"fixture image")
        cases.append(
            {
                "capture_id": capture_id,
                "story_id": "px.mesh_plot.fixture",
                "renderer_id": "metal",
                "actual_path": actual_path.as_posix(),
                "status": "Captured",
            }
        )
    return {
        "schema_version": 1,
        "report_type": "gpui-component-lab-render-capture",
        "renderer_id": "metal",
        "passed": True,
        "requested_count": MESH_PLOT_LOCAL_CAPTURE_COUNT,
        "captured_count": MESH_PLOT_LOCAL_CAPTURE_COUNT,
        "failed_count": 0,
        "cases": cases,
    }


def mesh_plot_diff_fixture() -> dict[str, object]:
    return {
        "schema_version": 1,
        "report_type": "gpui-component-lab-visual-diff",
        "passed": True,
        "compared_count": MESH_PLOT_VERSIONED_BASELINE_COUNT,
        "failed_count": 0,
        "max_changed_pixels": 0,
        "cases": [
            {
                "capture_id": capture_id,
                "status": "Passed",
                "changed_pixels": 0,
                "max_channel_delta": 0,
            }
            for capture_id in MESH_PLOT_BASELINE_IDS
        ],
    }


class ReleaseEvidenceTests(unittest.TestCase):
    def make_repo(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for relative in REQUIRED_ARTIFACTS:
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            if relative in {"qa/perf/baseline.json", "target/qa/perf/current.json"}:
                path.write_text(json.dumps(perf_fixture()), encoding="utf-8")
            elif relative == MESH_PLOT_VISUAL_CAPTURE_ARTIFACT:
                path.write_text(json.dumps(mesh_plot_capture_fixture(root)), encoding="utf-8")
            elif relative == MESH_PLOT_VISUAL_DIFF_ARTIFACT:
                path.write_text(json.dumps(mesh_plot_diff_fixture()), encoding="utf-8")
            else:
                path.write_text(f"fixture for {relative}\n", encoding="utf-8")
        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        subprocess.run(
            [
                "git",
                "-c",
                "user.name=Evidence Test",
                "-c",
                "user.email=evidence@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
            cwd=root,
            check=True,
            env={
                **__import__("os").environ,
                "GIT_AUTHOR_DATE": "2024-01-01T00:00:00Z",
                "GIT_COMMITTER_DATE": "2024-01-01T00:00:00Z",
            },
        )
        return root

    def build(self, root: Path, visual_members: list[str] | None = None, **kwargs):
        with (
            mock.patch(
                "qa_release_evidence.toolchain_provenance",
                return_value={"cargo": "cargo test", "just": "just test", "python": "3.test", "rustc": "rustc test"},
            ),
            mock.patch(
                "qa_release_evidence.visual_baseline_members",
                return_value=visual_members
                if visual_members is not None
                else [f"metal/baseline/{capture_id}.png" for capture_id in MESH_PLOT_BASELINE_IDS],
            ),
        ):
            return build_manifest(root, **kwargs)

    def test_manifest_binds_required_artifacts_without_private_paths(self):
        root = self.make_repo()
        manifest = self.build(root, require_clean=True)

        self.assertFalse(manifest["source"]["dirty"])
        self.assertEqual(len(manifest["source"]["revision"]), 40)
        self.assertEqual(len(manifest["artifacts"]), len(REQUIRED_ARTIFACTS))
        encoded = json.dumps(manifest, sort_keys=True)
        self.assertNotIn(str(root), encoded)
        for row in manifest["artifacts"]:
            self.assertEqual(len(row["sha256"]), 64)
            self.assertGreater(row["size_bytes"], 0)

        markdown = render_markdown(manifest)
        self.assertIn("source_dirty: `false`", markdown)
        self.assertIn("manifest-bound", markdown)

    def test_missing_required_artifact_fails(self):
        root = self.make_repo()
        missing = root / REQUIRED_ARTIFACTS[0]
        missing.unlink()
        with self.assertRaisesRegex(EvidenceError, "missing required QA artifacts"):
            self.build(root)

    def test_missing_mesh_plot_benchmark_evidence_fails(self):
        root = self.make_repo()
        baseline = root / "qa/perf/baseline.json"
        data = json.loads(baseline.read_text(encoding="utf-8"))
        data["records"] = []
        baseline.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "MeshPlot benchmark evidence is missing"):
            self.build(root)

    def test_missing_mesh_plot_visual_baseline_fails(self):
        root = self.make_repo()
        with self.assertRaisesRegex(EvidenceError, "MeshPlot visual baseline evidence"):
            self.build(root, visual_members=["metal/baseline/px-line__fixture.png"])

    def test_macos_archive_sidecars_do_not_count_as_baselines(self):
        root = self.make_repo()
        members = [
            f"metal/baseline/._{capture_id}.png"
            for capture_id in MESH_PLOT_BASELINE_IDS
        ] + [
            f"metal/baseline/{capture_id}.png"
            for capture_id in MESH_PLOT_BASELINE_IDS
        ]
        manifest = self.build(root, visual_members=members)
        self.assertEqual(len(manifest["artifacts"]), len(REQUIRED_ARTIFACTS))

    def test_mesh_plot_local_capture_requires_99_actual_cases(self):
        root = self.make_repo()
        capture = root / MESH_PLOT_VISUAL_CAPTURE_ARTIFACT
        data = json.loads(capture.read_text(encoding="utf-8"))
        data["captured_count"] = MESH_PLOT_LOCAL_CAPTURE_COUNT - 1
        capture.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "99 requested/captured"):
            self.build(root)

    def test_mesh_plot_diff_must_match_nine_baselines(self):
        root = self.make_repo()
        diff = root / MESH_PLOT_VISUAL_DIFF_ARTIFACT
        data = json.loads(diff.read_text(encoding="utf-8"))
        data["compared_count"] = MESH_PLOT_VERSIONED_BASELINE_COUNT - 1
        diff.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "9 compared cases"):
            self.build(root)

    def test_require_clean_rejects_dirty_worktree(self):
        root = self.make_repo()
        (root / "untracked.txt").write_text("dirty\n", encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "clean worktree"):
            self.build(root, require_clean=True)

    def test_required_platform_must_match_clean_manifest_source(self):
        root = self.make_repo()
        revision = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=root, check=True, capture_output=True, text=True
        ).stdout.strip()
        evidence = root / PLATFORM_EVIDENCE["android-emulator"]
        evidence.parent.mkdir(parents=True)
        evidence.write_text(
            json.dumps({"source_revision": revision, "source_dirty": False}), encoding="utf-8"
        )
        manifest = self.build(root, required_platforms=["android-emulator"])
        platform_row = next(
            row for row in manifest["artifacts"] if row["path"] == PLATFORM_EVIDENCE["android-emulator"]
        )
        self.assertTrue(platform_row["embedded_source"]["matches_manifest_source"])

        evidence.write_text(
            json.dumps({"source_revision": "0" * 40, "source_dirty": False}), encoding="utf-8"
        )
        with self.assertRaisesRegex(EvidenceError, "another source revision"):
            self.build(root, required_platforms=["android-emulator"])


if __name__ == "__main__":
    unittest.main()
