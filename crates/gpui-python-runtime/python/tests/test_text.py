import unittest
import contextlib
import io
import json

from gpui_toolkit import SessionContext
from gpui_toolkit.commands import CommandResult
from gpui_toolkit.text import (
    EngineProfile, KnuthPlassParams, LayoutCursor, PrepareProfile,
    TextBudget, TextPreparationRequest, WhiteSpaceMode, prepare_layout, prepared_layout_from_command,
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

    def test_native_prepare_layout_command_and_result_are_typed(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            prepare_layout(SessionContext(), "layout", "one two", max_width=24, char_width=8)
        self.assertEqual(json.loads(output.getvalue())["command"], "text.prepare_layout")
        layout = prepared_layout_from_command(CommandResult.from_wire("layout", {
            "ok": True, "line_count": 2, "height": 32.0, "segments": ["one", " ", "two"],
            "lines": [{"text": "one", "width": 24.0,
                "start": {"segment_index": 0, "grapheme_index": 0},
                "end": {"segment_index": 1, "grapheme_index": 0}}],
        }))
        self.assertEqual(layout.result.line_count, 2)
        self.assertEqual(layout.lines[0][0], "one")


if __name__ == "__main__":
    unittest.main()
