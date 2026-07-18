import unittest

import qa_zed_source_check as chk


class ZedSourceCheckTests(unittest.TestCase):
    def test_detects_zed_git_source(self):
        meta = {"packages": [{"name": "gpui", "source": "git+https://github.com/zed-industries/zed.git?tag=v1.9.0#abc"}]}
        self.assertEqual(chk.find_zed_sources(meta), ["gpui"])

    def test_ignores_path_registry_and_other_zed_repos(self):
        meta = {"packages": [
            {"name": "gpui", "source": None},
            {"name": "serde", "source": "registry+https://github.com/rust-lang/crates.io-index"},
            {"name": "wgpu", "source": "git+https://github.com/zed-industries/wgpu.git?branch=v29#abc"},
        ]}
        self.assertEqual(chk.find_zed_sources(meta), [])


if __name__ == "__main__":
    unittest.main()
