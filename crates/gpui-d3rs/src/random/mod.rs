//! Random number generators (d3-random)
//!
//! This module provides various random number distributions useful for visualization.
//!
//! Note: These are simple implementations for visualization purposes, not cryptographically secure.
//!
//! # Example
//!
//! ```
//! use d3rs::random::{RandomUniform, RandomNormal};
//!
//! let uniform = RandomUniform::new(0.0, 100.0);
//! let value = uniform.sample();
//! assert!(value >= 0.0 && value < 100.0);
//!
//! let normal = RandomNormal::new(0.0, 1.0);
//! let value = normal.sample(); // Standard normal distribution
//! ```

use std::cell::Cell;

/// A simple linear congruential generator for reproducible random numbers
#[derive(Debug, Clone)]
pub struct LcgRng {
    state: Cell<u64>,
}

impl LcgRng {
    const A: u64 = 6364136223846793005;
    const C: u64 = 1442695040888963407;

    /// Create a new RNG with the given seed
    pub fn new(seed: u64) -> Self {
        Self {
            state: Cell::new(seed),
        }
    }

    /// Create a new RNG with a default seed based on system time
    pub fn default_seed() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42);
        Self::new(seed)
    }

    /// Generate the next random value in [0, 1)
    pub fn next_f64(&self) -> f64 {
        let state = self.state.get();
        let new_state = state.wrapping_mul(Self::A).wrapping_add(Self::C);
        self.state.set(new_state);
        (new_state >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Generate a random integer in [0, max)
    pub fn next_u64(&self, max: u64) -> u64 {
        (self.next_f64() * max as f64) as u64
    }
}

impl Default for LcgRng {
    fn default() -> Self {
        Self::default_seed()
    }
}

/// Uniform distribution random generator
///
/// Generates random numbers uniformly distributed in [min, max).
#[derive(Debug, Clone)]
pub struct RandomUniform {
    rng: LcgRng,
    min: f64,
    max: f64,
}

impl RandomUniform {
    /// Create a uniform generator in [min, max)
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            min,
            max,
        }
    }

    /// Create a uniform generator with a specific seed
    pub fn with_seed(min: f64, max: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            min,
            max,
        }
    }

    /// Create a uniform generator in [0, 1)
    pub fn unit() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Sample a random value
    pub fn sample(&self) -> f64 {
        self.min + self.rng.next_f64() * (self.max - self.min)
    }
}

/// Normal (Gaussian) distribution random generator
///
/// Uses the Box-Muller transform.
#[derive(Debug, Clone)]
pub struct RandomNormal {
    rng: LcgRng,
    mean: f64,
    std_dev: f64,
}

impl RandomNormal {
    /// Create a normal generator with given mean and standard deviation
    pub fn new(mean: f64, std_dev: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            mean,
            std_dev,
        }
    }

    /// Create a normal generator with a specific seed
    pub fn with_seed(mean: f64, std_dev: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            mean,
            std_dev,
        }
    }

    /// Create a standard normal generator (mean=0, std_dev=1)
    pub fn standard() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Sample a random value using Box-Muller transform
    pub fn sample(&self) -> f64 {
        let u1 = self.rng.next_f64();
        let u2 = self.rng.next_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        self.mean + z * self.std_dev
    }
}

/// Log-normal distribution random generator
#[derive(Debug, Clone)]
pub struct RandomLogNormal {
    normal: RandomNormal,
}

impl RandomLogNormal {
    /// Create a log-normal generator
    ///
    /// The parameters mu and sigma are the mean and standard deviation
    /// of the underlying normal distribution.
    pub fn new(mu: f64, sigma: f64) -> Self {
        Self {
            normal: RandomNormal::new(mu, sigma),
        }
    }

    /// Create with a specific seed
    pub fn with_seed(mu: f64, sigma: f64, seed: u64) -> Self {
        Self {
            normal: RandomNormal::with_seed(mu, sigma, seed),
        }
    }

    /// Sample a random value
    pub fn sample(&self) -> f64 {
        self.normal.sample().exp()
    }
}

/// Exponential distribution random generator
#[derive(Debug, Clone)]
pub struct RandomExponential {
    rng: LcgRng,
    lambda: f64,
}

impl RandomExponential {
    /// Create an exponential generator with rate parameter lambda
    pub fn new(lambda: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            lambda,
        }
    }

    /// Create with a specific seed
    pub fn with_seed(lambda: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            lambda,
        }
    }

    /// Sample a random value
    pub fn sample(&self) -> f64 {
        -self.rng.next_f64().ln() / self.lambda
    }
}

