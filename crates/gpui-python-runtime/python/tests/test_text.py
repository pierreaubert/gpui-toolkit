import unittest
import contextlib
import io
import json

from gpui_toolkit import SessionContext
from gpui_toolkit.commands import CommandResult
from gpui_toolkit.text import (
    EngineProfile, KnuthPlassParams, LayoutCursor, PrepareProfile,
    TextBudget, TextPreparationRequest, VariableFontAxis, WhiteSpaceMode, analyze_rich_text,
    prepare_layout, prepared_layout_from_command, reports_from_command, rich_text_from_command,
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

    def test_native_rich_text_bidi_accessibility_and_variable_axes(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            analyze_rich_text(SessionContext(), "rich", "**hello** שלום", axes=(VariableFontAxis("wght", 100, 400, 900, 650),))
        self.assertEqual(json.loads(output.getvalue())["command"], "text.rich")
        analysis = rich_text_from_command(CommandResult.from_wire("rich", {"ok": True,
            "spans": [{"text": "hello", "style": {"bold": True, "italic": False, "code": False, "link": None}}],
            "accessibility_runs": [{"byte_start": 0, "byte_end": 5, "label": "hello", "role": "text"}],
            "bidi_levels": [0, 1], "axes": [{"tag": "wght", "min": 100, "default": 400, "max": 900, "value": 650}], "css_settings": '"wght" 650'}))
        self.assertTrue(analysis.spans[0].style.bold)
        self.assertEqual((analysis.bidi_levels, analysis.axes[0].value), ((0, 1), 650))

    def test_native_language_locale_and_benchmark_reports(self):
        reports = reports_from_command(CommandResult.from_wire("reports", {"ok": True,
            "language": {"schema_version": 1, "report_type": "language", "notes": [{"category": "bidi", "level": "supported", "summary": "levels", "recommendation": "test"}]},
            "locale": {"schema_version": 1, "report_type": "locale", "cases": [{"id": "rtl", "locale": "ar", "category": "bidi", "text": "مرحبا", "white_space": "normal", "max_width": 100, "line_height": 16, "expected_lines": ["مرحبا"], "note": "golden"}], "markdown": "# Locales"},
            "benchmark": {"schema_version": 1, "report_type": "benchmark", "criterion_command": "cargo bench", "baseline_policy": "required", "cases": [{"id": "latin", "benchmark_id": "layout", "focus": "throughput", "baseline_artifact": "base", "comparator_artifact": "compare", "release_requirement": "pass"}], "comparators": [{"id": "coretext", "platform": "macOS", "backend": "CoreText", "artifact": "report", "requirement": "compare"}], "locale_case_ids": ["rtl"], "markdown": "# Benchmarks"}}))
        self.assertEqual(reports.language.notes[0].level, "supported")
        self.assertEqual(reports.locale.cases[0].expected_lines, ("مرحبا",))
        self.assertEqual(reports.benchmark.comparators[0].backend, "CoreText")


if __name__ == "__main__":
    unittest.main()
