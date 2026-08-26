# gpui-design

Platform-adaptive design system for GPUI applications.

Defines shape, spacing, interaction, and animation rules that vary per platform while the Theme system handles colors independently. The two layers are independently combinable: any color theme works with any design system.

## Presets

| Preset | Platform | Key traits |
|--------|----------|------------|
| `DesignSystem::neutral()` | Cross-platform default | Generic `system-ui` font with platform-neutral spacing and controls |
| `DesignSystem::apple_hig()` | macOS 26 / iOS | Liquid Glass-inspired floating controls, continuous rounded corners, 44px touch targets, fluid spring motion |
| `DesignSystem::material3()` | Android / ChromeOS | 48px touch targets, card separators, Roboto |
| `DesignSystem::fluent()` | Windows 11 | 4px control corners, 8px overlay corners, compact Mica/Acrylic-inspired elevation, Segoe UI Variable |
| `DesignSystem::adwaita()` | GNOME / GTK / Libadwaita | Adwaita Sans, broad rounded controls, accessible pointer/touch sizing |
| `DesignSystem::breeze()` | KDE / Kirigami | Noto Sans, compact Breeze controls, border-separated groups, fast standard motion |
| `DesignSystem::carbon()` | IBM Carbon | Square corners, productive spacing, IBM Plex, flat layered surfaces |
| `DesignSystem::platform_default()` | Auto-detect | Selects based on `target_os` |

## Usage

```rust
use gpui_design::{DesignSystem, DesignLanguage};

let ds = DesignSystem::platform_default();

// Spacing
let padding = ds.spacing.card_padding;      // 12px (Neutral), 16px (Apple)
let gap     = ds.spacing.control_gap;        // 8px

// Corners
let radius  = ds.corners.md;                 // 8px (Neutral), 10px (Apple)

// Typography
let size    = ds.typography.base_size;        // 14px (Neutral), 15px (Apple)

// Animation
let dur     = ds.animation.duration_ms;       // 200ms (Neutral), 350ms (Apple)
let spring  = ds.animation.prefer_spring;     // false (Neutral), true (Apple)
```

## GPUI Integration

Enable the `gpui` feature for `DesignSystemState` global and `DesignExt` trait:

```toml
gpui-design = { version = "0.6", features = ["gpui"] }
```

```rust
use gpui_design::DesignExt;

// In any Render impl:
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let ds = cx.design(); // returns DesignSystem::platform_default() if no global set

    div()
        .p(px(ds.spacing.card_padding))
        .rounded(px(ds.corners.md))
        .text_size(px(ds.typography.base_size))
}
```

`MiniApp` (from `gpui-ui-kit`) automatically sets the `DesignSystemState` global on startup.

## Design System vs Theme

| Concern | Layer | Crate |
|---------|-------|-------|
| Colors, accents, backgrounds | **Theme** | `gpui-ui-kit` (`cx.theme()`) |
| Spacing, corners, touch targets | **Design System** | `gpui-design` (`cx.design()`) |
| Animation timing, spring physics | **Design System** | `gpui-design` |
| Typography sizes, font family | **Design System** | `gpui-design` |
| Shadow/elevation model | **Design System** | `gpui-design` |

Linux defaults to `DesignSystem::adwaita()` because GNOME's HIG is the GTK 4 /
Libadwaita baseline. KDE-style applications can opt into
`DesignSystem::breeze()` explicitly.

Carbon is also opt-in rather than selected by `platform_default()`, because it
is a product design language instead of an operating-system default. Pair
`DesignSystem::carbon()` with one of the Carbon theme variants from
`gpui-ui-kit` (`CarbonWhite`, `CarbonGray10`, `CarbonGray90`, or
`CarbonGray100`) for the complete look.

## Conformance Gate

`DesignConformanceMatrix::all_presets()` validates every built-in preset in
standard and reduced-motion modes. It checks touch-target rules, typography
ordering, spacing/radius sanity, motion duration ordering, reduced-motion
collapse, audio-control geometry, and token export coverage.

```rust
use gpui_design::DesignConformanceMatrix;

let matrix = DesignConformanceMatrix::all_presets();
assert!(matrix.passed(), "{}", matrix.to_markdown_table());
```

`DesignDocumentationReport::for_all_presets()` packages the same gate into a
stable, serializable docs artifact with `schema_version = 1`,
`report_type = "gpui-design-documentation"`, preset summaries, the full
conformance matrix, and generated Markdown. CI can serialize this report as JSON
for machine checks and publish the Markdown as release documentation.

```rust
use gpui_design::DesignDocumentationReport;

let report = DesignDocumentationReport::for_all_presets();
assert!(report.passed(), "{}", report.markdown);
```

`DesignReleasePresentation::for_all_presets()` records the release-note
attachment story for those generated docs. It links the JSON report, Markdown
report, release-note excerpt, and one screenshot slot per built-in preset under
stable `release/gpui-design/...` paths. Generated text/report assets are marked
ready, while screenshot rows remain explicit capture gates for release QA.

```rust
use gpui_design::DesignReleasePresentation;

let presentation = DesignReleasePresentation::for_all_presets();
assert_eq!(presentation.generated_assets().len(), 3);
```

`DesignTokenExport::for_all_presets()` returns a serializable Style
Dictionary-friendly export for tooling and future Figma integration.
`DesignSystem::style_dictionary_tokens()` returns a fresh `Arc` snapshot so
exports remain correct after an application edits its public design-rule
fields. The former borrowed-cache accessor is deprecated because a borrowed
value cannot safely remain current across such edits.

## Sub-structs

- `CornerRadii` — sm/md/lg/xl radius + continuous vs circular style
- `SpacingRules` — grid unit, control padding, gaps, card padding
- `InteractionRules` — min touch target, border/focus ring widths
- `ElevationRules` — shadow blur/opacity per elevation level
- `AnimationRules` — duration tiers, spring stiffness/damping
- `TypographyRules` — font family, base/small/large sizes, dynamic sizing
- `LayoutThresholds` — breakpoints for layout solver adaptations
- `AudioControlRules` — knob arc geometry, slider track widths

## Testing

```bash
cargo test -p gpui-design --lib
cargo check -p gpui-design --features gpui
```
