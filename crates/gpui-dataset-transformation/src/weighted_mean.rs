//! Exact finite weighted means with adaptive, bounded stack storage.

// Rust guideline compliant 2026-02-21

use std::cmp::Ordering;

// Products of finite f64 values occupy at most 4196 bits in units of 2^-2148.
// A 64-bit contributor count adds at most 64 bits; 67 limbs leave rounding room.
const LIMBS: usize = 67;
const FRACTION: u64 = (1_u64 << 52) - 1;
const _: () = assert!(usize::BITS <= 64);

#[derive(Clone, Copy, Debug, Default)]
struct BinarySum {
    magnitude: u128,
    unit: usize,
    negative: bool,
}

impl BinarySum {
    fn add(&mut self, magnitude: u128, unit: usize, negative: bool) -> bool {
        if magnitude == 0 {
            return true;
        }
        if self.magnitude == 0 {
            *self = Self {
                magnitude,
                unit,
                negative,
            };
        } else {
            let common = self.unit.min(unit);
            let Some(a) = shifted(self.magnitude, self.unit - common) else {
                return false;
            };
            let Some(b) = shifted(magnitude, unit - common) else {
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
            *self = Self {
                magnitude,
                unit: common,
                negative,
            };
        }
        if self.magnitude != 0 {
            let zeros = self.magnitude.trailing_zeros() as usize;
            self.magnitude >>= zeros;
            self.unit += zeros;
        }
        true
    }
}

fn shifted(value: u128, shift: usize) -> Option<u128> {
    (shift < 128 && value.leading_zeros() as usize >= shift).then(|| value << shift)
}

#[inline]
fn parts64(value: f64) -> (u64, usize, bool) {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as usize;
    let significand = (bits & FRACTION) | (u64::from(exponent != 0) << 52);
    if significand == 0 {
        return (0, 0, false);
    }
    let zeros = significand.trailing_zeros() as usize;
    (
        significand >> zeros,
        exponent.saturating_sub(1) + zeros,
        bits >> 63 != 0,
    )
}

#[inline]
fn parts(value: f64) -> (u128, usize, bool) {
    let (significand, unit, negative) = parts64(value);
    (u128::from(significand), unit, negative)
}

/// Averages finite values with strictly positive finite binary weights.
pub(crate) fn exact_weighted_average(mut values: impl Iterator<Item = (f64, f64)>) -> f64 {
    let mut numerator = BinarySum::default();
    let mut denominator = BinarySum::default();
    while let Some((value, weight)) = values.next() {
        debug_assert!(value.is_finite() && weight.is_finite() && weight > 0.0);
        let (a, a_unit, negative) = parts(value);
        let (b, b_unit, _) = parts(weight);
        let mut next_numerator = numerator;
        let mut next_denominator = denominator;
        if !next_numerator.add(a * b, a_unit + b_unit, negative)
            || !next_denominator.add(b, b_unit, false)
        {
            return finish_wide(numerator, denominator, (value, weight), values);
        }
        numerator = next_numerator;
        denominator = next_denominator;
    }
    if denominator.magnitude == 0 {
        return f64::NAN;
    }
    if numerator.magnitude == 0 {
        return 0.0;
    }
    small_ratio(
        numerator.magnitude,
        denominator.magnitude,
        numerator.unit as i32 - denominator.unit as i32 - 1074,
        numerator.negative,
    )
}

fn small_ratio(a: u128, b: u128, unit: i32, negative: bool) -> f64 {
    let a_high = 127 - a.leading_zeros() as i32;
    let b_high = 127 - b.leading_zeros() as i32;
    let initial = a_high - b_high;
    let (n, d) = if initial >= 0 {
        (a, b << initial)
    } else {
        (a << -initial, b)
    };
    let (power, mut remainder) = if n >= d {
        (initial, n - d)
    } else {
        let (remainder, bit) = double_remainder(n, d);
        debug_assert!(bit);
        (initial - 1, remainder)
    };
    let exponent = power + unit;
    if exponent < -1075 {
        return f64::from_bits(u64::from(negative) << 63);
    }
    let bits = if exponent >= -1022 {
        53
    } else {
        exponent + 1075
    };
    // A single native u128 division handles the usual bounded product ranges.
    let shift = bits - 1 - power;
    let operands = if shift >= 0 {
        shifted(a, shift as usize).map(|n| (n, b))
    } else {
        shifted(b, (-shift) as usize).map(|d| (a, d))
    };
    let significand = if let Some((n, d)) = operands {
        let (q, r) = if (n | d) <= u128::from(u64::MAX) {
            let (n, d) = (n as u64, d as u64);
            (u128::from(n / d), u128::from(n % d))
        } else {
            (n / d, n % d)
        };
        let gap = d - r;
        q as u64 + u64::from(r > gap || (r == gap && q & 1 != 0))
    } else if bits == 0 {
        u64::from(remainder != 0)
    } else {
        // Doubling a remainder via d-r avoids u128 overflow at capacity.
        let mut q = 1_u64;
        for _ in 1..bits {
            let (next, bit) = double_remainder(remainder, d);
            remainder = next;
            q = (q << 1) | u64::from(bit);
        }
        let gap = d - remainder;
        q + u64::from(remainder > gap || (remainder == gap && q & 1 != 0))
    };
    encode(significand, exponent, negative)
}

fn double_remainder(remainder: u128, denominator: u128) -> (u128, bool) {
    let gap = denominator - remainder;
    if remainder >= gap {
        (remainder - gap, true)
    } else {
        (remainder * 2, false)
    }
}

fn ratio64(a: u64, b: u64, unit: i32, negative: bool) -> f64 {
    let initial = b.leading_zeros() as i32 - a.leading_zeros() as i32;
    let (n, d) = if initial >= 0 {
        (a, b << initial)
    } else {
        (a << -initial, b)
    };
    let power = initial - i32::from(n < d);
    let exponent = power + unit;
    if exponent < -1075 {
        return f64::from_bits(u64::from(negative) << 63);
    }
    let bits = if exponent >= -1022 {
        53
    } else {
        exponent + 1075
    };
    let shift = bits - 1 - power;
    let operands = if shift >= 0 {
        (shift < 64 && a.leading_zeros() >= shift as u32).then(|| (a << shift, b))
    } else {
        let shift = -shift;
        (shift < 64 && b.leading_zeros() >= shift as u32).then(|| (a, b << shift))
    };
    let Some((n, d)) = operands else {
        return small_ratio(u128::from(a), u128::from(b), unit, negative);
    };
    let q = n / d;
    let r = n % d;
    let gap = d - r;
    encode(
        q + u64::from(r > gap || (r == gap && q & 1 != 0)),
        exponent,
        negative,
    )
}

fn encode(mut significand: u64, mut exponent: i32, negative: bool) -> f64 {
    let magnitude = if exponent < -1022 {
        significand
    } else {
        if significand == 1_u64 << 53 {
            significand >>= 1;
            exponent += 1;
        }
        debug_assert!((-1022..=1023).contains(&exponent));
        (((exponent + 1023) as u64) << 52) | (significand & FRACTION)
    };
    f64::from_bits(magnitude | (u64::from(negative) << 63))
}

/// Common product/weight units with certified machine-word prefix bounds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProductPlan {
    numerator_unit: usize,
    denominator_unit: usize,
}

