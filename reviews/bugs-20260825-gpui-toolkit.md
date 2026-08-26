# Bug Review: gpui-toolkit — 2026-08-25

Scope: the `gpui-toolkit` aggregate crate at `crates/gpui-toolkit/` — the
whole crate, i.e. `Cargo.toml` (61 lines) and all 7 files under `src/`
(~3,820 lines: `lib.rs`, `stability.rs`, `dependency_hygiene.rs`,
`release_qa.rs`, `release_notes.rs`, `publish_plan.rs`,
`release_packaging.rs`, `vendored_patches.rs`). This crate is pure static
release-QA metadata: `&'static` tables plus tiny iterator filters and
Markdown renderers. It contains no UI rendering, no wgpu/GPU code, no
threading, and no per-frame hot paths, so those review categories are not
applicable and are reported as such below. Verification: `cargo test -p
gpui-toolkit` (default features) passes 49 unit tests + 1 doctest; `cargo
test -p gpui-toolkit --no-default-features` fails (finding 1).

## Findings

## Resolved during follow-up — 2026-08-26

- **Minimal-feature doctest:** the crate-level example now uses only feature-free aggregate APIs, so `cargo test -p gpui-toolkit --no-default-features` compiles its doctest.
- **Release-note stability drift:** UI Kit, Audio Kit, Keybinding, and Themes report the manifest’s `beta` stability level, and a regression compares every release-notes entry with a matching stability-manifest row.
- **Deferred registry wave:** `ReleasePackagingStatus::Deferred` distinguishes postponed ordered publishing from deliberate exclusion. Pretext and Builder now use the `deferred-registry` lane and remain release-blocking; the Python report enum recognizes the status and the additive report-schema change is versioned as v2.
- **Vendored-review cadence:** the documentation policy now checks each literal `VendoredPatch` maintenance date against its own cadence. The three 30-day records were reviewed/renewed on 2026-08-26.

Verified `cargo test -p gpui-toolkit`, `cargo test -p gpui-toolkit --no-default-features` (51 tests across both runs), and `PYTHONPATH=scripts python3 -m unittest discover -s scripts/tests -p 'test_qa_docs_policy.py'`.

Ranked by severity. There are no Critical or High findings: the crate has no
runtime logic capable of corrupting state, panicking in production, or
deadlocking. The real risks here are correctness-of-metadata issues, which
matter because this crate's entire purpose is to be the single source of
truth for release QA.

### Medium

