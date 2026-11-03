import pathlib
import tempfile
import unittest
from unittest import mock

import qa_docs_policy


class DocumentationPolicyTests(unittest.TestCase):
    def test_resolves_gpui_revision_when_block_uses_another_fork_revision(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "Cargo.lock").write_text('''
version = 4
[[package]]
name = "block"
version = "0.1.6"
source = "git+https://github.com/pierreaubert/sotf-3rdparties?rev=compat#1111111111111111111111111111111111111111"
[[package]]
name = "sotf-gpui-toolkit-gpui"
version = "0.2.3"
source = "git+https://github.com/pierreaubert/sotf-3rdparties#2222222222222222222222222222222222222222"
''')
            with mock.patch.object(qa_docs_policy, "ROOT", root):
                self.assertEqual(
                    qa_docs_policy._cargo_lock_third_parties_rev(),
                    "2222222222222222222222222222222222222222",
                )

    def test_repository_documentation_policy(self) -> None:
        self.assertEqual(qa_docs_policy.check(), [])


if __name__ == "__main__":
    unittest.main()