impl ProductPlan {
    pub(crate) fn certify<'a, const N: usize, const BITS: usize>(
        rows: impl Iterator<Item = (&'a [f64; N], f64)>,
        maximum: usize,
    ) -> Option<Self> {
        debug_assert!(BITS == 53 || BITS == 64 || BITS == 128);
        if maximum == 0 {
            return None;
        }
        let growth = (maximum - 1).bit_width() as usize;
        let (mut n_low, mut n_high, mut d_low, mut d_high) = (usize::MAX, 0, usize::MAX, 0);
        for (row, weight) in rows {
            let (b, b_unit, _) = parts(weight);
            debug_assert!(weight.is_finite() && weight > 0.0);
            d_low = d_low.min(b_unit);
            d_high = d_high.max(b_unit + 127 - b.leading_zeros() as usize);
            if d_high + growth - d_low >= BITS {
                return None;
            }
            if BITS == 53 && d_high + growth > 2097 {
                return None;
            }
            for &value in row {
                let (a, a_unit, _) = parts(value);
                if a == 0 {
                    continue;
                }
                let product = a * b;
                let unit = a_unit + b_unit;
                n_low = n_low.min(unit);
                n_high = n_high.max(unit + 127 - product.leading_zeros() as usize);
                if n_high + growth - n_low >= BITS - 1 {
                    return None;
                }
                // Products use units of 2^-2148. Certify representable products
                // and every floating prefix, including subnormal boundaries.
                if BITS == 53 && (n_low < 1074 || n_high + growth > 3171) {
                    return None;
                }
            }
        }
        (d_low != usize::MAX).then_some(Self {
            numerator_unit: if n_low == usize::MAX { 0 } else { n_low },
            denominator_unit: d_low,
        })
    }