1. **Doctest is not feature-gated and breaks minimal-feature builds** —
   `crates/gpui-toolkit/src/lib.rs:47-65`. The crate-level doc example
   imports `gpui_ui_kit` and `gpui_design`, both gated behind the default
   `ui` feature (lib.rs:124,148). Verified: `cargo test -p gpui-toolkit
   --no-default-features` fails with E0432 ("found an item that was
   configured out … gated behind the `ui` feature") while compiling the
   doctest. The library itself and all 49 unit tests compile fine without
   default features, so any feature-matrix/doctest CI lane for this crate
   goes red. Fix: either drop the `gpui_ui_kit, gpui_design` imports from
   the example (the metadata APIs it asserts are all feature-free), or add
   `#[cfg(feature = "ui")]`-guarded statements inside the example.

2. **Stability labels contradict each other across the crate's own
   reports** — `crates/gpui-toolkit/src/stability.rs:121` (and 154, 178,
   184) vs `crates/gpui-toolkit/src/release_notes.rs:256` (and 246, 276,
   286). The stability manifest marks `gpui-audio-kit`, `gpui-keybinding`,
   `gpui-themes`, and `gpui-ui-kit` as `StabilityLevel::Beta` (label
   `"beta"`), while the release-notes entries for the same four crates say
   `"release-candidate"` (ui-kit: `"release-candidate with
   keyboard/accessibility caveats"`). The two reports are both exposed as
   authoritative release-QA APIs, yet they disagree on the headline
   stability claim of four crates; release notes assembled from
   `release_notes_report()` will overstate maturity relative to
   `crate_stability_manifest()`. Fix: derive the release-notes `stability`
   field from the manifest (or at least add a test asserting the labels
   agree per crate).

3. **Deferred crates are misclassified as `Excluded` / `public-core` in the
   packaging report** — `crates/gpui-toolkit/src/release_packaging.rs:141-157`
   vs `crates/gpui-toolkit/src/publish_plan.rs:153-172`. `gpui-pretext` and
   `gpui-builder` are `Deferred` in lane `"deferred-registry"` in the publish
   plan, but the packaging report puts them in lane `"public-core"` with
   status `Excluded` — a status documented as "intentionally not published
   from this workspace" (release_packaging.rs:18-19). They are not excluded;
   they are postponed pending registry predecessors. The root cause is
   structural: `ReleasePackagingStatus` (release_packaging.rs:10-22) has no
   `Deferred` variant, so the only available non-passing non-blocking bucket
   is `Excluded`. Impact: `blocking_entries()` treats genuinely pending
   registry work as settled, and generated packaging tables contradict the
   publish plan. Fix: add a `Deferred` variant to `ReleasePackagingStatus`
   (non-release-ready, so it shows up in `blocking_entries()`), use it for
   these two rows, and align the lane label with the publish plan.

### Low

4. **Vendored-patch review cadence is already overdue and nothing enforces
   it** — `crates/gpui-toolkit/src/vendored_patches.rs:354-355` (gpui_wgpu),
   377-378 (gpui_windows), 628-629 (zed-font-kit). All three declare
   `review_cadence_days: 30` with `last_reviewed: "2026-07-12"`, i.e. due
   2026-08-11; as of this review (2026-08-25) they are two weeks past
   cadence. The struct models the cadence but no test or script compares
   `last_reviewed + cadence` against the current date, so staleness is
   invisible. Fix: extend `scripts/qa_docs_policy.py` (which already parses
   `vendored_patches.rs`, see scripts/qa_docs_policy.py:80) or a unit test
   with an injected "today" to flag overdue entries.

5. **No cross-check between the stability/publish manifests and
   `Cargo.toml`** — `crates/gpui-toolkit/Cargo.toml:43-61` vs
   `crates/gpui-toolkit/src/stability.rs:117-270`. The vendored-patch
   manifest has a filesystem ratchet (`vendored_patches.rs:754-785`
   `every_vendored_crate_dir_has_manifest_entry`), but nothing verifies that
   every optional dependency / feature in the aggregate's manifest has a
   stability entry and that `AggregateFeature::as_str()` names still match
   the `[features]` table. Adding an optional dep without a manifest entry
   would pass all 49 tests. Fix: a test that reads `Cargo.toml` via
   `CARGO_MANIFEST_DIR` (the same pattern the vendored tests already use)
   and asserts set equality between optional deps and manifest crate names.

6. **`DependencyHygieneReport::all_release_ready()` ignores advisory
   triage** — `crates/gpui-toolkit/src/dependency_hygiene.rs:127-131`. The
   method only folds over `checks`; a report whose `advisory_triage`
   contains `ReleaseBlocking` rows would still return `true`. Today no
   advisory is `ReleaseBlocking` so behavior is correct, but the name
   promises more than it delivers and the lib.rs doctest asserts it as a
   release-readiness signal. Fix: also require
   `self.blocking_advisories().next().is_none()`, or rename to
   `checks_release_ready()`.

7. **`ReleaseNotesArtifactReport` has no schema version of its own** —
   `crates/gpui-toolkit/src/release_notes.rs:452-459`. It reuses
   `RELEASE_NOTES_SCHEMA_VERSION` while having a distinct
   `RELEASE_NOTES_ARTIFACT_REPORT_TYPE`. An artifact-report shape change
   would be indistinguishable from a release-notes-report version bump.
   Fix: introduce `RELEASE_NOTES_ARTIFACT_SCHEMA_VERSION`.

8. **`artifacts_for` requires `&'static str`** —
   `crates/gpui-toolkit/src/release_notes.rs:123-130`. The filter parameter
   is `crate_name: &'static str`, so callers cannot look up a runtime-owned
   name (e.g. read from a file or CLI arg). The comparison itself needs no
   'static bound. Fix: change the parameter to `&str` (the iterator still
   returns `&'static ReleaseNotesArtifact`).

