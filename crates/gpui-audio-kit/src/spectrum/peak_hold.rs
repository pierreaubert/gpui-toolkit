//! Display-only peak envelopes, measured in seconds and normalized height.

#[derive(Default)]
pub(super) struct SpectrumPeakHold {
    pub heights: Vec<f32>,
    deadlines: Vec<f64>,
    last_update: Option<f64>,
}

impl SpectrumPeakHold {
    pub fn update(&mut self, values: &[f32], now: f64) {
        let previous = self.last_update.unwrap_or(now);
        let elapsed = (now - previous).max(0.0);
        if self.heights.len() != values.len() || elapsed > 0.5 {
            self.heights.clear();
            self.heights.extend_from_slice(values);
            self.deadlines.resize(values.len(), now + 1.0);
            self.deadlines.fill(now + 1.0);
        }
        for (index, &value) in values.iter().enumerate() {
            if value >= self.heights[index] {
                self.heights[index] = value;
                self.deadlines[index] = now + 1.0;
            } else {
                let decay_seconds = (now - previous.max(self.deadlines[index])).max(0.0);
                // Fall 20 dB/s after holding for one second; the chart spans 103 dB.
                self.heights[index] =
                    (self.heights[index] - decay_seconds as f32 * 20.0 / 103.0).max(value);
            }
        }
        self.last_update = Some(now);
    }
}