/// Bernoulli distribution random generator
#[derive(Debug, Clone)]
pub struct RandomBernoulli {
    rng: LcgRng,
    p: f64,
}

impl RandomBernoulli {
    /// Create a Bernoulli generator with probability p
    pub fn new(p: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            p: p.clamp(0.0, 1.0),
        }
    }

    /// Create with a specific seed
    pub fn with_seed(p: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            p: p.clamp(0.0, 1.0),
        }
    }

    /// Sample a random boolean
    pub fn sample(&self) -> bool {
        self.rng.next_f64() < self.p
    }

    /// Sample as 0 or 1
    pub fn sample_int(&self) -> u32 {
        if self.sample() { 1 } else { 0 }
    }
}

/// Poisson distribution random generator
#[derive(Debug, Clone)]
pub struct RandomPoisson {
    rng: LcgRng,
    lambda: f64,
}

impl RandomPoisson {
    /// Create a Poisson generator with rate parameter lambda
    pub fn new(lambda: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            lambda,
        }
    }

    /// Create with a specific seed
    pub fn with_seed(lambda: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            lambda,
        }
    }

    /// Sample a random value using the Knuth algorithm
    pub fn sample(&self) -> u64 {
        let l = (-self.lambda).exp();
        let mut k = 0u64;
        let mut p = 1.0;

        loop {
            k += 1;
            p *= self.rng.next_f64();
            if p <= l {
                break;
            }
        }

        k - 1
    }
}

/// Irwin-Hall distribution (sum of n uniform random variables)
#[derive(Debug, Clone)]
pub struct RandomIrwinHall {
    rng: LcgRng,
    n: usize,
}

impl RandomIrwinHall {
    /// Create an Irwin-Hall generator with n uniform summands
    pub fn new(n: usize) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            n,
        }
    }

    /// Create with a specific seed
    pub fn with_seed(n: usize, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            n,
        }
    }

    /// Sample a random value
    pub fn sample(&self) -> f64 {
        (0..self.n).map(|_| self.rng.next_f64()).sum()
    }
}

/// Bates distribution (mean of n uniform random variables)
#[derive(Debug, Clone)]
pub struct RandomBates {
    irwin_hall: RandomIrwinHall,
}

impl RandomBates {
    /// Create a Bates generator with n uniform summands
    pub fn new(n: usize) -> Self {
        Self {
            irwin_hall: RandomIrwinHall::new(n),
        }
    }

    /// Create with a specific seed
    pub fn with_seed(n: usize, seed: u64) -> Self {
        Self {
            irwin_hall: RandomIrwinHall::with_seed(n, seed),
        }
    }

    /// Sample a random value
    pub fn sample(&self) -> f64 {
        self.irwin_hall.sample() / self.irwin_hall.n as f64
    }
}

/// Generate a shuffled copy of a slice
pub fn shuffle<T: Clone>(rng: &LcgRng, data: &[T]) -> Vec<T> {
    let mut result = data.to_vec();
    shuffle_in_place(rng, &mut result);
    result
}

