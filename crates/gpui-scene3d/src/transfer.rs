//! Color and opacity transfer functions.

// Rust guideline compliant 2026-02-21

use std::sync::Arc;

type ColorStop = (f64, [f32; 4]);
const VIRIDIS: [ColorStop; 5] = [
    (0.0, [0.267, 0.005, 0.329, 1.0]),
    (0.25, [0.283, 0.141, 0.458, 1.0]),
    (0.5, [0.128, 0.567, 0.551, 1.0]),
    (0.75, [0.369, 0.789, 0.383, 1.0]),
    (1.0, [0.993, 0.906, 0.144, 1.0]),
];
const COOLWARM: [ColorStop; 3] = [
    (0.0, [0.230, 0.299, 0.754, 1.0]),
    (0.5, [0.865, 0.865, 0.865, 1.0]),
    (1.0, [0.706, 0.016, 0.150, 1.0]),
];

#[derive(Debug, Clone)]
enum Stops {
    Static(&'static [ColorStop]),
    Shared(Arc<[ColorStop]>),
}

impl Stops {
    fn as_slice(&self) -> &[ColorStop] {
        match self {
            Self::Static(stops) => stops,
            Self::Shared(stops) => stops,
        }
    }

    fn same_storage(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(a), Self::Static(b)) => std::ptr::eq(*a, *b),
            (Self::Shared(a), Self::Shared(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Domain {
    Constant,
    Linear {
        lo: f64,
        hi: f64,
        span: f64,
        inverse: f64,
        half: bool,
    },
    Log {
        lo: f64,
        hi: f64,
        log_lo: f64,
        inverse_span: f64,
        relative: bool,
    },
}

/// Validated, allocation-free sampler borrowing immutable transfer settings.
///
/// Domain constants are prepared once for repeated rendering or LUT generation.
#[derive(Debug, Clone, Copy)]
pub struct PreparedTransfer<'a> {
    stops: &'a [ColorStop],
    domain: Domain,
    bands: usize,
}

impl PreparedTransfer<'_> {
    /// Normalized scalar position; missing scalar values return NaN.
    #[must_use]
    pub fn normalize(&self, value: f64) -> f64 {
        if !value.is_finite() {
            return f64::NAN;
        }
        let t = match self.domain {
            Domain::Constant => return 0.0,
            Domain::Linear {
                lo,
                hi,
                span,
                inverse,
                half,
            } => {
                if value <= lo {
                    0.0
                } else if value >= hi {
                    1.0
                } else {
                    let numerator = if half {
                        value * 0.5 - lo * 0.5
                    } else {
                        value - lo
                    };
                    if inverse == 0.0 {
                        numerator / span
                    } else {
                        numerator * inverse
                    }
                }
            }
            Domain::Log {
                lo,
                hi,
                log_lo,
                inverse_span,
                relative,
            } => {
                if value <= lo {
                    0.0
                } else if value >= hi {
                    1.0
                } else if relative {
                    ((value - lo) / lo).ln_1p() * inverse_span
                } else {
                    (value.ln() - log_lo) * inverse_span
                }
            }
        }
        .clamp(0.0, 1.0);
        if self.bands == 0 {
            t
        } else {
            let count = self.bands as f64;
            ((t * count).floor().min(count - 1.0) + 0.5) / count
        }
    }

    /// Color for a scalar, with missing samples mapped to transparent.
    #[must_use]
    pub fn map(&self, value: f64) -> [f32; 4] {
        if self.stops.is_empty() {
            return [0.0; 4];
        }
        let t = self.normalize(value);
        if !t.is_finite() {
            return [0.0; 4];
        }
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        let last = self.stops[self.stops.len() - 1];
        if t >= last.0 {
            return last.1;
        }
        let upper = self.stops.partition_point(|stop| stop.0 <= t);
        let (t1, c1) = self.stops[upper];
        let (t0, c0) = self.stops[upper - 1];
        let f = if t1 == t0 {
            0.0
        } else {
            ((t - t0) / (t1 - t0)) as f32
        };
        std::array::from_fn(|axis| c0[axis] + (c1[axis] - c0[axis]) * f)
    }
}

/// Maps scalar values to RGBA through immutable control-point stops.
///
/// Built-in palettes use immutable tables; custom palettes share their sorted
/// storage when cloned. Public domain settings remain independently mutable.
#[derive(Debug, Clone)]
pub struct TransferFunction {
    /// Finite ordered scalar domain; an equal-ended domain maps to its start.
    pub range: [f64; 2],
    stops: Stops,
    stops_valid: bool,
    stops_opaque: bool,
    /// Logarithmic domain mapping, requiring a positive domain.
    pub log_scale: bool,
    /// Number of discrete color bands; zero is invalid.
    pub bands: Option<usize>,
}

impl PartialEq for TransferFunction {
    fn eq(&self, other: &Self) -> bool {
        self.range == other.range
            && self.log_scale == other.log_scale
            && self.bands == other.bands
            && self.stops_valid == other.stops_valid
            && self.stops_opaque == other.stops_opaque
            && ((self.stops_valid && self.stops.same_storage(&other.stops))
                || self.stops.as_slice() == other.stops.as_slice())
    }
}

impl TransferFunction {
    /// Checks domain settings and normalized control-point colors.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid domains, zero bands, or nonfinite/out-of-range stops.
    pub fn validate(&self) -> Result<(), crate::SceneError> {
        if !self.metadata_valid() {
            return Err(crate::SceneError::InvalidMaterial(
                "invalid transfer domain, logarithmic range or band count",
            ));
        }
        if !self.stops_valid {
            return Err(crate::SceneError::InvalidMaterial(
                "transfer stops and RGBA must be finite and in [0,1]",
            ));
        }
        Ok(())
    }

