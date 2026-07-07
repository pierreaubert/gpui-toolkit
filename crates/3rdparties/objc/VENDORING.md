# Vendoring Notes: objc

## Upstream

- Source: `https://github.com/SSheldon/rust-objc`
- Local package: `objc 0.2.7`
- Last reviewed: 2026-07-07

## Build Status

This directory is active. The root `Cargo.toml` patches crates.io so `objc`
resolves to this local directory.

Confirm with:

```sh
cargo tree -i objc
```

## Why Vendored

This crate is the Objective-C runtime binding used by Apple platform code. The
local patch gives this workspace a stable place to handle compatibility issues
with modern Rust lints, cfg checking, and Apple backend builds while the
upstream crate remains old and mostly stable.

## Local Changes

- Current builds still emit warnings from this crate, including
  `unexpected cfg condition value: cargo-clippy`, deprecated implicit ABI
  syntax, and missing-doc warnings.
- Intentional source-level differences from upstream are not yet documented.
  On the next upgrade or cleanup, compare this directory with upstream
  `objc 0.2.7` and list each retained change here.

## Upgrade Procedure

1. Compare against upstream `objc 0.2.7` and any newer available release.
2. Preserve only compatibility changes still required by Apple platform builds.
3. Re-run cfg/lint-sensitive builds and update the warning inventory above.
4. Confirm the root `[patch.crates-io]` still points to this directory.

## Verification

Recommended checks:

```sh
cargo check -p gpui-ui-kit --examples
cargo check -p gpui-miniapp
```

Apple target checks are preferred when available.

## Upstreaming Status

Unknown. Any modernization patches should be evaluated for upstreaming, but this
crate may be effectively maintenance-mode.