/// Shuffle a slice in place using Fisher-Yates algorithm
pub fn shuffle_in_place<T>(rng: &LcgRng, data: &mut [T]) {
    let n = data.len();
    for i in (1..n).rev() {
        let j = rng.next_u64(i as u64 + 1) as usize;
        data.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_range() {
        let uniform = RandomUniform::with_seed(0.0, 100.0, 12345);
        for _ in 0..1000 {
            let v = uniform.sample();
            assert!((0.0..100.0).contains(&v));
        }
    }

    #[test]
    fn test_uniform_reproducible() {
        let u1 = RandomUniform::with_seed(0.0, 1.0, 42);
        let u2 = RandomUniform::with_seed(0.0, 1.0, 42);
        for _ in 0..100 {
            assert_eq!(u1.sample(), u2.sample());
        }
    }

    #[test]
    fn test_normal_distribution() {
        let normal = RandomNormal::with_seed(0.0, 1.0, 12345);
        let samples: Vec<f64> = (0..10000).map(|_| normal.sample()).collect();

        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance: f64 =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (samples.len() - 1) as f64;

        // Mean should be close to 0, variance close to 1
        assert!(mean.abs() < 0.1);
        assert!((variance - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_exponential() {
        let exp = RandomExponential::with_seed(1.0, 12345);
        let samples: Vec<f64> = (0..10000).map(|_| exp.sample()).collect();

        // All values should be non-negative
        assert!(samples.iter().all(|&x| x >= 0.0));

        // Mean should be close to 1/lambda = 1
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_bernoulli() {
        let bern = RandomBernoulli::with_seed(0.7, 12345);
        let count: u32 = (0..10000).map(|_| bern.sample_int()).sum();
        let proportion = count as f64 / 10000.0;

        // Should be close to 0.7
        assert!((proportion - 0.7).abs() < 0.05);
    }

    #[test]
    fn test_shuffle() {
        let rng = LcgRng::new(12345);
        let data = vec![1, 2, 3, 4, 5];
        let shuffled = shuffle(&rng, &data);

        // Same elements
        let mut sorted = shuffled.clone();
        sorted.sort();
        assert_eq!(sorted, data);

        // Usually different order (extremely unlikely to be same with seed 12345)
        assert_ne!(shuffled, data);
    }

    #[test]
    fn test_log_normal() {
        let ln = RandomLogNormal::with_seed(0.0, 0.5, 12345);
        let samples: Vec<f64> = (0..1000).map(|_| ln.sample()).collect();

        // All values should be positive
        assert!(samples.iter().all(|&x| x > 0.0));
    }

    #[test]
    fn test_irwin_hall() {
        let ih = RandomIrwinHall::with_seed(12, 12345);
        let samples: Vec<f64> = (0..1000).map(|_| ih.sample()).collect();

        // Values should be in [0, n]
        assert!(samples.iter().all(|&x| (0.0..=12.0).contains(&x)));

        // Mean should be close to n/2 = 6
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 6.0).abs() < 0.5);
    }

    #[test]
    fn test_bates() {
        let bates = RandomBates::with_seed(12, 12345);
        let samples: Vec<f64> = (0..1000).map(|_| bates.sample()).collect();

        // Values should be in [0, 1]
        assert!(samples.iter().all(|&x| (0.0..=1.0).contains(&x)));

        // Mean should be close to 0.5
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 0.5).abs() < 0.1);
    }

    #[test]
    fn constructors_and_defaults_produce_finite_samples() {
        assert!((0.0..1.0).contains(&RandomUniform::unit().sample()));
        assert!((10.0..20.0).contains(&RandomUniform::new(10.0, 20.0).sample()));
        assert!(RandomNormal::new(3.0, 2.0).sample().is_finite());
        assert!(RandomNormal::standard().sample().is_finite());
        assert!(RandomLogNormal::new(0.0, 0.5).sample().is_sign_positive());
        assert!(RandomExponential::new(2.0).sample().is_sign_positive());
        assert!(RandomPoisson::new(2.0).sample() < u64::MAX);
        assert!((0.0..=4.0).contains(&RandomIrwinHall::new(4).sample()));
        assert!((0.0..=1.0).contains(&RandomBates::new(4).sample()));
        let _ = LcgRng::default().next_f64();
    }

    #[test]
    fn integer_generation_and_shuffle_cover_edge_cases() {
        let rng = LcgRng::new(7);
        assert_eq!(rng.next_u64(0), 0);
        for _ in 0..100 {
            assert!(rng.next_u64(3) < 3);
        }

        let mut empty: [u8; 0] = [];
        shuffle_in_place(&rng, &mut empty);
        let mut singleton = [1];
        shuffle_in_place(&rng, &mut singleton);
        assert_eq!(singleton, [1]);
    }

    #[test]
    fn bernoulli_clamps_probability_and_poisson_is_reproducible() {
        assert!(!(0..20).any(|_| RandomBernoulli::with_seed(-1.0, 1).sample()));
        assert!((0..20).all(|_| RandomBernoulli::new(2.0).sample()));

        let first = RandomPoisson::with_seed(3.0, 42);
        let second = RandomPoisson::with_seed(3.0, 42);
        for _ in 0..20 {
            assert_eq!(first.sample(), second.sample());
        }
    }
}

/// Standard normal draw from a raw RNG (Box-Muller), shared by samplers.
fn standard_normal_sample(rng: &LcgRng) -> f64 {
    let u1 = rng.next_f64();
    let u2 = rng.next_f64();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

/// Integer distribution random generator (d3 `randomInt`).
///
/// Generates integers `n` with `min <= n < max`.
#[derive(Debug, Clone)]
pub struct RandomInt {
    rng: LcgRng,
    min: i64,
    max: i64,
}

impl RandomInt {
    /// Create an integer generator for `[min, max)`.
    pub fn new(min: i64, max: i64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            min,
            max,
        }
    }

    /// Create with a specific seed.
    pub fn with_seed(min: i64, max: i64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            min,
            max,
        }
    }

    /// Sample a random integer.
    pub fn sample(&self) -> i64 {
        let span = (self.max - self.min).max(1) as f64;
        self.min + (self.rng.next_f64() * span).floor() as i64
    }
}

