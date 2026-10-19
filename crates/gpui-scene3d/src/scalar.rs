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
