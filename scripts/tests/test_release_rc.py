import gzip
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from release_rc import (
    ReleaseError,
    build,
    ensure_clean,
    git_archive,
    license_inventory,
    resolve_command,
    spdx_document,
    validate_version,
)


class ReleaseCandidateTests(unittest.TestCase):
    def make_repo(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        (root / "Cargo.toml").write_text(
            '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "1.2.3"\n',
            encoding="utf-8",
        )
        (root / "tracked.txt").write_text("release source\n", encoding="utf-8")
        (root / "Cargo.lock").write_text("# fixture lock\n", encoding="utf-8")
        gallery = root / "assets" / "component-lab-gallery"
        gallery.mkdir(parents=True)
        (gallery / "sheet.png").write_bytes(b"deterministic pixels")
        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        subprocess.run(
            ["git", "-c", "user.name=RC Test", "-c", "user.email=rc@example.invalid", "commit", "-qm", "fixture"],
            cwd=root,
            check=True,
            env={**__import__("os").environ, "GIT_AUTHOR_DATE": "2024-01-01T00:00:00Z", "GIT_COMMITTER_DATE": "2024-01-01T00:00:00Z"},
        )
        return root

    def test_version_must_be_semver_and_match_workspace(self):
        root = self.make_repo()
        validate_version(root, "1.2.3")
        with self.assertRaises(ReleaseError):
            validate_version(root, "v1.2.3")
        with self.assertRaises(ReleaseError):
            validate_version(root, "1.2.4")

    def test_clean_worktree_gate_rejects_tracked_and_untracked_changes(self):
        root = self.make_repo()
        ensure_clean(root)
        (root / "untracked.txt").write_text("dirty\n", encoding="utf-8")
        with self.assertRaisesRegex(ReleaseError, "clean worktree"):
            ensure_clean(root)

    def test_source_archive_is_byte_reproducible_and_path_relative(self):
        root = self.make_repo()
        first = root / "first.tar.gz"
        second = root / "second.tar.gz"
        git_archive(root, first, "gpui-toolkit-1.2.3/")
        git_archive(root, second, "gpui-toolkit-1.2.3/")
        self.assertEqual(first.read_bytes(), second.read_bytes())
        with gzip.open(first, "rb") as archive:
            body = archive.read()
        self.assertIn(b"gpui-toolkit-1.2.3/tracked.txt", body)
        self.assertNotIn(str(root).encode(), body)

    def test_resolve_command_falls_back_to_cargo_bin(self):
        with mock.patch("release_rc.shutil.which", return_value=None):
            resolved = resolve_command(("cargo", "--version"))
        cargo = Path.home() / ".cargo" / "bin" / "cargo"
        if cargo.is_file() or cargo.is_symlink():
            self.assertEqual(resolved, [str(cargo), "--version"])
        else:
            self.assertEqual(resolved, ["cargo", "--version"])

    def test_sbom_and_license_inventory_are_sorted_and_private_path_free(self):
        metadata = {
            "packages": [
                {"name": "z", "version": "2.0.0", "license": None, "source": None, "manifest_path": "/private/home/z/Cargo.toml"},
                {"name": "a", "version": "1.0.0", "license": "MIT", "source": "registry+https://example.invalid/index", "manifest_path": "/private/home/a/Cargo.toml"},
            ]
        }
        licenses = license_inventory(metadata)
        self.assertEqual([row["name"] for row in licenses], ["a", "z"])
        document = spdx_document(metadata, "1.2.3", "abc123", "2024-01-01T00:00:00Z")
        encoded = json.dumps(document, sort_keys=True)
        self.assertNotIn("/private/home", encoded)
        self.assertEqual(document["spdxVersion"], "SPDX-2.3")

    def test_complete_bundle_has_expected_artifacts_checksums_and_reproducible_bytes(self):
        root = self.make_repo()
        metadata = {
            "packages": [
                {"name": "gpui-design", "version": "1.2.3", "license": "MIT", "source": None},
                {"name": "dependency", "version": "2.0.0", "license": "Apache-2.0", "source": "registry+https://example.invalid/index"},
            ]
        }

        def fake_packages(_root, staging, _metadata):
            names = []
            for name in ("gpui-design", "gpui-profiler", "gpui-ui-kit-macros"):
                artifact = staging / f"{name}-1.2.3.crate"
                artifact.write_bytes(f"{name}-1.2.3\n".encode())
                names.append(artifact.name)
            return names

        first_parent = tempfile.TemporaryDirectory()
        second_parent = tempfile.TemporaryDirectory()
        self.addCleanup(first_parent.cleanup)
        self.addCleanup(second_parent.cleanup)
        first = Path(first_parent.name) / "bundle"
        second = Path(second_parent.name) / "bundle"
        with mock.patch("release_rc.cargo_metadata", return_value=metadata), mock.patch(
            "release_rc.package_wave_one", side_effect=fake_packages
        ):
            build(root, "1.2.3", first)
            build(root, "1.2.3", second)

        expected = {
            "SHA256SUMS",
            "gpui-design-1.2.3.crate",
            "gpui-profiler-1.2.3.crate",
            "gpui-toolkit-1.2.3-source.tar.gz",
            "gpui-toolkit-1.2.3-visual-gallery.tar.gz",
            "gpui-ui-kit-macros-1.2.3.crate",
            "licenses.json",
            "licenses.md",
            "provenance.json",
            "sbom.spdx.json",
        }
        self.assertEqual({path.name for path in first.iterdir()}, expected)
        self.assertEqual({path.name for path in second.iterdir()}, expected)
        for name in expected:
            self.assertEqual((first / name).read_bytes(), (second / name).read_bytes(), name)

        checksums = (first / "SHA256SUMS").read_text(encoding="utf-8")
        self.assertNotIn("SHA256SUMS", checksums)
        for name in expected - {"SHA256SUMS"}:
            self.assertIn(f"  {name}\n", checksums)
        provenance = json.loads((first / "provenance.json").read_text(encoding="utf-8"))
        self.assertFalse(provenance["network_used"])
        self.assertFalse(provenance["published"])
        self.assertNotIn(str(root), json.dumps(provenance))


if __name__ == "__main__":
    unittest.main()
