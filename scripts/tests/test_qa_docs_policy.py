import unittest

import qa_docs_policy


class DocumentationPolicyTests(unittest.TestCase):
    def test_repository_documentation_policy(self) -> None:
        self.assertEqual(qa_docs_policy.check(), [])


if __name__ == "__main__":
    unittest.main()
