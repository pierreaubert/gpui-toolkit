"""Unit tests for qa_showcase_native_wasm pure helpers (no browser)."""

import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "qa_showcase_native_wasm.py"
SPEC = importlib.util.spec_from_file_location("qa_showcase_native_wasm", SCRIPT)


def load():
    module = importlib.util.module_from_spec(SPEC)
    sys.modules[SPEC.name] = module
    SPEC.loader.exec_module(module)
    return module


class NativeWasmParityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mod = load()
        from PIL import Image

        cls.Image = Image

    def test_section_url(self):
        self.assertEqual(
            self.mod.section_url(8081, "form-controls", "dark", "neutral"),
            "http://127.0.0.1:8081/?section=form-controls&theme=dark&style=neutral",
        )

    def test_px_diff_identical_is_zero(self):
        a = self.Image.new("RGB", (64, 64), (30, 30, 32))
        self.assertEqual(self.mod.px_diff_ratio(a, a.copy()), 0.0)

    def test_px_diff_ignores_antialias_noise(self):
        a = self.Image.new("RGB", (64, 64), (30, 30, 30))
        b = self.Image.new("RGB", (64, 64), (45, 45, 45))
        self.assertEqual(self.mod.px_diff_ratio(a, b), 0.0)

    def test_px_diff_catches_recolor(self):
        a = self.Image.new("RGB", (64, 64), (30, 30, 30))
        b = self.Image.new("RGB", (64, 64), (200, 30, 30))
        self.assertGreater(self.mod.px_diff_ratio(a, b), 0.9)

    def test_edge_iou_identical_is_one(self):
        from PIL import ImageDraw

        a = self.Image.new("RGB", (64, 64), (0, 0, 0))
        ImageDraw.Draw(a).rectangle([10, 10, 30, 30], fill=(255, 255, 255))
        self.assertEqual(self.mod.edge_iou(a, a.copy()), 1.0)

    def test_edge_iou_tolerates_one_pixel_shift(self):
        from PIL import ImageDraw

        a = self.Image.new("RGB", (64, 64), (0, 0, 0))
        b = self.Image.new("RGB", (64, 64), (0, 0, 0))
        ImageDraw.Draw(a).rectangle([10, 10, 30, 30], fill=(255, 255, 255))
        ImageDraw.Draw(b).rectangle([11, 10, 31, 30], fill=(255, 255, 255))
        # Exact-pixel IoU punishes the shift; dilated IoU must recover most of it.
        exact = self.mod.edge_iou(a, b, dilate=0)
        tolerant = self.mod.edge_iou(a, b, dilate=2)
        self.assertLess(exact, 0.9)
        self.assertGreater(tolerant, 0.75)
        self.assertGreater(tolerant, exact)

    def test_edge_iou_catches_missing_shape(self):
        from PIL import ImageDraw

        a = self.Image.new("RGB", (64, 64), (0, 0, 0))
        b = self.Image.new("RGB", (64, 64), (0, 0, 0))
        ImageDraw.Draw(a).rectangle([10, 10, 30, 30], fill=(255, 255, 255))
        self.assertLess(self.mod.edge_iou(a, b), 0.5)

    def test_block_corr_identical_is_one(self):
        from PIL import ImageDraw

        a = self.Image.new("RGB", (64, 64), (20, 20, 20))
        ImageDraw.Draw(a).rectangle([0, 0, 32, 64], fill=(200, 200, 200))
        self.assertAlmostEqual(self.mod.block_corr(a, a.copy()), 1.0)

    def test_block_corr_catches_inversion(self):
        a = self.Image.new("RGB", (64, 64), (20, 20, 20))
        b = self.Image.new("RGB", (64, 64), (235, 235, 235))
        self.assertLess(abs(self.mod.block_corr(a, b)), 0.5)

    def test_gates_are_probabilities(self):
        for gate in (self.mod.PX_GATE, self.mod.EDGE_IOU_GATE, self.mod.CORR_GATE):
            self.assertGreater(gate, 0.0)
            self.assertLess(gate, 1.0)

    def test_policy_lists_carry_reasons(self):
        for table in (self.mod.DYNAMIC_CASES, self.mod.EXPECTED_CONSOLE_ERRORS):
            self.assertTrue(table)
            for case_id, reason in table.items():
                self.assertTrue(case_id.endswith("-desktop"), case_id)
                self.assertTrue(reason.strip(), case_id)


if __name__ == "__main__":
    unittest.main()
