# Vendoring Notes: zed-font-kit

## Upstream

- Source: `https://github.com/zed-industries/font-kit`
- Upstream base: Servo `font-kit`
- Local package: `zed-font-kit 0.14.1-zed`
- Root dependency pin: `110523127440aefb11ce0cf280ae7c5071337ec5`
- Last reviewed: 2026-07-08

## Build Status

This directory is active. The root `Cargo.toml` patches
`https://github.com/zed-industries/font-kit` so `zed-font-kit` resolves to this
local directory.

Confirm with:

```sh
cargo tree -i zed-font-kit
```

## Why Vendored

The git font-kit revision used by the workspace lacks the Apple mobile target
cfg coverage needed here and has CoreText import/manifest issues for these
targets. The vendored copy carries the Apple mobile target fixes.

## Local Changes

- Manifest includes Apple mobile targets in CoreText-related dependency cfgs:
  `ios`, `tvos`, `watchos`, and `visionos`.
- Manifest excludes those Apple mobile targets from FreeType/fontconfig
  dependency cfgs.
- `canvas.rs` implements A8/RGBA bitmap conversion and 1bpp bitmap expansion
  into A8, RGB24, and RGBA32 canvases instead of panicking on those glyph
  rasterization paths.
- `source.rs` maps CSS-generic family names that arrive as titled strings
  (`serif`, `sans-serif`, `monospace`, `cursive`, `fantasy`, and common
  `ui-*`/`system-ui` aliases) through the same platform defaults as typed
  `FamilyName` variants before falling back to literal family lookup.
- `src/loaders/core_text/tests.rs` keeps its helper references compatible with
  direct vendored crate test runs.
- `Cargo.toml` has an empty `[workspace]` table so this vendored crate can be
  tested directly from inside the containing repository without joining the
  main workspace.
- The crate is a normalized vendored manifest; keep `Cargo.toml.orig` when
  available to aid future imports.
- `gpui_toolkit::vendored_patch_manifest()` records this crate as an active
  patch and repeats the retained-change list for release QA.
- On upgrade, diff this directory against the target `zed-industries/font-kit`
  revision; add any newly retained source changes to this file and the manifest
  before release.

## Upgrade Procedure

1. Pick the target `zed-industries/font-kit` revision.
2. Replace this directory with that upstream snapshot.
3. Reapply Apple mobile target cfg, CoreText manifest fixes, standalone
   `[workspace]`, canvas bitmap conversion fixes, CSS-generic family title
   aliasing, and direct-test CoreText test path fixes if still needed.
4. Update the root `font-kit` dependency rev and this file together.
5. Confirm the `[patch."https://github.com/zed-industries/font-kit"]` entry
   still points here.
6. Update `gpui_toolkit::vendored_patch_manifest()` with the new upstream base,
   retained changes, and verification gate.

## Verification

Recommended checks:

```sh
cargo check -p gpui-ui-kit --examples
cargo check -p gpui-miniapp --target aarch64-apple-ios
cargo check -p gpui-miniapp --target aarch64-apple-tvos
cargo test --manifest-path crates/3rdparties/zed-font-kit/Cargo.toml canvas --lib
cargo test --manifest-path crates/3rdparties/zed-font-kit/Cargo.toml source --lib
```

If Linux font discovery is touched, also run a Linux check that exercises
fontconfig/FreeType.

## Upstreaming Status

Apple mobile target cfg and CoreText manifest fixes should be good upstreaming
candidates if they are still absent upstream.
