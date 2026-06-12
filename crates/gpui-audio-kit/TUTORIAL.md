# gpui-audio-kit Tutorial

`gpui-audio-kit` provides audio-focused GPUI controls for plugin and playback
interfaces.

## 1. Add the crate

```toml
[dependencies]
gpui = { workspace = true }
gpui-audio-kit = { workspace = true }
gpui-ui-kit = { workspace = true }
```

## 2. Pick a control

The crate exports controls such as:

- `Potentiometer`
- `VerticalSlider`
- `VolumeKnob`
- meter and spectrum helpers
- audio scale and tick rendering helpers

## 3. Render a control

```rust
use gpui::*;
use gpui_audio_kit::VolumeKnob;

struct MixerStrip {
    gain_db: f32,
}

impl Render for MixerStrip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_3()
            .child(VolumeKnob::new("gain", self.gain_db))
    }
}
```

If you need a fully wired example with state and theme handling, start from:

```bash
cargo run -p gpui-audio-kit --example volume_knob_debug
cargo run -p gpui-audio-kit --example potentiometer_debug
cargo run -p gpui-audio-kit --example vertical_slider_debug
```

## 4. Connect to audio state

1. Store parameter values in your GPUI entity.
2. Convert display units, such as decibels or frequency, before rendering.
3. On value changes, update the entity state.
4. Send normalized values to the audio engine through a lock-free or host-safe
   control path.
5. Avoid doing DSP work inside UI callbacks.

## 5. Verify

```bash
cargo test -p gpui-audio-kit
cargo build --examples -p gpui-audio-kit
```
