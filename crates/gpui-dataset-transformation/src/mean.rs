//! Bounded exact averages for rare overflowing compensated sums.

// Rust guideline compliant 2026-02-21

// Every finite f64 is an integer multiple of 2^-1074. Its highest occupied bit
// is 2097; adding at most usize::MAX (<=64-bit) values needs at most 2162 bits.
// Separate signs avoid intermediate overflow and retain arbitrarily small
// residuals after cancellation. This scratch lives on the stack, per cluster.
const LIMBS: usize = 34;
const FRACTION: u64 = (1_u64 << 52) - 1;
const _: () = assert!(usize::BITS <= 64);

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
            let mut mean = ExactMean::default();
            for word in words {
                mean.add(f64::from_bits(word));
            }
            assert_eq!(mean.finish().to_bits(), expected, "oracle row {index}");
        }
    }
}
