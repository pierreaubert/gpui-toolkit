# Vendoring Notes: gpui_wgpu

## Upstream

- Source: `https://github.com/zed-industries/zed`
- Upstream path: `crates/gpui_wgpu`
- Upstream ref: `v1.9.0`
- Local package: `gpui_wgpu 0.1.0`
- Last reviewed: 2026-07-07

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
- `zed-font-kit` dependency is pinned to
  `110523127440aefb11ce0cf280ae7c5071337ec5`, matching the root font-kit
  dependency and local `[patch]`.
- `gpui_toolkit::vendored_patch_manifest()` records this crate as an active
  patch and repeats the retained-change list for release QA.
- No additional retained source-level behavior is claimed here. On upgrade,
  diff this directory against Zed's `crates/gpui_wgpu`; add any newly retained
  source changes to this file and the manifest before release.

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