/// Pareto distribution random generator (d3 `randomPareto`).
///
/// Samples are `>= 1` with shape parameter `alpha`.
#[derive(Debug, Clone)]
pub struct RandomPareto {
    rng: LcgRng,
    alpha: f64,
}

impl RandomPareto {
    /// Create a Pareto generator with shape `alpha`.
    pub fn new(alpha: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            alpha,
        }
    }

    /// Create with a specific seed.
    pub fn with_seed(alpha: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            alpha,
        }
    }

    /// Sample a random value.
    pub fn sample(&self) -> f64 {
        (1.0 - self.rng.next_f64()).powf(-1.0 / self.alpha)
    }
}

/// Geometric distribution random generator (d3 `randomGeometric`).
///
/// Counts failures before the first success; samples are `>= 0`.
#[derive(Debug, Clone)]
pub struct RandomGeometric {
    rng: LcgRng,
    p: f64,
}

impl RandomGeometric {
    /// Create a geometric generator with success probability `p`.
    pub fn new(p: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            p: p.clamp(0.0, 1.0),
        }
    }

    /// Create with a specific seed.
    pub fn with_seed(p: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            p: p.clamp(0.0, 1.0),
        }
    }

    /// Sample a random value.
    pub fn sample(&self) -> u64 {
        if self.p >= 1.0 {
            return 0;
        }
        if self.p <= 0.0 {
            return u64::MAX;
        }
        (self.rng.next_f64().ln() / (1.0 - self.p).ln()).floor() as u64
    }
}

/// Gamma distribution sampler core (Marsaglia-Tsang), `k > 0`.
fn sample_gamma_shape(rng: &LcgRng, k: f64, theta: f64) -> f64 {
    // Boost k < 1 up: Gamma(k) = Gamma(k+1) * U^(1/k).
    if k < 1.0 {
        return sample_gamma_shape(rng, k + 1.0, theta) * rng.next_f64().powf(1.0 / k);
    }
    if k == 1.0 {
        return -rng.next_f64().ln() * theta;
    }
    let d = k - 1.0 / 3.0;
    let c = 1.0 / (3.0 * d.sqrt());
    loop {
        let x = standard_normal_sample(rng);
        let v = 1.0 + c * x;
        if v <= 0.0 {
            continue;
        }
        let v = v * v * v;
        let u = rng.next_f64();
        if u < 1.0 - 0.0331 * (x * x) * (x * x) {
            return d * v * theta;
        }
        if u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            return d * v * theta;
        }
    }
}

/// Gamma distribution random generator (d3 `randomGamma`).
#[derive(Debug, Clone)]
pub struct RandomGamma {
    rng: LcgRng,
    k: f64,
    theta: f64,
}

impl RandomGamma {
    /// Create a gamma generator with shape `k` and scale `theta`.
    pub fn new(k: f64, theta: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            k,
            theta,
        }
    }

    /// Create with a specific seed.
    pub fn with_seed(k: f64, theta: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            k,
            theta,
        }
    }

    /// Sample a random value (non-positive shapes yield 0).
    pub fn sample(&self) -> f64 {
        if self.k <= 0.0 || self.theta <= 0.0 {
            return 0.0;
        }
        sample_gamma_shape(&self.rng, self.k, self.theta)
    }
}

/// Beta distribution random generator (d3 `randomBeta`).
///
/// Samples lie in [0, 1].
#[derive(Debug, Clone)]
pub struct RandomBeta {
    x_gamma: RandomGamma,
    y_gamma: RandomGamma,
}

impl RandomBeta {
    /// Create a beta generator with shape parameters `alpha` and `beta`.
    pub fn new(alpha: f64, beta: f64) -> Self {
        Self {
            x_gamma: RandomGamma::new(alpha, 1.0),
            y_gamma: RandomGamma::new(beta, 1.0),
        }
    }

