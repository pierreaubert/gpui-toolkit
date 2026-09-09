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

    def test_tutorial_readme_links_resolve(self):
        readme = (TUTORIAL / "README.md").read_text()
        self.assertIn("demo_app.py", readme)
        for target in ("../examples/chart_gallery.py", "../showcase.py"):
            self.assertIn(target, readme)
            self.assertTrue((TUTORIAL / target).is_file())


if __name__ == "__main__":
    unittest.main()
