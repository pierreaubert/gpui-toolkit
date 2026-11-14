# Mobile Python — Phase 1 interpreter spike (gate record)

Issue: [#10 — Python toolkit: generate iOS and Android mobile apps](http://192.168.1.32:3001/pierre/gpui-toolkit/issues/10)
Branch/worktree: `mobile` at `/Volumes/home_worktrees/mobile`, based on `main` `3d76f6ee`.
Date: 2026-11-14.

## Decision

**Embed CPython 3.15.2 (official python.org mobile builds). Maturin stays the
desktop wheel builder only; it is not an interpreter and plays no on-device
role. RustPython is rejected for v1 (see scoring).**

Pinned artifacts (downloaded to `/Volumes/home_tmp/tmp/mobile-plan/cpython/`,
not committed):

- `python-3.15.2-iOS-XCframework.tar.gz` (82M, sha256
  `0f9fc7f61ceb8278a69fc7ec6089aad81789a96fa42f31cfe6d2f5f860d20a62`)
- `python-3.15.2-aarch64-linux-android.tar.gz` (23M, sha256
  `cfd4f0fdaff465ba62ebf3c3df2c8c410d0802fef3f98897082460df704cfef6`)

3.15.2 matches the host interpreter and both builds ship headers, `libpython`,
and `_sysconfigdata` (what PyO3 cross-compilation needs). `_native` is
`abi3-py310`, so the floor stays compatible.

## Evidence (all run 2026-11-14 in this worktree)

1. **In-process embedding works.** A throwaway PyO3 `auto-initialize` binary
   (`/Volumes/home_tmp/tmp/mobile-plan/spike-embed`, not committed) imported
   `gpui_toolkit`, reported `native.AVAILABLE = True`, built the tutorial
   `DemoApp` via `build_app()`, and produced a 4073-byte IR with 2 sections —
   no subprocess, no stdio.
2. **iOS cross-compile passes for the full stack.**
   `cargo check -p gpui-python-runtime --features python-extension --target aarch64-apple-ios-sim`
   with `PYO3_CROSS=1 PYO3_CROSS_PYTHON_VERSION=3.15
   PYO3_CROSS_LIB_DIR=<xcframework>/ios-arm64_x86_64-simulator/lib-arm64`
   finished clean (one pre-existing unused-`tempfile` warning). This includes
   `gpui-px`, `gpui-ui-kit`, `gpui-miniapp`, and `gpui-ios`.
3. **Android: PyO3 cross passes, `gpui-android` is broken on clean `main`.**
   The spike binary cross-checks for `aarch64-linux-android`. The full
   `gpui-python-runtime` check fails with 16 errors in `gpui-android`
   (GPUI trait drift: `refresh_interval`, `restart` arity, `on_quit`,
   `RequestFrameOptions.signal_source`, …). The identical 16 errors reproduce
   with `cargo check -p gpui-showcase-android --target aarch64-linux-android`
   and no Python involved — pre-existing breakage, not a Python problem.
   **Phase 3 therefore starts with a `gpui-android` repair work unit.**
4. **Session seam is small.** TokenSave + source inspection: the child
   process lives only in `bin/showcase/python.rs`
   (`spawn_python_session_for_script`, `PythonSession`). Python-side I/O is
   four touch points in `gpui_toolkit/app.py` (`SessionContext` writers to
   `sys.stdout.buffer`, `_read_message`/`_messages` reading `sys.stdin`).
   The `session.rs` message types and parser are transport-agnostic.
5. **Stdlib audit of `gpui_toolkit/*.py`:** `asyncio`, `concurrent`,
   `dataclasses`, `hashlib`, `importlib`, `inspect`, `json`, `re`, `secrets`,
   `shutil`, `struct`, `subprocess`, `tempfile`, `threading`, `uuid`, … plus
   optional `numpy`/`pyarrow` imports that must remain lazy/optional on
   mobile (native wheels are out of scope by settled decision).

## RustPython scoring (rejected)

- Runtime test: **not run** — no binary on this machine and it is not
  installable as a PyPI tool (`uvx rustpython` unsatisfiable); building from
  source exceeded the spike timebox.
- `_native.abi3.so` is a CPython-ABI extension (PyO3 `extension-module`,
  Mach-O arm64 confirmed). Under RustPython every native function would need
  the pure-Python fallback path (`native.AVAILABLE = False`) or a
  re-implementation as RustPython modules — a second binding surface to keep
  in sync with the rustdoc-inventory gate.
- The stdlib surface above (`asyncio` + `ThreadPoolExecutor` reducer lane in
  `app.py`) is exactly where an alternative interpreter is riskiest, with no
  offsetting win: embedded CPython already passed items 1–2 above.

Revisit only if CPython embedding hits a store-review or size wall that
RustPython demonstrably avoids.

## Size notes (for Phase 3 trimming)

- iOS XCframework extracted: 321M total across slices; shared stdlib occupies
  most of it. Per-app bundles use one slice plus a trimmed stdlib (drop
  `test*`, `__pycache__`, unused modules).
- Android prefix: 85M `lib/` (66M stdlib). Same trimming applies; testbed
  pattern copies stdlib + app `.py` into `assets/python` and `libpython*.so`
  into `jniLibs`.

## Phase 2 starting points

- Rust: add an in-process `PythonSession` backend next to the subprocess one
  in `bin/showcase/python.rs`, feeding the same `parse_python_message`
  parser from channel-backed pipes; desktop default unchanged.
- Python: parametrize `App.serve()` I/O (reader/writer) or swap
  `sys.stdin`/`sys.stdout` before `serve()`; replace mmap-file resource
  frames with in-memory frames on mobile, keeping checksum/ack/backpressure.
- Audit `app.py` mobile-hostile calls (`os.execv`, `subprocess`, host-binary
  lookup) behind the in-process entry point.
- Reuse the existing MiniApp shell and `PythonIrShowcase` view for the mobile
  hosts (no new UI surface).

## Phase 2 — in-process session transport (implemented 2026-11-14)

The session-v1 wire protocol is unchanged; only the byte transport differs.

**Python (`gpui_toolkit/app.py`)**

- `App.serve(input_stream=..., output_stream=...)` accepts host-owned text or
  binary streams. With no streams it keeps the exact desktop behavior,
  including resolving `sys.stdin`/`sys.stdout` at call time and calling the
  private one-argument `_messages(context)` form existing tests substitute.
- `SessionContext(output=...)` routes JSON messages and all binary frames
  (`resource_frame`, `mesh_frame`, `dataset_frame`) to the injected stream.
  With no output it still prints to stdout and writes frames to
  `sys.stdout.buffer`.
- `App.run()` with `GPUI_TOOLKIT_EMBEDDED_SESSION=1` serves instead of trying
  to locate and `exec` a host executable.
- Embedded hosts do **not** negotiate `resource_mmap_frames`; dataset/array
  publication then selects the existing bounded `dataset_frame` fallback over
  the pipe, preserving checksums, acknowledgements, and backpressure with no
  resource files.

**Rust (`gpui-python-runtime`, feature `embedded`, Unix only)**

- `embedded::spawn_embedded_session(script, python_paths)` creates two OS
  pipes, runs embedded CPython on a dedicated thread, loads the app module
  under a unique `sys.modules` name, resolves `build_app()` or `app`, and calls
  `serve` with buffered pipe streams.
- `EmbeddedSession::send` writes typed `HostMessage` values; `reader` yields
  the same lines/binary frames as desktop child stdout for
  `session::parse_python_message`; `shutdown(reason)` sends shutdown, closes
  stdin, and joins. Dropping a session closes stdin and detaches the thread.
- PyO3 feature split: `extension-module` moved from the base dependency into
  `python-extension` (wheel builds unaffected); new `embedded` feature uses
  `pyo3/auto-initialize`.

**Evidence**

- New Python tests (`python/tests/test_embedded_session.py`, 3 tests): full
  initialize → ready → snapshot → event → patch → shutdown over `BytesIO`,
  binary frame routing, and embedded `run()` without exec. Full Python suite:
  467 tests pass (3 skipped), including all desktop subprocess/stdio session
  tests.
- New Rust integration test (`tests/embedded_session.rs` + fixture
  `tests/fixtures/embedded_app.py`): real in-process CPython completes
  initialize → ready → snapshot → event → patch → clean shutdown. Full crate
  suite with `--features embedded`: 229 unit + 1 embedded + 4 showcase + 1
  doctest pass; clippy is clean apart from pre-existing unused-dependency
  manifest warnings.
- Cross-checks pass: `embedded` for `aarch64-apple-ios-sim` and
  `aarch64-linux-android` (library only; the `gpui-android` app backend
  breakage above is Phase 3 work), plus `python-extension` for
  `aarch64-apple-ios-sim` and desktop.
- `uvx pyright gpui_toolkit/app.py`: the same 12 errors as the unmodified
  checkout — Phase 2 adds none.

Phase 3 consumes `spawn_embedded_session` from the iOS/Android host crates.
It still owns: repairing `gpui-android`, wiring lifecycle, bundling the
interpreter/stdlib/app into the app packages, and device/emulator QA.

## Phase 3 — mobile hosts (implemented 2026-11-14)

**Android backend repair.** `gpui-android` compiles again for
`aarch64-linux-android`: new `Platform`/`PlatformWindow`/`PlatformDisplay`
trait items mirrored from `gpui-ios` (`restart` args, `on_quit -> bool`,
displays-changed/sleep/wake hooks, `prevent_idle_sleep`, window
visibility/display-changed, `refresh_interval`), `completed_frame` removed,
atlas key by value, `RequestFrameOptions` default fill, plus drifted unit
tests (key names are `&str`, `dispatcher()`, `gpu_specs()`). Verified with
`cargo check`/`clippy --all-targets` for the Android target and host
`cargo test -p gpui-android` (26 tests).

**Shared host.** `PythonSession` supervision is now transport-agnostic
(`SessionSupervision::Child`/`Embedded`, one `assemble_python_session`,
one handshake with per-transport capabilities), and
`EmbeddedSession::into_transport` hands the raw pipes + interpreter thread
to the host's existing bounded reader/writer stack. The desktop
bins pass 66 tests each; with `--features embedded`, 67 (new real-CPython
host-handshake test). `showcase::run_embedded_app` installs the mobile
globals and opens the Python window from a platform app callback; both
mobile crates include the desktop facade verbatim by path.

**iOS host** (`mobile/ios`, bundle `org.spinorama.gpui-python-mobile`):
staticlib + Swift shell + vendored pinned XCframework + `install_python`
Xcode phase (stdlib into bundle, every C extension — including `_native`
— converted to a signed framework). Proven in the iOS 18.6 simulator:
tutorial app renders (dataset bound, 216 samples), dev-API `window_click`
drove `on_action` in Python and the Clicks metric went 0 → 1 with the
goal bar advancing (screenshots under `/Volumes/home_tmp/tmp/mobile-plan/qa-ios/`).

**Android host** (`mobile/android`, applicationId
`org.spinorama.gpui.pythonmobile`): cdylib via cargo-ndk, Gradle shell
reusing `dev.gpui.mobile.GpuiActivity`, stdlib + app staged as assets and
extracted to internal storage at first launch. Two packaging traps found
and fixed: AGP's default `ignoreAssetsPattern` silently drops
`_`-prefixed asset directories (66 stdlib files), and `cargo ndk` copies
the dependency cdylib into jniLibs (removed after build). Proven on the
GPUI_API_36 emulator: renders, `adb input tap` drove Clicks 0 → 1.

**Residual risks**

- Android text glyphs are lost after HOME/resume surface recreation
  while input/patches keep working — filed as
  [#11](http://192.168.1.32:3001/pierre/gpui-toolkit/issues/11).
- Untrimmed bundles (~333M iOS .app, ~110M debug APK) and no store
  signing/privacy manifests yet — Phase 5. Generated projects from a
  Python file (scaffolder template) are Phase 4; today the bundled app
  is staged by the scripts in each mobile crate (see `mobile/README.md`).

## Phase 4 — generator UX (implemented 2026-11-14)

`gpui-scaffolder --template python-mobile` generates a buildable project
from a name alone: tracked Python sources in `app/` (counter starter),
`gpui-mobile.toml` identity manifest, Swift/Gradle shells, and a Justfile
(`preflight`, `ios-sim`/`ios`, `android-apk`/`android-aab`) that drives
the toolkit's parameterized stage scripts (`[project-dir] [app-source-dir]`
arguments; crate-local defaults unchanged). No Rust crate is generated —
the toolkit's mobile host crates are built as-is. Python entry points:
`gpui_toolkit.mobile` (`MobileManifest`, `validate_app`,
`python -m gpui_toolkit.mobile scaffold|preflight`) and
`ScaffoldOptions(template=..., no_ios=..., no_android=...)` through the
host `scaffolder.preview`/`scaffolder.write` commands.

Proven end-to-end from a generated `gen-demo` project (outside the repo):
`just preflight` validates through the real host handshake, `just ios-sim`
builds and the starter app renders in the simulator, `just android-apk`
builds and the starter app renders in the emulator with a tap driving
Clicks 0 → 1, and `just android-aab` produces a release bundle.
Generation exposed and fixed two staging bugs: script
paths are now canonicalized before the workspace `cd`, and the scaffolder
copies the complete Android Java host set (6 files; its completeness test
now diffs against the checkout directory listing).

Remaining: store-ready hardening is Phase 5 (signing, privacy manifests,
bundle trimming, AAB/TestFlight flows).

## Phase 5 — store-ready hardening (implemented 2026-11-14)

Machinery only; no credentials committed and nothing was uploaded.
Signing identities, store accounts, and privacy answers remain the app
owner's (checklist in `mobile/STORE.md`).

- **iOS**: `PrivacyInfo.xcprivacy` (no tracking; file-timestamp and
  boot-time required-reason declarations for the shell, marked
  review-before-submit), single-size `AppIcon` asset catalog generated by
  `mobile/scripts/make_app_icon.py` (app-provided `icon-1024.png` wins at
  stage time), `UILaunchScreen`, and `MARKETING_VERSION`/
  `CURRENT_PROJECT_VERSION` settings consumed by both Info.plists.
- **Android**: target/compile SDK 36 (Play requirement for new apps
  since 2026-08-31, verified against developer.android.com), env-driven
  release signing (`GPUI_ANDROID_KEYSTORE` + passwords/alias; unsigned
  when unset), launcher icons in all densities, and version name/code as
  Gradle properties fed from `gpui-mobile.toml` by the generated recipes.
  Proven by signing and verifying an AAB with a throwaway local keystore
  (`jarsigner`: jar verified).
- **Trimming**: stdlib `test`, `idlelib`, `turtledemo`, `ensurepip`
  wheels, and `__pycache__` are stripped at bundle time on both
  platforms. Measured: iOS `.app` 333M → 107M (Python home 237M → 14M),
  Android staged stdlib 66M → 23M, release AAB stays 42M. Both trimmed
  builds were relaunched and render; first boot is slower without
  `__pycache__` (documented opt-out in `mobile/STORE.md`).

Outstanding before an actual submission: a real signing identity/keystore,
store metadata/listings, privacy re-audit against app behavior, and
issue #11 (Android glyphs after surface recreation).

## Phase 6 — example and docs (implemented 2026-11-14)

`python/examples/mobile_signal_lab.py` is the maintained reference app:
Overview/Signal/Settings sections with touch-sized actions, form state,
and a 96-sample revisioned dataset whose chart updates from slider and
waveform commits. It runs unchanged through the desktop host, is covered
by behavior tests (`tests/test_mobile.py`, plus the generic example-main
tests), and is also the default app staged into the toolkit's own iOS and
Android host bundles.

Generated-project evidence (outside the repo, under
`/Volumes/home_tmp/tmp/mobile-plan/phase6/`): `just preflight` passes
through the real desktop host handshake (“Mobile Signal Lab”, 3
sections); `just ios-sim` builds and launches in the iOS simulator;
`just android-apk` builds, launches in the GPUI_API_36 emulator and a tap
drives Python `on_action` (Clicks 0 → 1, goal bar advances — screenshots
inspected). Documentation now describes the supported subset and limits
in the runtime README, both tutorials, and `mobile/README.md`.

Generation exposed and fixed a scaffolder bug: Android projects copied
only two of six Java host sources and no longer compiled. The scaffolder
now copies the complete host source set, and a test fails if the checkout
gains a Java file the embedded list does not carry.