    /// Prepares immutable domain constants for repeated scalar lookups.
    ///
    /// # Errors
    ///
    /// Returns the same validation errors as [`Self::validate`].
    pub fn prepare(&self) -> Result<PreparedTransfer<'_>, crate::SceneError> {
        self.validate()?;
        let [lo, hi] = self.range;
        let domain = if lo == hi {
            Domain::Constant
        } else if self.log_scale {
            let relative_span = (hi - lo) / lo;
            if relative_span.is_finite() {
                Domain::Log {
                    lo,
                    hi,
                    log_lo: 0.0,
                    inverse_span: relative_span.ln_1p().recip(),
                    relative: true,
                }
            } else {
                let log_lo = lo.ln();
                Domain::Log {
                    lo,
                    hi,
                    log_lo,
                    inverse_span: (hi.ln() - log_lo).recip(),
                    relative: false,
                }
            }
        } else {
            let span = hi - lo;
            let half = !span.is_finite();
            let span = if half { hi * 0.5 - lo * 0.5 } else { span };
            let inverse = span.recip();
            Domain::Linear {
                lo,
                hi,
                span,
                inverse: if inverse.is_normal() { inverse } else { 0.0 },
                half,
            }
        };
        Ok(PreparedTransfer {
            stops: self.stops.as_slice(),
            domain,
            bands: self.bands.unwrap_or(0),
        })
    }

    fn metadata_valid(&self) -> bool {
        self.range.iter().all(|v| v.is_finite())
            && self.range[0] <= self.range[1]
            && self.bands != Some(0)
            && (!self.log_scale || self.range[0] > 0.0)
    }

