import tempfile
import unittest
from pathlib import Path

import build_docs_site


def has_markdown() -> bool:
    try:
        from markdown import Markdown  # noqa: F401
    except ImportError:
        return False
    return True


@unittest.skipUnless(
    has_markdown(),
    "markdown package not installed (uv run --with markdown ...)",
)
class BuildDocsSiteTest(unittest.TestCase):
    def test_builds_all_pages_with_nav_and_content(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "docs"
            written = build_docs_site.build(output)
            self.assertEqual(
                sorted(path.relative_to(output).as_posix() for path in written),
                ["index.html", "python/index.html", "rust/index.html"],
            )
            self.assertTrue((output / "docs.css").is_file())

            python_page = (output / "python" / "index.html").read_text(encoding="utf-8")
            self.assertIn("<title>Python guide — GPUI Toolkit</title>", python_page)
            self.assertIn("pip install gpui-toolkit", python_page)
            # Sidebar marks the current page and links every page.
            self.assertIn('href="/docs/python/" aria-current="page"', python_page)
            self.assertIn('href="/docs/rust/"', python_page)
            # Fenced code rendered as a block, not left as raw markdown.
            self.assertIn("<pre>", python_page)
            self.assertNotIn("```", python_page)

            rust_page = (output / "rust" / "index.html").read_text(encoding="utf-8")
            self.assertIn("gpui-scaffolder", rust_page)

    def test_page_title_falls_back_to_label(self):
        self.assertEqual(build_docs_site.page_title("no heading\n", "Label"), "Label")
        self.assertEqual(build_docs_site.page_title("# Heading\nbody", "Label"), "Heading")


if __name__ == "__main__":
    unittest.main()
