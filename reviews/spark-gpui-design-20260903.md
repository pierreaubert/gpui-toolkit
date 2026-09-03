# Code Review: gpui-design — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-design` (24 files, ~2.8k LOC)

## 1. Purpose / role
Platform-adaptive design system, data-only: shape/spacing/type/motion/elevation tokens per platform (Apple HIG, M3, Fluent, Adwaita, Breeze, Carbon, Neutral). `gpui` feature adds `DesignSystemState` global + `cx.design()`. Core: `design_system.rs` (~1000+), `*_rules.rs`, `design_token.rs`, `design_token_export.rs`, conformance/report, `design_ext.rs`, `design_system_state.rs` (50).

Public API: `DesignSystem::{neutral,apple_hig,material3,fluent,adwaita,breeze,carbon,platform_default,for_platform}`, `DesignLanguage/DesignPlatform`, `SpacingRules/CornerRadii/TypographyRules/AnimationRules/ElevationRules/InteractionRules/LayoutThresholds/AudioControlRules`, `DesignToken/token()`, `style_dictionary_tokens/build_style_dictionary_tokens`, `conformance_report/motion_spec`, `DesignSystemState`, `DesignExt::design()`.

## 2. SOTA gap analysis (vs Tailwind, M3 tokens, Fluent, ArkUI)
1. **No fluid type/spacing** (`clamp()`, container-query tokens) — all `f32` px constants.
2. **No state-layer/opacity system** (M3 hover 8% overlays) or elevation-tint; `elevation_rules.rs` is blur/opacity only.
3. **No density tiers** (M3 compact/medium) or dynamic-type ramp beyond base/small/large.
4. **No RTL-flip metadata** on spacing/corner tokens.
5. **Conformance is CI markdown, not a gate** (`design_system.rs:847`) — no breaking-token lint.
6. **Style-Dictionary emitter unverified** (`design_system.rs:600,614`, 231 lines, fan-out 54) — no round-trip test with `gpui-themes`.
7. **No reduced-motion policy** despite `prefer_spring` + stiffness/damping fields.

## 3. Performance evaluation
Trivial runtime: constructors are ~69-line struct literals (`design_system.rs:66,137,208,279,350,421`); `token()` (`design_token.rs:32`, fan-in 51) is a lookup. Only outliers are cold paths: `build_style_dictionary_tokens` (`:614`, risk 86) and `conformance_report` (`:847`, 173 lines, risk 225), both untested. Coverage 2% (1/58); top risk `design_language.rs:35 as_str()` (fan-in 13) is drift risk, not speed. No unsafe/unwrap in prod paths; sole global is opt-in `DesignSystemState`.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Add density + fluid-type tokens (`clamp(min,scale,max)`) + M3 state-layer opacities | M | expressive-scale parity |
| 2 | Emit Tailwind/Style-Dictionary/CSS-vars from `:614`; round-trip test with themes | M | cross-framework interop |
| 3 | Add RTL-flip flags; test mirrored layouts | S | i18n parity |
| 4 | Turn `conformance_report:847` into breaking-change gate | S | prevents token drift |
| 5 | Unit-test `as_str/from_id` (`design_language.rs:35,47`) and `for_platform:578` | S | cheapest risk cut |

## 5. Verdict
Clean data-only design. SOTA work is token expressiveness + exporters + gates. No perf action — keep it allocation-free.
