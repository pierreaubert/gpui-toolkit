from __future__ import annotations

import json
import struct
import tempfile
import unittest
import zlib
from pathlib import Path

from mesh_plot_visual_compare import VisualCompareError, compare_manifests


def png(width: int, height: int, pixels: bytes) -> bytes:
    rows = b"".join(
        b"\x00" + pixels[row * width * 4 : (row + 1) * width * 4]
        for row in range(height)
    )

    def chunk(kind: bytes, data: bytes) -> bytes:
        payload = kind + data
        return struct.pack(">I", len(data)) + payload + struct.pack(">I", zlib.crc32(payload) & 0xFFFFFFFF)

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(rows))
        + chunk(b"IEND", b"")
    )


def manifest(case_ids: tuple[str, ...]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "renderer": "fixture-adapter",
        "status": "captured",
        "cases": [
            {"id": case_id, "path": f"{case_id}.png"} for case_id in case_ids
        ],
    }


def svg_manifest(case_ids: tuple[str, ...]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "renderer": "fixture-adapter",
        "status": "captured",
        "cases": [
            {
                "id": case_id,
                "artifact_kind": "svg",
                "path": f"{case_id}.svg",
            }
            for case_id in case_ids
        ],
    }


class MeshPlotVisualCompareTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        left_dir = self.root / "left"
        right_dir = self.root / "right"
        left_dir.mkdir()
        right_dir.mkdir()
        self.left = left_dir / "manifest.json"
        self.right = right_dir / "manifest.json"
        self.left.write_text(json.dumps(manifest(("mesh", "revolve"))), encoding="utf-8")
        self.right.write_text(json.dumps(manifest(("mesh", "revolve"))), encoding="utf-8")
        pixels = bytes([10, 20, 30, 255] * 4)
        for directory in (left_dir, right_dir):
            (directory / "mesh.png").write_bytes(png(2, 2, pixels))
            (directory / "revolve.png").write_bytes(png(2, 2, pixels))

    def test_exact_match_passes_and_reports_all_cases(self) -> None:
        report = compare_manifests(self.left, self.right, repo_root=self.root)
        self.assertTrue(report["passed"])
        self.assertEqual(report["artifact_kind"], "png")
        self.assertEqual(report["compared_count"], 2)
        self.assertEqual([case["id"] for case in report["cases"]], ["mesh", "revolve"])
        self.assertEqual(report["cases"][0]["artifact_kind"], "png")
        self.assertEqual(report["cases"][0]["changed_pixels"], 0)
        self.assertEqual(report["cases"][0]["left_path"], "left/mesh.png")
        self.assertEqual(report["cases"][0]["right_path"], "right/mesh.png")

    def test_tolerance_accepts_antialiasing_but_boundary_fails(self) -> None:
        changed = bytes([22, 20, 30, 255] * 4)
        (self.left.parent / "mesh.png").write_bytes(png(2, 2, changed))
        report = compare_manifests(
            self.left,
            self.right,
            repo_root=self.root,
            max_channel_delta=16,
            max_changed_fraction=0.0,
        )
        self.assertTrue(report["passed"])
        (self.left.parent / "mesh.png").write_bytes(png(2, 2, bytes([27, 20, 30, 255] * 4)))
        report = compare_manifests(
            self.left,
            self.right,
            repo_root=self.root,
            max_channel_delta=16,
            max_changed_fraction=0.0,
        )
        self.assertFalse(report["passed"])
        self.assertEqual(report["cases"][0]["changed_pixels"], 4)

    def test_case_sets_must_match(self) -> None:
        self.right.write_text(json.dumps(manifest(("mesh",))), encoding="utf-8")
        with self.assertRaisesRegex(VisualCompareError, "case IDs do not match"):
            compare_manifests(self.left, self.right, repo_root=self.root)

    def test_component_lab_case_schema_is_supported(self) -> None:
        self.right.write_text(
            json.dumps(
                {
                    "report_type": "gpui-component-lab-render-capture",
                    "renderer_id": "metal",
                    "passed": True,
                    "cases": [
                        {"capture_id": case_id, "actual_path": f"{case_id}.png"}
                        for case_id in ("mesh", "revolve")
                    ],
                }
            ),
            encoding="utf-8",
        )
        report = compare_manifests(self.left, self.right, repo_root=self.root)
        self.assertTrue(report["passed"])
        self.assertEqual(report["left_renderer"], "fixture-adapter")
        self.assertEqual(report["right_renderer"], "metal")

    def test_unsafe_artifact_path_is_rejected(self) -> None:
        value = manifest(("mesh", "revolve"))
        value["cases"][0]["path"] = "../outside.png"  # type: ignore[index]
        self.left.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaisesRegex(VisualCompareError, "unsafe artifact path"):
            compare_manifests(self.left, self.right, repo_root=self.root)

    def test_corrupt_png_is_rejected(self) -> None:
        (self.left.parent / "mesh.png").write_bytes(b"not a PNG")
        with self.assertRaisesRegex(VisualCompareError, "unsupported or oversized PNG"):
            compare_manifests(self.left, self.right, repo_root=self.root)

    def test_exact_svg_match_uses_explicit_kind_and_canonicalization(self) -> None:
        left = self.left.parent / "svg-manifest.json"
        right = self.right.parent / "svg-manifest.json"
        left.write_text(json.dumps(svg_manifest(("mesh",))), encoding="utf-8")
        right.write_text(json.dumps(svg_manifest(("mesh",))), encoding="utf-8")
        (left.parent / "mesh.svg").write_text(
            '<svg width="2" height="2"><g id="plot" class="mesh"><title>Mesh</title></g></svg>',
            encoding="utf-8",
        )
        (right.parent / "mesh.svg").write_text(
            '<?xml version="1.0"?>\n<svg height="2" width="2">\n'
            '  <g class="mesh" id="plot"> <title>Mesh</title> </g>\n</svg>',
            encoding="utf-8",
        )

        report = compare_manifests(left, right, repo_root=self.root)

        self.assertTrue(report["passed"])
        self.assertEqual(report["artifact_kind"], "svg")
        self.assertTrue(report["cases"][0]["canonical_equal"])

    def test_svg_content_mismatch_is_reported(self) -> None:
        left = self.left.parent / "svg-manifest.json"
        right = self.right.parent / "svg-manifest.json"
        left.write_text(json.dumps(svg_manifest(("mesh",))), encoding="utf-8")
        right.write_text(json.dumps(svg_manifest(("mesh",))), encoding="utf-8")
        (left.parent / "mesh.svg").write_text(
            '<svg><title>Mesh</title></svg>', encoding="utf-8"
        )
        (right.parent / "mesh.svg").write_text(
            '<svg><title>Other mesh</title></svg>', encoding="utf-8"
        )

        report = compare_manifests(left, right, repo_root=self.root)

        self.assertFalse(report["passed"])
        self.assertEqual(report["cases"][0]["error"], "canonical SVG artifacts differ")

    def test_mixed_artifact_kinds_are_rejected(self) -> None:
        value = manifest(("mesh", "revolve"))
        value["cases"][0]["artifact_kind"] = "svg"  # type: ignore[index]
        value["cases"][0]["path"] = "mesh.svg"  # type: ignore[index]
        self.left.write_text(json.dumps(value), encoding="utf-8")
        (self.left.parent / "mesh.svg").write_text("<svg />", encoding="utf-8")
        with self.assertRaisesRegex(VisualCompareError, "artifact kinds do not match"):
            compare_manifests(self.left, self.right, repo_root=self.root)

    def test_artifact_kind_must_match_path_extension(self) -> None:
        value = svg_manifest(("mesh",))
        value["cases"][0]["path"] = "mesh.png"  # type: ignore[index]
        self.left.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaisesRegex(VisualCompareError, "does not match path extension"):
            compare_manifests(self.left, self.right, repo_root=self.root)

    def test_missing_artifact_is_rejected(self) -> None:
        value = manifest(("mesh", "revolve"))
        value["cases"][0]["path"] = "missing.png"  # type: ignore[index]
        self.left.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaisesRegex(VisualCompareError, "missing visual capture"):
            compare_manifests(self.left, self.right, repo_root=self.root)

    def test_dimension_mismatch_is_a_failed_case(self) -> None:
        (self.left.parent / "mesh.png").write_bytes(png(1, 1, bytes([10, 20, 30, 255])))
        report = compare_manifests(self.left, self.right, repo_root=self.root)
        self.assertFalse(report["passed"])
        self.assertEqual(report["cases"][0]["error"], "capture dimensions do not match")


if __name__ == "__main__":
    unittest.main()
