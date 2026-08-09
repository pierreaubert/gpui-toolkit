use std::sync::Arc;

use super::MeshValidationError;

/// How contour levels are chosen (spec §6.1).
#[derive(Debug, Clone, PartialEq)]
pub enum ContourLevels {
    /// Automatic nice-number levels over the displayed color range.
    Count(u32),
    /// Explicit levels: finite, strictly increasing, unique.
    Explicit(Arc<[f64]>),
}

impl ContourLevels {
    /// Resolve to concrete levels for `range` (displayed color range).
    /// A constant (empty) range yields no levels (spec §5.2).
    pub fn resolve(&self, range: [f64; 2]) -> Result<Arc<[f64]>, MeshValidationError> {
        match self {
            Self::Explicit(levels) => {
                let ok = levels.iter().all(|l| l.is_finite())
                    && levels.windows(2).all(|w| w[1] > w[0]);
                if !ok {
                    return Err(MeshValidationError::InvalidContourLevels);
                }
                Ok(levels.clone())
            }
            Self::Count(count) => {
                let [lo, hi] = range;
                if !(lo.is_finite() && hi.is_finite()) || hi <= lo {
                    return Ok(Vec::new().into());
                }
                Ok(nice_levels(lo, hi, (*count).max(2) as usize).into())
            }
        }
    }
}

/// Nice-number levels strictly inside (lo, hi), step from the 1-2-5 ladder.
fn nice_levels(lo: f64, hi: f64, target: usize) -> Vec<f64> {
    let span = hi - lo;
    let raw = span / (target as f64 + 1.0);
    let mag = 10f64.powf(raw.log10().floor());
    let norm = raw / mag;
    let nice = if norm <= 1.0 {
        1.0
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 5.0 {
        5.0
    } else {
        10.0
    };
    let step = nice * mag;
    let first = (lo / step).ceil();
    let mut out = Vec::new();
    let mut k = first;
    while k * step < hi {
        // normalize -0.0 and float dust
        let v = if (k * step).abs() < step * 1e-9 {
            0.0
        } else {
            k * step
        };
        if v > lo {
            out.push(v);
        }
        k += 1.0;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_levels_pass_through_sorted() {
        let levels = ContourLevels::Explicit(vec![0.1, 0.5, 0.9].into());
        let r = levels.resolve([0.0, 1.0]).unwrap();
        assert_eq!(&*r, &[0.1, 0.5, 0.9]);
    }

    #[test]
    fn explicit_levels_reject_unsorted() {
        let levels = ContourLevels::Explicit(vec![0.5, 0.1].into());
        assert_eq!(
            levels.resolve([0.0, 1.0]),
            Err(MeshValidationError::InvalidContourLevels)
        );
    }

    #[test]
    fn explicit_levels_reject_duplicates() {
        let levels = ContourLevels::Explicit(vec![0.5, 0.5].into());
        assert_eq!(
            levels.resolve([0.0, 1.0]),
            Err(MeshValidationError::InvalidContourLevels)
        );
    }

    #[test]
    fn explicit_levels_reject_non_finite() {
        let levels = ContourLevels::Explicit(vec![0.5, f64::NAN].into());
        assert_eq!(
            levels.resolve([0.0, 1.0]),
            Err(MeshValidationError::InvalidContourLevels)
        );
    }

    #[test]
    fn count_levels_use_nice_numbers() {
        let levels = ContourLevels::Count(5).resolve([0.0, 1.0]).unwrap();
        // nice step 0.2 → exactly 0.0,0.2,...,1.0 or interior subset; must be
        // strictly increasing, within range, and "round" (step × integer)
        assert!(levels.len() >= 3 && levels.len() <= 7);
        for w in levels.windows(2) {
            assert!(w[1] > w[0]);
        }
        let step = levels[1] - levels[0];
        assert!((step - 0.2).abs() < 1e-12);
    }

    #[test]
    fn constant_field_yields_empty_levels() {
        let levels = ContourLevels::Count(12).resolve([3.0, 3.0]).unwrap();
        assert!(levels.is_empty());
    }

    #[test]
    fn count_levels_span_negative_range() {
        let levels = ContourLevels::Count(5).resolve([-3.7, 2.4]).unwrap();
        for &l in &*levels {
            assert!(l > -3.7 && l < 2.4);
            // levels are multiples of a nice step
            let step = levels[1] - levels[0];
            assert!((l / step).fract().abs() < 1e-9 || (l / step).fract().abs() > 1.0 - 1e-9);
        }
    }
}
