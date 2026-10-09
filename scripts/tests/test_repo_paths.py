from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path

from repo_paths import display_path, within_repo


class RepoPathsTests(unittest.TestCase):
    def test_within_repo_accepts_source_tree(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            nested = root / "qa" / "visual"
            nested.mkdir(parents=True)
            image = nested / "actual.png"
            image.write_bytes(b"png")
            self.assertTrue(within_repo(root, image.resolve()))

    def test_within_repo_rejects_outside_paths(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "repo"
            root.mkdir()
            outside = Path(tmp) / "outside.png"
            outside.write_bytes(b"png")
            self.assertFalse(within_repo(root, outside.resolve()))

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks are unavailable")
    def test_within_repo_accepts_symlinked_target_dir(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "repo"
            root.mkdir()
            cache = Path(tmp) / "shared-cache"
            captures = cache / "qa" / "visual"
            captures.mkdir(parents=True)
            image = captures / "mesh.png"
            image.write_bytes(b"png")
            link = root / "target"
            try:
                link.symlink_to(cache, target_is_directory=True)
            except OSError as error:
                self.skipTest(f"cannot create symlink in test environment: {error}")
            # Resolves outside the source tree but beneath the repo's own
            # target/ pointer, so it counts as logically in-repo.
            self.assertTrue(within_repo(root, (link / "qa" / "visual" / "mesh.png").resolve()))
            outside = Path(tmp) / "elsewhere.png"
            outside.write_bytes(b"png")
            self.assertFalse(within_repo(root, outside.resolve()))

    def test_display_path_keeps_source_tree_form(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            nested = root / "qa" / "visual"
            nested.mkdir(parents=True)
            image = nested / "actual.png"
            image.write_bytes(b"png")
            self.assertEqual(display_path(image, root), "qa/visual/actual.png")

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks are unavailable")
    def test_display_path_uses_logical_target_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "repo"
            root.mkdir()
            cache = Path(tmp) / "shared-cache"
            captures = cache / "qa"
            captures.mkdir(parents=True)
            image = captures / "mesh.png"
            image.write_bytes(b"png")
            link = root / "target"
            try:
                link.symlink_to(cache, target_is_directory=True)
            except OSError as error:
                self.skipTest(f"cannot create symlink in test environment: {error}")
            self.assertEqual(
                display_path(link / "qa" / "mesh.png", root), "target/qa/mesh.png"
            )


if __name__ == "__main__":
    unittest.main()
