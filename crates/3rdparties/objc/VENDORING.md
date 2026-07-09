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

- Declare the legacy `cargo-clippy` cfg feature used by old macros so modern
  cfg checking stays quiet.
- Use explicit `extern "C"` ABI spellings for Objective-C runtime declarations,
  method implementations, message dispatch, examples, and tests.
- Replace deprecated `trim_left_matches` with `trim_start_matches`.
- Allow missing docs at the crate root because this is a vendored upstream
  snapshot, not a public API documented by this workspace.
- Replace test-only `ONCE_INIT` usage with `Once::new()` and make the
  `CustomStruct` test return type `#[repr(C)]`.
- `msg_send!` uses `addr_of!(*obj)` so nil raw pointers can reach Objective-C
  nil-message dispatch without first creating a null Rust reference.
- `gpui_toolkit::vendored_patch_manifest()` records this crate as an active
  patch and repeats the retained-change list for release QA.

## Upgrade Procedure

1. Compare against upstream `objc 0.2.7` and any newer available release.
2. Preserve only compatibility changes still required by Apple platform builds.
3. Re-run cfg/lint-sensitive builds and update the retained-change inventory
   above.
4. Confirm the root `[patch.crates-io]` still points to this directory.
5. Update `gpui_toolkit::vendored_patch_manifest()` with the new upstream base,
   retained changes, and verification gate.

## Verification

Recommended checks:

```sh
cargo check -p objc
cargo test -p objc
cargo check -p gpui-au --all-targets
cargo check -p gpui-toolkit --all-features
```

Apple target checks are preferred when available.

## Upstreaming Status

Unknown. Any modernization patches should be evaluated for upstreaming, but this
crate may be effectively maintenance-mode.