    pub(crate) fn terms<const N: usize>(self, row: &[f64; N], weight: f64) -> ([i128; N], u128) {
        let (b, b_unit, _) = parts(weight);
        let denominator = b << (b_unit - self.denominator_unit);
        let numerator = row.map(|value| {
            let (a, a_unit, negative) = parts(value);
            // Only zero products can have a unit below the certificate; their
            // masked shift remains zero. Every nonzero term fits its signed bound.
            let magnitude = (a * b)
                .wrapping_shl((a_unit + b_unit).wrapping_sub(self.numerator_unit) as u32)
                as i128;
            if negative { -magnitude } else { magnitude }
        });
        (numerator, denominator)
    }

    pub(crate) fn terms64<const N: usize>(self, row: &[f64; N], weight: f64) -> ([i64; N], u64) {
        let (b, b_unit, _) = parts64(weight);
        let denominator = b << (b_unit - self.denominator_unit);
        let numerator = row.map(|value| {
            let (a, a_unit, negative) = parts64(value);
            // The 64-bit certificate bounds the unshifted product too.
            let magnitude = (a * b)
                .wrapping_shl((a_unit + b_unit).wrapping_sub(self.numerator_unit) as u32)
                as i64;
            if negative { -magnitude } else { magnitude }
        });
        (numerator, denominator)
    }

    pub(crate) fn averages64<const N: usize>(
        self,
        numerator: [i64; N],
        denominator: u64,
    ) -> [f64; N] {
        if denominator == 0 {
            return [f64::NAN; N];
        }
        let unit = self.numerator_unit as i32 - self.denominator_unit as i32 - 1074;
        let exponent = unit - denominator.trailing_zeros() as i32;
        if denominator.is_power_of_two() && (-1022..=1023).contains(&exponent) {
            let scale = f64::from_bits(((exponent + 1023) as u64) << 52);
            return numerator.map(|sum| sum as f64 * scale);
        }
        numerator.map(|sum| {
            if sum == 0 {
                0.0
            } else {
                ratio64(sum.unsigned_abs(), denominator, unit, sum < 0)
            }
        })
    }

    pub(crate) fn averages<const N: usize>(
        self,
        numerator: [i128; N],
        denominator: u128,
    ) -> [f64; N] {
        if denominator == 0 {
            return [f64::NAN; N];
        }
        let unit = self.numerator_unit as i32 - self.denominator_unit as i32 - 1074;
        numerator.map(|sum| {
            if sum == 0 {
                0.0
            } else {
                small_ratio(sum.unsigned_abs(), denominator, unit, sum < 0)
            }
        })
    }
}

#[derive(Debug)]
struct Words([u64; LIMBS]);

impl Default for Words {
    fn default() -> Self {
        Self([0; LIMBS])
    }
}

