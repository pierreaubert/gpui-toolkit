# gpui-design

Platform-adaptive design system (Apple HIG, Material 3, Fluent, Neutral).

## Architecture

Pure data types only — no rendering code, no framework dependencies. Platform renderers consume these values alongside Theme colors.

- `lib.rs` — All types in a single file: `DesignLanguage`, `DesignSystem`, and sub-structs
- `CornerRadii` — sm/md/lg/xl radius + continuous (squircle) vs circular style
- `SpacingRules` — grid unit, control padding, gaps, card padding
- `InteractionRules` — min touch target, border/focus ring widths
- `ElevationRules` — shadow blur/opacity per elevation level
- `AnimationRules` — duration tiers, spring stiffness/damping, `prefer_spring` flag
- `TypographyRules` — font family, base/small/large sizes, dynamic sizing
- `LayoutThresholds` — breakpoints for layout solver adaptations
- `AudioControlRules` — knob arc geometry, slider track widths

## Key Public API

- `DesignSystem::neutral()` / `::apple_hig()` / `::material3()` / `::fluent()` — preset constructors
- `DesignSystem::platform_default()` — auto-selects based on `target_os`
- `DesignLanguage` enum — `AppleHig`, `Material3`, `Fluent`, `Neutral`
- With `gpui` feature: `DesignSystemState` global + `DesignExt` trait (`cx.design()`)

## Features

- `gpui` — enables `DesignSystemState` global and `DesignExt` trait for GPUI integration

## Testing

```bash
cargo test -p gpui-design --lib
cargo check -p gpui-design --features gpui
```

## Important Notes

- Design System handles shape/spacing/animation; Theme handles colors — the two are independent
- `MiniApp` (from `gpui-ui-kit`) automatically sets `DesignSystemState` on startup
- All values are `f32` pixel values, ready for direct use in GPUI `px()` calls
- Serializable via serde for persistence
