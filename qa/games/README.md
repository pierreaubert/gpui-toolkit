# Games delivery checks

The Python demo and native Rust showcase share Scene2D drawing and input APIs.
Mobile delivery uses native Rust apps, as agreed in Gitea issue #1. Simulator
and emulator captures verify packaging and rendering. Controller and platform
tests verify input behavior; these captures do not measure physical finger
input or input-to-present latency.

## Run the games

Desktop Python:

```sh
cargo run -p gpui-python-runtime --features showcase --bin gpui-python-host -- crates/gpui-python-runtime/python/examples/games_demo.py
```

Native showcase:

```sh
GPUI_SHOWCASE_SECTION=Games GPUI_GAMES_GAME=sudoku cargo run -p gpui-showcase --bin gpui-showcase
```

The game selector accepts `zip`, `queens`, `sudoku`, and `tetris`. On Android,
launch the showcase activity with Intent string extras `section=Games` and
`game=<name>`. For iOS simulator launches, use the corresponding
`SIMCTL_CHILD_GPUI_SHOWCASE_SECTION` and `SIMCTL_CHILD_GPUI_GAMES_GAME` variables.
Existing showcase iOS/XcodeGen and Android/Gradle projects package the apps.

## Reproduce the image matrix

The matrix covers six games, two themes, and four logical window sizes:
390×844, 844×390, 900×1280, and 1280×900. Native macOS Metal painting uses
CoreText and a 2× framebuffer. The harness resizes the layout viewport and
checks board bounds before capturing each case.

```sh
PYTHONPATH=crates/gpui-python-runtime/python python3 scripts/export_games_qa_ir.py --output qa/games/fixtures
QA_NATIVE_REQUIRED=1 GPUI_GAMES_QA_IR="$PWD/qa/games/fixtures" GPUI_GAMES_QA_DIR="$PWD/qa/games/matrix" cargo test -p gpui-python-runtime --features native-qa --bin gpui-python-host native_metal_game_matrix -- --nocapture --test-threads=1
```

`matrix/manifest.json` records logical and physical dimensions, retained node
counts, CPU draw measurements, and GPU-synchronized screenshot capture time.
Screenshot capture time is separate from CPU draw time. The manifest explicitly
marks input-to-present latency as unmeasured. Fixtures and PPM diagnostics are
generated locally and ignored; the fixture exporter is checked in.

The four-game baseline (32 cases) passed the bounds checks and visual review. With mobile builds
idle, warm whole-window draw CPU p95 ranged from 1.56 to 5.22 ms. The eight
Tetris cases exceeded the proposed 4 ms CPU target. These measurements include
the full Python showcase layout and are not a surface-only paint benchmark or
an input-to-present measurement.

## Verification

| Check | Result |
| --- | --- |
| Python suite | 438 tests run; 45 skipped |
| Rust runtime library | 200 tests passed |
| Shared Scene2D | 27 unit, 3 component, 2 integration tests passed |
| Native game controllers | 29 tests passed |
| Native Metal host | 7 tests passed; 32 game/theme/size captures |
| Native host transport/layout | 73 tests passed; image matrix checked separately |
| iOS accessibility bridge | 4 tests passed, including 2×/3× scale conversion |
| Android accessibility bridge | 3 Rust and 5 Java tests passed |
| Wasm closure | Checked with nightly build-std |
| Python surface gates | Registry, v2 classification, and rustdoc freshness passed |

Mobile captures use an iPhone 16e simulator running iOS 18.6 and an arm64
Android 16 / API 36 emulator. Android hierarchy checks verify cell counts,
positive board bounds, contained cell bounds, and activation
metadata. Capture JSON records the rendering and inspection scope. Manual
VoiceOver/TalkBack navigation, physical finger input, and present latency have
not been measured.

## Controller soak

```sh
GPUI_GAMES_SOAK_SECONDS=600 cargo test -p gpui-showcase native_controller_soak_when_requested -- --nocapture
```

The completed ten-minute headless run performed 10,522 game-switch cycles
across six retained surfaces, with a peak of 285 scene nodes. It checked contact
release, lifecycle handling, bounded undo history, unique IDs, revisions, and
node counts. The result is in `controller-soak.json`.

## Audio

The Python cue adapter has a bounded background queue and deduplicates cue IDs.
Its default backend is silent. A live sotf-daw playback binding is a separate
integration; no audio playback result is claimed here.
