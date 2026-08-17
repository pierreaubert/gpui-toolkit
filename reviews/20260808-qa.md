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
- literal Rust `include_str!`/`include_bytes!` assets present in Git so clean
  worktrees and source archives compile without developer-local downloads;

CI additionally runs workspace tests and publishes the coverage artifact. The
platform matrix must compile on Linux, macOS, and Windows. `just
qa-apple-simulators` adds build/install/launch/pixel evidence for iOS and tvOS;
`just qa-android-emulator` adds launch, touch-navigation, before/after pixel,
and native accessibility-tree evidence. Native screen readers, Audio Unit,
physical devices, and hardware interaction gates remain explicit entries until
their device/host evidence is attached.

The selected crates.io wave (`gpui-design`, `gpui-profiler`, and
`gpui-ui-kit-macros`) declares and continuously checks Rust 1.89 as its MSRV.
`gpui-pretext` is also checked at that floor but is deferred until its
`gpui-profiler` registry predecessor exists and a locked dry-run passes. The
remaining crates do not claim that MSRV until the unpublished GPUI dependency
and target-specific toolchains can be validated at the same floor.

## Unsafe-code policy

Portable first-party code must be safe Rust. `just qa-scripts` runs
`scripts/qa_unsafe_policy.py`, which rejects unsafe Rust constructs outside
the explicit native boundary crates (`gpui-au`, `gpui-android`, and
`gpui-ios`) and their Android/iOS/tvOS showcase entry libraries. Generated mobile FFI attributes in
`gpui-scaffolder` are textually exempt, while that crate itself uses
`forbid(unsafe_code)`. Vendored third-party sources are governed separately.
An unavoidable unsafe trait API, such as allocator instrumentation, must be
delegated to a reviewed dependency rather than implemented in portable
first-party code.

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
measurement reviewed on 2026-08-07 was 74.76%. This is a ratchet: it may be
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
audio-meter formatting, 1,000 warmed 1,024-bin spectrum meter updates,
normalized keybinding search, pretext width/grapheme
cache hits, and `gpui-builder::solve_tree_into` across 1,000 responsive nested
resize solves (including priority collapse). UI-kit also covers 1,000 warmed
insert/backspace, word-kill/restore, and selection replacement cycles. The
iOS accessibility diff path covers 1,000 cached 1,000-node comparisons with
1% churn using window-owned index scratch. Line/scatter preparation covers
1,000 alternating 10,000-point shared-data frames with uniquely owned mapped
slice reuse. The remaining initial-set items stay release work rather than
inheriting a pass from these contracts.

The release-machine streaming baselines are approximately 7.6 µs at 10k,
82–84 µs at 100k, and 0.79 ms at 1m points for both line and scatter mapped
point preparation. These six workloads are part of the hard comparator.

## Visual and UX policy

Manifests are inventories, not rendered proof. Release evidence must include
actual captures and diffs for supported renderers with backend, OS, scale
factor, font set, color scheme, and viewport recorded. Interactive components
must have machine-checked keyboard, pointer, touch, focus, disabled, reduced
motion, high-contrast, accessibility, and narrow-layout behavior where those
capabilities apply.

`just qa-visual` writes `target/qa/visual/report.md`, renderer-scoped component
manifests, capture/diff reports, and the showcase inventory. On macOS it
captures a deterministic 200-case Metal profile at 2x scale, generates contact
sheets, extracts the versioned baseline archive from `qa/visual/baselines/`,
and performs a strict pixel comparison. Missing, blank, undecodable,
wrong-sized, or changed captures fail. Baseline promotion requires the explicit
`QA_VISUAL_UPDATE_BASELINES=1` opt-in and review of the resulting gallery and
diff evidence.

The scheduled nightly workflow shards all 1,922 registered component cases
across eight macOS jobs and uploads the actual PNGs, manifests, capture reports,
and contact sheets for 14 days. The PR archive remains a compact 200-case
selection with at least one capture for every story; thousands of individual
baseline PNGs are not committed to Git.

