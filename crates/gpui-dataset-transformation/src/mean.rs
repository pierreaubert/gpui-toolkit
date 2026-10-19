//! Single-pass exact averages with bounded adaptive storage.

// Rust guideline compliant 2026-02-21

// Finite f64 values are integer multiples of 2^-1074. Up to usize::MAX
// (<=64-bit) terms need at most 2162 bits; 34 limbs cover the wide fallback.
const LIMBS: usize = 34;
const FRACTION: u64 = (1_u64 << 52) - 1;
const _: () = assert!(usize::BITS <= 64);

#[derive(Debug)]
struct SumCertificate {
    min_bit: usize,
    max_bit: usize,
}

impl Default for SumCertificate {
    fn default() -> Self {
        Self {
            min_bit: usize::MAX,
            max_bit: 0,
        }
    }
}

impl SumCertificate {
    #[inline]
    fn include(&mut self, value: f64) {
        if value == 0.0 {
            return;
        }
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let shift = exponent.saturating_sub(1);
        let significand = (bits & FRACTION) | if exponent == 0 { 0 } else { 1_u64 << 52 };
        self.min_bit = self
            .min_bit
            .min(shift + significand.trailing_zeros() as usize);
        self.max_bit = self
            .max_bit
            .max(shift + 63 - significand.leading_zeros() as usize);
    }

    #[inline]
    fn allows(&self, count: usize) -> bool {
        if count == 0 || count as u128 > 1_u128 << 53 {
            return false;
        }
        if self.min_bit == usize::MAX {
            return true;
        }
        let growth = (count - 1).bit_width() as usize;
        self.max_bit + growth <= 2097 && self.max_bit + growth - self.min_bit < 53
    }
}

#[derive(Debug, Default)]
struct FloatingSum {
    sum: f64,
    count: usize,
    certificate: SumCertificate,
}

#[derive(Debug, Default)]
struct SmallSum {
    magnitude: u128,
    negative: bool,
    shift: usize,
    count: usize,
}

impl SmallSum {
    fn from_float(sum: f64, count: usize) -> Self {
        if sum == 0.0 {
            return Self {
                count,
                ..Self::default()
            };
        }
        let (significand, shift, negative) = parts(sum);
        Self {
            magnitude: u128::from(significand),
            negative,
            shift,
            count,
        }
    }

    #[inline]
    fn try_add(&mut self, value: f64) -> bool {
        if value == 0.0 {
            self.count += 1;
            return true;
        }
        let (significand, shift, negative) = parts(value);
        if self.magnitude == 0 {
            self.magnitude = u128::from(significand);
            self.shift = shift;
            self.negative = negative;
            self.count += 1;
            return true;
        }
        let common = self.shift.min(shift);
        let Some(a) = shift_exact(self.magnitude, self.shift - common) else {
            return false;
        };
        let Some(b) = shift_exact(u128::from(significand), shift - common) else {
            return false;
        };
        let (magnitude, negative) = if self.negative == negative {
            let Some(total) = a.checked_add(b) else {
                return false;
            };
            (total, negative)
        } else if a >= b {
            (a - b, self.negative)
        } else {
            (b - a, negative)
        };
        self.magnitude = magnitude;
        self.negative = negative;
        self.shift = common;
        self.count += 1;
        true
    }
}

#[inline]
fn parts(value: f64) -> (u64, usize, bool) {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as usize;
    (
        (bits & FRACTION) | if exponent == 0 { 0 } else { 1_u64 << 52 },
        exponent.saturating_sub(1),
        bits >> 63 != 0,
    )
}

#[inline]
fn shift_exact(value: u128, shift: usize) -> Option<u128> {
    if shift == 0 {
        Some(value)
    } else if shift < 128 && value.leading_zeros() as usize >= shift {
        Some(value << shift)
    } else {
        None
    }
}

/// A finite-value mean that never replays its input or allocates heap scratch.
#[derive(Debug)]
pub(crate) struct AdaptiveMean {
    floating: Option<FloatingSum>,
    small: SmallSum,
    wide: Option<ExactMean>,
}

impl Default for AdaptiveMean {
    fn default() -> Self {
        Self {
            floating: Some(FloatingSum::default()),
            small: SmallSum::default(),
            wide: None,
        }
    }
}

impl AdaptiveMean {
    #[inline]
    pub(crate) fn add(&mut self, value: f64) {
        debug_assert!(value.is_finite());
        if let Some(wide) = &mut self.wide {
            wide.add(value);
            return;
        }
        if let Some(floating) = &mut self.floating {
            floating.certificate.include(value);
            if floating.certificate.allows(floating.count + 1) {
                floating.sum += value;
                floating.count += 1;
                return;
            }
            // The previous prefix was certified exact. Seed integer storage
            // from that sum and its original count before adding this term.
            self.small = SmallSum::from_float(floating.sum, floating.count);
            self.floating = None;
        }
        if !self.small.try_add(value) {
            let mut wide = ExactMean::from_scaled(
                self.small.magnitude,
                self.small.shift,
                self.small.negative,
                self.small.count,
            );
            wide.add(value);
            self.wide = Some(wide);
        }
    }

