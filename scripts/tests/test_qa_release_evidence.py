import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from qa_release_evidence import (
    EvidenceError,
    PLATFORM_EVIDENCE,
    REQUIRED_ARTIFACTS,
    build_manifest,
    render_markdown,
)


class ReleaseEvidenceTests(unittest.TestCase):
    def make_repo(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for relative in REQUIRED_ARTIFACTS:
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
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

    def build(self, root: Path, **kwargs):
        with mock.patch(
            "qa_release_evidence.toolchain_provenance",
            return_value={"cargo": "cargo test", "just": "just test", "python": "3.test", "rustc": "rustc test"},
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
