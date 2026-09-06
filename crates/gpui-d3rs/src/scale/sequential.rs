//! Sequential and diverging scales (d3-scale `scaleSequential`, `scaleDiverging`
//! and the log/pow/symlog sequential variants).
//!
//! Unlike the color ramps in [`crate::color`], these are true scales: they own
//! a domain, an interpolator, clamping, and tick generation, and they map data
//! values directly to [`D3Color`].
//!
//! ```
//! use d3rs::color::SequentialScheme;
//! use d3rs::scale::{SequentialScale, Scale};
//!
//! let magma = SequentialScheme::magma();
//! let scale = SequentialScale::new()
//!     .domain(0.0, 100.0)
//!     .interpolator(move |t| magma.get(t));
//!
//! let c = scale.scale(50.0);
//! assert!(c.r >= 0.0 && c.r <= 1.0);
//! ```

use super::{Scale, generate_linear_ticks, generate_log_ticks};
use crate::color::D3Color;
use std::rc::Rc;

/// Invalid (NaN) input maps to transparent, mirroring d3's `unknown` value.
fn unknown_color() -> D3Color {
    D3Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    }
}

/// Piecewise-linear RGB interpolation through `values` at `t` in [0, 1].
fn piecewise_color(values: &[D3Color], t: f64) -> D3Color {
    let n = values.len();
    if n == 0 {
        return unknown_color();
    }
    if n == 1 || t <= 0.0 {
        return values[0];
    }
    if t >= 1.0 {
        return values[n - 1];
    }
    let pos = t * (n - 1) as f64;
    let i = pos.floor() as usize;
    let f = (pos - i as f64) as f32;
    let a = values[i];
    let b = values[i + 1];
    D3Color {
        r: a.r + (b.r - a.r) * f,
        g: a.g + (b.g - a.g) * f,
        b: a.b + (b.b - a.b) * f,
        a: a.a + (b.a - a.a) * f,
    }
}

/// Normalize `x` in `[d0, d1]` to `t` in [0, 1].
///
/// Descending domains work naturally; degenerate domains yield 0.5 like d3.
fn normalize(x: f64, d0: f64, d1: f64, clamped: bool) -> f64 {
    if x.is_nan() || d0.is_nan() || d1.is_nan() {
        return f64::NAN;
    }
    let span = d1 - d0;
    let mut t = if span == 0.0 { 0.5 } else { (x - d0) / span };
    if clamped {
        if t.is_nan() {
            return f64::NAN;
        }
        // Clamp toward [0, 1] regardless of domain direction.
        t = t.clamp(0.0, 1.0);
    }
    t
}

/// Normalize-then-transform helper shared by the power-family variants:
/// `t = (f(x) - f(d0)) / (f(d1) - f(d0))`.
fn normalize_transformed(x: f64, d0: f64, d1: f64, clamped: bool, f: impl Fn(f64) -> f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    normalize(f(x), f(d0), f(d1), clamped)
}

/// A sequential scale maps a continuous domain to colors via an interpolator.
///
/// Mirrors `d3.scaleSequential`: default domain [0, 1], optional clamping,
/// linear ticks. The default interpolator is a grayscale ramp; set a
/// chromatic one (e.g. a `SequentialScheme::magma` sampler) with
/// [`SequentialScale::interpolator`].
#[derive(Clone)]
pub struct SequentialScale {
    domain: [f64; 2],
    interpolator: Rc<dyn Fn(f64) -> D3Color>,
    clamped: bool,
}

impl Default for SequentialScale {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialScale {
    /// Create a sequential scale with domain [0, 1] and a grayscale ramp.
    pub fn new() -> Self {
        Self {
            domain: [0.0, 1.0],
            interpolator: Rc::new(|t: f64| {
                let v = t.clamp(0.0, 1.0) as f32;
                D3Color {
                    r: v,
                    g: v,
                    b: v,
                    a: 1.0,
                }
            }),
            clamped: false,
        }
    }

    /// Set the domain (input extent). Descending domains are allowed.
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        self.domain = [min, max];
        self
    }

    /// Set the interpolator mapping normalized `t` in [0, 1] to a color.
    pub fn interpolator(mut self, f: impl Fn(f64) -> D3Color + 'static) -> Self {
        self.interpolator = Rc::new(f);
        self
    }

    /// Set discrete output colors, sampled piecewise like a stepped ramp.
    ///
    /// Note: d3 binds `range` to a quantizing interpolator; here the values
    /// are interpolated piecewise-linearly in RGB.
    pub fn range(mut self, values: Vec<D3Color>) -> Self {
        self.interpolator = Rc::new(move |t| piecewise_color(&values, t));
        self
    }

