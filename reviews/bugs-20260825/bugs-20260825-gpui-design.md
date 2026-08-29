# Bug Review: gpui-design — 2026-08-25


## Completion audit — 2026-08-26

- [x] The cache/equality, complete token export, constructor validation, reduced-motion conformance, serde persistence, cross-platform font, and documentation findings are resolved in the follow-up section above.
- [x] Empty shared Cargo feature aliases are deliberate workspace compatibility surface, not unused crate features.

Verified cargo test -p gpui-design (43 passed).
Scope: full scan of `crates/gpui-design` — all 22 files under `src/` (~2,600 LOC,
including the 510-line test module), plus `Cargo.toml`, `README.md`, and
`AGENTS.md`. This is a pure-data crate (design tokens/presets + CI-facing
conformance/documentation report builders, and a `OnceLock`-cached style-token
export); there is no rendering, GPU, async, or threading code beyond
`OnceLock`/`Arc`. Baseline verified: `cargo test -p gpui-design --lib` →
38 passed, 0 failed. The earlier perf review
(`reviews/perf-gpui-design-20260822.md`) findings 1–2 (Clone dropping the token
cache, split/join in `token()`) are already fixed in the current code and are
not re-reported.

## Findings

Ranked by severity. No Critical or High issues found — this is a data crate with
no production hot paths, so the worst bugs are stale-state and API-semantics
surprises.

## Resolved during follow-up (2026-08-26)

- **Stale token exports and cache-dependent equality:** removed the un-invalidatable `OnceLock` cache. `style_dictionary_tokens()` now returns a current `Arc` snapshot after public fields are edited, and `DesignSystem` equality no longer depends on export-call history. The old `style_dictionary_tokens_ref` name remains only as a deprecated `Arc`-returning migration helper because a borrow cannot be made safe across public mutation.
- **Incomplete Style Dictionary coverage:** export now contains all 51 public rule values, including extra radii/style, elevation offset, full typography/motion/audio geometry, layout thresholds, and categorical choices. The conformance gate requires that exact count and regression coverage checks the formerly omitted fields.
- **Constructor/conformance disagreement:** spacing grid units require a finite positive value; audio geometry validates finite `(0, 360]` sweeps, at least 12 arc segments, and positive finite slider tracks; animation constructors enforce duration order and finite spring values. These match the report invariants before malformed data reaches consumers.
- **Dead reduced-motion conformance branch:** removed the tautological check. `motion_spec(true)` is directly tested as the authoritative output policy, while conformance continues to validate raw animation rule ordering and spring inputs.
- **One-way persistence claim:** `DesignSystem`, all rule types, and design enums now support `Deserialize`; typography stores an owned `String`, enabling a complete serde round trip.
- **Redundant opacity finiteness check:** removed because the inclusive range test already rejects `NaN` and infinities.
- **Cross-platform font tokens:** Neutral now requests GPUI's `system-ui` generic family and Material 3 requests `Roboto`; Apple retains `.SystemUIFont`.
- **Documentation drift:** `AGENTS.md` now describes the actual modular source layout and all seven presets; README documents current token snapshots and the neutral system font.
- **Manifest feature flags (disproved):** the intentionally empty workspace feature names are repeated across first-party crates so unified workspace QA feature sets remain source-compatible. They are not crate-local optional implementations to remove.

Verification: `cargo test -p gpui-design --lib` (43 passed) and `cargo check -p gpui-design --features gpui` pass.

### Medium

