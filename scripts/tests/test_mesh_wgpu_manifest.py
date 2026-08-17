from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path

from mesh_wgpu_manifest import (
    CASE_IDS,
    WgpuManifestError,
    compare_manifests,
    validate_manifest,
    write_skip,
)


def captured_manifest(paths: list[str] | None = None) -> dict[str, object]:
    paths = paths or [f"{case_id}.png" for case_id in CASE_IDS]
    return {
        "schema_version": 1,
        "renderer": "wgpu-headless",
        "status": "captured",
        "width": 256,
        "height": 192,
        "cases": [
            {
                "id": case_id,
                "description": f"fixture {case_id}",
                "path": path,
                "opaque_pixels": index + 1,
                "rgba_checksum": f"fnv1a64:{index + 1:016x}",
            }
            for index, (case_id, path) in enumerate(zip(CASE_IDS, paths))
        ],
    }


class WgpuManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.manifest_path = self.root / "target/manifest.json"
        self.manifest_path.parent.mkdir(parents=True)
        self.manifest_path.write_text(json.dumps(captured_manifest()), encoding="utf-8")
        for case_id in CASE_IDS:
            (self.manifest_path.parent / f"{case_id}.png").write_bytes(b"png")

    def write_manifest(self, value: dict[str, object]) -> None:
        self.manifest_path.write_text(json.dumps(value), encoding="utf-8")

    def test_valid_manifest_requires_all_images_and_canonical_cases(self):
        manifest = validate_manifest(
            self.manifest_path,
            repo_root=self.root,
            require_images=True,
            allow_skipped=False,
        )
        self.assertEqual({case["id"] for case in manifest["cases"]}, set(CASE_IDS))

    def test_parent_absolute_windows_and_dot_paths_are_rejected(self):
        unsafe_paths = [
            "../outside.png",
            "/tmp/outside.png",
            "C:/outside.png",
            r"nested\\outside.png",
            ".",
        ]
        for unsafe_path in unsafe_paths:
            with self.subTest(unsafe_path=unsafe_path):
                value = captured_manifest()
                value["cases"][0]["path"] = unsafe_path
                self.write_manifest(value)
                with self.assertRaisesRegex(WgpuManifestError, "unsafe image path"):
                    validate_manifest(
                        self.manifest_path,
                        repo_root=self.root,
                        require_images=False,
                        allow_skipped=False,
                    )

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks are unavailable")
    def test_symlinked_image_cannot_escape_repository(self):
        outside = self.root.parent / "wgpu-manifest-outside.png"
        outside.write_bytes(b"outside")
        self.addCleanup(lambda: outside.unlink(missing_ok=True))
        link = self.manifest_path.parent / "mesh.png"
        try:
            link.symlink_to(outside)
        except OSError as error:
            self.skipTest(f"cannot create symlink in test environment: {error}")
        value = captured_manifest()
        value["cases"][0]["path"] = "mesh.png"
        self.write_manifest(value)
        with self.assertRaisesRegex(WgpuManifestError, "escapes the repository"):
            validate_manifest(
                self.manifest_path,
                repo_root=self.root,
                require_images=True,
                allow_skipped=False,
            )

    def test_malformed_case_and_checksum_are_rejected(self):
        value = captured_manifest()
        value["cases"][0] = {"id": "mesh", "path": "mesh.png"}
        self.write_manifest(value)
        with self.assertRaisesRegex(WgpuManifestError, "no description"):
            validate_manifest(
                self.manifest_path,
                repo_root=self.root,
                require_images=True,
                allow_skipped=False,
            )

        value = captured_manifest()
        value["cases"][0]["rgba_checksum"] = "sha256:bad"
        self.write_manifest(value)
        with self.assertRaisesRegex(WgpuManifestError, "invalid RGBA checksum"):
            validate_manifest(
                self.manifest_path,
                repo_root=self.root,
                require_images=True,
                allow_skipped=False,
            )

    def test_skip_manifest_requires_reason_and_is_disallowed_when_required(self):
        skip_path = self.root / "target/skip.json"
        write_skip(skip_path, "fixture has no adapter")
        manifest = validate_manifest(
            skip_path,
            repo_root=self.root,
            require_images=True,
            allow_skipped=True,
        )
        self.assertEqual(manifest["status"], "skipped")
        with self.assertRaisesRegex(WgpuManifestError, "is skipped"):
            validate_manifest(
                skip_path,
                repo_root=self.root,
                require_images=True,
                allow_skipped=False,
            )

        invalid = json.loads(skip_path.read_text(encoding="utf-8"))
        invalid["reason"] = ""
        self.write_manifest(invalid)
        with self.assertRaisesRegex(WgpuManifestError, "must include a reason"):
            validate_manifest(
                self.manifest_path,
                repo_root=self.root,
                require_images=True,
                allow_skipped=True,
            )

    def test_compare_manifests_catches_opaque_pixel_and_checksum_drift(self):
        actual = captured_manifest()
        baseline = captured_manifest()
        compare_manifests(actual, baseline)
        baseline["cases"][0]["opaque_pixels"] += 1
        with self.assertRaisesRegex(WgpuManifestError, "opaque_pixels"):
            compare_manifests(actual, baseline)


if __name__ == "__main__":
    unittest.main()
