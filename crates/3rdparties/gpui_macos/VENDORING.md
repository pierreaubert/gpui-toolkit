# Vendoring Notes: gpui_macos

## Upstream

- Source: `https://github.com/zed-industries/zed`
- Upstream path: `crates/gpui_macos`
- Upstream ref: `v1.9.0`
- Local package: `gpui_macos 0.1.0`
- Last reviewed: 2026-07-07

## Build Status

This directory is a local snapshot. The root `Cargo.toml` currently depends on
`gpui_macos` from `zed-industries/zed` at tag `v1.9.0`; it does not currently
patch `gpui_macos` to this local directory.

If this local copy is intended to affect builds, add or restore a matching
`[patch."https://github.com/zed-industries/zed.git"]` entry and confirm with:

```sh
cargo tree -i gpui_macos
```

## Why Vendored

The local snapshot documents a Mac App Store compatibility fork. Upstream
`gpui_macos` references private Core Graphics Services symbols
`CGSMainConnectionID` and `CGSSetWindowBackgroundBlurRadius` for old blur
support. SotF requires macOS 13+, so that old path should be unreachable, but
Apple's static analyzer can still reject binaries that mention the symbols.

## Local Changes

- Removes private CGS symbol declarations and the unreachable call path.
- Expands upstream `.workspace = true` dependencies into explicit literal
  dependencies because this crate can live outside the workspace.
- Adds a `cargo-clippy` feature shim for objc 0.2 macro cfg compatibility.
- Pins Zed dependencies to `v1.9.0`.

## Upgrade Procedure

1. Copy `crates/gpui_macos` from the target Zed tag.
2. Reapply the private CGS symbol removal.
3. Re-expand workspace dependencies if the crate remains outside the workspace.
4. Check whether the `cargo-clippy` feature shim is still needed.
5. Confirm the font-kit pin matches the intended root `zed-font-kit` patch.

## Verification

Recommended checks:

```sh
cargo check --manifest-path crates/3rdparties/gpui_macos/Cargo.toml --target aarch64-apple-darwin
cargo check -p gpui-miniapp
```

For release builds, inspect the final binary for the private CGS symbol names.

## Upstreaming Status

The private-symbol removal may not be appropriate upstream if Zed still supports
older macOS blur behavior. Keep this local unless upstream raises its minimum
macOS support and removes the legacy path.