    #[inline]
    pub(crate) fn finish(self) -> f64 {
        if let Some(wide) = self.wide {
            return wide.finish();
        }
        if let Some(floating) = self.floating {
            return if floating.count == 0 {
                f64::NAN
            } else {
                floating.sum / floating.count as f64
            };
        }
        rounded_small(
            self.small.magnitude,
            self.small.count,
            self.small.shift as i32 - 1074,
            self.small.negative,
        )
    }
}

pub(crate) fn exact_average(mut values: impl Iterator<Item = f64>) -> f64 {
    // Cell-sized reductions usually fit in a signed u128. Keep the wide state
    // out of this hot loop; construct it only after a checked promotion fails.
    let mut small = SmallSum::default();
    while let Some(value) = values.next() {
        debug_assert!(value.is_finite());
        if !small.try_add(value) {
            return finish_wide(small, value, values);
        }
    }
    if small.count == 0 {
        return f64::NAN;
    }
    rounded_small(
        small.magnitude,
        small.count,
        small.shift as i32 - 1074,
        small.negative,
    )
}

#[cold]
#[inline(never)]
fn finish_wide(small: SmallSum, value: f64, values: impl Iterator<Item = f64>) -> f64 {
    let mut wide =
        ExactMean::from_scaled(small.magnitude, small.shift, small.negative, small.count);
    wide.add(value);
    for value in values {
        wide.add(value);
    }
    wide.finish()
}

/// Certifies exact floating sums for bounded contributor counts.
pub(crate) fn certified_float_rows<'a, const N: usize>(
    rows: impl Iterator<Item = &'a [f64; N]>,
    max_terms: usize,
) -> bool {
    let mut certificate = SumCertificate::default();
    for row in rows {
        for &value in row {
            certificate.include(value);
            if !certificate.allows(max_terms) {
                return false;
            }
        }
    }
    true
}

/// A shared binary unit whose bounded sums fit a signed machine word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SmallFieldSum {
    shift: usize,
}

impl SmallFieldSum {
    pub(crate) fn certify<'a, const N: usize>(
        rows: impl Iterator<Item = &'a [f64; N]>,
        max_terms: usize,
    ) -> Option<Self> {
        if max_terms == 0 {
            return None;
        }
        let growth = (max_terms - 1).bit_width() as usize;
        let mut shift = usize::MAX;
        let mut high = 0;
        for row in rows {
            for &value in row {
                if value == 0.0 {
                    continue;
                }
                let (significand, unit, _) = parts(value);
                shift = shift.min(unit);
                high = high.max(unit + 63 - significand.leading_zeros() as usize);
                if high + growth - shift >= 63 {
                    return None;
                }
            }
        }
        (shift != usize::MAX).then_some(Self { shift })
    }

    #[inline]
    pub(crate) fn integer(self, value: f64) -> i64 {
        if value == 0.0 {
            return 0;
        }
        let (significand, unit, negative) = parts(value);
        let magnitude = (significand << (unit - self.shift)) as i64;
        if negative { -magnitude } else { magnitude }
    }

    #[inline]
    pub(crate) fn average(self, sum: i64, count: usize) -> f64 {
        let exponent = self.shift as i32 - 1074 - count.trailing_zeros() as i32;
        if count.is_power_of_two() && (-1022..=1023).contains(&exponent) {
            // Integer conversion rounds once; the normal power-of-two scale
            // preserves that rounding. The certificate bounds every prefix.
            return sum as f64 * f64::from_bits(((exponent + 1023) as u64) << 52);
        }
        rounded_small(
            u128::from(sum.unsigned_abs()),
            count,
            self.shift as i32 - 1074,
            sum < 0,
        )
    }
}

/// Whether a component contains both positive and negative supported values.
pub(crate) fn has_mixed_rows<'a, const N: usize>(rows: impl Iterator<Item = &'a [f64; N]>) -> bool {
    let mut positive = [false; N];
    let mut negative = [false; N];
    for row in rows {
        for axis in 0..N {
            positive[axis] |= row[axis] > 0.0;
            negative[axis] |= row[axis] < 0.0;
            if positive[axis] && negative[axis] {
                return true;
            }
        }
    }
    false
}

/// Two bits per component record nonzero positive/negative contributors.
pub(crate) fn component_sign(value: f64, axis: usize) -> u16 {
    debug_assert!(axis < 8);
    if value == 0.0 {
        0
    } else {
        (1 + ((value.to_bits() >> 63) as u16)) << (2 * axis)
    }
}

