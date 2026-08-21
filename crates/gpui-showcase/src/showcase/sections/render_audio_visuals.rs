use super::prelude::*;

/// Showcase for the custom-painted audio surfaces. The first row deliberately
/// uses ordinary constructors so it proves the public default is Vello; the
/// matrix row exercises the deterministic CPU renderer and the Legacy escape
/// hatch without changing the normal story.
impl Showcase {
    pub(crate) fn render_audio_visuals_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let default_renderer = if Renderer2D::default().is_vello() {
            "Vello · Auto (CPU fallback)"
        } else {
            "Legacy (Vello feature unavailable)"
        };
        let magnitudes: Vec<f32> = (0..32)
            .map(|index| -54.0 + ((index as f32 * 0.47).sin() + 1.0) * 24.0)
            .collect();

        let default_spectrum = SpectrumElement::new(magnitudes.clone())
            .height(px(150.0))
            .bar_gap(px(1.0));
        let default_meters = div()
            .flex()
            .items_end()
            .gap(px(10.0))
            .child(
                div()
                    .w(px(34.0))
                    .h(px(150.0))
                    .child(LevelMeterElement::new(-12.0, "L").peak(-3.0)),
            )
            .child(
                div()
                    .w(px(34.0))
                    .h(px(150.0))
                    .child(LevelMeterElement::new(-18.0, "R").peak(-6.0)),
            );
        let horizontal_theme = HorizontalMeterTheme {
            use_gradient: true,
            ..Default::default()
        };
        let default_horizontal = render_horizontal_meter_bar_with(
            "LUFS",
            0.68,
            rgba(0x38bdf8ff),
            "-10.2 dB",
            horizontal_theme,
        );
        let default_pot = Potentiometer::new("audio-showcase-pot")
            .value(63.0)
            .min(0.0)
            .max(100.0)
            .unit("%")
            .label("Pan")
            .size(PotentiometerSize::Md);
        let default_volume = VolumeKnob::new()
            .id("audio-showcase-volume")
            .value(0.72)
            .label("Monitor")
            .size(px(72.0));

        let cpu_spectrum = SpectrumElement::new(magnitudes.clone())
            .height(px(110.0))
            .renderer_2d(Renderer2D::Vello)
            .vello_backend(VelloBackend::Cpu);
        let legacy_spectrum = SpectrumElement::new(magnitudes)
            .height(px(110.0))
            .renderer_2d(Renderer2D::Legacy);
        let cpu_knob = VolumeKnob::new()
            .id("audio-showcase-cpu-volume")
            .value(0.45)
            .label("CPU")
            .size(px(60.0))
            .renderer_2d(Renderer2D::Vello)
            .vello_backend(VelloBackend::Cpu);
        let legacy_knob = VolumeKnob::new()
            .id("audio-showcase-legacy-volume")
            .value(0.45)
            .label("Legacy")
            .size(px(60.0))
            .renderer_2d(Renderer2D::Legacy);

        div()
            .id("audio-visuals-section")
            .flex()
            .flex_col()
            .gap_4()
            .child(Heading::h2("Audio Visuals"))
            .child(
                div()
                    .id("audio-renderer-metadata")
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(format!("Default renderer: {default_renderer}")),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Default constructors"))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Spectrum")
                                    .child(default_spectrum),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Meters")
                                    .child(default_meters),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .w(px(260.0))
                                    .child("Horizontal meter")
                                    .child(default_horizontal),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Potentiometer")
                                    .child(default_pot),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Volume knob")
                                    .child(default_volume),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Renderer QA matrix"))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Vello · CPU")
                                    .child(cpu_spectrum),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Legacy")
                                    .child(legacy_spectrum),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Vello · CPU")
                                    .child(cpu_knob),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Legacy")
                                    .child(legacy_knob),
                            ),
                    ),
            )
    }
}
