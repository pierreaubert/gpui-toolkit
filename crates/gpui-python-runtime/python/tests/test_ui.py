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

    def test_shell_and_navigation_keep_stable_contracts(self):
        shell = ui.app_shell(
            id="shell", sidebar_side="right",
            header=ui.top_nav(
                id="nav", brand="Demo", action="select_nav",
                items=[ui.TopNavItem("home", "Home", active=True)],
            ),
            content=ui.text("Body"),
            footer=ui.mobile_nav(
                id="tabs", selected="home", action="select_tab",
                items=[ui.MobileNavItem("home", "Home", icon="H")],
            ),
        ).to_spec()

        self.assertEqual(shell["kind"], "app_shell")
        self.assertEqual(shell["sidebar_side"], "right")
        self.assertEqual(shell["header"]["kind"], "top_nav")
        self.assertEqual(shell["header"]["items"][0]["id"], "home")
        self.assertTrue(shell["header"]["items"][0]["active"])
        self.assertEqual(shell["footer"]["kind"], "mobile_nav")
        self.assertEqual(shell["footer"]["selected"], "home")
        self.assertIsNone(shell["sidebar"])

    def test_date_range_input_keeps_bounds_contract(self):
        spec = ui.date_range_input(
            id="stay", start="2026-10-01", end="2026-10-07",
            clearable=True, action="change_range",
        ).to_spec()

        self.assertEqual(spec["kind"], "date_range_input")
        self.assertEqual(spec["start"], "2026-10-01")
        self.assertEqual(spec["end"], "2026-10-07")
        self.assertTrue(spec["clearable"])

    def test_time_input_keeps_value_contract(self):
        spec = ui.time_input(
            id="standup", value="09:30",
            clearable=True, action="change_time",
        ).to_spec()

        self.assertEqual(spec["kind"], "time_input")
        self.assertEqual(spec["value"], "09:30")
        self.assertTrue(spec["clearable"])

    def test_date_time_input_keeps_parts_contract(self):
        spec = ui.date_time_input(
            id="launch", date="2026-10-07", time="09:30",
            action="change_datetime",
        ).to_spec()

        self.assertEqual(spec["kind"], "date_time_input")
        self.assertEqual(spec["date"], "2026-10-07")
        self.assertEqual(spec["time"], "09:30")

    def test_chat_keeps_message_contract(self):
        spec = ui.chat(
            id="thread",
            messages=[
                ui.ChatMessage(body="Hi", author="Ada", role="user"),
                {"author": "Bot", "body": "Hello", "role": "assistant"},
            ],
        ).to_spec()

        self.assertEqual(spec["kind"], "chat")
        self.assertEqual(len(spec["messages"]), 2)
        self.assertEqual(spec["messages"][0]["role"], "user")
        self.assertEqual(spec["messages"][1]["body"], "Hello")

    def test_markdown_and_blockquote_keep_content_contract(self):
        md = ui.markdown("# Title", id="notes").to_spec()
        self.assertEqual(md["kind"], "markdown")
        self.assertEqual(md["source"], "# Title")

        quote = ui.blockquote("Ship it.", cite="Captain").to_spec()
        self.assertEqual(quote["kind"], "blockquote")
        self.assertEqual(quote["quote"], "Ship it.")
        self.assertEqual(quote["cite"], "Captain")

    def test_carousel_keeps_slide_contract(self):
        spec = ui.carousel(
            [
                ui.CarouselSlide(title="Fast", body="Starts fast."),
                {"title": "Portable", "body": "Runs anywhere."},
            ],
            index=1, id="tour", change_action="change_slide",
        ).to_spec()

        self.assertEqual(spec["kind"], "carousel")
        self.assertEqual(spec["index"], 1)
        self.assertEqual(spec["change_action"], "change_slide")
        self.assertEqual(len(spec["slides"]), 2)
        self.assertEqual(spec["slides"][0]["title"], "Fast")
        self.assertEqual(spec["slides"][1]["body"], "Runs anywhere.")

    def test_selectable_card_keeps_state_contract(self):
        spec = ui.selectable_card(
            "Pro", description="For teams.", selected=True,
            id="plan-pro", action="pick_plan",
        ).to_spec()

        self.assertEqual(spec["kind"], "selectable_card")
        self.assertEqual(spec["title"], "Pro")
        self.assertEqual(spec["description"], "For teams.")
        self.assertTrue(spec["selected"])
        self.assertFalse(spec["disabled"])
        self.assertEqual(spec["action"], "pick_plan")

    def test_citation_timestamp_and_metadata_keep_contract(self):
        quote = ui.citation(
            "Move slowly.", source="Handbook", variant="block",
        ).to_spec()
        self.assertEqual(quote["kind"], "citation")
        self.assertEqual(quote["text"], "Move slowly.")
        self.assertEqual(quote["source"], "Handbook")
        self.assertEqual(quote["variant"], "block")

        stamp = ui.timestamp("Edited 2 hours ago").to_spec()
        self.assertEqual(stamp["kind"], "timestamp")
        self.assertEqual(stamp["text"], "Edited 2 hours ago")

        spec = ui.metadata_list(
            [
                ui.MetadataEntry(label="Author", value="Ada"),
                {"label": "License", "value": "MIT"},
            ],
            id="file-meta",
        ).to_spec()
        self.assertEqual(spec["kind"], "metadata_list")
        self.assertEqual(len(spec["entries"]), 2)
        self.assertEqual(spec["entries"][0]["label"], "Author")
        self.assertEqual(spec["entries"][1]["value"], "MIT")

    def test_file_input_keeps_name_contract(self):
        spec = ui.file_input(
            id="upload", file_name="portrait.png", accept=".png,.jpg",
            action="browse_file",
        ).to_spec()

        self.assertEqual(spec["kind"], "file_input")
        self.assertEqual(spec["file_name"], "portrait.png")
        self.assertEqual(spec["accept"], ".png,.jpg")
        self.assertFalse(spec["disabled"])
        self.assertEqual(spec["action"], "browse_file")

    def test_tokenizer_keeps_tokens_contract(self):
        spec = ui.tokenizer(
            id="tags", tokens=["drums", "bass"], action="remove_token",
        ).to_spec()

        self.assertEqual(spec["kind"], "tokenizer")
        self.assertEqual(spec["tokens"], ["drums", "bass"])
        self.assertEqual(spec["action"], "remove_token")

    def test_field_wraps_child_with_labels(self):
        spec = ui.field(
            ui.tokenizer(id="gain-control", tokens=["-6 dB"]),
            id="gain-field", label="Gain", required=True,
            help="Applied before the limiter.",
        ).to_spec()

        self.assertEqual(spec["kind"], "field")
        self.assertEqual(spec["label"], "Gain")
        self.assertTrue(spec["required"])
        self.assertEqual(spec["help"], "Applied before the limiter.")
        self.assertIsNone(spec["error"])
        self.assertEqual(spec["child"]["kind"], "tokenizer")

    def test_field_status_and_skeleton_keep_contract(self):
        status = ui.field_status(
            "Gain is required.", variant="error",
        ).to_spec()
        self.assertEqual(status["kind"], "field_status")
        self.assertEqual(status["message"], "Gain is required.")
        self.assertEqual(status["variant"], "error")

        bones = ui.skeleton(
            variant="circular", size="lg", width=64.0,
        ).to_spec()
        self.assertEqual(bones["kind"], "skeleton")
        self.assertEqual(bones["variant"], "circular")
        self.assertEqual(bones["size"], "lg")
        self.assertEqual(bones["width"], 64.0)

    def test_lightbox_hover_card_and_layer_keep_contract(self):
        viewer = ui.lightbox(
            "assets/painting.png", caption="Gallery preview",
        ).to_spec()
        self.assertEqual(viewer["kind"], "lightbox")
        self.assertEqual(viewer["src"], "assets/painting.png")
        self.assertEqual(viewer["caption"], "Gallery preview")

        card = ui.hover_card(
            "Ada Lovelace", description="First programmer.",
            placement="right", size="lg",
        ).to_spec()
        self.assertEqual(card["kind"], "hover_card")
        self.assertEqual(card["title"], "Ada Lovelace")
        self.assertEqual(card["placement"], "right")
        self.assertEqual(card["size"], "lg")

        overlay = ui.layer(
            [ui.text("Session expired.")], id="notice",
        ).to_spec()
        self.assertEqual(overlay["kind"], "layer")
        self.assertEqual(overlay["children"][0]["kind"], "text")
        self.assertTrue(overlay["show_backdrop"])

    def test_calendar_pagination_and_list_keep_contract(self):
        month = ui.calendar(
            2026, 10, id="departure", selected="2026-10-07",
            select_action="pick-day",
        ).to_spec()
        self.assertEqual(month["kind"], "calendar")
        self.assertEqual(month["year"], 2026)
        self.assertEqual(month["month"], 10)
        self.assertEqual(month["selected"], "2026-10-07")

        pager = ui.pagination(
            5, 12, id="results", change_action="turn-page",
        ).to_spec()
        self.assertEqual(pager["kind"], "pagination")
        self.assertEqual(pager["page"], 5)
        self.assertEqual(pager["total_pages"], 12)
        self.assertEqual(pager["siblings"], 1)

        rows = ui.list_view(
            [ui.list_item("a", "Alpha", description="Primary"),
             ui.list_item("b", "Beta", disabled=True)],
            id="servers", selected="a", select_action="pick-row",
        ).to_spec()
        self.assertEqual(rows["kind"], "list")
        self.assertEqual(rows["items"][0]["label"], "Alpha")
        self.assertEqual(rows["items"][1]["disabled"], True)
        self.assertEqual(rows["selected"], "a")

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

    def test_layout_primitives_preserve_native_contracts(self):
        grid = ui.grid([ui.badge("Cell 1"), ui.badge("Cell 2")], columns=2, gap=8.0).to_spec()
        self.assertEqual(grid["kind"], "grid")
        self.assertEqual(grid["columns"], 2)
        self.assertEqual(grid["gap"], 8.0)
        self.assertEqual(grid["children"][1]["kind"], "badge")

        centered = ui.center([ui.text("Centered")], max_width=280.0).to_spec()
        self.assertEqual(centered["kind"], "center")
        self.assertEqual(centered["max_width"], 280.0)
        self.assertEqual(centered["children"][0]["kind"], "text")

        ratio = ui.aspect_ratio([ui.text("16 : 9")], ratio=16.0 / 9.0).to_spec()
        self.assertEqual(ratio["kind"], "aspect_ratio")
        self.assertAlmostEqual(ratio["ratio"], 16.0 / 9.0)
        self.assertEqual(ratio["children"][0]["kind"], "text")

        panel = ui.resizable(
            [ui.text("Drag my corner")], handle="corner", width=320.0, height=120.0,
            min_width=160.0, min_height=80.0,
        ).to_spec()
        self.assertEqual(panel["kind"], "resizable")
        self.assertEqual(panel["handle"], "corner")
        self.assertEqual(panel["width"], 320.0)
        self.assertEqual(panel["min_height"], 80.0)
        self.assertEqual(panel["children"][0]["kind"], "text")

        hidden = ui.visually_hidden(
            [ui.text("Skip to content")], focusable=True, label="Skip link",
        ).to_spec()
        self.assertEqual(hidden["kind"], "visually_hidden")
        self.assertTrue(hidden["focusable"])
        self.assertEqual(hidden["label"], "Skip link")
        self.assertEqual(hidden["children"][0]["kind"], "text")


if __name__ == "__main__":
    unittest.main()
