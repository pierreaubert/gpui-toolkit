# Vendoring Notes: gpui_windows

## Upstream

- Source: `https://github.com/zed-industries/zed`
- Upstream path: `crates/gpui_windows`
- Upstream ref: `v1.9.0`
- Local package: `gpui_windows 0.1.0`
- Last reviewed: 2026-07-07

## Build Status

This directory is active. The root `Cargo.toml` patches
`https://github.com/zed-industries/zed.git` so `gpui_windows` resolves to this
local directory.

Confirm with:

```sh
cargo tree -i gpui_windows
```

## Why Vendored

This crate is the local GPUI Windows backend patch point. Keeping it local lets
the workspace track a Zed tag while adjusting Windows dependency versions,
features, and build behavior.

## Local Changes

- Manifest uses workspace dependency pins for this repository.
- `hide_other_apps` and `unhide_other_apps` intentionally no-op on Windows
  instead of panicking, matching the absence of a direct Windows equivalent.
- `gpui_toolkit::vendored_patch_manifest()` records this crate as an active
  patch and repeats the retained-change list for release QA.
- On upgrade, diff this directory against Zed's `crates/gpui_windows`; add any
  newly retained source changes to this file and the manifest before release.

## Upgrade Procedure

1. Copy `crates/gpui_windows` from the target Zed tag.
2. Reapply workspace dependency pins and Windows feature choices.
3. Diff local source files against upstream and document retained changes.
4. Confirm the root `[patch]` still points to this directory.
5. Update `gpui_toolkit::vendored_patch_manifest()` with the new upstream base,
   retained changes, and verification gate.

## Verification

Recommended checks:

```sh
cargo check -p gpui_windows --target x86_64-pc-windows-msvc
cargo check -p gpui-miniapp --target x86_64-pc-windows-msvc
```

## Upstreaming Status

Unknown. Document source-level differences before deciding what should be
upstreamed.
