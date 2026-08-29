# Bug Review: gpui-hello-web — 2026-08-25

Scope: `gpui-hello-web` is the minimal wasm/browser spike crate — one
`src/main.rs` (55 lines, a single `Render` impl drawing one colored quad and
one text line), one wasm smoke test (`tests/wasm_smoke.rs`), plus build glue
(`Cargo.toml`, `Trunk.toml`, `index.html`, `CHANGELOG.md`). I read all six
tracked files end to end and checked the untracked `dist/` output only for
size/build-config implications. There is no shader, no custom wgpu code, no
threading, and no Python/JS glue of its own (the JS it serves,
`qa/perf/wasm-scheduling-baseline.js`, lives outside the crate and is only
copied by Trunk). Given the size, this review is necessarily short; most bug
categories have nothing to find.

## Findings

No Critical, High, or Medium findings. Two Low items:

- **Low — Native-binary exit code lies about failure.**
  `crates/gpui-hello-web/src/main.rs:53-55` — the non-wasm `main()` prints an
  error to stderr ("only runs on wasm32-unknown-unknown") but returns `()`,
  so the process exits 0. Any script or CI step that accidentally invokes the
  native binary sees success. Fix: `std::process::exit(1)` after the eprintln,
  or return `Result`/`ExitCode::FAILURE` from `main`.

- **Low — Served wasm payload is unoptimized by configuration.**
  `crates/gpui-hello-web/index.html:7` — the Trunk link sets `data-keep-debug`
  and `data-wasm-opt="0"`, producing a ~135 MB `_bg.wasm` in `dist/` (observed
  locally, untracked). This is presumably deliberate for the spike/dev loop,
  but if this page is ever the basis for a deployable demo it is a real
  foot-gun. Fix when needed: drop `data-keep-debug` and set a real
  `data-wasm-opt` level for release builds.

Notes on things that look alarming but are correct:

- `std::mem::forget(handle)` at `src/main.rs:48` is an intentional,
  well-commented keep-alive for the `ApplicationHandle`; the comment at lines
  31-34 explains exactly why. Not a leak bug.
- The two `.expect()` calls (`src/main.rs:30`, `src/main.rs:44`) are
  startup-fatal paths in a spike binary; on wasm a panic surfaces in the
  console, which is acceptable here.
- The fixed `size(px(640.), px(560.))` windowed bounds (`src/main.rs:36`)
  against a full-viewport canvas CSS (`index.html:11`) is a cosmetic
  mismatch at most, and matches how the sibling showcase spikes are set up.

## UI/UX consistency

The crate renders one static text line and one colored rounded quad. It does
not use `gpui-design` tokens (hardcoded `rgb(0x1e1e2e)` / `rgb(0xf38ba8)`
Catppuccin-ish literals at `src/main.rs:15` and `src/main.rs:22`), has no
focus/keyboard/ARIA handling, and no interactive elements. For a minimal
platform-bringup spike this is appropriate; if it graduates to a real demo,
text should be exposed to the accessibility tree and colors should come from
the design system. No action recommended now.

## Clean bill

- No threading, no locks, no channels, no `RefCell`, no unsafe — nothing to
  deadlock or borrow-panic.
- No per-frame allocation concerns: `render()` builds a static element tree
  with two children; no Vecs/Strings grow, no closures capture.
- No wgpu/GPU code in the crate itself, so there are no GPU→CPU→GPU cycles,
  readbacks, or buffer re-creation issues to report (rendering lives in the
  gpui web/wgpu backend, outside this crate's scope).
- Build glue is consistent: `dist/` is gitignored (verified with
  `git check-ignore`), the Trunk COOP/COEP headers match the documented
  WebGPU requirement in the workspace AGENTS.md, the copied
  `qa/perf/wasm-scheduling-baseline.js` exists, and the wasm smoke test is
  correctly gated with `#![cfg(target_family = "wasm")]`.

## Resolution — 2026-08-25

- Fixed the native-binary exit status: `main` now returns `ExitCode::FAILURE`, with a native unit test. Verified with `cargo test -p gpui-hello-web` and `cargo run -p gpui-hello-web` (exit status 1).
- Verified the unoptimized wasm payload is intentional for this minimal development spike: `data-keep-debug` and `data-wasm-opt="0"` are not a defect under the crate's documented `wasm-serve-hello` workflow. No production/deployment contract exists, so no change was made.
