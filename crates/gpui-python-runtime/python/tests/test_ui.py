import unittest

from gpui_toolkit import ui


class UiBuilderTests(unittest.TestCase):
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

    def test_heatmap_preserves_missing_cells(self):
        from gpui_toolkit import charts

        spec = charts.heatmap("field", [1.0, None, 3.0, 4.0], 2, 2).to_spec()
        self.assertEqual(spec["z"], [1.0, None, 3.0, 4.0])

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


if __name__ == "__main__":
    unittest.main()