    /// Clamp normalized values to [0, 1] before interpolation.
    pub fn clamp(mut self, enabled: bool) -> Self {
        self.clamped = enabled;
        self
    }


    /// Copy the scale.
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

impl Scale<f64, D3Color> for SequentialScale {
    fn scale(&self, value: f64) -> D3Color {
        let t = normalize(value, self.domain[0], self.domain[1], self.clamped);
        if t.is_nan() {
            return unknown_color();
        }
        (self.interpolator)(t)
    }

    fn invert(&self, _value: D3Color) -> Option<f64> {
        // Interpolators are not generally invertible (matches d3, which
        // exposes no invert on sequential scales).
        None
    }

    fn ticks(&self, count: usize) -> Vec<f64> {
        let (lo, hi) = (self.domain[0].min(self.domain[1]), self.domain[0].max(self.domain[1]));
        generate_linear_ticks(lo, hi, count)
    }

    fn domain(&self) -> (f64, f64) {
        (
            self.domain[0].min(self.domain[1]),
            self.domain[0].max(self.domain[1]),
        )
    }

    fn range(&self) -> (D3Color, D3Color) {
        ((self.interpolator)(0.0), (self.interpolator)(1.0))
    }
}

/// A diverging scale maps a continuous three-point domain `[min, mid, max]`
/// to colors, with `mid` pinned to `t = 0.5`.
///
/// Mirrors `d3.scaleDiverging`.
#[derive(Clone)]
pub struct DivergingScale {
    domain: [f64; 3],
    interpolator: Rc<dyn Fn(f64) -> D3Color>,
    clamped: bool,
}

impl Default for DivergingScale {
    fn default() -> Self {
        Self::new()
    }
}

impl DivergingScale {
    /// Create a diverging scale with domain [0, 0.5, 1] and a grayscale ramp.
    pub fn new() -> Self {
        Self {
            domain: [0.0, 0.5, 1.0],
            interpolator: Rc::new(|t: f64| {
                let v = t.clamp(0.0, 1.0) as f32;
                D3Color {
                    r: v,
                    g: v,
                    b: v,
                    a: 1.0,
                }
            }),
            clamped: false,
        }
    }

    /// Set the domain `[min, mid, max]`. Descending domains are allowed.
    pub fn domain(mut self, min: f64, mid: f64, max: f64) -> Self {
        self.domain = [min, mid, max];
        self
    }

    /// Set the interpolator mapping normalized `t` in [0, 1] to a color.
    pub fn interpolator(mut self, f: impl Fn(f64) -> D3Color + 'static) -> Self {
        self.interpolator = Rc::new(f);
        self
    }

    /// Set three output colors `[min, mid, max]`, interpolated piecewise.
    pub fn range(mut self, values: [D3Color; 3]) -> Self {
        self.interpolator = Rc::new(move |t| piecewise_color(&values, t));
        self
    }

    /// Clamp normalized values to [0, 1] before interpolation.
    pub fn clamp(mut self, enabled: bool) -> Self {
        self.clamped = enabled;
        self
    }


