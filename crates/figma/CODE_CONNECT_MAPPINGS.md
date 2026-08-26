# Code Connect Mappings — SOTF gpui-toolkit

Figma file: https://www.figma.com/design/c7bS9PCykhmP2pjpaT0zKu
File key: `c7bS9PCykhmP2pjpaT0zKu`

## Mappings to apply via Figma MCP `add_code_connect_map`

All use `label: "Swift"` (closest to Rust in the allowed list).

| Node ID | Component Name | Source File |
|---------|---------------|-------------|
| 1:289 | Button | crates/gpui-ui-kit/src/button.rs |
| 1:455 | Input | crates/gpui-ui-kit/src/input.rs |
| 1:493 | NumberInput | crates/gpui-ui-kit/src/number_input.rs |
| 1:531 | Select | crates/gpui-ui-kit/src/select.rs |
| 1:569 | Checkbox | crates/gpui-ui-kit/src/checkbox.rs |
| 1:611 | Toggle | crates/gpui-ui-kit/src/toggle.rs |
| 1:655 | Slider | crates/gpui-ui-kit/src/slider.rs |
| 1:685 | Badge | crates/gpui-ui-kit/src/badge.rs |
| 1:734 | Alert | crates/gpui-ui-kit/src/alert.rs |
| 1:786 | InlineAlert | crates/gpui-ui-kit/src/alert.rs |
| 1:803 | Toast | crates/gpui-ui-kit/src/toast.rs |
| 1:841 | Progress | crates/gpui-ui-kit/src/progress.rs |
| 1:873 | CircularProgress | crates/gpui-ui-kit/src/progress.rs |
| 1:892 | Spinner | crates/gpui-ui-kit/src/spinner.rs |
| 1:923 | Tabs | crates/gpui-ui-kit/src/tabs.rs |
| 1:956 | ButtonSet | crates/gpui-ui-kit/src/button_set.rs |
| 1:984 | Breadcrumbs | crates/gpui-ui-kit/src/breadcrumbs.rs |
| 1:1021 | Menu | crates/gpui-ui-kit/src/menu.rs |
| 1:1072 | Accordion | crates/gpui-ui-kit/src/accordion.rs |
| 1:1103 | Tooltip | crates/gpui-ui-kit/src/tooltip.rs |
| 1:1126 | Card | crates/gpui-ui-kit/src/card.rs |
| 1:354 | IconButton | crates/gpui-ui-kit/src/icon_button.rs |
| px:scatter | ScatterChart | crates/gpui-px/src/scatter.rs |
| px:line | LineChart | crates/gpui-px/src/line.rs |
| px:bar | BarChart | crates/gpui-px/src/bar.rs |
| px:area | AreaChart | crates/gpui-px/src/area.rs |
| px:heatmap | HeatmapChart | crates/gpui-px/src/heatmap.rs |
| px:contour | ContourChart | crates/gpui-px/src/contour.rs |
| px:isoline | IsolineChart | crates/gpui-px/src/isoline.rs |
| px:pie | PieChart | crates/gpui-px/src/pie.rs |
| px:boxplot | BoxPlotChart | crates/gpui-px/src/boxplot.rs |
| px:treemap | Treemap | crates/gpui-px/src/treemap.rs |
| audio:potentiometer | Potentiometer | crates/gpui-audio-kit/src/audio/potentiometer.rs |
| audio:vertical-slider | VerticalSlider | crates/gpui-audio-kit/src/audio/vertical_slider.rs |
| audio:volume-knob | VolumeKnob | crates/gpui-audio-kit/src/audio/volume_knob.rs |
| audio:level-meter | LevelMeterElement | crates/gpui-audio-kit/src/meter.rs |
| audio:horizontal-meter | HorizontalMeterTheme + render_horizontal_meter_bar | crates/gpui-audio-kit/src/meter.rs |
| audio:spectrum | SpectrumElement | crates/gpui-audio-kit/src/spectrum.rs |
| audio:spectrum-axis | SpectrumAxisTheme + spectrum axis render helpers | crates/gpui-audio-kit/src/spectrum.rs |

