"""Unit tests for qa_wasm_renderer_parity pure helpers (no browser)."""

import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "qa_wasm_renderer_parity.py"
SPEC = importlib.util.spec_from_file_location("qa_wasm_renderer_parity", SCRIPT)


def load():
    module = importlib.util.module_from_spec(SPEC)
    sys.modules[SPEC.name] = module
    SPEC.loader.exec_module(module)
    return module


class RendererParityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mod = load()
        from PIL import Image

        cls.Image = Image

    def test_with_renderer_appends(self):
        self.assertEqual(
            self.mod.with_renderer("http://127.0.0.1:8082", "cpu"),
            "http://127.0.0.1:8082?renderer=cpu",
        )

    def test_with_renderer_preserves_and_replaces(self):
        self.assertEqual(
            self.mod.with_renderer("http://h:1/?section=s&renderer=auto", "cpu"),
            "http://h:1/?section=s&renderer=cpu",
        )

    def test_normalize_pair_common_size(self):
        big = self.Image.new("RGB", (200, 100), (10, 20, 30))
        small = self.Image.new("RGB", (100, 50), (10, 20, 30))
        a, b = self.mod.normalize_pair(big, small)
        self.assertEqual(a.size, (100, 50))
        self.assertEqual(b.size, (100, 50))

    def test_edge_diff_ignores_flat_recolor(self):
        a = self.Image.new("RGB", (64, 64), (200, 30, 30))
        b = self.Image.new("RGB", (64, 64), (30, 30, 200))
        self.assertEqual(self.mod.edge_diff_ratio(a, b), 0.0)

    def test_edge_diff_catches_geometry(self):
        from PIL import ImageDraw

        a = self.Image.new("RGB", (64, 64), (0, 0, 0))
        b = self.Image.new("RGB", (64, 64), (0, 0, 0))
        ImageDraw.Draw(b).rectangle([10, 10, 20, 20], fill=(255, 255, 255))
        self.assertGreater(self.mod.edge_diff_ratio(a, b), 0.0)

    def test_changed_bbox_none_when_identical(self):
        a = self.Image.new("RGB", (32, 32), (1, 2, 3))
        self.assertIsNone(self.mod.changed_bbox(a, a.copy()))


if __name__ == "__main__":
    unittest.main()