    /// Copy the scale.
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

/// Sign-preserving power transform, mirroring d3's pow scale.
fn transform_pow(exponent: f64) -> impl Fn(f64) -> f64 {
    move |x: f64| {
        if x < 0.0 {
            -(-x).powf(exponent)
        } else {
            x.powf(exponent)
        }
    }
}

/// Log transform with d3's sign reflection for negative domains.
fn transform_log(base: f64, negative: bool) -> impl Fn(f64) -> f64 {
    let ln_base = base.ln();
    move |x: f64| {
        if negative {
            -(-x).ln() / ln_base
        } else {
            x.ln() / ln_base
        }
    }
}

/// Symlog transform with constant `c`, mirroring d3's symlog scale.
fn transform_symlog(c: f64) -> impl Fn(f64) -> f64 {
    move |x: f64| x.signum() * (1.0 + (x / c).abs()).ln()
}

macro_rules! impl_sequential_variant {
    ($name:ident, $doc:expr, $field:ident : $ty:ty = $default:expr) => {
        #[doc = $doc]
        #[derive(Clone)]
        pub struct $name {
            inner: SequentialScale,
            $field: $ty,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

impl_sequential_variant!(
    SequentialLogScale,
    "A sequential scale with logarithmic normalization (`d3.scaleSequentialLog`).",
    base_value: f64 = 10.0
);
impl_sequential_variant!(
    SequentialPowScale,
    "A sequential scale with power normalization (`d3.scaleSequentialPow`; use exponent 0.5 for sqrt).",
    exponent_value: f64 = 1.0
);
impl_sequential_variant!(
    SequentialSymlogScale,
    "A sequential scale with symmetric-log normalization (`d3.scaleSequentialSymlog`).",
    constant_value: f64 = 1.0
);

impl SequentialLogScale {
    /// Create a log sequential scale with domain [1, 10] and base 10.
    pub fn new() -> Self {
        Self {
            inner: SequentialScale::new().domain(1.0, 10.0),
            base_value: 10.0,
        }
    }

    /// Create with an explicit base.
    pub fn base(base: f64) -> Self {
        let mut scale = Self::new();
        scale.base_value = base;
        scale
    }

    /// Set the domain (must not span zero, like d3).
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        self.inner = self.inner.domain(min, max);
        self
    }

    /// Set the interpolator mapping normalized `t` in [0, 1] to a color.
    pub fn interpolator(mut self, f: impl Fn(f64) -> D3Color + 'static) -> Self {
        self.inner = self.inner.interpolator(f);
        self
    }

    /// Set discrete output colors, interpolated piecewise.
    pub fn range(mut self, values: Vec<D3Color>) -> Self {
        self.inner = self.inner.range(values);
        self
    }

    /// Clamp normalized values to [0, 1].
    pub fn clamp(mut self, enabled: bool) -> Self {
        self.inner = self.inner.clamp(enabled);
        self
    }


    /// Copy the scale.
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

impl Scale<f64, D3Color> for SequentialLogScale {
    fn scale(&self, value: f64) -> D3Color {
        let [d0, d1] = self.inner.domain;
        let negative = d0 < 0.0;
        let t = normalize_transformed(
            value,
            d0,
            d1,
            self.inner.clamped,
            transform_log(self.base_value, negative),
        );
        if t.is_nan() {
            return unknown_color();
        }
        (self.inner.interpolator)(t)
    }

    fn invert(&self, _value: D3Color) -> Option<f64> {
        None
    }

    fn ticks(&self, count: usize) -> Vec<f64> {
        let (lo, hi) = (
            self.inner.domain[0].min(self.inner.domain[1]),
            self.inner.domain[0].max(self.inner.domain[1]),
        );
        generate_log_ticks(lo, hi, self.base_value, true)
            .into_iter()
            .take(count.max(1))
            .collect()
    }

    fn domain(&self) -> (f64, f64) {
        (
            self.inner.domain[0].min(self.inner.domain[1]),
            self.inner.domain[0].max(self.inner.domain[1]),
        )
    }

    fn range(&self) -> (D3Color, D3Color) {
        (
            (self.inner.interpolator)(0.0),
            (self.inner.interpolator)(1.0),
        )
    }
}

impl SequentialPowScale {
    /// Create a power sequential scale with exponent 1 and domain [0, 1].
    pub fn new() -> Self {
        Self {
            inner: SequentialScale::new(),
            exponent_value: 1.0,
        }
    }

    /// Set the exponent (0.5 behaves like `d3.scaleSequentialSqrt`).
    pub fn exponent(mut self, exponent: f64) -> Self {
        self.exponent_value = exponent;
        self
    }

    /// Set the domain.
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        self.inner = self.inner.domain(min, max);
        self
    }

    /// Set the interpolator mapping normalized `t` in [0, 1] to a color.
    pub fn interpolator(mut self, f: impl Fn(f64) -> D3Color + 'static) -> Self {
        self.inner = self.inner.interpolator(f);
        self
    }

    /// Set discrete output colors, interpolated piecewise.
    pub fn range(mut self, values: Vec<D3Color>) -> Self {
        self.inner = self.inner.range(values);
        self
    }

    /// Clamp normalized values to [0, 1].
    pub fn clamp(mut self, enabled: bool) -> Self {
        self.inner = self.inner.clamp(enabled);
        self
    }


    /// Copy the scale.
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

impl Scale<f64, D3Color> for SequentialPowScale {
    fn scale(&self, value: f64) -> D3Color {
        let [d0, d1] = self.inner.domain;
        let t = normalize_transformed(
            value,
            d0,
            d1,
            self.inner.clamped,
            transform_pow(self.exponent_value),
        );
        if t.is_nan() {
            return unknown_color();
        }
        (self.inner.interpolator)(t)
    }

    fn invert(&self, _value: D3Color) -> Option<f64> {
        None
    }

    fn ticks(&self, count: usize) -> Vec<f64> {
        Scale::ticks(&self.inner, count)
    }

    fn domain(&self) -> (f64, f64) {
        Scale::domain(&self.inner)
    }

    fn range(&self) -> (D3Color, D3Color) {
        (
            (self.inner.interpolator)(0.0),
            (self.inner.interpolator)(1.0),
        )
    }
}

impl SequentialSymlogScale {
    /// Create a symlog sequential scale with constant 1 and domain [0, 1].
    pub fn new() -> Self {
        Self {
            inner: SequentialScale::new(),
            constant_value: 1.0,
        }
    }

    /// Set the symlog constant (default 1, like d3).
    pub fn constant(mut self, c: f64) -> Self {
        self.constant_value = c;
        self
    }

    /// Set the domain.
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        self.inner = self.inner.domain(min, max);
        self
    }

    /// Set the interpolator mapping normalized `t` in [0, 1] to a color.
    pub fn interpolator(mut self, f: impl Fn(f64) -> D3Color + 'static) -> Self {
        self.inner = self.inner.interpolator(f);
        self
    }

    /// Set discrete output colors, interpolated piecewise.
    pub fn range(mut self, values: Vec<D3Color>) -> Self {
        self.inner = self.inner.range(values);
        self
    }

    /// Clamp normalized values to [0, 1].
    pub fn clamp(mut self, enabled: bool) -> Self {
        self.inner = self.inner.clamp(enabled);
        self
    }


    /// Copy the scale.
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

impl Scale<f64, D3Color> for SequentialSymlogScale {
    fn scale(&self, value: f64) -> D3Color {
        let [d0, d1] = self.inner.domain;
        let t = normalize_transformed(
            value,
            d0,
            d1,
            self.inner.clamped,
            transform_symlog(self.constant_value),
        );
        if t.is_nan() {
            return unknown_color();
        }
        (self.inner.interpolator)(t)
    }

    fn invert(&self, _value: D3Color) -> Option<f64> {
        None
    }

    fn ticks(&self, count: usize) -> Vec<f64> {
        Scale::ticks(&self.inner, count)
    }

    fn domain(&self) -> (f64, f64) {
        Scale::domain(&self.inner)
    }

    fn range(&self) -> (D3Color, D3Color) {
        (
            (self.inner.interpolator)(0.0),
            (self.inner.interpolator)(1.0),
        )
    }
}

impl Scale<f64, D3Color> for DivergingScale {
    fn scale(&self, value: f64) -> D3Color {
        if value.is_nan() {
            return unknown_color();
        }
        let [d0, d1, d2] = self.domain;
        // The half containing d0 ("lower") vs the half containing d2.
        let side = (value - d1) * (d0 - d1);
        let t = if side == 0.0 {
            0.5
        } else if side > 0.0 {
            let span = d1 - d0;
            if span == 0.0 {
                0.25
            } else {
                0.5 * (value - d0) / span
            }
        } else {
            let span = d2 - d1;
            if span == 0.0 {
                0.75
            } else {
                0.5 + 0.5 * (value - d1) / span
            }
        };
        let t = if self.clamped { t.clamp(0.0, 1.0) } else { t };
        if t.is_nan() {
            return unknown_color();
        }
        (self.interpolator)(t)
    }

    fn invert(&self, _value: D3Color) -> Option<f64> {
        None
    }

    fn ticks(&self, count: usize) -> Vec<f64> {
        let lo = self.domain[0].min(self.domain[1]).min(self.domain[2]);
        let hi = self.domain[0].max(self.domain[1]).max(self.domain[2]);
        generate_linear_ticks(lo, hi, count)
    }

    fn domain(&self) -> (f64, f64) {
        let lo = self.domain[0].min(self.domain[1]).min(self.domain[2]);
        let hi = self.domain[0].max(self.domain[1]).max(self.domain[2]);
        (lo, hi)
    }

    fn range(&self) -> (D3Color, D3Color) {
        ((self.interpolator)(0.0), (self.interpolator)(1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::SequentialScheme;

    fn gray(t: f64) -> D3Color {
        SequentialScale::new().interpolator(|_| D3Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        let _ = t;
        D3Color {
            r: t as f32,
            g: t as f32,
            b: t as f32,
            a: 1.0,
        }
    }

    #[test]
    fn sequential_maps_domain_to_interpolator() {
        let scale = SequentialScale::new()
            .domain(0.0, 100.0)
            .interpolator(gray);
        assert_eq!(scale.scale(0.0).r, 0.0);
        assert_eq!(scale.scale(100.0).r, 1.0);
        assert!((scale.scale(50.0).r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sequential_descending_and_clamp() {
        let scale = SequentialScale::new()
            .domain(100.0, 0.0)
            .interpolator(gray);
        assert_eq!(scale.scale(100.0).r, 0.0);
        assert_eq!(scale.scale(0.0).r, 1.0);
        // Unclamped extrapolation passes t through.
        assert!((scale.scale(-50.0).r - 1.5).abs() < 1e-6);
        let clamped = SequentialScale::new()
            .domain(100.0, 0.0)
            .clamp(true)
            .interpolator(gray);
        assert_eq!(clamped.scale(-50.0).r, 1.0);
        // NaN maps to transparent unknown.
        assert_eq!(scale.scale(f64::NAN).a, 0.0);
    }

    #[test]
    fn sequential_range_piecewise() {
        let black = D3Color::rgb(0, 0, 0);
        let white = D3Color::rgb(255, 255, 255);
        let scale = SequentialScale::new()
            .domain(0.0, 1.0)
            .range(vec![black, white]);
        let mid = scale.scale(0.5);
        assert!((mid.r - 0.5).abs() < 0.01);
        assert_eq!(scale.scale(0.0).r, 0.0);
        assert_eq!(scale.scale(1.0).r, 1.0);
    }

    #[test]
    fn sequential_ticks_follow_domain() {
        let scale = SequentialScale::new().domain(0.0, 100.0);
        let ticks = scale.ticks(5);
        assert!(ticks.contains(&0.0));
        assert!(ticks.contains(&100.0));
    }

    #[test]
    fn diverging_pins_midpoint() {
        let scale = DivergingScale::new()
            .domain(-10.0, 0.0, 10.0)
            .interpolator(gray);
        assert_eq!(scale.scale(-10.0).r, 0.0);
        assert_eq!(scale.scale(0.0).r, 0.5);
        assert_eq!(scale.scale(10.0).r, 1.0);
        assert!((scale.scale(5.0).r - 0.75).abs() < 1e-9);
    }

    #[test]
    fn diverging_descending_domain() {
        let scale = DivergingScale::new()
            .domain(10.0, 0.0, -10.0)
            .interpolator(gray);
        assert_eq!(scale.scale(10.0).r, 0.0);
        assert_eq!(scale.scale(0.0).r, 0.5);
        assert_eq!(scale.scale(-10.0).r, 1.0);
        assert!((scale.scale(-5.0).r - 0.75).abs() < 1e-9);
    }

    #[test]
    fn diverging_range_three_stops() {
        let red = D3Color::rgb(255, 0, 0);
        let white = D3Color::rgb(255, 255, 255);
        let blue = D3Color::rgb(0, 0, 255);
        let scale = DivergingScale::new()
            .domain(0.0, 5.0, 10.0)
            .range([red, white, blue]);
        let mid = scale.scale(5.0);
        assert!(mid.r > 0.99 && mid.g > 0.99 && mid.b > 0.99);
        let lo = scale.scale(0.0);
        assert!(lo.r > 0.99 && lo.g < 0.01);
    }

    #[test]
    fn pow_variant_matches_hand_computation() {
        let scale = SequentialPowScale::new()
            .domain(0.0, 100.0)
            .exponent(2.0)
            .interpolator(gray);
        // t = (50^2 - 0) / (100^2 - 0) = 0.25.
        assert!((scale.scale(50.0).r - 0.25).abs() < 1e-9);
        let sqrt = SequentialPowScale::new()
            .domain(0.0, 100.0)
            .exponent(0.5)
            .interpolator(gray);
        // t = sqrt(25)/sqrt(100) = 0.5.
        assert!((sqrt.scale(25.0).r - 0.5).abs() < 1e-9);
    }

    #[test]
    fn log_variant_matches_hand_computation() {
        let scale = SequentialLogScale::new()
            .domain(1.0, 1000.0)
            .interpolator(gray);
        // t = ln(10)/ln(1000) = 1/3.
        assert!((scale.scale(10.0).r - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn symlog_variant_handles_zero_and_negatives() {
        let scale = SequentialSymlogScale::new()
            .domain(-10.0, 10.0)
            .interpolator(gray);
        let t0 = scale.scale(0.0).r as f64;
        let tp = scale.scale(10.0).r as f64;
        let tn = scale.scale(-10.0).r as f64;
        assert!((t0 - 0.5).abs() < 1e-9);
        assert!((tp + tn - 1.0).abs() < 1e-9);
    }

    #[test]
    fn sequential_drives_chromatic_ramp() {
        let magma = SequentialScheme::magma();
        let scale = SequentialScale::new()
            .domain(8.0, 0.0)
            .interpolator(move |t| magma.get(t));
        // Descending domain maps height 8 -> t=0, height 0 -> t=1.
        let hi = scale.scale(8.0);
        let lo = scale.scale(0.0);
        assert!(hi.r != lo.r || hi.g != lo.g || hi.b != lo.b);
    }
}
