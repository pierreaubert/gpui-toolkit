//! Scalar coordinates shared by preparation, software and GPU upload.

// Rust guideline compliant 2026-02-21

use crate::{Actor, SceneError};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScalarTransport {
    Flat,
    Raw,
    Linear([f64; 2]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ScalarIdentity {
    Flat,
    Raw,
    Linear([u64; 2]),
}

pub(crate) fn raw_range(source: [f64; 2]) -> Result<[f32; 2], SceneError> {
    let range = source.map(|v| v as f32);
    let span = range[1] - range[0];
    // Keep about 12 bits of domain-relative resolution before interpolation.
    // Magnitude * EPSILON conservatively estimates an f32 ULP. Rebasing
    // prevents a large offset from consuming most scalar mantissa bits.
    // This is a coordinate profile, not a universal palette error bound.
    const MAX_RELATIVE_ULP: f64 = 1.0 / 4_096.0;
    let magnitude = source[0].abs().max(source[1].abs());
    let coarse = source[0] != source[1]
        && magnitude * f64::from(f32::EPSILON) > (source[1] - source[0]) * MAX_RELATIVE_ULP;
    if range.iter().any(|v| !v.is_finite() || v.is_subnormal())
        || !span.is_finite()
        || span.is_subnormal()
        || (source[0] != source[1] && span == 0.0)
        || coarse
    {
        return Err(crate::SceneError::UnsupportedPrecision(
            "transfer domain cannot be retained by the raw f32 scalar backend",
        ));
    }
    Ok(range)
}

impl ScalarIdentity {
    pub(crate) fn transport(self) -> ScalarTransport {
        match self {
            Self::Flat => ScalarTransport::Flat,
            Self::Raw => ScalarTransport::Raw,
            Self::Linear(bits) => ScalarTransport::Linear(bits.map(f64::from_bits)),
        }
    }
}

impl ScalarTransport {
    pub(crate) fn of(actor: &Actor) -> Result<Self, SceneError> {
        let Some(transfer) = actor.active_transfer() else {
            return Ok(Self::Flat);
        };
        match transfer.raw_f32_range() {
            Ok(_) => Ok(Self::Raw),
            Err(SceneError::UnsupportedPrecision(_)) => Ok(Self::Linear(transfer.range)),
            Err(error) => Err(error),
        }
    }
    pub(crate) fn identity(self) -> ScalarIdentity {
        match self {
            Self::Flat => ScalarIdentity::Flat,
            Self::Raw => ScalarIdentity::Raw,
            Self::Linear(range) => ScalarIdentity::Linear(range.map(f64::to_bits)),
        }
    }
    /// Maps the current display range into this buffer's fixed coordinates.
    pub(crate) fn display_range(self, actor: &Actor) -> Result<[f64; 2], SceneError> {
        let unsupported = || SceneError::UnsupportedPrecision("scalar encoding needs replacement");
        match (self, actor.active_transfer()) {
            (Self::Flat, None) => Ok([0.0, 1.0]),
            (Self::Raw, Some(tf)) => {
                tf.raw_f32_range()?;
                Ok(tf.range)
            }
            (Self::Linear(domain), Some(tf)) => {
                tf.validate()?;
                if domain == tf.range {
                    return Ok([0.0, 1.0]);
                }
                // Constant encoding has discarded all source differences.
                if domain[0] == domain[1] {
                    return Err(unsupported());
                }
                let range = [self.coordinate(tf.range[0])?, self.coordinate(tf.range[1])?];
                if tf.range[0] != tf.range[1] && range[0] == range[1] {
                    return Err(unsupported());
                }
                // A zoom with insufficient relative resolution must reencode
                // original f64 samples, rather than magnifying rounded buffers.
                raw_range(range)?;
                Ok(range)
            }
            _ => Err(unsupported()),
        }
    }

    pub(crate) fn supports_display(self, actor: &Actor) -> Result<bool, SceneError> {
        match self.display_range(actor) {
            Ok(_) => Ok(true),
            Err(SceneError::UnsupportedPrecision(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
    #[inline]
    pub(crate) fn domain(self) -> Option<[f64; 2]> {
        match self {
            Self::Linear(range) => Some(range),
            _ => None,
        }
    }
    #[inline]
    pub(crate) fn coordinate(self, value: f64) -> Result<f64, SceneError> {
        let Self::Linear([lo, hi]) = self else {
            return Ok(if matches!(self, Self::Flat) {
                0.0
            } else {
                value
            });
        };
        if lo == hi {
            return Ok(0.0);
        }
        let numerator = value - lo;
        let span = hi - lo;
        // Halving before subtraction retains finite coordinates for opposite
        // extrema even when the raw difference overflows. Never clamp vertices:
        // out-of-range values must still participate correctly in interpolation.
        let coordinate = if numerator.is_finite() && span.is_finite() {
            numerator / span
        } else {
            (value * 0.5 - lo * 0.5) / (hi * 0.5 - lo * 0.5)
        };
        if !coordinate.is_finite() {
            return Err(SceneError::UnsupportedPrecision(
                "scalar coordinate exceeds f64 range",
            ));
        }
        Ok(coordinate)
    }
    #[inline]
    pub(crate) fn pack(self, value: f64) -> Result<f32, SceneError> {
        let value = self.coordinate(value)?;
        let converted = value as f32;
        if !converted.is_finite()
            || (value != 0.0 && (converted == 0.0 || converted.is_subnormal()))
        {
            return Err(SceneError::UnsupportedPrecision(
                "scalar coordinate exceeds the f32 transport profile",
            ));
        }
        Ok(converted)
    }
}

#[cfg(feature = "gpui-gpu")]
pub(crate) fn gpu_supported(actor: &Actor) -> Result<bool, SceneError> {
    let transport = ScalarTransport::of(actor)?;
    let field = crate::retained::selected_field(actor)?.map(|(_, f)| f);
    if let Some(field) = field {
        for (index, &value) in field
            .as_scalar()
            .expect("selected scalar")
            .iter()
            .enumerate()
        {
            if !field.is_valid(index) {
                continue;
            }
            match transport.pack(value) {
                Ok(_) => {}
                Err(SceneError::UnsupportedPrecision(_)) => return Ok(false),
                Err(e) => return Err(e),
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linear_transport_retains_unclamped_values_and_opposite_extrema() {
        let transport = ScalarTransport::Linear([1e100, 2e100]);
        assert_eq!(transport.pack(-1e100).unwrap(), -2.0);
        assert_eq!(transport.pack(3e100).unwrap(), 2.0);
        assert_eq!(
            ScalarTransport::Linear([-f64::MAX, f64::MAX])
                .pack(0.0)
                .unwrap(),
            0.5
        );
        assert_eq!(
            ScalarTransport::Linear([1e12, 1e12 + 1.0])
                .pack(1e12 + 0.25)
                .unwrap(),
            0.25
        );
        assert!(
            ScalarTransport::Linear([1e308, f64::MAX])
                .coordinate(-f64::MAX)
                .unwrap()
                .is_finite()
        );
        for value in [1e100, 1e-100, 1e-40] {
            assert!(matches!(
                ScalarTransport::Raw.pack(value),
                Err(SceneError::UnsupportedPrecision(_))
            ));
        }
        assert_eq!(ScalarTransport::Flat.pack(1e100).unwrap(), 0.0);
    }
}
