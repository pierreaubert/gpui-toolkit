# GPUI Toolkit QA policy

This document defines the evidence required to call a crate or platform
release-ready. The canonical local gate is:

```bash
just qa
```

Unlike an aspirational checklist, the canonical gate must stay green on the
main branch. Targets that are not available on the current host are tracked in
`gpui_toolkit::release_qa_matrix()` and proved by platform CI or an attached
manual/device report.

## Enforced gates

`just qa` currently enforces:

- warning-free workspace Clippy (`-D warnings`);
- property and invariant tests;
- visual manifests, GPU/golden tests, design-token validation, and component
  conformance;
- performance non-regression against `qa/perf/baseline.json`;
- the focused GPUI component/layout/chart test matrix;
- portable-core llvm-cov coverage; and
- cargo-deny advisory, license, duplicate, and source-origin policy.

CI additionally runs workspace tests and publishes the coverage artifact. The
platform matrix must compile on Linux, macOS, and Windows. Apple mobile,
Android, Audio Unit, and hardware interaction gates remain explicit entries in
the release matrix until their simulator/device/host evidence is attached.

The currently publishable GPUI-free crates (`gpui-design`, `gpui-pretext`, and
`gpui-ui-kit-macros`) declare and continuously check Rust 1.89 as their MSRV.
The remaining crates do not claim that MSRV until the unpublished GPUI
dependency and target-specific toolchains can be validated at the same floor.

## Coverage policy

Coverage is reported only for portable production library code. Tests,
examples, benches, demos, patched third-party code, and platform/FFI backends
are excluded from this single aggregate because mixing them would create a
misleading percentage. Platform code needs separate target-specific contract
and smoke evidence.

`gpui-scaffolder` is excluded from the coverage execution as well as the metric
because its compile-contract tests invoke nightly Apple/tvOS targets. Those
tests run in the Apple platform lane; excluding them here prevents host setup
from changing the portable-core percentage.

The current enforced portable-core floor is **73.5% lines**. The latest stored
measurement reviewed on 2026-07-12 was 73.60%. This is a ratchet: it may be
raised after a verified report and must not be lowered to accommodate a change.
The release target is 90%, with these per-crate priorities:

| Priority | Crates | Requirement |
| --- | --- | --- |
| Tier A | gpui-builder, gpui-design, gpui-design-tools, gpui-keybinding, gpui-pretext, gpui-px, gpui-python-runtime, gpui-toolkit | Keep at or above 90%; do not regress an already higher crate. |
| Tier B | gpui-audio-kit, gpui-d3rs, gpui-themes, gpui-ui-kit, gpui-ui-kit-macros | Raise uncovered behavior and error paths toward 90%; publish crate-level numbers. |
| Story/demo | gpui-component-lab, gpui-showcase | Gate story, command, screenshot, and navigation coverage rather than optimizing line coverage alone. |
| Platform | gpui-au, gpui-android, gpui-ios, gpui_wgpu, gpui_windows, mobile hosts | Report target-specific contract/build/runtime evidence separately. |

Every coverage artifact must state its ignore expression and must not be called
“workspace coverage” without the exclusions.

## Performance and allocation policy

`just qa-perf` is a hard non-regression gate. Baseline updates are intentional
and reviewed with `just qa-perf-update`; a slower baseline must not be committed
merely to make the gate pass.

Performance baselines use schema 2 and record OS, architecture, hardware model,
Rust/Cargo versions, and source revision. Incomparable hosts/toolchains are
rejected rather than reported as code regressions. The hard slowdown threshold
is 20% for cases at or above the 150 ns noise floor; this is above the observed
paired-run variance while still catching material changes.

Criterion latency does not prove heap reuse. Frame/event hot paths should also
use `gpui-profiler` allocation scopes after warm-up. The required initial set is
meter/spectrum updates, repeated layout solves, text cache hits, edit events,
chart streaming/render preparation, key dispatch/search, and accessibility
snapshot diffs. Each contract must document warm-up, input size, allowed
allocations, and whether retained capacity is part of the API guarantee.

Implemented zero-allocation steady-state contracts currently cover cached
audio-meter formatting, normalized keybinding search, pretext width/grapheme
cache hits, and `gpui-builder::solve_tree_into` across 1,000 responsive nested
resize solves (including priority collapse). UI-kit also covers 1,000 warmed
insert/backspace, word-kill/restore, and selection replacement cycles. The
remaining initial-set items stay release work rather than inheriting a pass
from these contracts.

## Visual and UX policy

Manifests are inventories, not rendered proof. Release evidence must include
actual captures and diffs for supported renderers with backend, OS, scale
factor, font set, color scheme, and viewport recorded. Interactive components
must have machine-checked keyboard, pointer, touch, focus, disabled, reduced
motion, high-contrast, accessibility, and narrow-layout behavior where those
capabilities apply.

`just qa-visual` writes `target/qa/visual/report.md` and the showcase capture
inventory. The report deliberately records renderer capture as pending; a
logic-only host must not turn that state into a pass.

## Platform evidence

Desktop core changes require Linux, macOS, and Windows compile/test evidence.
Platform backends additionally require:

- iOS: simulator build/launch plus touch, safe-area, rotation, keyboard, and
  VoiceOver smoke evidence;
- Android: target build plus emulator/device launch, touch, IME, lifecycle,
  density, and TalkBack evidence;
- tvOS: simulator/device build and focus/remote walkthrough;
- Audio Unit: attach/detach/resize/text smoke tests in a named AUv3 host; and
- Windows: native DirectWrite/DirectX, IME, pointer, DPI, and accessibility
  smoke evidence.

Unsupported or unexecuted capabilities remain `partial`, `pending`,
`manual-required`, or `blocked` in the release matrix. A portable-core pass
must never silently upgrade a platform gate to `passed`.

## Release checklist

The `just qa-api` lane is the documentation and public-contract authority. It
checks no-default-feature library builds, warning-free rustdoc, generated
project compilation, exact README workspace inventory, the Cargo-pinned GPUI
revision, and vendored documentation/review freshness. Vendored Zed/wgpu-facing
renderer and platform patches have a 30-day upstream review cadence; other
vendored snapshots have a 90-day cadence. Each entry must name an owner,
removal condition, reproducible delta command, and verification gate.

1. Run `just qa` and attach coverage, performance, visual/conformance, and
   cargo-deny reports.
2. Run the platform CI matrix and attach required simulator/device/host smoke
   evidence.
3. Run public API/semver checks for publishable crates and compile generated
   scaffolds.
4. Review dependency advisory exceptions in `deny.toml`; every exception needs
   an owner, rationale, and removal condition in release metadata.
5. Generate the release QA matrix, release notes report, publish plan, and
   crate stability manifest. Do not tag while a required gate is not passed or
   explicitly accepted with documented residual risk.