impl Words {
    fn add_limb(&mut self, mut index: usize, mut value: u64) {
        while value != 0 {
            let (sum, carry) = self.0[index].overflowing_add(value);
            self.0[index] = sum;
            value = u64::from(carry);
            index += 1;
        }
    }

    fn add(&mut self, value: u128, shift: usize) {
        let index = shift / 64;
        let offset = shift % 64;
        let low = value as u64;
        let high = (value >> 64) as u64;
        self.add_limb(index, low << offset);
        if offset == 0 {
            self.add_limb(index + 1, high);
        } else {
            self.add_limb(index + 1, (low >> (64 - offset)) | (high << offset));
            self.add_limb(index + 2, high >> (64 - offset));
        }
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.0.iter().rev().cmp(other.0.iter().rev())
    }

    fn subtract(&mut self, other: &Self) {
        let mut borrow = false;
        for (word, &subtract) in self.0.iter_mut().zip(&other.0) {
            let (difference, first) = word.overflowing_sub(subtract);
            let (difference, second) = difference.overflowing_sub(u64::from(borrow));
            *word = difference;
            borrow = first || second;
        }
        debug_assert!(!borrow);
    }

    fn highest(&self) -> Option<usize> {
        self.0
            .iter()
            .rposition(|&word| word != 0)
            .map(|index| index * 64 + 63 - self.0[index].leading_zeros() as usize)
    }

    fn shift(&mut self, shift: usize) {
        if shift == 0 {
            return;
        }
        debug_assert!(self.highest().is_none_or(|high| high + shift < LIMBS * 64));
        let whole = shift / 64;
        let offset = shift % 64;
        for index in (0..LIMBS).rev() {
            self.0[index] = if index < whole {
                0
            } else {
                let source = index - whole;
                (self.0[source] << offset)
                    | if offset != 0 && source > 0 {
                        self.0[source - 1] >> (64 - offset)
                    } else {
                        0
                    }
            };
        }
    }

    fn double(&mut self) {
        let mut carry = 0;
        for word in &mut self.0 {
            let next = *word >> 63;
            *word = (*word << 1) | carry;
            carry = next;
        }
        debug_assert_eq!(carry, 0);
    }
}

#[derive(Debug, Default)]
struct Wide {
    positive: Words,
    negative: Words,
    denominator: Words,
}

impl Wide {
    fn add(&mut self, value: f64, weight: f64) {
        let (a, a_unit, negative) = parts(value);
        let (b, b_unit, _) = parts(weight);
        if negative {
            &mut self.negative
        } else {
            &mut self.positive
        }
        .add(a * b, a_unit + b_unit);
        self.denominator.add(b, b_unit);
    }

    fn finish(self) -> f64 {
        let ordering = self.positive.compare(&self.negative);
        if ordering == Ordering::Equal {
            return 0.0;
        }
        let negative = ordering == Ordering::Less;
        let (mut remainder, smaller) = if negative {
            (self.negative, self.positive)
        } else {
            (self.positive, self.negative)
        };
        remainder.subtract(&smaller);
        let mut denominator = self.denominator;
        let initial = remainder.highest().expect("nonzero difference") as i32
            - denominator.highest().expect("positive weights") as i32;
        if initial >= 0 {
            denominator.shift(initial as usize);
        } else {
            remainder.shift((-initial) as usize);
        }
        let power = if remainder.compare(&denominator) == Ordering::Less {
            remainder.double();
            initial - 1
        } else {
            initial
        };
        remainder.subtract(&denominator);
        // Numerator units are 2^-2148; denominator units are 2^-1074.
        let exponent = power - 1074;
        if exponent < -1075 {
            return f64::from_bits(u64::from(negative) << 63);
        }
        let bits = if exponent >= -1022 {
            53
        } else {
            exponent + 1075
        };
        if bits == 0 {
            return encode(u64::from(remainder.highest().is_some()), exponent, negative);
        }
        let mut q = 1_u64;
        for _ in 1..bits {
            remainder.double();
            let bit = remainder.compare(&denominator) != Ordering::Less;
            if bit {
                remainder.subtract(&denominator);
            }
            q = (q << 1) | u64::from(bit);
        }
        remainder.double();
        let rounding = remainder.compare(&denominator);
        q +=
            u64::from(rounding == Ordering::Greater || (rounding == Ordering::Equal && q & 1 != 0));
        encode(q, exponent, negative)
    }
}

