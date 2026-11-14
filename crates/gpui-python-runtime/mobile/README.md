# Python apps on mobile (embedded CPython hosts)

Native iOS and Android hosts that run `gpui_toolkit` Python apps with an
embedded CPython 3.15.2 interpreter — the same `App`/`build_app()` code,
session-v1 protocol, and renderer as the desktop `gpui-python-host`, with
OS pipes instead of a child process. Background and gate evidence live in
[`../MOBILE.md`](../MOBILE.md); tracking issue is
[#10](http://192.168.1.32:3001/pierre/gpui-toolkit/issues/10).

For your own app, generate a project instead of editing these host crates:

```sh
cargo run -p gpui-scaffolder -- my-app --output-dir .. --template python-mobile
cp crates/gpui-python-runtime/python/examples/mobile_signal_lab.py ../my-app/app/main_app.py
cd ../my-app
just preflight
just ios-sim      # or: just android-apk
```

Both hosts compile the shared desktop host facade
(`crates/gpui-python-runtime/bin/showcase.rs` and its modules, included by
path) and launch it through `showcase::run_embedded_app`. The example app
bundled by the staging scripts is
[`mobile_signal_lab.py`](../python/examples/mobile_signal_lab.py), a
touch-sized signal controls app that also runs through the desktop host.

Bundle contract on both platforms:

- `python/` — CPython home (`lib/python3.15` stdlib), found via `PYTHONHOME`.
- `app/main_app.py` — app entry defining `build_app()` or `app`.
- `app/gpui_toolkit/` — toolkit package plus the target `_native.abi3.so`.
- Embedded hosts never negotiate `resource_mmap_frames`; datasets and
  arrays travel as bounded binary frames over the pipe.

## iOS (`ios/`)

```sh
cd crates/gpui-python-runtime/mobile/ios
export CARGO_TARGET_DIR=...   # optional
./build-rust.sh               # stages resources, builds lib/libpython_mobile_ios.a
xcodegen generate
xcodebuild -project GPUIPythonMobile.xcodeproj -scheme GPUIPythonMobile \
    -configuration Release -sdk iphonesimulator -arch arm64 build
```

`scripts/stage-ios-resources.sh` downloads the pinned CPython XCframework
(sha256-verified), builds `_native` for the target slice, and assembles
`app/`. The Xcode "Install Python" phase then copies the stdlib into the
bundle and converts every C extension (including `_native`) into a signed
framework, which is how iOS requires loadable Python extensions to ship.

## Android (`android/`)

```sh
cd crates/gpui-python-runtime/mobile/android
export ANDROID_HOME=/opt/homebrew/share/android-commandlinetools
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.2.12479018
scripts/build-apk.sh          # debug APK under gradle/app/build/outputs/
```

`scripts/stage-android-resources.sh` downloads the pinned CPython Android
build, builds `_native`, and stages `assets/python`, `assets/app`,
`assets/gpui_bundle_manifest.txt`, plus `libpython3.15.so` and its
`lib*_python.so` support libraries in `jniLibs/arm64-v8a`. At first launch
the Rust entry extracts the asset tree into internal storage (keyed by a
manifest hash marker) and sets `PYTHONHOME` there, mirroring the CPython
Android testbed. Note `ignoreAssetsPattern = ""` in the Gradle config:
AGP's default pattern silently drops `_`-prefixed asset directories such
as `_pyrepl/`, which breaks the stdlib at runtime.

## Known limitations

- Pure-Python packages must be copied into `app/` at build time; compiled
  third-party wheels are out of scope (see `../MOBILE.md`).
- Android: text glyphs do not survive HOME/resume surface recreation
  (backend atlas bug, input and rendering otherwise keep working) —
  [#11](http://192.168.1.32:3001/pierre/gpui-toolkit/issues/11).
- Store signing is env-driven machinery, not a completed submission:
  you still supply signing identities, store accounts, metadata, and a
  privacy audit. See [`STORE.md`](STORE.md). Only arm64 is supported.