    /// Create with a specific seed.
    pub fn with_seed(alpha: f64, beta: f64, seed: u64) -> Self {
        Self {
            x_gamma: RandomGamma::with_seed(alpha, 1.0, seed),
            y_gamma: RandomGamma::with_seed(beta, 1.0, seed.wrapping_add(1)),
        }
    }

    /// Sample a random value.
    pub fn sample(&self) -> f64 {
        let x = self.x_gamma.sample();
        if x == 0.0 {
            return 0.0;
        }
        x / (x + self.y_gamma.sample())
    }
}

/// Weibull distribution random generator (d3 `randomWeibull`).
#[derive(Debug, Clone)]
pub struct RandomWeibull {
    rng: LcgRng,
    k: f64,
    a: f64,
    b: f64,
}

impl RandomWeibull {
    /// Create a Weibull generator with shape `k`, scale `a`, location `b`.
    pub fn new(k: f64, a: f64, b: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            k,
            a,
            b,
        }
    }

    /// Create a standard Weibull generator (`a = 1`, `b = 0`).
    pub fn standard(k: f64) -> Self {
        Self::new(k, 1.0, 0.0)
    }

    /// Create with a specific seed.
    pub fn with_seed(k: f64, a: f64, b: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            k,
            a,
            b,
        }
    }

    /// Sample a random value.
    pub fn sample(&self) -> f64 {
        self.a * (-self.rng.next_f64().ln()).powf(1.0 / self.k) + self.b
    }
}

/// Cauchy distribution random generator (d3 `randomCauchy`).
#[derive(Debug, Clone)]
pub struct RandomCauchy {
    rng: LcgRng,
    a: f64,
    b: f64,
}

impl RandomCauchy {
    /// Create a Cauchy generator with location `a` and scale `b`.
    pub fn new(a: f64, b: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            a,
            b,
        }
    }

    /// Create a standard Cauchy generator (`a = 0`, `b = 1`).
    pub fn standard() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Create with a specific seed.
    pub fn with_seed(a: f64, b: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            a,
            b,
        }
    }

    /// Sample a random value.
    pub fn sample(&self) -> f64 {
        self.a + self.b * (std::f64::consts::PI * self.rng.next_f64()).tan()
    }
}

/// Logistic distribution random generator (d3 `randomLogistic`).
#[derive(Debug, Clone)]
pub struct RandomLogistic {
    rng: LcgRng,
    a: f64,
    b: f64,
}

impl RandomLogistic {
    /// Create a logistic generator with location `a` and scale `b`.
    pub fn new(a: f64, b: f64) -> Self {
        Self {
            rng: LcgRng::default_seed(),
            a,
            b,
        }
    }

    /// Create a standard logistic generator (`a = 0`, `b = 1`).
    pub fn standard() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Create with a specific seed.
    pub fn with_seed(a: f64, b: f64, seed: u64) -> Self {
        Self {
            rng: LcgRng::new(seed),
            a,
            b,
        }
    }

    /// Sample a random value.
    pub fn sample(&self) -> f64 {
        let u = self.rng.next_f64().clamp(f64::EPSILON, 1.0 - f64::EPSILON);
        self.a + self.b * (u / (1.0 - u)).ln()
    }
}

/// Binomial distribution random generator (d3 `randomBinomial`).
///
/// Counts successes over `n` trials with probability `p`.
#[derive(Debug, Clone)]
pub struct RandomBinomial {
    trials: RandomBernoulli,
    n: u64,
}

impl RandomBinomial {
    /// Create a binomial generator with `n` trials and probability `p`.
    pub fn new(n: u64, p: f64) -> Self {
        Self {
            trials: RandomBernoulli::new(p),
            n,
        }
    }

    /// Create with a specific seed.
    pub fn with_seed(n: u64, p: f64, seed: u64) -> Self {
        Self {
            trials: RandomBernoulli::with_seed(p, seed),
            n,
        }
    }

    /// Sample a random value.
    pub fn sample(&self) -> u64 {
        (0..self.n).filter(|_| self.trials.sample()).count() as u64
    }
}

#[cfg(test)]
mod distribution_tests {
    use super::{
        RandomBates, RandomBernoulli, RandomBeta, RandomBinomial, RandomCauchy, RandomExponential,
        RandomGamma, RandomGeometric, RandomInt, RandomIrwinHall, RandomLogistic, RandomLogNormal,
        RandomNormal, RandomPareto, RandomPoisson, RandomUniform, RandomWeibull,
    };

