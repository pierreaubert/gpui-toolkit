import unittest

from gpui_toolkit.text import (
    EngineProfile, KnuthPlassParams, LayoutCursor, PrepareProfile,
    TextBudget, TextPreparationRequest, WhiteSpaceMode,
)


class TextDeclarationsTests(unittest.TestCase):
    def test_preparation_request_is_json_safe_and_host_oriented(self):
        request = TextPreparationRequest("one\ntwo", include_segments=True)
        spec = request.to_spec()
        self.assertEqual(spec["options"]["white_space"], WhiteSpaceMode.NORMAL.value)
        self.assertTrue(spec["include_segments"])

    def test_engine_and_algorithm_inputs_reject_invalid_values(self):
        with self.assertRaises(ValueError):
            EngineProfile(line_fit_epsilon=-0.1)
        with self.assertRaises(ValueError):
            KnuthPlassParams(tolerance=-1)
        with self.assertRaises(ValueError):
            TextBudget(max_segments=-1)

    def test_result_snapshots_validate_native_invariants(self):
        self.assertEqual(LayoutCursor(2, 3).segment_index, 2)
        self.assertEqual(PrepareProfile(3, 2, 1).breakable_segments, 1)
        with self.assertRaises(ValueError):
            LayoutCursor(-1, 0)


if __name__ == "__main__":
    unittest.main()
