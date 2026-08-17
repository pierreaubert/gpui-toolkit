import json
import struct
import tempfile
import unittest
import zlib
from pathlib import Path

from qa_mesh_cvd import build_report
from qa_release_evidence import validate_mesh_plot_cvd


def png(width: int, height: int, rgba: bytes) -> bytes:
    def chunk(kind: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + kind
            + data
            + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
        )

    raw = b"".join(b"\x00" + rgba * width for _ in range(height))
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


class MeshPlotCvdTests(unittest.TestCase):
    def test_rendered_screen_covers_all_deficiencies_and_selection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "target/qa/visual/mesh-plot-product"
            for renderer in ("metal", "wgpu"):
                (output / renderer).mkdir(parents=True)
                (output / renderer / "plain.png").write_bytes(
                    png(1200, 800, bytes((20, 40, 80, 255)))
                )
                (output / renderer / "selected.png").write_bytes(
                    png(1200, 800, bytes((220, 180, 40, 255)))
                )
            cases = []
            for renderer in ("metal", "wgpu"):
                for state in ("plain", "selected"):
                    cases.append(
                        {
                            "id": f"{renderer}-{state}",
                            "renderer": f"{renderer}-headless",
                            "state": state,
                            "path": f"{renderer}/{state}.png",
                        }
                    )
            manifest = output / "manifest.json"
            manifest.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "report_type": "gpui-mesh-plot-product-capture",
                        "status": "captured",
                        "source_revision": "a" * 40,
                        "source_dirty": False,
                        "cases": cases,
                    }
                ),
                encoding="utf-8",
            )
            report = build_report(root, manifest)
            path = root / "target/qa/visual/mesh-plot-cvd.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(report), encoding="utf-8")
            validate_mesh_plot_cvd(
                root,
                require_capture=True,
                source_revision="a" * 40,
            )
            self.assertEqual(set(report["deficiencies"]), {"protan", "deutan", "tritan"})
            for metrics in report["deficiencies"].values():
                self.assertGreater(metrics["selection_changed_pixels"]["metal"], 0)
                self.assertGreater(metrics["selection_changed_pixels"]["wgpu"], 0)


if __name__ == "__main__":
    unittest.main()
