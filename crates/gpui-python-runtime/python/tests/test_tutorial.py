import importlib.util
import json
from pathlib import Path
import sys
import unittest


TUTORIAL = Path(__file__).parents[1] / "tutorial"


def load_tutorial(name: str):
    path = TUTORIAL / name
    module_spec = importlib.util.spec_from_file_location(path.stem, path)
    assert module_spec is not None
    assert module_spec.loader is not None
    module = importlib.util.module_from_spec(module_spec)
    # dataclass subclasses resolve their defining module from sys.modules, so
    # register first like a real import (or `python demo_app.py`) would.
    sys.modules[path.stem] = module
    try:
        module_spec.loader.exec_module(module)
    finally:
        sys.modules.pop(path.stem, None)
    return module


def find_node(node, *, kind, node_id):
    """Recursively find a spec node by kind and id."""
    if isinstance(node, dict):
        if node.get("kind") == kind and node.get("id") == node_id:
            return node
        for value in node.values():
            found = find_node(value, kind=kind, node_id=node_id)
            if found is not None:
                return found
    elif isinstance(node, list):
        for item in node:
            found = find_node(item, kind=kind, node_id=node_id)
            if found is not None:
                return found
    return None


class StubContext:
    """Minimal SessionContext double capturing acknowledge/patch calls."""

    def __init__(self):
        self.acked = []
        self.patches = []

    def acknowledge(self, event):
        self.acked.append(event.id if hasattr(event, "id") else event)

    def patch(self, ops, *, request_id=None):
        self.patches.extend(ops)


