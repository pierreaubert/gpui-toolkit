//! Colour and opacity transfer functions.

// Rust guideline compliant 2026-02-21

/// Maps scalar values to RGBA colours through control-point stops.
///
/// Lookups are binary-searched over sorted stops and linearly interpolated,
/// so a transfer function is cheap to evaluate per vertex on large meshes.
#[derive(Debug, Clone, PartialEq)]
pub struct TransferFunction {
    /// Scalar domain mapped by this function.
    pub range: [f64; 2],
    stops: Vec<(f64, [f32; 4])>,
    /// Whether the domain is mapped logarithmically (positive ranges only).
    pub log_scale: bool,
    /// Number of discrete colour bands (filled-contour mode); `None` is
    /// a continuous ramp.
    pub bands: Option<usize>,
}

impl TransferFunction {
    /// Whether every finite scalar maps to an opaque color.
    #[must_use]
    pub fn is_opaque(&self) -> bool {
        !self.stops.is_empty() && self.stops.iter().all(|(_, color)| color[3] >= 1.0)
    }

    /// Viridis-like perceptually uniform ramp over `range`.
    #[must_use]
    pub fn viridis(range: [f64; 2]) -> Self {
        Self::from_stops(
            range,
            &[
                (0.0, [0.267, 0.005, 0.329, 1.0]),
                (0.25, [0.283, 0.141, 0.458, 1.0]),
                (0.5, [0.128, 0.567, 0.551, 1.0]),
                (0.75, [0.369, 0.789, 0.383, 1.0]),
                (1.0, [0.993, 0.906, 0.144, 1.0]),
            ],
        )
    }

    /// Blue-white-red diverging ramp over `range`, centred at the midpoint.
    #[must_use]
    pub fn coolwarm(range: [f64; 2]) -> Self {
        Self::from_stops(
            range,
            &[
                (0.0, [0.230, 0.299, 0.754, 1.0]),
                (0.5, [0.865, 0.865, 0.865, 1.0]),
                (1.0, [0.706, 0.016, 0.150, 1.0]),
            ],
        )
    }

    /// Builds a transfer function from normalized `[0,1]` stops.
    #[must_use]
    pub fn from_stops(range: [f64; 2], stops: &[(f64, [f32; 4])]) -> Self {
        let mut stops: Vec<(f64, [f32; 4])> = stops.to_vec();
        stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Self {
            range,
            stops,
            log_scale: false,
            bands: None,
        }
    }

    /// Enables logarithmic domain mapping.
    #[must_use]
    pub fn log(mut self) -> Self {
        self.log_scale = true;
        self
    }

    /// Quantises the ramp into `bands` discrete colours (filled contours).
    #[must_use]
    pub fn banded(mut self, bands: usize) -> Self {
        self.bands = Some(bands.max(1));
        self
    }

    /// Normalized `[0,1]` position of a value in the domain.
    #[must_use]
    pub fn normalize(&self, value: f64) -> f64 {
        let [lo, hi] = self.range;
        if self.log_scale && lo > 0.0 && hi > lo {
            let t = (value.max(lo).ln() - lo.ln()) / (hi.ln() - lo.ln());
            return t.clamp(0.0, 1.0);
        }
        if hi <= lo {
            return 0.0;
        }
        let t = ((value - lo) / (hi - lo)).clamp(0.0, 1.0);
        match self.bands {
            Some(bands) => {
                let band = (t * bands as f64).floor().min(bands as f64 - 1.0);
                (band + 0.5) / bands as f64
            }
            None => t,
        }
    }

    /// Colour for a scalar value; NaN maps to transparent.
    #[must_use]
    pub fn map(&self, value: f64) -> [f32; 4] {
        if !value.is_finite() || self.stops.is_empty() {
            return [0.0, 0.0, 0.0, 0.0];
        }
        let t = self.normalize(value);
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        let last = self.stops[self.stops.len() - 1];
        if t >= last.0 {
            return last.1;
        }
        let upper = self.stops.partition_point(|s| s.0 <= t);
        let (t1, c1) = self.stops[upper];
        let (t0, c0) = self.stops[upper - 1];
        let f = if (t1 - t0).abs() < f64::EPSILON {
            0.0
        } else {
            ((t - t0) / (t1 - t0)) as f32
        };
        [
            c0[0] + (c1[0] - c0[0]) * f,
            c0[1] + (c1[1] - c0[1]) * f,
            c0[2] + (c1[2] - c0[2]) * f,
            c0[3] + (c1[3] - c0[3]) * f,
        ]
    }

    /// 256-entry RGBA lookup table bytes for GPU upload.
    #[must_use]
    pub fn lut_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(256 * 4);
        for i in 0..256 {
            let t = f64::from(i) / 255.0;
            let value = self.range[0] + t * (self.range[1] - self.range[0]);
            // Map by normalized position directly to avoid range inversion.
            let colour = if self.stops.is_empty() {
                [0.0; 4]
            } else if t <= self.stops[0].0 {
                self.stops[0].1
            } else {
                let _ = value;
                self.map(self.range[0] + t * (self.range[1] - self.range[0]))
            };
            for c in colour {
                bytes.push((c.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_and_midpoint() {
        let tf = TransferFunction::coolwarm([0.0, 10.0]);
        assert_eq!(tf.map(0.0), [0.230, 0.299, 0.754, 1.0]);
        assert_eq!(tf.map(10.0), [0.706, 0.016, 0.150, 1.0]);
        let mid = tf.map(5.0);
        assert!((mid[0] - 0.865).abs() < 1e-6);
        assert_eq!(tf.lut_bytes().len(), 256 * 4);
        assert_eq!(tf.map(f64::NAN)[3], 0.0);
    }

    #[test]
    fn clamps_outside_range() {
        let tf = TransferFunction::viridis([-1.0, 1.0]);
        assert_eq!(tf.map(-99.0), tf.map(-1.0));
        assert_eq!(tf.map(99.0), tf.map(1.0));
    }
}