The runtime story registry for design review and responsive matrices lives in
`crates/gpui-component-lab`. Current first-party story ids are
`ui-kit.button`, `ui-kit.form`, `ui-kit.status`, `ui-kit.navigation`,
`ui-kit.feedback`, `ui-kit.card`, `px.line`, `px.bar`, `px.scatter`,
`px.area`, `px.heatmap`, `px.contour`, `px.isoline`, `px.pie`,
`px.boxplot`, `px.treemap`, `audio-kit.potentiometer`,
`audio-kit.vertical-slider`, `audio-kit.volume-knob`, `audio-kit.meter`,
`audio-kit.horizontal-meter`, `audio-kit.spectrum`, and
`audio-kit.spectrum-axis`. Token exports and validation live in
`crates/gpui-design-tools`.

## Code Connect Examples

When `get_design_context` returns a Button instance, generate:
```rust
Button::new("id", "Label")
    .variant(ButtonVariant::Primary)  // from Figma variant property
    .size(ButtonSize::Md)             // from Figma size property
```

When `get_design_context` returns an Input instance, generate:
```rust
Input::new("id")
    .placeholder("Type here...")
    .variant(InputVariant::Default)
    .size(InputSize::Md)
```

When `get_design_context` returns a Toggle, generate:
```rust
Toggle::new("id")
    .checked(checked)
    .style(ToggleStyle::Sliding)
    .size(ToggleSize::Md)
    .label("Label")
```

When `get_design_context` returns a Badge, generate:
```rust
Badge::new("text")
    .variant(BadgeVariant::Primary)
    .size(BadgeSize::Md)
```

When `get_design_context` returns an Alert, generate:
```rust
Alert::new("id", "Message")
    .variant(AlertVariant::Info)
    .title("Title")
    .closeable(true)
```

When `get_design_context` returns Tabs, generate:
```rust
Tabs::new("id")
    .tabs(vec![
        TabItem::new("tab1", "General"),
        TabItem::new("tab2", "Audio"),
    ])
    .variant(TabVariant::Underline)
```

When `get_design_context` returns a Select, generate:
```rust
Select::new("id").options(vec![
    SelectOption::new("opt1", "Option One"),
    SelectOption::new("opt2", "Option Two"),
])
.size(SelectSize::Md)
```

When `get_design_context` returns a VStack/HStack layout, generate:
```rust
VStack::new()
    .spacing(StackSpacing::Md)
    .align(StackAlign::Stretch)
    .children(vec![...])
```

When `get_design_context` returns an audio control, generate imports from
`gpui_audio_kit`:
```rust
use gpui_audio_kit::{
    AudioScale, Potentiometer, PotentiometerSize, VerticalSlider, VerticalSliderSize,
    VolumeKnob,
};

Potentiometer::new("frequency")
    .label("Frequency")
    .value(1000.0)
    .min(20.0)
    .max(20_000.0)
    .scale(AudioScale::Logarithmic)
    .size(PotentiometerSize::Md)

VerticalSlider::new("gain")
    .label("Gain")
    .value(-6.0)
    .min(-60.0)
    .max(6.0)
    .unit("dB")
    .with_ticks()
    .size(VerticalSliderSize::Lg);

VolumeKnob::new()
    .id("output")
    .label("Output")
    .value(0.72)
    .muted(false);
```

When `get_design_context` returns a horizontal meter or spectrum axis, keep the
imports in `gpui_audio_kit` and map visual settings from `DesignSystem`/theme
tokens:
```rust
use gpui_audio_kit::{
    HorizontalMeterTheme, SpectrumAxisTheme, TickConfig, render_horizontal_meter_bar,
    render_spectrum_db_axis, render_spectrum_frequency_axis,
};

render_horizontal_meter_bar(
    "LUFS",
    -18.0,
    &TickConfig::lufs(),
    HorizontalMeterTheme::default(),
);

render_spectrum_db_axis(SpectrumAxisTheme::default());
render_spectrum_frequency_axis(20.0, 20_000.0, SpectrumAxisTheme::default());
```

When `get_design_context` returns a chart surface, generate responsive
`gpui_px` builders by default and keep fixed pixel sizing as an explicit opt-in:
```rust
use gpui_px::{ColorScale, heatmap, line, scatter};

line(&x, &y)
    .title("Frequency Sweep")
    .fill()
    .min_size(320.0, 220.0)
    .aspect_ratio(1.6)
    .build()?;

scatter(&x, &y)
    .fill()
    .min_size(320.0, 220.0)
    .build()?;

heatmap(&z, width, height)
    .color_scale(ColorScale::Viridis)
    .fill()
    .min_size(320.0, 220.0)
    .build()?;
```
