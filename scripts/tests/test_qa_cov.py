import unittest
from pathlib import Path

import qa_cov_check


class CoverageReportTests(unittest.TestCase):
    def test_per_crate_coverage_aggregates_only_workspace_crates(self):
        root = qa_cov_check.ROOT
        summary = {
            "data": [
                {
                    "files": [
                        self.file(root / "crates" / "alpha" / "src" / "a.rs", 8, 10, 2, 4),
                        self.file(root / "crates" / "alpha" / "src" / "b.rs", 1, 2, 1, 1),
                        self.file(root / "crates" / "beta" / "src" / "lib.rs", 3, 3, 1, 2),
                        self.file(Path("/tmp/external.rs"), 100, 100, 10, 10),
                    ]
                }
            ]
        }

        rows = qa_cov_check.per_crate_coverage(summary)
        self.assertEqual([row["crate"] for row in rows], ["alpha", "beta"])
        self.assertEqual(rows[0]["lines_covered"], 9)
        self.assertEqual(rows[0]["lines_total"], 12)
        self.assertEqual(rows[0]["lines_percent"], 75.0)
        self.assertEqual(rows[0]["functions_covered"], 3)

    def test_per_crate_coverage_requires_data(self):
        with self.assertRaisesRegex(RuntimeError, "data.*empty"):
            qa_cov_check.per_crate_coverage({"data": []})

    @staticmethod
    def file(path, lines_covered, lines_total, functions_covered, functions_total):
        return {
            "filename": str(path),
            "summary": {
                "lines": {"covered": lines_covered, "count": lines_total},
                "functions": {"covered": functions_covered, "count": functions_total},
            },
        }


if __name__ == "__main__":
    unittest.main()
