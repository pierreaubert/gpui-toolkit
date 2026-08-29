# Bug Review: gpui-scaffolder — 2026-08-25

Scope: the entire `gpui-scaffolder` crate (`crates/gpui-scaffolder/`) — 4 files
total: `Cargo.toml`, `CHANGELOG.md`, `src/main.rs` (49 lines), and
`src/lib.rs` (1853 lines, of which ~700 are unit/integration tests). The crate
is a one-shot codegen CLI that materializes a standalone GPUI mini-app project
(Cargo manifest, Rust sources, XcodeGen/Swift iOS host, Gradle/Android host)
from `format!`-based string templates; it has no event loop, no GPU, no
threading, and no per-frame paths. I read both source files in full, cross-checked
the generated GPUI `version`/`tag` pair against the upstream Zed manifest at
`v1.9.0`, verified the preview file list against the actual writes, and checked
the workspace root `AGENTS.md` plus the existing perf review
(`reviews/perf-gpui-scaffolder-20260822.md`) for prior context.

## Findings

Ranked by severity. No Critical or High findings.

### Medium

1. **Stale compile-time toolkit root baked into generated manifests** —
   `crates/gpui-scaffolder/src/lib.rs:267-277` (`toolkit_root()`) uses
   `env!("CARGO_MANIFEST_DIR")`, so the gpui-toolkit checkout path is frozen
   into the binary at compile time. The six `path = "..."` dependencies written
   into the generated `Cargo.toml` (`src/lib.rs:96-105`) are computed relative
   to that baked-in path. If the workspace is moved/renamed after the
   scaffolder is built (or the binary is run from a copied location), the
   scaffold silently points at the old checkout — still "working" but building
   against stale sources — or fails with `failed to resolve …` if the old path
   is gone. Fix: resolve the toolkit root at runtime instead (e.g. a
   `GPUI_TOOLKIT_ROOT` env var or `--toolkit-root` flag, falling back to
   `current_exe()`-relative discovery), or stamp the binary with a build-time
   path check that errors loudly when it has moved.

2. **Generated app has no `.gitignore`** — the write list in
   `crates/gpui-scaffolder/src/lib.rs:107-192` (and its mirror
   `planned_scaffold_files`, `src/lib.rs:214-247`) emits 21 files but no
   `.gitignore`. A user who runs `git init && git add .` in a fresh scaffold
   after a first build stages `target/`, `ios/build/`, and
   `android/gradle/.gradle/` — multi-GB accidents in the first commit. Fix:
   add a `.gitignore` template (`/target`, `/ios/build`, `/ios/lib`,
   `**/local.properties`, `.gradle/`) to both the write sequence and the
   preview list.

3. **`planned_scaffold_files` is a hand-maintained duplicate of the write
   sequence** — `src/lib.rs:214-247` re-lists by hand every path that
   `scaffold_app` writes at `src/lib.rs:107-192`. Today the two match exactly
   (I counted 21 files on both sides), but nothing enforces this: the next
   template file added to `scaffold_app` silently makes `preview_scaffold`
   under-report. Fix: derive both from a single table of
   `(relative_path, render_fn)` entries, or add a test that scaffolds into a
   tempdir and asserts the on-disk file set equals `preview_scaffold().files`.

### Low

4. **`--force` refuses directories that are "empty" only because of
   `.DS_Store`** — `ensure_directory_is_replaceable` at `src/lib.rs:249-261`
   bails if `fs::read_dir` yields any entry. On macOS, any directory the Finder
   has displayed acquires a `.DS_Store`, so `gpui-scaffolder foo --force` on a
   Finder-touched empty dir fails with "not empty; refusing to replace it".
   Fix: ignore `.DS_Store` (and possibly `Thumbs.db`/`desktop.ini`) when
   counting entries — while still refusing anything else.

5. **TOCTOU window between the emptiness check and `remove_dir_all`** —
   `src/lib.rs:60-66`: `ensure_directory_is_replaceable` verifies the directory
   is empty, then `fs::remove_dir_all` deletes it in a separate syscall. A
   concurrent process dropping a file in between would have it deleted. For a
   local single-user CLI this is mostly theoretical, but the fix is cheap:
   attempt `fs::remove_dir` (which fails atomically on non-empty directories)
   and only fall back to `remove_dir_all` for the verified-empty race case.

6. **Non-ASCII app names are silently mangled** —
   `separated_identifier` (`src/lib.rs:380-397`) drops every non-ASCII byte, so
   `gpui-scaffolder café` creates directory `café` whose package is `caf`,
   Xcode target `Caf`, view `CafView`, and bundle id `com.example.caf`. The
   scaffold succeeds with identities that no longer match the requested name,
   and two distinct names (`café`, `càf`) collapse to the same package. Fix:
   either reject names whose derived identifiers lose characters (compare
   `separated_identifier` output against the input and bail with a clear
   message), or transliterate before deriving.

7. **Eight dead feature flags in the crate manifest** —
   `crates/gpui-scaffolder/Cargo.toml:13-22` declares `autoeq`, `gpu-2d`,
   `gpu-3d`, `reqwest`, `showcase`, `spinorama`, `tokio`, `urlencoding`, but no
   dependency is optional and `grep` finds zero `cfg(feature = …)` sites in the
   crate. `--features gpu-3d` compiles byte-identical code. Fix: delete the
   stubs (or wire them into template selection if scaffold variants are
   actually planned — the perf review already flagged them as stubs).

