# Third-Party Vendor Notes

This directory contains local copies of external crates. Treat each directory as
an upstream snapshot plus a small local patch stack. Keep upstream README,
CHANGELOG, and generated Cargo metadata intact when practical; put SotF-specific
upgrade notes in each crate's `VENDORING.md`.

## Inventory

| Library | Upstream | Local version/ref | Build status | Why it is here | Patch burden |
| --- | --- | --- | --- | --- | --- |
| `gpui_macos` | `zed-industries/zed`, `crates/gpui_macos` | `v1.9.0`, `0.1.0` | Local snapshot; not currently patched in root `Cargo.toml` | App Store private-symbol removal and explicit manifest copy | Medium |
| `gpui_wgpu` | `zed-industries/zed`, `crates/gpui_wgpu` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Local renderer/backend patch point while tracking the Zed tag | Medium |
| `gpui_windows` | `zed-industries/zed`, `crates/gpui_windows` | `v1.9.0`, `0.1.0` | Active `[patch."https://github.com/zed-industries/zed.git"]` | Local Windows backend patch point while tracking the Zed tag | Medium |
| `mach2` | `JohnTitor/mach2` | `0.5.0` | Local snapshot; current lockfile resolves registry `mach2` | Mach kernel bindings snapshot for platform work | Low |
| `objc` | `SSheldon/rust-objc` | `0.2.7` | Active `[patch.crates-io]` | Local Objective-C runtime binding patch point | Medium |
| `psm` | `rust-lang/stacker`, `psm` crate | `0.1.30` | Local snapshot; current lockfile resolves registry `psm 0.1.31` | Portable stack manipulation snapshot | Low |
| `zed-font-kit` | `zed-industries/font-kit` / Servo `font-kit` fork | `0.14.1-zed`, root pin `110523127440aefb11ce0cf280ae7c5071337ec5` | Active `[patch."https://github.com/zed-industries/font-kit"]` | Apple mobile target cfg and CoreText manifest fixes | Medium |

## Upgrade Workflow

1. Read the target crate's `VENDORING.md`.
2. Identify the current upstream ref and the target upstream ref.
3. Replace the upstream snapshot, preserving local documentation files.
4. Reapply only the local changes listed in `VENDORING.md`.
5. For each local change, decide whether it is still needed.
6. Run the verification commands listed in the crate-specific file.
7. Update the upstream ref, local changes, and verification date in `VENDORING.md`.
8. If a directory is meant to affect the build, confirm `cargo tree -i <crate>` resolves to the local path.

## Documentation Rules

- Do not hide local changes only in source comments.
- Prefer a short local-change inventory over broad prose.
- Mark unknown history as unknown instead of guessing.
- Keep generated or upstream files close to upstream unless a local patch requires otherwise.
- If a local patch should be upstreamed, record that in `VENDORING.md`.

