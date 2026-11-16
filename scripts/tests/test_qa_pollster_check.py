import unittest

import qa_pollster_check as chk


class PollsterCheckTests(unittest.TestCase):
    def test_single_version_is_collected_once(self):
        meta = {
            "packages": [
                {"name": "pollster", "version": "1.0.1"},
                {"name": "pollster", "version": "1.0.1"},
                {"name": "rfd", "version": "0.17.2"},
            ]
        }
        self.assertEqual(chk.pollster_versions(meta), ["1.0.1"])

    def test_duplicate_versions_are_reported_sorted(self):
        meta = {
            "packages": [
                {"name": "pollster", "version": "1.0.1"},
                {"name": "pollster", "version": "0.4.0"},
            ]
        }
        self.assertEqual(chk.pollster_versions(meta), ["0.4.0", "1.0.1"])

    def test_no_pollster_is_an_empty_list(self):
        self.assertEqual(chk.pollster_versions({"packages": []}), [])


if __name__ == "__main__":
    unittest.main()