1. **Stale token cache after public-field mutation** — `src/design_system.rs:626-636`.
   `style_dictionary_tokens()` / `style_dictionary_tokens_ref()` compute tokens
   once via `OnceLock::get_or_init` and never invalidate. All rule fields on
   `DesignSystem` are `pub`, and mutation is an explicitly supported pattern
   (`tests.rs:402` `conformance_report_catches_mutated_public_fields` mutates
   fields directly). So `let mut ds = DesignSystem::neutral();
   ds.style_dictionary_tokens(); ds.spacing.grid_unit = 8.0;` leaves the export
   permanently reporting `grid_unit = 4`. Impact: exported Style Dictionary
   tokens can silently disagree with the live design system, and the
   `tokens.coverage`/`token_count` values in conformance and documentation
   reports can describe pre-mutation state. Fix (pick one): make the rule
   fields private with cache-invalidating setters; take the cache out of the
   struct and memoize per `(preset, fingerprint)`; or drop the cache — the
   build is ~32 small allocations at export time, which the 2026-08-22 perf
   review already classed as negligible.

2. **`PartialEq` result depends on cache state** — `src/design_system.rs:31`
   (`#[derive(PartialEq)]` on a struct containing `OnceLock<Arc<[DesignToken]>>`
   at line 59). `std::sync::OnceLock: PartialEq` compares `self.get() ==
   other.get()`, so an untouched `DesignSystem::neutral()` compares **unequal**
   to an identical `DesignSystem::neutral()` whose token cache happens to be
   initialized (e.g. because `conformance_report()` ran — note line 937
   initializes the cache as a side effect). Impact: change-detection,
   snapshot, or test code using `==` on `DesignSystem` gets false negatives
   depending on incidental call history. Fix: implement `PartialEq` manually
   comparing only the rule fields (the derived `Debug` has the same
   cache-state leakage, lower stakes).

3. **Style-dictionary export silently omits public fields** —
   `src/design_system.rs:638-761`. `build_style_dictionary_tokens()` exports 32
   tokens but drops `corners.xl`, `corners.style`, `typography.small_size`,
   `typography.large_size`, `animation.spring_stiffness`,
   `animation.spring_damping`, `elevation.shadow_y_offset`,
   `audio_controls.knob_border_width`, the entire `LayoutThresholds` struct (8
   fields), and the categorical fields `toggle_variant`, `label_position`,
   `group_separator`. The README sells this as a "stable Style
   Dictionary-friendly export for tooling", and the conformance gate only
   requires `token_count >= 24` (`src/design_system.rs:937-943`), so a consumer
   regenerating a design system from tokens loses roughly a third of the
   struct. Fix: export the remaining fields (layout thresholds at minimum, the
   categorical fields as string tokens), and tighten the coverage gate to the
   exact expected count so dropped tokens fail CI.

### Low

4. **Dead conformance check: `motion.reduced` can never fire** —
   `src/design_system.rs:892-900`. The check asserts that
   `motion_spec(true)` returns zeroed durations, but `motion_spec`
   (`src/design_system.rs:948-966`) constructs those zeros unconditionally when
   `reduced_motion` is set — the finding validates the helper against itself
   and is tautologically false. Fix: either delete the check, or check the raw
   `self.animation` fields against a reduced-motion policy that has actual
   teeth (e.g. warn when `slow_ms` exceeds an accessibility ceiling).

5. **Constructor validation and conformance validation disagree** —
   `src/spacing_rules.rs:29` allows `grid_unit == 0.0` while conformance
   requires it positive (`finite_positive`, `src/design_system.rs:767-772`);
   `AudioControlRules::new` (`src/audio_control_rules.rs:21-43`) does not check
   `knob_arc_sweep_deg` at all while conformance requires `(0, 360]`
   (`src/design_system.rs:911-919`); `AnimationRules::new`
   (`src/animation_rules.rs:29-33`) doesn't check `fast <= duration <= slow`
   while conformance does (`src/design_system.rs:871-878`). And since all
   fields are `pub`, both layers are bypassable anyway. Fix: pick one source of
   truth — ideally validate in the constructors (or a single `validate()`),
   since the panicking `assert!`s in these `new()`s are reachable in production
   if values ever come from deserialized config.