8. **`toml_string` does not escape control characters** —
   `src/lib.rs:442-444` escapes only `\` and `"`. TOML basic strings also
   forbid raw control characters (`\n`, `\t`, …). Today all user-controlled
   strings that reach TOML are pre-sanitized (`package_name`, `title` via
   whitespace-splitting), so this is latent rather than live — it would only
   fire if a future interpolation passes a raw path or name containing a
   control character. Fix: extend the escape table (`\n` → `\\n`, `\t` → `\\t`,
   `\r` → `\\r`) or route TOML emission through the `toml` crate's serializer,
   which the crate already depends on for tests.

9. **Generated iOS `ffi_guard` swallows panics without logging** — the
   template at `src/lib.rs:711-720` emits a `catch_unwind` that returns
   `R::default()` on panic with no log line, so a panic during
   `{ffi_start_symbol}` startup on device fails silently (app never opens a
   window, no console output). Fix: add an `eprintln!`/`NSLog` in the `Err`
   arm of the template before returning the default.

## GPU/CPU data-flow notes

Not applicable: the crate has no wgpu/rendering dependency and performs no GPU
work (verified by reading all of `src/`; the only embedded native code is the
two Java host files copied verbatim via `include_str!`).

## UI/UX consistency

Not applicable to the crate's own runtime (it renders no UI). The generated
`src/app.rs` template (`src/lib.rs:632-697`) does use toolkit components
(`Heading`, `Text`, `Button` with `ButtonVariant::Primary`) and theme globals,
consistent with the workspace conventions. The one nit is that the generated
"Refresh" `Button` has no `on_click` handler, so the starter app ships a
visually-primary button that does nothing — arguably fine as a placeholder,
but a one-line comment or a `cx.refresh_windows()` handler would make the
template self-explanatory.

## Clean bill

- **Name/path validation:** `AppNames::new` + `validate_directory_name`
  (`src/lib.rs:279-317`) correctly reject empty names, `..`, nested paths, and
  absolute paths; path-traversal attempts (`../outside`, `nested/app`) are
  covered by tests (`src/lib.rs:1188-1193`).
- **Escaping in templates:** `toml_string`, `rust_string`, and `xml_string`
  are applied at the right sites — the title is XML-escaped in `Info.plist`
  and `strings.xml`, Rust-escaped in `app.rs`, and TOML-escaped in
  `Cargo.toml`; no raw unescaped user input reaches a structured file in the
  normal (ASCII) path.
- **GPUI version pinning:** `GPUI_VERSION = "0.2.2"` + `GPUI_ZED_TAG =
  "v1.9.0"` (`src/lib.rs:8-9`) matches the upstream Zed manifest at that tag
  (upstream `crates/gpui` is `version = "0.2.2"` at `v1.9.0`; the vendored
  0.2.3 is a local rename/bump), and a dedicated test
  (`generated_gpui_tag_matches_vendored_revision`, `src/lib.rs:1604-1614`)
  keeps the tag in sync with the vendored provenance.
- **Dry-run/force semantics:** collision rules, `--force` on empty dirs only,
  and dry-run non-mutation are all covered by tests
  (`src/lib.rs:1689-1852`) and behave as the CLI help describes.
- **Threading/panics:** no concurrency, no locks, no `unwrap`/`expect`/
  `panic!` in production code paths (only in tests), and `#![forbid(unsafe_code)]`
  at `src/lib.rs:1`.

## Resolution status

- [x] 1. **Runtime toolkit root discovery** (2026-08-26): replaced the compile-time manifest-dir lookup with executable-ancestor discovery and the documented `GPUI_TOOLKIT_ROOT` override. The resolver validates the selected workspace instead of silently generating references to an old checkout. Verified by `cargo test -p gpui-scaffolder toolkit_root --lib` (2 passed).
- [x] 2. **Missing generated `.gitignore`** (2026-08-26): fresh projects now ignore Rust, Xcode, and Gradle build artifacts plus Android `local.properties` before their first commit. Verified by `cargo test -p gpui-scaffolder preview_matches_the_complete_generated_file_set --lib`.
- [x] 3. **Preview/write-list drift** (2026-08-26): added a regression test that scaffolds a project and compares every emitted file with `preview_scaffold`; future template additions cannot silently under-report the preview.
- [x] 4. **System metadata prevents `--force`** (2026-08-26): `.DS_Store`, `Thumbs.db`, and `desktop.ini` are now treated as ignorable metadata in an otherwise empty scaffold directory. Verified by `scaffold_force_replaces_system_metadata_only_directory`.
- [x] 5. **Directory replacement race** (2026-08-26): replacement now unlinks only the explicitly accepted metadata files and calls atomic `remove_dir`; it never performs recursive deletion after the emptiness check. Files added concurrently cause a safe failure. Verified by `replacement_never_removes_files_added_after_validation`.
- [x] 6. **Non-ASCII identifier mangling** (2026-08-26): names that cannot map losslessly to Cargo/Xcode/Rust identifiers are rejected with an explicit ASCII-only error instead of silently producing unrelated project identities. Verified by `app_names_reject_non_ascii_input`.
- [x] 7. **Dead feature flags** (2026-08-26): removed the eight no-op feature flags (and their empty default) from the scaffolder manifest. `cargo check -p gpui-scaffolder --all-targets` verifies the crate’s only actual build surface.
- [x] 8. **TOML control-character escaping** (2026-08-26): TOML strings now escape all TOML control characters, including uncommon C0 controls and DEL, rather than emitting syntactically invalid manifests. Verified by parsing the generated string in `toml_string_escapes_control_characters`.
- [x] 9. **Silent mobile FFI panic** (2026-08-26): generated iOS/tvOS startup guards now emit an explicit diagnostic before returning the ABI-safe default value. Verified by `generated_mobile_ffi_guard_reports_panics`.
