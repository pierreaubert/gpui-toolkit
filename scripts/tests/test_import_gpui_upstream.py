import tempfile
import unittest
from pathlib import Path

import import_gpui_upstream as imp

REF = "v0.0.1"

ZED_ROOT_TOML = """
[workspace]
members = ["crates/*"]

[workspace.package]
edition = "2024"
license = "Apache-2.0"
version = "0.1.0"

[workspace.dependencies]
internal_b = { path = "crates/internal_b", version = "0.1.0" }
serde = { version = "1.0", features = ["derive"] }
font-kit = { git = "https://github.com/zed-industries/font-kit", rev = "deadbeef", package = "zed-font-kit", version = "0.14.1-zed" }
"""

INTERNAL_A_TOML = """
[package]
name = "internal_a"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false

[lints]
workspace = true

[lib]
path = "src/internal_a.rs"
doctest = false

[dependencies]
internal_b.workspace = true
serde = { workspace = true, optional = true }
async-task = "4.7"
font-kit = { workspace = true, optional = true }

[dev-dependencies]
reqwest_client.workspace = true

[target.'cfg(target_os = "macos")'.dependencies]
internal_b = { workspace = true, features = ["extra"] }

[[example]]
name = "demo"
path = "examples/demo.rs"
"""

INTERNAL_B_TOML = """
[package]
name = "internal_b"
version = "0.1.0"
edition = "2024"

[features]
extra = []
"""


def make_fake_zed(root: Path) -> Path:
    """Create a minimal fake zed checkout and return its path."""
    (root / "Cargo.toml").write_text(ZED_ROOT_TOML)
    (root / "LICENSE-APACHE").write_text("Apache License\n")
    for name, body in (("internal_a", INTERNAL_A_TOML), ("internal_b", INTERNAL_B_TOML)):
        cdir = root / "crates" / name
        (cdir / "src").mkdir(parents=True)
        (cdir / "Cargo.toml").write_text(body)
        (cdir / "src" / f"{name}.rs").write_text("// stub\n")
        (cdir / "examples").mkdir()
        (cdir / "examples" / "demo.rs").write_text("fn main() {}\n")
    return root


class DepResolutionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        zdir = make_fake_zed(Path(self.tmp.name))
        self.ctx = imp.load_zed(zdir, REF)

    def tearDown(self):
        self.tmp.cleanup()

    def test_internal_dep_becomes_git_tag_form(self):
        name, spec = imp.resolve_dep("internal_b", {"workspace": True}, self.ctx)
        self.assertEqual(name, "internal_b")
        self.assertEqual(
            imp.dumps_toml({"dependencies": {"internal_b": spec}}),
            '[dependencies]\ninternal_b = { version = "0.1.0", git = "https://github.com/zed-industries/zed.git", tag = "v0.0.1" }\n',
        )

    def test_external_dep_merges_extras(self):
        name, spec = imp.resolve_dep("serde", {"workspace": True, "optional": True}, self.ctx)
        self.assertEqual(
            imp.dumps_toml({"dependencies": {"serde": spec}}),
            '[dependencies]\nserde = { version = "1.0", features = ["derive"], optional = true }\n',
        )

    def test_external_git_dep_keeps_package_rename(self):
        name, spec = imp.resolve_dep("font-kit", {"workspace": True, "optional": True}, self.ctx)
        self.assertEqual(name, "zed-font-kit")
        line = imp.dumps_toml({"dependencies": {"font-kit": spec}})
        self.assertIn('package = "zed-font-kit"', line)
        self.assertIn("https://github.com/zed-industries/font-kit", line)
        self.assertIn('rev = "deadbeef"', line)
        self.assertIn("optional = true", line)

    def test_excluded_dev_dep_is_dropped(self):
        name, spec = imp.resolve_dep("reqwest_client", {"workspace": True}, self.ctx)
        self.assertIsNone(name)
        self.assertIsNone(spec)

    def test_plain_string_dep_passes_through(self):
        name, spec = imp.resolve_dep("async-task", "4.7", self.ctx)
        self.assertEqual((name, spec), ("async-task", "4.7"))


class SerializerTests(unittest.TestCase):
    def test_cfg_section_header_is_single_quoted(self):
        doc = {"target": {'cfg(target_os = "macos")': {"dependencies": {"objc": "0.2"}}}}
        self.assertEqual(
            imp.dumps_toml(doc),
            '[target.\'cfg(target_os = "macos")\'.dependencies]\nobjc = "0.2"\n',
        )

    def test_scalar_types(self):
        doc = {"package": {"name": "x", "version": "1.0.0", "publish": False, "authors": ["a", "b"]}}
        out = imp.dumps_toml(doc)
        self.assertIn('name = "x"', out)
        self.assertIn("publish = false", out)
        self.assertIn('authors = ["a", "b"]', out)


if __name__ == "__main__":
    unittest.main()