    fn mean_f64(n: usize, mut f: impl FnMut() -> f64) -> f64 {
        (0..n).map(|_| f()).sum::<f64>() / n as f64
    }

    #[test]
    fn int_stays_in_range_and_reproduces() {
        let a = RandomInt::with_seed(1, 6, 11);
        let b = RandomInt::with_seed(1, 6, 11);
        for _ in 0..50 {
            let (x, y) = (a.sample(), b.sample());
            assert_eq!(x, y);
            assert!((1..6).contains(&x));
        }
    }

    #[test]
    fn pareto_shape_and_tail() {
        let g = RandomPareto::with_seed(2.0, 7);
        for _ in 0..100 {
            assert!(g.sample() >= 1.0);
        }
        // Mean of Pareto(1, 3) is 1.5.
        let pareto = RandomPareto::with_seed(3.0, 99);
        let m = mean_f64(2000, || pareto.sample());
        assert!((m - 1.5).abs() < 0.2, "mean {m}");
    }

    #[test]
    fn geometric_counts_failures() {
        assert_eq!(RandomGeometric::with_seed(1.0, 3).sample(), 0);
        let g = RandomGeometric::with_seed(0.5, 5);
        for _ in 0..100 {
            let _ = g.sample();
        }
        // Mean of Geometric(0.5) failures is (1-p)/p = 1.
        let geo = RandomGeometric::with_seed(0.5, 21);
        let m = mean_f64(2000, || geo.sample() as f64);
        assert!((m - 1.0).abs() < 0.2, "mean {m}");
    }

    #[test]
    fn gamma_beta_shapes() {
        let g = RandomGamma::with_seed(2.0, 3.0, 13);
        for _ in 0..50 {
            assert!(g.sample() > 0.0);
        }
        // Mean of Gamma(2, 3) is 6.
        let gamma = RandomGamma::with_seed(2.0, 3.0, 17);
        let m = mean_f64(2000, || gamma.sample());
        assert!((m - 6.0).abs() < 0.6, "mean {m}");
        assert_eq!(RandomGamma::with_seed(0.0, 1.0, 1).sample(), 0.0);

        let b = RandomBeta::with_seed(2.0, 5.0, 23);
        for _ in 0..100 {
            let v = b.sample();
            assert!((0.0..=1.0).contains(&v));
        }
        // Mean of Beta(2, 5) is 2/7.
        let beta = RandomBeta::with_seed(2.0, 5.0, 29);
        let m = mean_f64(2000, || beta.sample());
        assert!((m - 2.0 / 7.0).abs() < 0.05, "mean {m}");
    }

    #[test]
    fn weibull_cauchy_logistic_ranges() {
        let w = RandomWeibull::with_seed(1.5, 2.0, 1.0, 31);
        for _ in 0..100 {
            assert!(w.sample() >= 1.0);
        }
        let c = RandomCauchy::with_seed(0.0, 1.0, 37);
        assert!(c.sample().is_finite());
        let l = RandomLogistic::with_seed(0.0, 1.0, 41);
        for _ in 0..50 {
            assert!(l.sample().is_finite());
        }
        // Symmetric logistic mean is ~location.
        let logi = RandomLogistic::with_seed(5.0, 1.0, 43);
        let m = mean_f64(2000, || logi.sample());
        assert!((m - 5.0).abs() < 0.3, "mean {m}");
    }

    #[test]
    fn binomial_counts_successes() {
        let b = RandomBinomial::with_seed(10, 0.5, 47);
        for _ in 0..50 {
            assert!(b.sample() <= 10);
        }
        // Mean of Binomial(20, 0.25) is 5.
        let bin = RandomBinomial::with_seed(20, 0.25, 53);
        let m = mean_f64(500, || bin.sample() as f64);
        assert!((m - 5.0).abs() < 0.8, "mean {m}");
        assert_eq!(RandomBinomial::with_seed(10, 1.0, 1).sample(), 10);
    }

    #[test]
    fn preexisting_distributions_still_reproduce() {
        let a = RandomBernoulli::with_seed(0.3, 9);
        let b = RandomBernoulli::with_seed(0.3, 9);
        for _ in 0..20 {
            assert_eq!(a.sample(), b.sample());
        }
        let _ = (
            RandomUniform::unit(),
            RandomNormal::standard(),
            RandomLogNormal::new(0.0, 1.0),
            RandomExponential::new(1.0),
            RandomPoisson::new(1.0),
            RandomIrwinHall::new(2),
            RandomBates::new(2),
        );
    }
}