pub(crate) fn has_mixed_signs(bits: u16) -> bool {
    // Retain the low bit of each two-bit component lane after pairing signs.
    bits & (bits >> 1) & 0x5555 != 0
}

fn rounded_small(magnitude: u128, count: usize, unit_exponent: i32, negative: bool) -> f64 {
    if magnitude == 0 {
        return 0.0;
    }
    let high = 127 - magnitude.leading_zeros() as usize;
    let count_high = (usize::BITS - 1 - count.leading_zeros()) as usize;
    // Keep at least 53 quotient bits plus a guard. The denominator is at most
    // 64 bits, so adding quotient precision never overflows u128.
    let extra = (count_high + 54).saturating_sub(high);
    let numerator = magnitude << extra;
    let denominator = count as u128;
    let (quotient, remainder) = if count.is_power_of_two() {
        (numerator >> count_high, numerator & (denominator - 1))
    } else {
        (numerator / denominator, numerator % denominator)
    };
    let quotient_high = 127 - quotient.leading_zeros() as usize;
    let mut exponent = quotient_high as i32 + unit_exponent - extra as i32;
    let subnormal = exponent < -1022;
    let shift = if subnormal {
        (-1074 - unit_exponent + extra as i32) as usize
    } else {
        quotient_high - 52
    };
    debug_assert!((1..128).contains(&shift));
    let mut mantissa = (quotient >> shift) as u64;
    let guard = (quotient >> (shift - 1)) & 1 != 0;
    let sticky = quotient & ((1_u128 << (shift - 1)) - 1) != 0 || remainder != 0;
    mantissa += u64::from(guard && (sticky || mantissa & 1 != 0));
    let bits = if subnormal {
        mantissa
    } else {
        if mantissa == 1_u64 << 53 {
            mantissa >>= 1;
            exponent += 1;
        }
        debug_assert!((-1022..=1023).contains(&exponent));
        (((exponent + 1023) as u64) << 52) | (mantissa & FRACTION)
    };
    f64::from_bits(bits | (u64::from(negative) << 63))
}

#[derive(Debug)]
pub(crate) struct ExactMean {
    positive: [u64; LIMBS],
    negative: [u64; LIMBS],
    count: usize,
}

impl Default for ExactMean {
    fn default() -> Self {
        Self {
            positive: [0; LIMBS],
            negative: [0; LIMBS],
            count: 0,
        }
    }
}

impl ExactMean {
    fn from_scaled(magnitude: u128, shift: usize, negative: bool, count: usize) -> Self {
        let mut mean = Self {
            count,
            ..Self::default()
        };
        let words = if negative {
            &mut mean.negative
        } else {
            &mut mean.positive
        };
        let index = shift / 64;
        let offset = shift % 64;
        words[index] = (magnitude as u64) << offset;
        if offset == 0 {
            words[index + 1] = (magnitude >> 64) as u64;
        } else {
            words[index + 1] = (magnitude >> (64 - offset)) as u64;
            words[index + 2] = (magnitude >> (128 - offset)) as u64;
        }
        mean
    }

    pub(crate) fn add(&mut self, value: f64) {
        debug_assert!(value.is_finite());
        self.count += 1;
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let significand = (bits & FRACTION) | if exponent == 0 { 0 } else { 1_u64 << 52 };
        let shift = exponent.saturating_sub(1);
        let words = if bits >> 63 == 0 {
            &mut self.positive
        } else {
            &mut self.negative
        };
        let mut index = shift / 64;
        let mut carry = u128::from(significand) << (shift % 64);
        while carry != 0 {
            let total = u128::from(words[index]) + (carry & u128::from(u64::MAX));
            words[index] = total as u64;
            carry = (carry >> 64) + (total >> 64);
            index += 1;
        }
    }