`just qa-gpui-obvious` also writes deterministic desktop interaction and
accessibility evidence to `target/qa/accessibility/`. The JSON/Markdown report
links the exact pointer, keyboard, focus order/restoration, disabled-state,
accessible-name/action, native-adapter parity, reduced-motion, and
high-contrast contracts. It is portable component/renderer proof, not evidence
that VoiceOver, Narrator, Orca/AT-SPI, or TalkBack was executed.

Desktop CI separately launches the `gpui-builder` layout showcase on native
Linux, macOS, and Windows backends. Its smoke artifact proves platform
initialization, window creation, a sidebar state transition, and a second
root-view render. The same lane runs deterministic
GPUI pointer contracts for tree selection, divider collapse, and divider
dragging. Linux additionally captures the Xvfb window and rejects blank or
near-uniform pixels. macOS and Windows still record `pixel_capture: false`;
their hosted native backend smoke is not presented as screenshot/diff proof.

The macOS release host also provides explicit local renderer capture recipes:

- `just qa-native-ui-macos` builds and captures the real macOS GPUI window
  directly; Docker is not involved.
- `just qa-native-ui-utm-linux` starts or resumes `Ubuntu 24.04 ARM`, syncs the
  workspace over key-authenticated SSH, runs against the logged-in X11/XWayland
  desktop, and copies the exact window PNG and JSON report back to the host.
- `just qa-native-ui-utm-windows` starts or resumes `Win11 ARM AutoEQ`, uses the
  QEMU guest agent for isolated workspace transfer/build orchestration, and
  schedules rendering plus window capture in the logged-in `pierre` desktop.

These recipes restore a VM that they started to its prior stopped/suspended
state. Set `GPUI_UTM_KEEP_RUNNING=1` only for interactive debugging. A lock
screen, UTM host-window capture, headless SYSTEM session, missing SSH key, or
missing desktop login fails without changing `pixel_capture` to true. Ubuntu
requires this Mac's SSH public key in the guest account and `xdotool` plus
ImageMagick. Windows requires UTM guest tools, the user Rust toolchain, and an
interactive desktop login. VM names, user/host, and dedicated guest roots can
be overridden with the documented `GPUI_UTM_*` environment variables in the
scripts.

Local UTM evidence is a release artifact, not an ordinary GitHub-hosted job.
Promoting it to scheduled CI requires a dedicated self-hosted macOS runner with
screen-capture permission and provisioned, non-personal QA guest accounts.

## Platform evidence

Desktop core changes require Linux, macOS, and Windows compile/test evidence.
Platform backends additionally require:

- iOS: `just qa-ios-simulator` covers build/install/launch/pixel capture; touch,
  rotation, keyboard/IME, VoiceOver, and physical-device smoke remain manual;
- Android: `just qa-android-emulator` covers target/APK build, install, launch,
  touch navigation, before/after pixels, and native accessibility-tree export;
  TalkBack actions, IME, rotation/lifecycle, hardware GPU, and physical-device
  evidence remain manual;
- tvOS: `just qa-tvos-simulator` covers simulator build/install/launch/pixel
  capture; focus/remote, VoiceOver, and physical-device walkthroughs remain
  manual;
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

`just qa-release-contract` extends this with required contributor, security,
support, conduct, changelog, release-lane, and MSRV metadata plus locked package
verification for the three crates.io wave-one packages. Publishing or uploading
anything remains a separate explicit maintainer action.

The final `just qa` step emits `target/qa/release-evidence.json` and Markdown.
This manifest hashes every required coverage, performance, conformance,
accessibility, and visual report plus its comparator inputs and records the
source revision, dirty state, commit timestamp, host, and toolchains. Use
`just qa-release-evidence` for an RC: it repeats the full gate and refuses a
dirty worktree. Platform evidence can be made mandatory with repeated
`--require-platform` arguments; the generator rejects captures made from a
different revision or a dirty tree.

`just release-rc <version>` is the offline artifact authority. It refuses a
dirty or version-mismatched worktree and emits the source and visual-gallery
archives, locked wave-one crate packages, SPDX 2.3 SBOM, license inventory,
path-free provenance, and SHA-256 manifest. Final release evidence requires
two clean-worktree runs at the same commit with byte-identical outputs. The
recipe never tags, signs, pushes, publishes, or uploads.

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
