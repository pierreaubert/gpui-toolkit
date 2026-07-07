# Vendoring Notes: zed-font-kit

## Upstream

- Source: `https://github.com/zed-industries/font-kit`
- Upstream base: Servo `font-kit`
- Local package: `zed-font-kit 0.14.1-zed`
- Root dependency pin: `110523127440aefb11ce0cf280ae7c5071337ec5`
- Last reviewed: 2026-07-07

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
- The crate is a normalized vendored manifest; keep `Cargo.toml.orig` when
  available to aid future imports.
- Source-level differences from the exact root git rev are not yet fully
  documented. On the next upgrade, compare this directory against the target
  `zed-industries/font-kit` revision and list any retained source changes here.

## Upgrade Procedure

1. Pick the target `zed-industries/font-kit` revision.
2. Replace this directory with that upstream snapshot.
3. Reapply Apple mobile target cfg and CoreText manifest fixes if still needed.
4. Update the root `font-kit` dependency rev and this file together.
5. Confirm the `[patch."https://github.com/zed-industries/font-kit"]` entry
   still points here.

## Verification

Recommended checks:

```sh
cargo check -p gpui-ui-kit --examples
cargo check -p gpui-miniapp --target aarch64-apple-ios
cargo check -p gpui-miniapp --target aarch64-apple-tvos
```

If Linux font discovery is touched, also run a Linux check that exercises
fontconfig/FreeType.

## Upstreaming Status

Apple mobile target cfg and CoreText manifest fixes should be good upstreaming
candidates if they are still absent upstream.

