import pathlib
import tempfile
import unittest

import qa_unsafe_policy


class UnsafePolicyTests(unittest.TestCase):
    def test_workspace_satisfies_policy(self) -> None:
        self.assertEqual(qa_unsafe_policy.check(), [])

    def test_rejects_unsafe_in_portable_first_party_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "crates/gpui-core/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "pub fn dereference(ptr: *const u8) -> u8 {\n"
                "    unsafe { *ptr }\n"
                "}\n",
                encoding="utf-8",
            )

            errors = qa_unsafe_policy.check(root)

            self.assertEqual(len(errors), 1)
            self.assertIn("crates/gpui-core/src/lib.rs:2", errors[0])

    def test_rejects_direct_ffi_in_python_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "crates/gpui-python-runtime/src/keychain.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                'unsafe extern "C" { fn SecItemAdd(); }\n',
                encoding="utf-8",
            )

            errors = qa_unsafe_policy.check(root)

            self.assertEqual(len(errors), 1)
            self.assertIn(
                "crates/gpui-python-runtime/src/keychain.rs:1", errors[0]
            )

    def test_allows_native_ffi_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "crates/gpui-ios/src/ffi.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                'unsafe extern "C" { fn native_callback(); }\n',
                encoding="utf-8",
            )

            self.assertEqual(qa_unsafe_policy.check(root), [])

    def test_allows_generated_ffi_template_in_safe_scaffolder(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "crates/gpui-scaffolder/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                '#![forbid(unsafe_code)]\nconst TEMPLATE: &str = '
                '"#[unsafe(no_mangle)]";\n',
                encoding="utf-8",
            )

            self.assertEqual(qa_unsafe_policy.check(root), [])

    def test_similarly_named_crate_is_not_a_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "crates/gpui-ios-helper/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("unsafe fn helper() {}\n", encoding="utf-8")

            errors = qa_unsafe_policy.check(root)

            self.assertEqual(len(errors), 1)

    def test_allows_documentation_to_discuss_unsafe_code(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "crates/gpui-core/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "//! Portable first-party code does not use unsafe Rust.\n",
                encoding="utf-8",
            )

            self.assertEqual(qa_unsafe_policy.check(root), [])


if __name__ == "__main__":
    unittest.main()