9. **Wave-1 public crate is only reachable through the `tooling`
   feature** — `crates/gpui-toolkit/Cargo.toml:31-38` and
   `crates/gpui-toolkit/src/stability.rs:222-229`. `gpui-profiler` is a
   `PublicCoreAfterGates` wave-1 crate, yet the aggregate exposes it only
   via `tooling` (classified `AggregateFeature::Tooling`), which also pulls
   in component-lab, design-tools, miniapp, python-runtime, and scaffolder.
   A consumer of the aggregate cannot depend on just the profiler. Fix:
   move `dep:gpui-profiler` into the `ui` or a new minimal feature, or
   reclassify the feature boundary.

### Informational / nits

- `PublishDecision::BetaAfterGates.as_str()` returns
  `"source-beta-after-gates"` (`stability.rs:90`) — the only label that
  doesn't match its variant name; harmless but inconsistent with the
  otherwise mechanical naming.
- Several status variants are never constructed in the current tables:
  `PublishPlanStatus::{BlockedByPredecessor, PendingDryRun}`,
  `ReleasePackagingStatus::{Blocked, Pending}`; one test even asserts
  `ReleaseQaStatus::{Pending, Blocked}` are absent
  (`release_qa.rs:659-662`). They are part of a versioned schema so keeping
  them is defensible, but they are dead code today.
- The report structs (`DependencyHygieneReport`, `PublishPlan`,
  `ReleaseQaMatrix`, `PlatformCapabilityMatrix`, `ReleaseNotesReport`,
  `ReleaseNotesArtifactReport`, `ReleasePackagingReport`,
  `VendoredPatchManifest`) have undocumented public fields while every
  entry/row struct documents each field — cosmetic doc inconsistency;
  `missing_docs` is not enabled so nothing flags it.
- `PublishPlanStatus::is_release_ready` counts `Deferred` and `Excluded` as
  release-ready (`publish_plan.rs:40-45`). That is intentional for gating
  *this* release, but combined with finding 3 it means "all release ready"
  can coexist with unpublished wave-2 crates; consumers should read it as
  "nothing blocks the current wave", which the doc comment could state.

## GPU/CPU data-flow notes

Not applicable: the crate contains no wgpu, rendering, or buffer code — only
`&'static` metadata tables and Markdown string builders. No GPU→CPU→GPU
cycles exist here.

## UI/UX consistency

Not applicable: the crate renders no UI. Its only user-visible output is
generated Markdown tables; those are consistent in structure across the six
reports (same header format, same `key: value` preamble), and all payload
strings are compile-time constants so there is no injection/escaping risk in
practice (no `|` or newline appears in any table payload).

## Clean bill

- **Memory/threading/panics**: all data is `const`/`&'static`; accessor
  functions are `const fn` returning slices; the only allocations are
  `String`/`format!` in the cold Markdown renderers. No `unsafe`, no locks,
  no channels, no `RefCell`. `unwrap`/`expect` appear only in `#[cfg(test)]`
  code. No production panic paths found.
- **Test quality**: 49 unit tests with genuine ratchets — unique-id checks
  for every table, label non-emptiness, ordering checks
  (`publish_plan.rs:276-288` asserts `order == index + 1`), a filesystem
  cross-check that every vendored directory with a provenance doc appears in
  the manifest (`vendored_patches.rs:754-785`), and negative assertions that
  blocking reports are not yet all-passed. `cargo test -p gpui-toolkit`
  passes 49/49 unit tests plus the doctest (default features).
