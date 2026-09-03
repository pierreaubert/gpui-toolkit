import contextlib
import io
import json
import unittest

from gpui_toolkit import SessionContext
from gpui_toolkit import ui
from gpui_toolkit.commands import CommandResult


class UiBuilderTests(unittest.TestCase):
    def test_native_accessibility_focus_and_behavior_reports_are_typed(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            ui.request_reports(SessionContext(), "ui-reports")
        self.assertEqual(json.loads(output.getvalue())["command"], "ui.reports")

        report = {
            "schema_version": 1,
            "report_type": "test",
            "reviewed_on": "2026-08-07",
            "entry_count": 3,
            "all_release_ready": True,
            "markdown": "| component | status |",
        }
        reports = ui.reports_from_command(
            CommandResult.from_wire(
                "ui-reports",
                {"ok": True, "accessibility": report, "focus": report, "behavior": report},
            )
        )
        self.assertTrue(reports.accessibility.all_release_ready)
        self.assertEqual(reports.focus.entry_count, 3)
        self.assertIn("component", reports.behavior.markdown)

    def test_accordion_normalizes_children_and_preserves_item_ids(self):
        spec = ui.accordion(
            id="advanced",
            items=[("solver", "Solver", [ui.text("Tolerance")])],
            expanded=["solver"],
            action="set_advanced",
        ).to_spec()

        self.assertEqual(spec["kind"], "accordion")
        self.assertEqual(spec["items"][0]["id"], "solver")
        self.assertEqual(spec["items"][0]["children"][0]["kind"], "text")
        self.assertEqual(spec["action"], "set_advanced")

    def test_context_menu_uses_typed_items_and_semantic_actions(self):
        spec = ui.context_menu(
            id="run-menu",
            items=[ui.MenuItem("run", "Run", shortcut="cmd-r"), ui.MenuItem.divider()],
            position=(24, 36),
            action="select_run_action",
            close_action="close_run_menu",
        ).to_spec()

        self.assertEqual(spec["kind"], "context_menu")
        self.assertEqual(spec["items"][0]["shortcut"], "cmd-r")
        self.assertTrue(spec["items"][1]["separator"])
        self.assertEqual(spec["position"], [24.0, 36.0])

    def test_menu_and_menu_bar_keep_stable_selection_contracts(self):
        menu = ui.menu(
            id="actions", items=[ui.MenuItem("run", "Run")],
            focused_index=0, action="select_action", focus_action="focus_action",
        ).to_spec()
        bar = ui.menu_bar(
            id="application-menu",
            items=[ui.MenuBarItem("file", "File", [ui.MenuItem("quit", "Quit")])],
            active_menu="file", action="select_menu_item", toggle_action="toggle_menu",
        ).to_spec()

        self.assertEqual(menu["kind"], "menu")
        self.assertEqual(menu["items"][0]["id"], "run")
        self.assertEqual(bar["items"][0]["items"][0]["id"], "quit")
        self.assertEqual(bar["active_menu"], "file")

    def test_popover_retains_typed_trigger_and_content_slots(self):
        spec = ui.popover(
            ui.button("More", id="more"), id="more-popover",
            content=[ui.text("Details")], placement="bottom_end", width=240,
            close_action="close_more",
        ).to_spec()

        self.assertEqual(spec["kind"], "popover")
        self.assertEqual(spec["trigger"]["id"], "more")
        self.assertEqual(spec["content"][0]["kind"], "text")
        self.assertEqual(spec["placement"], "bottom_end")

    def test_confirmation_dialog_has_explicit_outcome_actions(self):
        spec = ui.confirm_dialog(
            id="delete-run", title="Delete run?", message="This cannot be undone.",
            variant="destructive", confirm_action="delete", cancel_action="keep",
        ).to_spec()

        self.assertEqual(spec["kind"], "confirm_dialog")
        self.assertEqual(spec["variant"], "destructive")
        self.assertEqual(spec["confirm_action"], "delete")

    def test_table_preserves_sorting_contract(self):
        spec = ui.table(
            id="runs",
            columns=[{"id": "frequency", "label": "Frequency", "sortable": True, "width": 140}],
            sort_action="sort_runs",
            sort_column="frequency",
            sort_direction="descending",
        ).to_spec()

        self.assertEqual(spec["sort_action"], "sort_runs")
        self.assertEqual(spec["sort_column"], "frequency")
        self.assertEqual(spec["sort_direction"], "descending")

    def test_action_button_carries_an_explicit_stable_id(self):
        spec = ui.button("Run", id="run-simulation", action="run-simulation").to_spec()
        self.assertEqual(spec["id"], "run-simulation")
        self.assertEqual(spec["action"], "run-simulation")

    def test_table_preserves_column_resize_action(self):
        spec = ui.table(
            id="runs",
            columns=[{"id": "frequency", "label": "Frequency", "width": 140}],
            resize_action="resize_column",
        ).to_spec()

        self.assertEqual(spec["resize_action"], "resize_column")

    def test_scene_selection_action_is_serialized(self):
        spec = ui.scene3d(
            {"id": "speaker", "kind": "mesh"}, id="speaker-view",
            selection_action="select_speaker",
        ).to_spec()
        self.assertEqual(spec["selection_action"], "select_speaker")

    def test_list_editor_preserves_stable_rows_and_actions(self):
        spec = ui.list_editor(
            id="frequencies",
            label="Evaluation frequencies",
            rows=[{"id": "f-100", "label": "100 Hz", "value": 100.0}],
            add_action="add_frequency",
            remove_action="remove_frequency",
            reorder_action="reorder_frequency",
        ).to_spec()

        self.assertEqual(spec["rows"][0]["id"], "f-100")
        self.assertEqual(spec["reorder_action"], "reorder_frequency")

    def test_form_exposes_validation_summary_references(self):
        spec = ui.form(
            id="simulation",
            children=[ui.number_input(id="frequency", value="", label="Frequency")],
            errors=[{"control_id": "frequency", "message": "Enter a frequency"}],
        ).to_spec()

        self.assertEqual(spec["kind"], "form")
        self.assertEqual(spec["errors"][0]["control_id"], "frequency")

    def test_heatmap_uses_dense_array_resource(self):
        from array import array
        from gpui_toolkit import data, px

        field = data.ArrayData.from_buffer(
            array("d", [1.0, 2.0, 3.0, 4.0]),
            shape=(2, 2),
            dtype="f64",
            id="ui-test-field",
        )
        spec = px.heatmap("field").data(field).to_spec()
        self.assertEqual(spec["data"]["source"]["shape"], [2, 2])
        self.assertNotIn("values", spec["data"]["source"])

    def test_stepper_preserves_active_and_disabled_steps(self):
        spec = ui.stepper(
            id="workflow", steps=["Model", "Solver", "Run"], active=1,
            disabled_steps=[2], action="set_step",
        ).to_spec()
        self.assertEqual(spec["active"], 1)
        self.assertEqual(spec["disabled_steps"], [2])

    def test_slider_preserves_preview_and_commit_actions(self):
        spec = ui.slider(
            id="gain",
            value=0.5,
            minimum=0,
            maximum=1,
            step=0.1,
            action="preview",
            commit_action="commit",
        ).to_spec()

        self.assertEqual(spec["kind"], "slider")
        self.assertEqual(spec["min"], 0.0)
        self.assertEqual(spec["max"], 1.0)
        self.assertEqual(spec["action"], "preview")
        self.assertEqual(spec["commit_action"], "commit")

    def test_text_selection_action_uses_positions_without_exposing_password_value(self):
        spec = ui.text_input(
            id="remote-token",
            value="must-not-be-serialized",
            password=True,
            selection_action="track-token-selection",
        ).to_spec()

        self.assertEqual(spec["selection_action"], "track-token-selection")
        self.assertEqual(spec["value"], "")

    def test_common_form_presentation_properties_are_serialized(self):
        spec = ui.number_input(
            id="frequency",
            value=100.0,
            help="Use a positive value.",
            default_value=20.0,
            visible=False,
            width=240.0,
        ).to_spec()

        self.assertEqual(spec["help"], "Use a positive value.")
        self.assertEqual(spec["default_value"], 20.0)
        self.assertFalse(spec["visible"])
        self.assertEqual(spec["width"], 240.0)

    def test_color_picker_uses_native_hex_contract(self):
        spec = ui.color_picker(id="accent", value="#ff00ffaa", label="Accent").to_spec()
        self.assertEqual(spec["kind"], "color_picker")
        self.assertEqual(spec["value"], "#ff00ffaa")

    def test_thinking_orb_exposes_native_animation_controls(self):
        spec = ui.thinking_orb(
            "working",
            id="status-orb",
            size=192.0,
            points_per_sphere=512.0,
            speed=0.25,
            dot_scale=4.0,
            dot_color="#60a5fa",
        ).to_spec()
        self.assertEqual(spec["kind"], "thinking_orb")
        self.assertEqual(spec["state"], "working")
        self.assertEqual(spec["size"], 192.0)
        self.assertEqual(spec["points_per_sphere"], 512.0)
        self.assertEqual(spec["speed"], 0.25)
        self.assertEqual(spec["dot_scale"], 4.0)
        self.assertEqual(spec["dot_color"], "#60a5fa")

    def test_navigation_and_feedback_nodes_preserve_native_event_contracts(self):
        crumbs = ui.breadcrumbs(
            id="location", items=[("home", "Home"), {"id": "run", "label": "Run"}],
            separator="chevron", action="navigate",
        ).to_spec()
        notice = ui.alert(
            "Model saved", id="saved", variant="success", closeable=True, action="dismiss",
        ).to_spec()

        self.assertEqual(crumbs["items"][1]["id"], "run")
        self.assertEqual(crumbs["separator"], "chevron")
        self.assertEqual(crumbs["action"], "navigate")
        self.assertTrue(notice["closeable"])
        self.assertEqual(notice["action"], "dismiss")

        toast = ui.toast("Queued", id="queue", duration_secs=3.0, action="dismiss_toast").to_spec()
        self.assertEqual(toast["duration_secs"], 3.0)
        self.assertEqual(toast["action"], "dismiss_toast")

        tip = ui.tooltip(ui.button("Help", id="help"), "Explain this", id="help-tip").to_spec()
        self.assertEqual(tip["child"]["kind"], "button")
        self.assertEqual(tip["delay_ms"], 200)

        empty = ui.empty_state("No runs", action=ui.button("Create", id="create")).to_spec()
        self.assertEqual(empty["action"]["kind"], "button")

        modal = ui.dialog(id="details", title="Details", content=[ui.text("Ready")], close_action="close").to_spec()
        self.assertEqual(modal["content"][0]["kind"], "text")
        self.assertEqual(modal["close_action"], "close")


if __name__ == "__main__":
    unittest.main()
