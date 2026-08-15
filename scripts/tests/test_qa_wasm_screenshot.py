import unittest

from PIL import Image

import qa_wasm_screenshot as q


class DiffRatioTest(unittest.TestCase):
    def test_identical_images_have_zero_diff(self):
        img = Image.new("RGB", (8, 8), (30, 30, 46))
        self.assertEqual(q.diff_ratio(img, img), 0.0)

    def test_fully_different_images_have_diff_one(self):
        a = Image.new("RGB", (8, 8), (0, 0, 0))
        b = Image.new("RGB", (8, 8), (255, 255, 255))
        self.assertEqual(q.diff_ratio(a, b), 1.0)

    def test_size_mismatch_is_full_diff(self):
        a = Image.new("RGB", (8, 8))
        b = Image.new("RGB", (16, 16))
        self.assertEqual(q.diff_ratio(a, b), 1.0)


if __name__ == "__main__":
    unittest.main()