class TutorialTests(unittest.TestCase):
    def test_demo_app_builds_two_sections(self):
        module = load_tutorial("demo_app.py")
        spec = module.build_app().to_spec()

        self.assertEqual(spec["title"], "GPUI Python Demo")
        self.assertEqual(
            [(section["id"], section["label"]) for section in spec["sections"]],
            [("overview", "Overview"), ("chart", "Chart")],
        )
        json.dumps(spec)

    def test_demo_app_chart_binds_demo_wave_dataset(self):
        module = load_tutorial("demo_app.py")
        app = module.build_app()
        spec = app.to_spec()

        chart = spec["sections"][1]["content"]["children"][1]
        self.assertEqual(chart["id"], "demo-line")
        self.assertEqual(
            chart["data"]["source"]["id"], "demo-wave",
        )
        self.assertIn("demo-wave", [resource.id for resource in app.resources])

    def test_demo_line_chart_reports_viewport_changes(self):
        module = load_tutorial("demo_app.py")
        spec = module.build_app().to_spec()

        chart = spec["sections"][1]["content"]["children"][1]
        self.assertEqual(chart["id"], "demo-line")
        self.assertEqual(chart["viewport_action"], "demo_viewport")

    def test_demo_bar_chart_reports_viewport_changes(self):
        module = load_tutorial("demo_app.py")
        spec = module.build_app().to_spec()

        chart = spec["sections"][1]["content"]["children"][2]
        self.assertEqual(chart["id"], "demo-peaks")
        self.assertEqual(chart["viewport_action"], "demo_viewport")

    def test_demo_app_button_action_targets_click_metric(self):
        module = load_tutorial("demo_app.py")
        spec = module.build_app().to_spec()

        overview_children = spec["sections"][0]["content"]["children"]
        button = next(
            node for node in overview_children if node.get("kind") == "button"
        )
        metric_ids = [
            node.get("id")
            for row in overview_children
            if row.get("kind") == "hstack"
            for node in row.get("children", [])
        ]
        self.assertEqual(button["action"], "demo_bump")
        self.assertIn("demo-clicks", metric_ids)

    def test_demo_app_runs_in_themed_miniapp_shell(self):
        module = load_tutorial("demo_app.py")
        spec = module.build_app().to_spec()

        self.assertEqual(
            (spec["title"], spec["width"], spec["height"]),
            ("GPUI Python Demo", 1280.0, 860.0),
        )
        self.assertTrue(spec["miniapp"]["with_theme"])
        self.assertEqual(spec["miniapp"]["initial_theme"], "dark")

    def test_demo_app_theme_picker_lists_supported_themes(self):
        module = load_tutorial("demo_app.py")
        spec = module.build_app().to_spec()

        picker = find_node(spec, kind="select", node_id="demo-appearance")
        self.assertEqual(picker["action"], "theme:demo_theme")
        self.assertEqual(picker["value"], "dark")
        self.assertEqual(
            [option["value"] for option in picker["options"]],
            list(module.THEME_OPTIONS),
        )

    def test_demo_bump_patches_clicks_and_goal_progress(self):
        module = load_tutorial("demo_app.py")
        app = module.build_app()
        context = StubContext()

        app.on_action(
            module.Event(
                id="e-bump",
                sequence=1,
                node_id="demo-button",
                event="click",
                action="demo_bump",
            ),
            context,
        )

        self.assertEqual(app.clicks, 1)
        self.assertEqual(context.acked, ["e-bump"])
        self.assertEqual(
            [(op["id"], op["property"]) for op in context.patches],
            [("demo-clicks", "value"), ("demo-progress", "value")],
        )
        self.assertEqual(context.patches[0]["value"], "1")
        self.assertAlmostEqual(
            context.patches[1]["value"], 1 / module.CLICK_GOAL
        )

    def test_demo_theme_action_patches_theme_metric(self):
        module = load_tutorial("demo_app.py")
        app = module.build_app()
        context = StubContext()

        app.on_action(
            module.Event(
                id="e-theme",
                sequence=2,
                node_id="demo-appearance",
                event="change",
                action="demo_theme",
                payload={"value": "forest"},
            ),
            context,
        )

        self.assertEqual(app.theme, "forest")
        self.assertEqual(context.acked, ["e-theme"])
        self.assertEqual(len(context.patches), 1)
        self.assertEqual(
            (context.patches[0]["id"], context.patches[0]["value"]),
            ("demo-theme", "Forest"),
        )

    def test_demo_theme_metric_value_is_a_string(self):
        # The host validates patched trees: metric values must be strings,
        # so the bump patch stringifies the click count.
        module = load_tutorial("demo_app.py")
        app = module.build_app()
        context = StubContext()

        app.on_action(
            module.Event(
                id="e-bump-str",
                sequence=4,
                node_id="demo-button",
                event="click",
                action="demo_bump",
            ),
            context,
        )

        self.assertIsInstance(context.patches[0]["value"], str)

    def test_demo_shell_theme_change_syncs_app_theme(self):
        module = load_tutorial("demo_app.py")
        app = module.build_app()
        context = StubContext()

        app.on_action(
            module.Event(
                id="e-shell-theme",
                sequence=5,
                node_id="miniapp",
                event="theme_changed",
                action="miniapp_theme_changed",
                payload={"theme": "Black & White"},
            ),
            context,
        )

        self.assertEqual(app.theme, "black_and_white")
        self.assertEqual(context.acked, ["e-shell-theme"])
        self.assertEqual(
            [(op["id"], op["value"]) for op in context.patches],
            [("demo-theme", "Black And White"), ("demo-appearance", "black_and_white")],
        )

    def test_demo_shell_theme_change_ignores_unknown_name(self):
        module = load_tutorial("demo_app.py")
        app = module.build_app()
        context = StubContext()

        app.on_action(
            module.Event(
                id="e-bad-shell-theme",
                sequence=6,
                node_id="miniapp",
                event="theme_changed",
                action="miniapp_theme_changed",
                payload={"theme": "Solarized"},
            ),
            context,
        )

        self.assertEqual(app.theme, "dark")
        self.assertEqual(context.acked, [])
        self.assertEqual(context.patches, [])

    def test_demo_theme_action_ignores_unknown_choice(self):
        module = load_tutorial("demo_app.py")
        app = module.build_app()
        context = StubContext()

        app.on_action(
            module.Event(
                id="e-bad-theme",
                sequence=3,
                node_id="demo-appearance",
                event="change",
                action="demo_theme",
                payload={"value": "solarized"},
            ),
            context,
        )

        self.assertEqual(app.theme, "dark")
        self.assertEqual(context.acked, [])
        self.assertEqual(context.patches, [])

    def test_tutorial_readme_links_resolve(self):
        readme = (TUTORIAL / "README.md").read_text()
        self.assertIn("demo_app.py", readme)
        for target in ("../examples/chart_gallery.py", "../showcase.py"):
            self.assertIn(target, readme)
            self.assertTrue((TUTORIAL / target).is_file())


if __name__ == "__main__":
    unittest.main()
