import runpy
from pathlib import Path
import unittest


def _walk(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from _walk(child)
    elif isinstance(value, (list, tuple)):
        for child in value:
            yield from _walk(child)


class ChartGalleryV2Tests(unittest.TestCase):
    def test_gallery_uses_only_resource_backed_px_charts(self) -> None:
        example = Path(__file__).parents[1] / "examples" / "chart_gallery.py"
        namespace = runpy.run_path(str(example))
        app = namespace["build_app"]()
        nodes = tuple(_walk(app.to_spec()))
        charts = tuple(node for node in nodes if node.get("kind") == "px_chart_v2")

        self.assertEqual(len(charts), 11)
        self.assertFalse(any(node.get("kind") == "chart" for node in nodes))
        self.assertEqual(len(app.resources), 7)
        self.assertTrue(any("dash" in chart["data"]["roles"] for chart in charts))
        self.assertTrue(any("y0" in chart["data"]["roles"] for chart in charts))
        for chart in charts:
            source = chart["data"]["source"]
            self.assertIn(source["kind"], {"dataset", "array_data"})
            self.assertNotIn("values", source)
            self.assertNotIn("rows", source)

    def test_spinorama_charts_use_v2_resources(self) -> None:
        example = Path(__file__).parents[1] / "examples" / "spinorama_demo.py"
        namespace = runpy.run_path(str(example))
        app = namespace["build_app"]()
        nodes = tuple(_walk(app.to_spec()))
        charts = tuple(node for node in nodes if node.get("kind") == "px_chart_v2")

        self.assertEqual(len(charts), 4)
        self.assertFalse(any(node.get("kind") == "chart" for node in nodes))
        self.assertEqual(len(app.resources), 5)
        surface = next(node for node in nodes if node.get("kind") == "surface")
        self.assertEqual(surface["z"]["kind"], "array_data")
        self.assertNotIn("values", surface["z"])
        for chart in charts:
            source = chart["data"]["source"]
            self.assertIn(source["kind"], {"dataset", "array_data"})
            self.assertNotIn("values", source)
            self.assertNotIn("rows", source)


if __name__ == "__main__":
    unittest.main()
