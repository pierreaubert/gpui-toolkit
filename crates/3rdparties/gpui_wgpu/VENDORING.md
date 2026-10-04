# Vendoring Notes: gpui_wgpu

## Upstream

- Source: `https://github.com/zed-industries/zed`
- Upstream path: `crates/gpui_wgpu`
- Upstream ref: `v1.9.0`
- Local package: `gpui-toolkit-gpui-wgpu 0.1.2` (`gpui_wgpu` library)
- Last reviewed: 2026-10-04 against pinned Zed `ced90fc636c4ede05402befc38a63bae7fd741bd`

## Pinned source comparison

- Upstream `crates/gpui_wgpu` tree: `830b79b0110a44303a6f2c29bdf1c3c60fe59613`.
- Local tree at toolkit `77d13fa8d9cf1f7462aece14dfd946a3900483be`:
  `80f354219ed1f9cb5123546da4d4d4a8ff694f02`.
- Seven common Rust/WGSL source files differ. The local tree also adds
  `src/custom.rs`, `src/cosmic_text_system/`, and `src/wgpu_renderer/`.
  `PATCHES.md` documents custom draw dispatch, frame format/full bounds,
  capability probing, and premultiplied sprite shading.
- The split text/renderer modules and other changed source files are retained
  pending source-level classification; tree inequality alone is not evidence
  that each difference changes runtime behavior.

The changed common source files are `src/cosmic_text_system.rs`,
`src/gpui_wgpu.rs`, `src/shaders.wgsl`, `src/shaders_subpixel.wgsl`,
`src/wgpu_atlas.rs`, `src/wgpu_context.rs`, and `src/wgpu_renderer.rs`.

To reproduce from clean checkouts, run these commands outside active worktrees
and inspect the source diff (exit status 1 means files differ):

```sh
git clone https://github.com/zed-industries/zed /tmp/zed-gpui-audit
git -C /tmp/zed-gpui-audit checkout --detach ced90fc636c4ede05402befc38a63bae7fd741bd
git -C gpui-toolkit worktree add --detach /tmp/gpui-toolkit-audit 77d13fa8d9cf1f7462aece14dfd946a3900483be
git diff --no-index -- /tmp/zed-gpui-audit/crates/gpui_wgpu/src /tmp/gpui-toolkit-audit/crates/3rdparties/gpui_wgpu/src
```

## Build Status

This directory is active. The root `Cargo.toml` patches
`https://github.com/zed-industries/zed.git` so `gpui_wgpu` resolves to this
local directory.

Confirm with:

```sh
cargo tree -i gpui_wgpu
```

## Why Vendored

This crate is the local GPUI WGPU renderer/backend patch point. Keeping it local
lets the workspace track a Zed tag while adjusting renderer dependencies and
platform compatibility without forking all of GPUI.

## Local Changes

- Manifest tracks Zed `v1.9.0` dependencies.
- The standalone manifest sets `gpui` to `default-features = false`, matching
  Zed v1.9.0's workspace dependency policy.
- `zed-font-kit` dependency is pinned to
  `94b0f28166665e8fd2f53ff6d268a14955c82269`, matching the root font-kit
  dependency and local `[patch]`.
- `gpui_toolkit::vendored_patch_manifest()` records this crate as an active
  patch and repeats the retained-change list for release QA.
- The local renderer is not source-identical to pinned Zed v1.9.0. Retain it
  until the module split and remaining source differences are classified, the
  custom-draw behavior is available upstream or replaced without losing it,
  and native, wasm, and AU rendering gates pass on the proposed replacement.

## Upgrade Procedure

1. Copy `crates/gpui_wgpu` from the target Zed tag.
2. Reapply local manifest pins required by this workspace.
3. Confirm the `zed-font-kit` rev matches the root dependency and patch.
4. Diff local source files against upstream and document any retained changes.
5. Update `gpui_toolkit::vendored_patch_manifest()` with the new upstream base,
   retained changes, and verification gate.

## Verification

Recommended checks:

```sh
cargo check -p gpui_wgpu
cargo check -p gpui-ui-kit --examples
```

For rendering changes, also run at least one GPUI miniapp/showcase that exercises
text, gradients, images, and shadows.

## Upstreaming Status

Unknown. Document source-level differences before deciding what should be
upstreamed.
