# Third-Party Vendor Notes

This directory contains local copies of external crates. Treat each directory as
an upstream snapshot plus a small local patch stack. Keep upstream README,
CHANGELOG, and generated Cargo metadata intact when practical; put SotF-specific
upgrade notes in each crate's `VENDORING.md`.

## Inventory

The 16-crate GPUI closure (`gpui` through `util_macros` below, including
`gpui_macos`) was imported from zed `v1.9.0` as history-free snapshots by
`scripts/import_gpui_upstream.py` (re-runnable: `--skip`, `--check` drift
report). Each closure crate's `VENDORED.md` records its upstream path, base
ref, import exclusions (`examples/`, `benches/`, dev-deps on
`reqwest_client`/`gpui_platform`/`gpui_web`, GPL-3.0 `zlog`/`ztracing`), and
local patches. `gpui_wgpu` and `gpui_windows` predate the import script and
remain hand-maintained.

| Library | Upstream | Local version/ref | Build status | Why it is here | Patch burden |
| --- | --- | --- | --- | --- | --- |
| `block` | `SSheldon/rust-block` | `0.1.6` | Active `[patch.crates-io]` | Fix Rust future-incompatibility warning for the Objective-C block runtime class symbol | Low |
| `collections` | `zed-industries/zed`, `crates/collections` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Collection types used across the GPUI closure | Low |
| `derive_refineable` | `zed-industries/zed`, `crates/derive_refineable` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Derive macro for `refineable` | Low |
| `gpui` | `zed-industries/zed`, `crates/gpui` | `v1.9.0`, `0.2.2` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Core GPUI UI framework snapshot; crate-root lint allows and restored test fonts recorded in `VENDORED.md` | Low |
| `gpui_linux` | `zed-industries/zed`, `crates/gpui_linux` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Linux platform backend for `gpui` | Low |
| `gpui_macos` | `zed-industries/zed`, `crates/gpui_macos` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Pristine re-vendor plus recorded CGS private-symbol removal (Mac App Store static-analysis rejection risk) | Medium |
| `gpui_macros` | `zed-industries/zed`, `crates/gpui_macros` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Proc macros for `gpui` | Low |
| `gpui_shared_string` | `zed-industries/zed`, `crates/gpui_shared_string` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Shared-string type used by `gpui` text | Low |
| `gpui_util` | `zed-industries/zed`, `crates/gpui_util` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Utility helpers for `gpui` | Low |
| `gpui_wgpu` | `zed-industries/zed`, `crates/gpui_wgpu` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Hand-maintained renderer/backend patch point while tracking the Zed tag (not script-vendored) | Medium |
| `gpui_windows` | `zed-industries/zed`, `crates/gpui_windows` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Hand-maintained Windows backend patch point while tracking the Zed tag (not script-vendored) | Medium |
| `http_client` | `zed-industries/zed`, `crates/http_client` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | HTTP client abstraction used by the GPUI closure | Low |
| `mach2` | `JohnTitor/mach2` | `0.5.0` | Local snapshot; current lockfile resolves registry `mach2` | Mach kernel bindings snapshot for platform work | Low |
| `media` | `zed-industries/zed`, `crates/media` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Media and screen-capture types used by the GPUI closure | Low |
| `objc` | `SSheldon/rust-objc` | `0.2.7` | Active `[patch.crates-io]` | Local Objective-C runtime binding patch point | Medium |
| `perf` | `zed-industries/zed`, `crates/perf` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Profiling helpers used by the GPUI closure | Low |
| `psm` | `rust-lang/stacker`, `psm` crate | `0.1.30` | Local snapshot; current lockfile resolves registry `psm 0.1.31` | Portable stack manipulation snapshot | Low |
| `refineable` | `zed-industries/zed`, `crates/refineable` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Refinement trait for GPUI style types | Low |
| `scheduler` | `zed-industries/zed`, `crates/scheduler` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Async scheduler/executor used by `gpui` | Low |
| `sum_tree` | `zed-industries/zed`, `crates/sum_tree` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Sequence-tree storage for text; carries a recorded `ztracing::instrument` to `tracing::instrument` patch | Low |
| `util` | `zed-industries/zed`, `crates/util` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Shared platform/command utilities used across the closure | Low |
| `util_macros` | `zed-industries/zed`, `crates/util_macros` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Proc macros for `util` | Low |
| `zed-font-kit` | `zed-industries/font-kit` / Servo `font-kit` fork | `0.14.1-zed`, root pin `110523127440aefb11ce0cf280ae7c5071337ec5` | Active `[patch."https://github.com/zed-industries/font-kit"]` | Apple mobile target cfg, CoreText manifest fixes, canvas bitmap conversion fixes, CSS-generic family title aliases | Medium |

## Upgrade Workflow

1. Read the target crate's `VENDORING.md`.
2. Identify the current upstream ref and the target upstream ref.
3. Replace the upstream snapshot, preserving local documentation files.
4. Reapply only the local changes listed in `VENDORING.md`.
5. For each local change, decide whether it is still needed.
6. Run the verification commands listed in the crate-specific file.
7. Update the upstream ref, local changes, and verification date in `VENDORING.md`.
8. If a directory is meant to affect the build, confirm `cargo tree -i <crate>` resolves to the local path.

## Structured Manifest

`gpui_toolkit::vendored_patch_manifest()` mirrors this inventory in code. It is
the release-facing contract for upstream refs, local paths, active/inactive
status, retained local changes, and upgrade verification gates. Update the
crate-specific `VENDORING.md` file and the manifest together when adding,
removing, or changing a vendored patch.

Note: the manifest currently reflects the pre-closure state — it covers only
the original patch crates and still lists `gpui_macos` as an inactive snapshot
(now an active pristine-plus-CGS-patch vendor). The script-vendored GPUI
closure crates are tracked through their per-crate `VENDORED.md` files and the
root `[patch]` table until the manifest is extended (follow-up work).

## Documentation Rules

- Do not hide local changes only in source comments.
- Prefer a short local-change inventory over broad prose.
- Mark unknown history as unknown instead of guessing.
- Keep generated or upstream files close to upstream unless a local patch requires otherwise.
- If a local patch should be upstreamed, record that in `VENDORING.md`.
