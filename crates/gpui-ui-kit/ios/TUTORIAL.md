# gpui-ui-kit-ios-showcase Tutorial

This crate builds the `gpui-ui-kit` showcase as a static library for iOS and
tvOS-style Apple mobile targets.

## 1. Understand the layout

- `src/lib.rs` exports the Rust FFI entry point `showcase_ios_start`.
- `ShowcaseApp/` contains the Swift host app.
- `project.yml` describes the Xcode project for XcodeGen.
- `build-rust.sh` builds and copies `libshowcase_ios.a`.
- `hot-reload-showcase.sh` builds the simulator hot-reload dylib.

## 2. Build the Rust library for simulator

```bash
just ios-rust-sim
```

To build and copy into the Xcode project:

```bash
just ios-build-rust-sim
```

## 3. Build the Xcode app

```bash
just ios-xcodegen
just ios-sim
```

For device:

```bash
just ios-rust-device
just ios-device
```

## 4. Use hot reload in simulator

```bash
just ios-hot-reload
```

The script writes a manifest under `target/gpui-ios-hot-reload`.

## 5. Build tvOS Rust artifacts

Install prerequisites:

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

Then run:

```bash
just tvos-sim
just tvos-device
```

The tvOS recipes produce and copy Rust static libraries. This repo currently
ships the iOS Swift host project; add a tvOS host project before expecting a
full tvOS app bundle.

## 6. Verify

```bash
cargo check -p gpui-ui-kit-ios-showcase
just --dry-run ios-sim
just --dry-run tvos-sim
```