    /// Whether the valid control ramp is conservatively classified as opaque.
    #[must_use]
    pub fn is_opaque(&self) -> bool {
        self.metadata_valid() && self.stops_opaque
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn gpu_range(&self) -> Result<[f32; 2], crate::SceneError> {
        self.validate()?;
        let range = self.range.map(|v| v as f32);
        let span = range[1] - range[0];
        if range.iter().any(|v| !v.is_finite() || v.is_subnormal())
            || !span.is_finite()
            || span.is_subnormal()
            || (self.range[0] != self.range[1] && span == 0.0)
        {
            return Err(crate::SceneError::UnsupportedPrecision(
                "transfer domain cannot be retained by the raw f32 scalar backend",
            ));
        }
        Ok(range)
    }

    /// Viridis-like perceptually uniform ramp over `range`.
    #[must_use]
    pub fn viridis(range: [f64; 2]) -> Self {
        Self::builtin(range, &VIRIDIS)
    }

    /// Blue-white-red diverging ramp over `range`.
    #[must_use]
    pub fn coolwarm(range: [f64; 2]) -> Self {
        Self::builtin(range, &COOLWARM)
    }

    fn builtin(range: [f64; 2], stops: &'static [ColorStop]) -> Self {
        Self {
            range,
            stops: Stops::Static(stops),
            stops_valid: true,
            stops_opaque: true,
            log_scale: false,
            bands: None,
        }
    }

    /// Builds a transfer function from normalized `[0,1]` stops and RGBA.
    ///
    /// Invalid settings are reported by [`Self::validate`]; direct lookups of
    /// invalid settings return transparent color rather than panicking.
    #[must_use]
    pub fn from_stops(range: [f64; 2], stops: &[(f64, [f32; 4])]) -> Self {
        let valid = stops.iter().all(|(position, color)| {
            position.is_finite()
                && (0.0..=1.0).contains(position)
                && color
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        });
        let opaque = valid && !stops.is_empty() && stops.iter().all(|(_, color)| color[3] == 1.0);
        let storage = if stops.is_empty() {
            Stops::Static(&[])
        } else {
            let mut sorted = stops.to_vec();
            sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
            Stops::Shared(sorted.into())
        };
        Self {
            range,
            stops: storage,
            stops_valid: valid,
            stops_opaque: opaque,
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

    /// Quantizes the ramp into at least one discrete color band.
    #[must_use]
    pub fn banded(mut self, bands: usize) -> Self {
        self.bands = Some(bands.max(1));
        self
    }

    /// Normalized `[0,1]` position; invalid scalar/settings return NaN.
    #[must_use]
    pub fn normalize(&self, value: f64) -> f64 {
        self.prepare()
            .map_or(f64::NAN, |prepared| prepared.normalize(value))
    }

    /// Color for a scalar; invalid settings and missing values are transparent.
    ///
    /// Use [`Self::prepare`] when mapping many values with the same settings.
    #[must_use]
    pub fn map(&self, value: f64) -> [f32; 4] {
        self.prepare()
            .map_or([0.0; 4], |prepared| prepared.map(value))
    }

    /// 256-entry RGBA lookup table; invalid settings produce transparent entries.
    #[must_use]
    pub fn lut_bytes(&self) -> Vec<u8> {
        let Ok(prepared) = self.prepare() else {
            return vec![0; 256 * 4];
        };
        let mut bytes = Vec::with_capacity(256 * 4);
        for i in 0..256 {
            let t = f64::from(i) / 255.0;
            let [lo, hi] = self.range;
            let value = if lo.is_sign_negative() == hi.is_sign_negative() {
                lo + (hi - lo) * t
            } else {
                lo * (1.0 - t) + hi * t
            };
            for c in prepared.map(value) {
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

    #[test]
    fn malformed_domains_and_stops_return_transparent_without_panicking() {
        for range in [[f64::NAN, 1.0], [0.0, f64::INFINITY], [1.0, 0.0]] {
            let transfer = TransferFunction::viridis(range);
            assert!(transfer.validate().is_err());
            assert_eq!(transfer.map(0.5), [0.0; 4]);
            assert!(!transfer.is_opaque());
            assert!(transfer.lut_bytes().iter().all(|&v| v == 0));
        }
        for stop in [
            (f64::NAN, [1.0; 4]),
            (-0.1, [1.0; 4]),
            (0.5, [f32::INFINITY; 4]),
            (0.5, [2.0; 4]),
        ] {
            let transfer =
                TransferFunction::from_stops([0.0, 1.0], &[(0.0, [0.0; 4]), stop, (1.0, [1.0; 4])]);
            assert!(transfer.validate().is_err());
            assert_eq!(transfer.map(0.5), [0.0; 4]);
        }
        let mut bands = TransferFunction::viridis([0.0, 1.0]);
        bands.bands = Some(0);
        assert!(bands.validate().is_err());
        assert_eq!(bands.map(0.5), [0.0; 4]);
        let log = TransferFunction::viridis([-1.0, 1.0]).log();
        assert!(log.validate().is_err());
        assert_eq!(log.map(0.5), [0.0; 4]);
    }

    #[test]
    fn wide_linear_and_close_or_wide_log_domains_normalize_stably() {
        let wide = TransferFunction::coolwarm([-f64::MAX, f64::MAX]);
        assert_eq!(wide.normalize(0.0), 0.5);
        assert_eq!(wide.map(0.0), [0.865, 0.865, 0.865, 1.0]);
        assert!(wide.lut_bytes().chunks_exact(4).all(|p| p[3] == 255));
        let logarithmic = TransferFunction::viridis([1e-300, 1e300]).log();
        assert!((logarithmic.normalize(1.0) - 0.5).abs() < 1e-12);
        let lo = 1e300f64;
        let close = TransferFunction::viridis([lo, f64::from_bits(lo.to_bits() + 8)]).log();
        assert!((close.normalize(f64::from_bits(lo.to_bits() + 4)) - 0.5).abs() < 0.001);
        let bands = TransferFunction::viridis([1.0, 10000.0]).log().banded(4);
        assert_eq!(bands.normalize(2.0), 0.125);
        assert_eq!(bands.normalize(20.0), 0.375);
    }

    #[test]
    fn immutable_stops_are_shared_while_domains_remain_independent() {
        let original = TransferFunction::from_stops(
            [0.0, 1.0],
            &[(0.0, [0.0, 0.0, 0.0, 1.0]), (1.0, [1.0; 4])],
        );
        let mut copy = original.clone();
        assert!(original.stops.same_storage(&copy.stops));
        assert_eq!(copy, original);
        copy.range = [0.0, 2.0];
        assert_eq!(original.map(1.0), [1.0; 4]);
        assert_eq!(copy.map(1.0), [0.5, 0.5, 0.5, 1.0]);
        let builtin = TransferFunction::viridis([0.0, 1.0]);
        assert!(matches!(builtin.stops, Stops::Static(_)));
        assert!(builtin.stops.same_storage(&builtin.clone().stops));
    }

    #[test]
    #[cfg(feature = "gpu")]
    fn raw_gpu_domains_report_collapse_overflow_and_subnormal_precision() {
        for range in [[-f64::MAX, f64::MAX], [1e12, 1e12 + 0.125], [0.0, 1e-40]] {
            assert!(matches!(
                TransferFunction::viridis(range).gpu_range(),
                Err(crate::SceneError::UnsupportedPrecision(_))
            ));
        }
        assert_eq!(
            TransferFunction::viridis([0.0, 1.0]).gpu_range().unwrap(),
            [0.0, 1.0]
        );
    }
}