#[cold]
#[inline(never)]
fn finish_wide(
    numerator: BinarySum,
    denominator: BinarySum,
    current: (f64, f64),
    values: impl Iterator<Item = (f64, f64)>,
) -> f64 {
    let mut wide = Wide::default();
    if numerator.negative {
        &mut wide.negative
    } else {
        &mut wide.positive
    }
    .add(numerator.magnitude, numerator.unit);
    wide.denominator
        .add(denominator.magnitude, denominator.unit);
    wide.add(current.0, current.1);
    for (value, weight) in values {
        wide.add(value, weight);
    }
    wide.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_means_match_independent_rational_bits_in_both_orders() {
        let mut checked = 0;
        for (index, line) in include_str!("../tests/fixtures/weighted_mean.tsv")
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.starts_with('#'))
        {
            let words = line
                .split_whitespace()
                .map(|word| u64::from_str_radix(word, 16).unwrap())
                .collect::<Vec<_>>();
            assert!(words.len() >= 3 && words.len() % 2 == 1);
            checked += 1;
            let pairs = words[1..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| (f64::from_bits(pair[0]), f64::from_bits(pair[1])))
                .collect::<Vec<_>>();
            let rows = pairs.iter().map(|&(value, _)| [value]).collect::<Vec<_>>();
            for bits in [53, 64, 128] {
                let inputs = || {
                    rows.iter()
                        .zip(&pairs)
                        .map(|(row, &(_, weight))| (row, weight))
                };
                let plan = if bits == 53 {
                    ProductPlan::certify::<1, 53>(inputs(), pairs.len())
                } else if bits == 64 {
                    ProductPlan::certify::<1, 64>(inputs(), pairs.len())
                } else {
                    ProductPlan::certify::<1, 128>(inputs(), pairs.len())
                };
                if let Some(plan) = plan {
                    if bits == 64 {
                        let (mut numerator, mut denominator) = (0_i64, 0_u64);
                        for (row, &(_, weight)) in rows.iter().zip(&pairs) {
                            let (n, d) = plan.terms64(row, weight);
                            numerator += n[0];
                            denominator += d;
                        }
                        assert_eq!(
                            plan.averages64([numerator], denominator)[0].to_bits(),
                            words[0],
                            "native product oracle row {index}"
                        );
                    }
                    if bits == 53 {
                        let numerator = pairs.iter().fold(0.0, |sum, &(v, w)| sum + v * w);
                        let denominator = pairs.iter().map(|&(_, w)| w).sum::<f64>();
                        assert_eq!(
                            (numerator / denominator).to_bits(),
                            words[0],
                            "floating product oracle row {index}"
                        );
                    }
                    let mut numerator = 0_i128;
                    let mut denominator = 0_u128;
                    for (row, &(_, weight)) in rows.iter().zip(&pairs) {
                        let (n, d) = plan.terms(row, weight);
                        numerator += n[0];
                        denominator += d;
                    }
                    assert_eq!(
                        plan.averages([numerator], denominator)[0].to_bits(),
                        words[0],
                        "product plan {bits} oracle row {index}"
                    );
                }
            }
            for iter in [pairs.clone(), pairs.iter().copied().rev().collect()] {
                assert_eq!(
                    exact_weighted_average(iter.into_iter()).to_bits(),
                    words[0],
                    "weighted oracle row {index}"
                );
            }
            let mut wide = Wide::default();
            for (value, weight) in pairs {
                wide.add(value, weight);
            }
            assert_eq!(
                wide.finish().to_bits(),
                words[0],
                "wide weighted oracle row {index}"
            );
        }
        assert_eq!(checked, 590);
        assert!(exact_weighted_average(std::iter::empty()).is_nan());
    }
}
