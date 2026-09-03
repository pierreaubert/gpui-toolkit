"""Regression coverage for the Python-only gpui-d3rs showcase."""

from __future__ import annotations

import contextlib
import io
import json
import importlib
import unittest
from unittest.mock import Mock, patch

from gpui_toolkit import Event, SessionContext, data
from gpui_toolkit.commands import CommandResult

from d3_showcase import D3RS_SECTION_ORDER, D3rsShowcase, build_app


class D3rsShowcaseTests(unittest.TestCase):
    def test_installed_module_reexports_the_gallery_builder(self):
        installed = importlib.import_module("gpui_toolkit.d3_showcase")
        self.assertIs(installed.build_app, build_app)
        self.assertEqual(installed.D3RS_SECTION_ORDER, D3RS_SECTION_ORDER)

    def test_v2_d3rs_module_reexports_typed_requests(self):
        d3rs = importlib.import_module("gpui_toolkit.d3rs")
        legacy = importlib.import_module("gpui_toolkit.d3")
        self.assertIs(d3rs.ScaleRequest, legacy.ScaleRequest)
        self.assertIs(d3rs.mean, legacy.mean)
        self.assertIs(d3rs.ticks, legacy.ticks)
        native = importlib.import_module("gpui_toolkit.native")
        if native.AVAILABLE:
            self.assertEqual(d3rs.mean([1.0, 2.0, 3.0]), 2.0)
            self.assertEqual(d3rs.extent([3.0, 1.0, 2.0]), (1.0, 3.0))

    def test_ready_uses_binary_dataset_publication_when_arrow_is_available(self):
        context = Mock()
        app = build_app()
        with patch("gpui_toolkit.d3_showcase.find_spec", return_value=object()):
            app.on_session_ready(context)
        dataset_resources = [
            resource for resource in app.gallery_resources if isinstance(resource, data.Dataset)
        ]
        array_resources = [
            resource for resource in app.gallery_resources if isinstance(resource, data.ArrayData)
        ]
        self.assertEqual(context.bind_dataset.call_count, 1 + len(dataset_resources))
        self.assertEqual(context.bind_array.call_count, len(array_resources))
        context.bind_resource.assert_not_called()
        self.assertEqual(context.bind_dataset.call_args_list[0].args[0].id, "d3rs-events")

    def test_gallery_matches_native_navigation_and_contains_public_chart_specs(self):
        app = build_app()
        self.assertEqual([item.id for item in app.sections], [item[0] for item in D3RS_SECTION_ORDER])
        spec = app.to_spec()
        self.assertEqual(spec["title"], "gpui-d3rs Python Showcase")
        charts = [
            child
            for item in spec["sections"][1:]
            for child in item["content"]["children"]
            if child["kind"] == "px_chart_v2"
        ]
        self.assertEqual(len(charts), len(D3RS_SECTION_ORDER) - 1)
        self.assertTrue(
            all(chart["data"]["source"]["kind"] in {"dataset", "array_data"} for chart in charts)
        )
        self.assertNotIn('"values"', json.dumps(charts))
        overview_chart = next(
            child
            for child in spec["sections"][0]["content"]["children"]
            if child["kind"] == "px_chart_v2"
        )
        overview_table = next(
            child
            for child in spec["sections"][0]["content"]["children"]
            if child["kind"] == "table_v2"
        )
        self.assertEqual(overview_table["data"]["key"], "event_id")
        self.assertEqual(overview_table["selection_action"], "d3rs-row-selected")
        self.assertEqual(
            overview_chart["data"]["source"]["operations"],
            [
                {
                    "op": "filter",
                    "expression": {
                        "op": "and",
                        "args": [
                            {"op": "field", "args": ["enabled"]},
                            {
                                "op": "gt",
                                "args": [
                                    {"op": "field", "args": ["spl"]},
                                    0.0,
                                ],
                            },
                        ],
                    },
                },
                {"op": "range", "start": 0, "stop": 2},
            ],
        )

    def test_ready_and_recompute_use_typed_d3_commands_without_showcase_host_api(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            D3rsShowcase().on_session_ready(SessionContext())
        commands = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(commands[0]["type"], "resource_descriptor")
        self.assertEqual(commands[0]["resource_id"], "d3rs-events")
        self.assertNotIn("63.0", output.getvalue())
        self.assertEqual([message["command"] for message in commands[1:]], ["d3.scale", "d3.algorithms", "d3.zoom", "d3.modules", "d3.reports"])

        output = io.StringIO()
        event = Event("recompute", 1, "d3-line-recompute", "click", "recompute-d3", {})
        with contextlib.redirect_stdout(output):
            D3rsShowcase().on_action(event, SessionContext())
        messages = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(messages[0]["type"], "acknowledged")
        self.assertEqual(messages[1]["command"], "d3.algorithms")
        self.assertEqual(messages[1]["arguments"]["operation"], "lod_m4")

    def test_selection_action_updates_a_python_declared_status_node(self):
        output = io.StringIO()
        selection = Event(
            "selection",
            2,
            "d3rs-events-table",
            "selection_change",
            "d3rs-row-selected",
            {"keys": ["3"]},
        )
        with contextlib.redirect_stdout(output):
            D3rsShowcase().on_action(selection, SessionContext())
        messages = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(messages[0]["type"], "acknowledged")
        self.assertEqual(messages[1]["ops"][0]["value"], "selected: 3")

    def test_command_results_update_a_python_declared_status_node(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            D3rsShowcase().on_command_result("scale", CommandResult.from_wire("scale", {"ok": True}), SessionContext())
        patch = json.loads(output.getvalue())
        self.assertEqual(patch["ops"][0], {"op": "set", "id": "d3rs-command-status", "property": "value", "value": "ready"})
