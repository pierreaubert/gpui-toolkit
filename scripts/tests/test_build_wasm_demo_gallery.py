import json
import tempfile
import unittest
from pathlib import Path

import build_wasm_demo_gallery as gallery


class DemoGalleryTest(unittest.TestCase):
    def test_parse_urls_requires_app_assignments(self):
        self.assertEqual(gallery.parse_urls(["px=http://127.0.0.1:8082/"]), {"px": "http://127.0.0.1:8082"})
        with self.assertRaises(ValueError):
            gallery.parse_urls(["missing-url"])

    def test_entries_prefix_manifest_ids_and_preserve_viewports(self):
        catalog = {
            "schema_version": 1,
            "apps": [{"id": "px", "title": "Charts", "description": "Charts", "route": "px/"}],
        }
        manifests = {
            "px": {
                "captures": [
                    {
                        "id": "scatter-desktop",
                        "section": "scatter",
                        "section_label": "Scatter",
                        "group": "Charts",
                        "viewport_id": "desktop",
                        "viewport_label": "Desktop",
                        "width": 1200,
                        "height": 900,
                        "scale_factor": 1,
                        "renderer": "vello-auto",
                        "renderer_query": "auto",
                        "renderer_qa_queries": ["auto", "cpu", "legacy"],
                    }
                ]
            }
        }
        entries = gallery.entries_for(catalog, manifests)
        self.assertEqual(entries[0]["id"], "px-scatter-desktop")
        self.assertEqual(entries[0]["image"], "snapshots/px/desktop/scatter.png")
        self.assertIn("section=scatter", entries[0]["live_url"])
        self.assertIn("renderer=auto", entries[0]["live_url"])
        self.assertEqual(entries[0]["renderer"], "vello-auto")
        self.assertEqual(entries[0]["renderer_qa_queries"], ["auto", "cpu", "legacy"])

    def test_site_generation_emits_manifest_and_headers(self):
        catalog = {"schema_version": 1, "title": "Demos", "description": "Test", "featured": [], "apps": []}
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            gallery.write_site(catalog, [], output, [])
            self.assertTrue((output / "index.html").exists())
            self.assertEqual(json.loads((output / "manifest.json").read_text())["entries"], [])
            self.assertIn("Cross-Origin-Embedder-Policy", (output / "_headers").read_text())


if __name__ == "__main__":
    unittest.main()
