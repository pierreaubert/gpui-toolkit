import unittest
from gpui_toolkit.scaffolder import ScaffoldOptions
class ScaffolderContractTests(unittest.TestCase):
 def test_non_destructive_request_is_serializable(self): self.assertTrue(ScaffoldOptions("demo", "/tmp").to_spec()["dry_run"])
 def test_name_cannot_escape_target_directory(self):
  for name in ("", "../demo", "a/b"):
   with self.assertRaises(ValueError): ScaffoldOptions(name, "/tmp")
if __name__ == "__main__": unittest.main()
