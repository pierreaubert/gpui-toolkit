import tempfile
import unittest
from pathlib import Path

from qa_embedded_assets import scan


class EmbeddedAssetQaTests(unittest.TestCase):
    def make_source(self, include: str, *, asset: bytes | None = b"fixture") -> tuple[Path, str]:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name).resolve()
        source = root / "crates" / "demo" / "src" / "lib.rs"
        source.parent.mkdir(parents=True)
        source.write_text(f'const DATA: &[u8] = include_bytes!("{include}");\n', encoding="utf-8")
        asset_path = (source.parent / include).resolve()
        if asset is not None:
            asset_path.parent.mkdir(parents=True, exist_ok=True)
            asset_path.write_bytes(asset)
        return root, asset_path.relative_to(root).as_posix()

    def test_tracked_literal_include_passes(self):
        root, relative = self.make_source("../assets/data.bin")
        count, errors = scan(root, {relative, "crates/demo/src/lib.rs"})
        self.assertEqual(count, 1)
        self.assertEqual(errors, [])

    def test_missing_literal_include_fails(self):
        root, relative = self.make_source("missing.bin", asset=None)
        count, errors = scan(root, {"crates/demo/src/lib.rs"})
        self.assertEqual(count, 1)
        self.assertEqual(len(errors), 1)
        self.assertIn(f"included asset is missing: {relative}", errors[0])

    def test_untracked_literal_include_fails(self):
        root, relative = self.make_source("data.bin")
        count, errors = scan(root, {"crates/demo/src/lib.rs"})
        self.assertEqual(count, 1)
        self.assertEqual(len(errors), 1)
        self.assertIn(f"included asset is not tracked by Git: {relative}", errors[0])


if __name__ == "__main__":
    unittest.main()
