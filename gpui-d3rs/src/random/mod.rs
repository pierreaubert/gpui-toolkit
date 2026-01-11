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
            assert!(v >= 0.0 && v < 100.0);
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
        assert!(samples.iter().all(|&x| x >= 0.0 && x <= 12.0));

        // Mean should be close to n/2 = 6
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 6.0).abs() < 0.5);
    }

    #[test]
    fn test_bates() {
        let bates = RandomBates::with_seed(12, 12345);
        let samples: Vec<f64> = (0..1000).map(|_| bates.sample()).collect();

        // Values should be in [0, 1]
        assert!(samples.iter().all(|&x| x >= 0.0 && x <= 1.0));

        // Mean should be close to 0.5
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 0.5).abs() < 0.1);
    }
}
