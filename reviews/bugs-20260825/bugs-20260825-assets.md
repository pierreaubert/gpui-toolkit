# Bug Review: assets (directory crates/assets — treat as a crate even if it is mostly asset/manifest files; review any code, build scripts, or loaders, and note if there is simply nothing to review) — 2026-08-25

Scope: `crates/assets/` contains 10 files and no source code at all — 8 TrueType font binaries (IBM Plex Sans Regular/Italic/SemiBold/SemiBoldItalic, Lilex Regular/Bold/Italic/BoldItalic) plus 2 SIL OFL 1.1 license files, ~1.6 MB total. There is no `Cargo.toml`, no `build.rs`, no loader, no README, and no shader/Python/JS glue in this directory, so there is essentially nothing to review in the conventional sense. I therefore (a) verified the asset payloads themselves (real TTF binaries per `file(1)`, not LFS pointers; licenses present and matching the OFL requirement for both families) and (b) audited the two consumers that bake these files into binaries via `include_bytes!`: `crates/3rdparties/gpui_web/src/platform.rs:23-32` (wasm web platform, embeds all 8 fonts) and `crates/3rdparties/gpui/src/svg_renderer.rs:306-307` (test-only module, embeds the two Regular variants).

## Findings

No Critical, High, or Medium findings. There is no executable code in the crate to contain correctness, threading, or GPU bugs.

- **Low — `crates/assets/` has no manifest or README, so the load-bearing path convention is implicit.** The `include_bytes!("../../../assets/fonts/...")` calls in `crates/3rdparties/gpui_web/src/platform.rs:24-31` and `crates/3rdparties/gpui/src/svg_renderer.rs:306-307` only resolve because `crates/3rdparties/<crate>/src` is exactly three levels below `crates/`. That convention is documented only in `crates/3rdparties/gpui/VENDORED.md:32` and enforced indirectly by the `just qa` include_bytes-in-Git gate (reviews/20260808-qa.md:27-28). A short `crates/assets/README.md` stating "these files are referenced by relative `include_bytes!` from `crates/3rdparties/*`; do not move or rename" would make the constraint discoverable at the point of change. Impact of today: a well-meaning reorganization (e.g. flattening `3rdparties/`) breaks compilation with a confusing missing-file error one crate away.
- **Low (downstream observation, recorded here since it concerns these payloads) — all 8 fonts (~1.6 MB) are statically embedded into every wasm bundle.** `crates/3rdparties/gpui_web/src/platform.rs:23-32` `include_bytes!`s the full set into `BUNDLED_FONTS`, which is parsed eagerly at `WebPlatform::new` (platform.rs:73-79). For the WebGPU-only browser target this inflates the wasm binary and startup parse cost; subsetting the TTFs to the glyphs actually used, or loading them asynchronously via `fetch` + `add_fonts`, would cut both. The fix belongs in `gpui_web`, not in this crate — noted here only because the asset set is what makes it 1.6 MB.

## Clean bill

- Licensing: both families ship their required SIL OFL 1.1 license files (`fonts/ibm-plex-sans/license.txt`, `fonts/lilex/OFL.txt`); binaries are unmodified upstream payloads (per `VENDORED.md:32`, sources unmodified, only relocated), so Reserved Font Name obligations are satisfied.
- Integrity: all 8 `.ttf` files are genuine TrueType data (verified with `file(1)`), not Git LFS pointer stubs — the clean-worktree/`include_bytes!` gate holds for these paths.
- Consumer error handling: `WebPlatform::new` logs via `log::error!` if `add_fonts` fails rather than panicking (platform.rs:77-79); the svg_renderer usage is test-only.
- No GPU/CPU data-flow or UI/UX sections apply: the crate renders nothing and touches no GPU API.

## Resolution — 2026-08-25

- Fixed the implicit asset-path convention by adding `crates/assets/README.md`. It names the `include_bytes!` consumers, states the directory is not a Cargo crate, and documents the filename/layout and license-retention requirements. Verified each documented consumer path with `rg`.
