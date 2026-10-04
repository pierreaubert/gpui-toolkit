# Vendoring Notes: gpui_windows

## Upstream

- Source: `https://github.com/zed-industries/zed`
- Upstream path: `crates/gpui_windows`
- Upstream ref: `v1.9.0`
- Local package: `gpui-toolkit-gpui-windows 0.1.2` (`gpui_windows` library)
- Last reviewed: 2026-10-04 against pinned Zed `ced90fc636c4ede05402befc38a63bae7fd741bd`

## Pinned source comparison

- Upstream `crates/gpui_windows` tree: `356dd80ca8b943a04d67898c1ee6a1d792eb0d6f`.
- Local tree at toolkit `77d13fa8d9cf1f7462aece14dfd946a3900483be`:
  `e5bfb74d26de9ffa09b245c3d962dbbbe71668d7`.
- Fourteen common Rust/HLSL source files plus the build script differ. The
  retained `PrimitiveBatch::Custom` no-op arm in `src/directx_renderer.rs`
  keeps scene matching exhaustive without claiming Windows custom-draw support.
  `hide_other_apps` and `unhide_other_apps` remain deliberate no-ops.
- The other source differences remain unclassified against this exact pin.
  Their presence is a reason to retain the fork until Windows native evidence
  is collected, not a claim that every difference is behavioral.

The changed common sources are `build.rs` and `src/direct_manipulation.rs`,
`src/direct_write.rs`, `src/directx_atlas.rs`, `src/directx_devices.rs`,
`src/directx_renderer.rs`, `src/dispatcher.rs`, `src/display.rs`,
`src/events.rs`, `src/platform.rs`, `src/shaders.hlsl`,
`src/system_settings.rs`, `src/util.rs`, `src/vsync.rs`, and `src/window.rs`.

Reproduce the comparison from clean checkouts outside active worktrees
(exit status 1 means files differ):

```sh
git clone https://github.com/zed-industries/zed /tmp/zed-gpui-audit
git -C /tmp/zed-gpui-audit checkout --detach ced90fc636c4ede05402befc38a63bae7fd741bd
git -C gpui-toolkit worktree add --detach /tmp/gpui-toolkit-audit 77d13fa8d9cf1f7462aece14dfd946a3900483be
git diff --no-index -- /tmp/zed-gpui-audit/crates/gpui_windows/src /tmp/gpui-toolkit-audit/crates/3rdparties/gpui_windows/src
```

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
- `PrimitiveBatch::Custom` is ignored by the DirectX renderer pending native
  custom-draw support; removing the arm would break the local GPUI scene API.
- `gpui_toolkit::vendored_patch_manifest()` records this crate as an active
  patch and repeats the retained-change list for release QA.
- Before de-vendoring, classify the remaining source differences against the
  pinned Zed revision, retain equivalent platform behavior, and pass Windows
  target compilation plus native UI interaction/paint checks. The native gate
  is not yet complete for this review.

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
