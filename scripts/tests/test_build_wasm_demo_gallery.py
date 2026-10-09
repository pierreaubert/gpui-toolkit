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
            gallery.write_site(catalog, [], output)
            self.assertTrue((output / "index.html").exists())
            self.assertEqual(json.loads((output / "manifest.json").read_text())["entries"], [])
            self.assertIn("Cross-Origin-Embedder-Policy", (output / "_headers").read_text())

    def test_live_links_carry_seeded_theme_and_style(self):
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
                    }
                ]
            }
        }
        first = gallery.entries_for(catalog, manifests)[0]
        second = gallery.entries_for(catalog, manifests)[0]
        self.assertEqual(first["live_url"], second["live_url"])
        self.assertIn("theme=", first["live_url"])
        self.assertIn("style=", first["live_url"])
        self.assertIn(first["live_theme"], gallery.THEME_VALUES)
        self.assertIn(first["live_style"], gallery.STYLE_VALUES)

    def test_featured_variants_cover_themes_and_styles(self):
        base = {
            "id": "showcase-buttons-desktop",
            "app_id": "showcase",
            "app_title": "UI Kit",
            "section": "buttons",
            "section_label": "Buttons",
            "viewport_id": "desktop",
            "width": 1200,
            "height": 900,
            "scale_factor": 1,
            "live_url": "showcase/?section=buttons&theme=dark",
        }
        catalog = {"featured": ["showcase-buttons-desktop"]}
        variants = gallery.featured_variants(catalog, [base])
        kinds = [variant["variant_kind"] for variant in variants]
        self.assertEqual(len(variants), len(gallery.THEME_VALUES) + len(gallery.STYLE_VALUES))
        self.assertEqual(kinds.count("theme"), len(gallery.THEME_VALUES))
        self.assertEqual(kinds.count("style"), len(gallery.STYLE_VALUES))
        for variant in variants:
            self.assertIn("capture_theme", variant)
            self.assertIn("capture_style", variant)
            self.assertIn(f"theme={variant['capture_theme']}", variant["live_url"])
            self.assertIn(f"style={variant['capture_style']}", variant["live_url"])
        self.assertEqual(gallery.featured_variants({"featured": []}, []), [])


if __name__ == "__main__":
    unittest.main()