    /// Returns the correctly rounded arithmetic mean, or NaN for no values.
    pub(crate) fn finish(self) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }
        let ordering = self.positive.iter().rev().cmp(self.negative.iter().rev());
        if ordering == std::cmp::Ordering::Equal {
            return 0.0;
        }
        let negative = ordering == std::cmp::Ordering::Less;
        let (mut words, smaller) = if negative {
            (self.negative, self.positive)
        } else {
            (self.positive, self.negative)
        };
        let mut borrow = false;
        for (word, subtract) in words.iter_mut().zip(smaller) {
            let (difference, first) = word.overflowing_sub(subtract);
            let (difference, second) = difference.overflowing_sub(u64::from(borrow));
            *word = difference;
            borrow = first || second;
        }
        debug_assert!(!borrow);
        let highest_word = words
            .iter()
            .rposition(|&word| word != 0)
            .expect("nonzero signed difference");
        let highest = highest_word * 64 + 63 - words[highest_word].leading_zeros() as usize;
        let count_bits = (usize::BITS - 1 - self.count.leading_zeros()) as usize;
        // Keep at least 54 quotient bits, plus the entire denominator width.
        // One u128 division suffices; discarded low bits only affect the sticky
        // rounding bit. Subnormal results keep the complete numerator.
        let shift = highest.saturating_sub(count_bits + 54);
        let top = high128(&words, shift);
        let denominator = self.count as u128;
        let quotient = top / denominator;
        let remainder = top % denominator;
        debug_assert!(quotient <= u128::from(u64::MAX));
        let quotient = quotient as u64;
        let quotient_high = if quotient == 0 {
            0
        } else {
            63 - quotient.leading_zeros() as usize
        };
        let round_shift = quotient_high.saturating_sub(52);
        let mut mantissa = quotient >> round_shift;
        let round_up = if round_shift == 0 {
            debug_assert_eq!(shift, 0);
            remainder * 2 > denominator || (remainder * 2 == denominator && mantissa & 1 != 0)
        } else {
            let guard = (quotient >> (round_shift - 1)) & 1 != 0;
            let sticky = quotient & ((1_u64 << (round_shift - 1)) - 1) != 0
                || remainder != 0
                || low_bits(&words, shift);
            guard && (sticky || mantissa & 1 != 0)
        };
        mantissa += u64::from(round_up);
        let mut high = quotient_high + shift;
        let magnitude = if high < 52 {
            mantissa
        } else {
            if mantissa == 1_u64 << 53 {
                mantissa >>= 1;
                high += 1;
            }
            debug_assert!(high <= 2097);
            (((high - 51) as u64) << 52) | (mantissa & FRACTION)
        };
        f64::from_bits(magnitude | (u64::from(negative) << 63))
    }
}

fn high128(words: &[u64; LIMBS], shift: usize) -> u128 {
    let index = shift / 64;
    let offset = shift % 64;
    let word = |index| u128::from(words.get(index).copied().unwrap_or(0));
    let mut result = (word(index) | word(index + 1) << 64) >> offset;
    if offset != 0 {
        result |= word(index + 2) << (128 - offset);
    }
    result
}

fn low_bits(words: &[u64; LIMBS], shift: usize) -> bool {
    let index = shift / 64;
    words[..index].iter().any(|&word| word != 0)
        || (!shift.is_multiple_of(64) && words[index] & ((1_u64 << (shift % 64)) - 1) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_means_survive_overflow_cancellation_and_subnormal_rounding() {
        let tiny = f64::from_bits(1);
        for (values, expected) in [
            (vec![f64::MAX; 4], f64::MAX),
            (
                vec![f64::MAX, f64::MAX, -f64::MAX, -f64::MAX, 1e-300],
                1e-300 / 5.0,
            ),
            (vec![f64::MAX, 1.0, -f64::MAX], 1.0 / 3.0),
            (vec![0.0, tiny, tiny], tiny),
            (vec![0.0, -tiny, -tiny], -tiny),
            (vec![0.0, tiny], 0.0),
            (vec![tiny, 2.0 * tiny], 2.0 * tiny),
        ] {
            let mut mean = ExactMean::default();
            for value in values {
                mean.add(value);
            }
            assert_eq!(mean.finish().to_bits(), expected.to_bits());
        }
        assert!(ExactMean::default().finish().is_nan());
    }

    #[test]
    fn exact_means_match_independent_rational_bit_patterns() {
        for (index, line) in include_str!("../tests/fixtures/exact_mean.tsv")
            .lines()
            .enumerate()
        {
            if line.starts_with('#') {
                continue;
            }
            let mut words = line
                .split_whitespace()
                .map(|word| u64::from_str_radix(word, 16).unwrap());
            let expected = words.next().unwrap();
            let values = words.map(f64::from_bits);
            let rows: Vec<_> = values.clone().map(|value| [value]).collect();
            if let Some(plan) = SmallFieldSum::certify(rows.iter(), rows.len()) {
                let sum = values.clone().map(|value| plan.integer(value)).sum();
                assert_eq!(
                    plan.average(sum, rows.len()).to_bits(),
                    expected,
                    "shared integer oracle row {index}"
                );
            }
            assert_eq!(
                exact_average(values.clone()).to_bits(),
                expected,
                "small oracle row {index}"
            );
            let mut adaptive = AdaptiveMean::default();
            for value in values.clone() {
                adaptive.add(value);
            }
            assert_eq!(
                adaptive.finish().to_bits(),
                expected,
                "adaptive oracle row {index}"
            );
            let mut mean = ExactMean::default();
            for value in values {
                mean.add(value);
            }
            assert_eq!(mean.finish().to_bits(), expected, "oracle row {index}");
        }
    }
}