6. **`Serialize` without `Deserialize` despite the persistence claim** —
   `src/design_system.rs:31`, `src/types.rs`, all rule structs. README/AGENTS
   say "Serializable via serde for persistence", but nothing in the crate
   derives `Deserialize`, so a `DesignSystem` cannot actually be restored
   (`TypographyRules.font_family: Cow<'static, str>` is the likely blocker —
   `Cow<'static, str>: Deserialize` is unsatisfiable for borrowed data). Fix:
   derive `Deserialize` where possible (switching `font_family` to
   `Cow<'de, str>`-compatible storage or `String`), or soften the docs to
   "serializable (one-way) for export/reporting".

7. **Dead feature flags in the manifest** — `Cargo.toml:15-23` declares
   `autoeq`, `gpu-2d`, `gpu-3d`, `reqwest`, `showcase`, `spinorama`, `tokio`,
   and `urlencoding`, none of which gate any code or optional dependency in
   this crate (only `gpui` is real). Looks copied from a sibling crate.
   Impact: downstream `features = ["gpu-2d"]` compiles silently while enabling
   nothing. Fix: delete the unused flags.

8. **Redundant finiteness check** — `src/design_system.rs:838-839`:
   `!(0.0..=1.0).contains(&opacity) || !opacity.is_finite()` — `RangeInclusive::contains`
   already returns `false` for NaN and ±∞, so the second clause is dead.
   Trivial cleanup.

9. **Neutral preset ships an Apple-specific font token** —
   `src/design_system.rs:129` (and `material3()` at line 273) use
   `font_family: ".SystemUIFont"` while Neutral is documented as the
   cross-platform default. Codified by `tests.rs:129-142`, so presumably
   intentional as "matches existing hardcoded values", but on Windows/Linux a
   Neutral-design app will miss this family name and fall back opaquely in the
   renderer. Worth a comment or a per-platform fallback chain.

10. **Doc drift** — `crates/gpui-design/AGENTS.md:9` still claims "lib.rs —
    All types in a single file" (the crate is now split across 22 modules) and
    line 3 lists only "Apple HIG, Material 3, Fluent, Neutral", missing
    Adwaita/Breeze/Carbon that `README.md` and the code cover. Fix: refresh
    AGENTS.md to match the module layout in `src/lib.rs:12-35`.

## GPU/CPU data-flow notes

None applicable — the crate contains no wgpu/rendering code (confirmed by grep:
no `map_async`, `device.poll`, `pollster`, buffer or texture creation). Corner
radii, elevation, and arc-segment counts are consumed as plain values by
downstream renderers; `audio_controls.knob_arc_segments = 48` for every preset
is the only GPU-adjacent knob (arc tessellation density) and is consistent
across presets.

## UI/UX consistency

The crate renders no UI itself, but its tokens drive sibling components. Preset
values are internally consistent and match their README descriptions
(44px Apple / 48px Material touch targets, Fluent 4px/8px corners, Carbon
square/flat, per-preset label positions and toggle variants all checked against
`src/design_system.rs`). The only cross-preset inconsistency found is the
shared `.SystemUIFont` font family on non-Apple presets (finding 9). Focus-ring
and reduced-motion tokens exist per preset, so downstream ARIA/focus handling
has the data it needs.

## Clean bill

- Threading: `OnceLock`/`Arc` usage is correct — `Clone` now carries the cache
  (`src/design_system.rs:62-84`), the `cx.design()` fallback is a
  `static OnceLock` (`src/design_system_state.rs:42-47`), no locks, channels,
  `RefCell`, or blocking calls anywhere.
- Allocation: per-paint path (`cx.design()`) is an atomic refcount bump only;
  report/token builders allocate freely but run once per CI/export, already
  covered by the 2026-08-22 perf review.
- Panic paths: the only `panic!`/`assert!`s are constructor validation
  (finding 5); no `unwrap()` outside tests except a `should_panic`-covered one.
- Preset math: ordering invariants (typography scale, motion durations, slider
  heights, threshold ladders) verified for all 7 presets by the conformance
  matrix; markdown table column counts in the three report builders match
  their headers.
- Tests: 38 pass, covering presets, platform mapping, conformance, cache
  pointer-stability, serialization, and release-presentation assets.
