import pathlib
import tempfile
import unittest

import qa_wasm_clock


def write(root: pathlib.Path, relative: str, text: str) -> None:
    source = root / relative
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(text, encoding="utf-8")


class WasmClockTests(unittest.TestCase):
    def test_workspace_satisfies_policy(self) -> None:
        self.assertEqual(qa_wasm_clock.check(), [])

    def test_rejects_std_instant_in_wasm_crate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            write(
                root,
                "crates/gpui-audio-kit/src/spectrum.rs",
                "use std::time::Instant;\n"
                "pub fn now() -> Instant {\n"
                "    Instant::now()\n"
                "}\n",
            )

            errors = qa_wasm_clock.check(root)

            self.assertEqual(len(errors), 1)
            self.assertIn("crates/gpui-audio-kit/src/spectrum.rs:1", errors[0])
            self.assertIn("web_time::Instant", errors[0])

    def test_rejects_grouped_and_inline_system_time(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            write(
                root,
                "crates/gpui-px/src/clock.rs",
                "use std::time::{Duration, SystemTime};\n"
                "pub fn epoch() -> u64 {\n"
                "    std::time::SystemTime::now()\n"
                "        .duration_since(std::time::UNIX_EPOCH)\n"
                "        .map_or(0, |d| d.as_secs())\n"
                "}\n",
            )

            errors = qa_wasm_clock.check(root)

            self.assertEqual(len(errors), 3)
            self.assertTrue(all("web_time::" in error for error in errors))

    def test_allows_duration_and_web_time(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            write(
                root,
                "crates/gpui-ui-kit/src/timer.rs",
                "use std::time::Duration;\n"
                "use web_time::Instant;\n"
                "pub fn tick() -> Duration {\n"
                "    let _ = Instant::now();\n"
                "    Duration::from_millis(16)\n"
                "}\n",
            )

            self.assertEqual(qa_wasm_clock.check(root), [])

    def test_allows_native_platform_and_test_code(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            write(
                root,
                "crates/gpui-ios/src/clock.rs",
                "use std::time::Instant;\n",
            )
            write(
                root,
                "crates/gpui-px/tests/timing.rs",
                "use std::time::Instant;\n",
            )
            write(
                root,
                "crates/gpui-d3rs/examples/demo.rs",
                "fn main() { let _ = std::time::Instant::now(); }\n",
            )

            self.assertEqual(qa_wasm_clock.check(root), [])

    def test_allows_documentation_to_name_std_clocks(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            write(
                root,
                "crates/gpui-ui-kit/src/timer.rs",
                "// std::time::Instant panics on wasm; use web_time instead.\n"
                "pub fn tick() {}\n",
            )

            self.assertEqual(qa_wasm_clock.check(root), [])


if __name__ == "__main__":
    unittest.main()
