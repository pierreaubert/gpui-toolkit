/// A group of level meters with smoothed animation.
#[derive(Clone)]
pub struct MeterData {
    pub levels: Vec<f32>,
    pub peaks: Vec<f32>,
    pub names: Vec<String>,
}

impl MeterData {
    pub fn new(channels: usize) -> Self {
        Self {
            levels: vec![0.0; channels],
            peaks: vec![0.0; channels],
            names: (0..channels).map(|i| format!("CH{}", i + 1)).collect(),
        }
    }

    /// Pull the latest [`super::MeterFifo`] frame into `scratch` and fold it
    /// into the smoothed levels. Returns `false` (leaving levels untouched)
    /// when no fresh frame arrived since the previous call.
    pub fn update_from_fifo(
        &mut self,
        fifo: &super::MeterFifo,
        scratch: &mut Vec<f32>,
        smoothing: f32,
    ) -> bool {
        if !fifo.snapshot(scratch) {
            return false;
        }
        self.update(scratch, smoothing);
        true
    }

    pub fn update(&mut self, new_levels: &[f32], smoothing: f32) {
        for (i, &new_level) in new_levels.iter().enumerate() {
            if i < self.levels.len() {
                self.levels[i] = self.levels[i] * smoothing + new_level * (1.0 - smoothing);
                if new_level > self.peaks[i] {
                    self.peaks[i] = new_level;
                } else {
                    self.peaks[i] *= 0.995;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::MeterFifo;
    use super::MeterData;

    #[test]
    fn new_initializes_channels() {
        let data = MeterData::new(4);
        assert_eq!(data.levels.len(), 4);
        assert_eq!(data.peaks.len(), 4);
        assert_eq!(data.names.len(), 4);
        assert!(data.levels.iter().all(|&v| v == 0.0));
        assert!(data.peaks.iter().all(|&v| v == 0.0));
        assert_eq!(data.names[0], "CH1");
        assert_eq!(data.names[3], "CH4");
    }

    #[test]
    fn update_applies_smoothing_and_tracks_peaks() {
        let mut data = MeterData::new(2);
        data.update(&[1.0, 0.5], 0.5);
        assert!((data.levels[0] - 0.5).abs() < 1e-6);
        assert_eq!(data.peaks[0], 1.0);

        data.update(&[0.8, 0.6], 0.5);
        assert!(data.peaks[0] > 0.99 && data.peaks[0] <= 1.0);
        assert!(data.peaks[1] > 0.5);
    }

    #[test]
    fn update_from_fifo_applies_fresh_frames_only() {
        let fifo = MeterFifo::new(2);
        let mut data = MeterData::new(2);
        let mut scratch = Vec::new();

        assert!(!data.update_from_fifo(&fifo, &mut scratch, 0.0));
        assert!(data.levels.iter().all(|&v| v == 0.0));

        fifo.push_frame(&[1.0, 0.5]);
        assert!(data.update_from_fifo(&fifo, &mut scratch, 0.0));
        assert!((data.levels[0] - 1.0).abs() < 1e-6);
        assert!((data.levels[1] - 0.5).abs() < 1e-6);

        assert!(!data.update_from_fifo(&fifo, &mut scratch, 0.0));
    }

    #[test]
    fn update_ignores_extra_channels() {
        let mut data = MeterData::new(1);
        data.update(&[1.0, 2.0, 3.0], 0.0);
        assert_eq!(data.levels.len(), 1);
        assert_eq!(data.levels[0], 1.0);
    }
}
