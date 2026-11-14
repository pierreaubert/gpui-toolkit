use gpui::Rgba;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Helper to create Rgba with alpha
pub(super) fn rgba(hex: u32, alpha: f32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: alpha,
    }
}

thread_local! {
    /// Reusable scratch buffer for formatting log tick labels.
    static TICK_LABEL_BUF: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Format tick labels for log scales with k/M suffixes into `buf`.
pub(super) fn format_log_tick_into(value: f64, buf: &mut String) {
    buf.clear();

    let abs_value = value.abs();

    // Handle zero
    if abs_value < 1e-10 {
        buf.push('0');
        return;
    }

    // Format based on magnitude
    if abs_value >= 1_000_000.0 {
        // Millions: 1M, 2M, etc.
        let millions = value / 1_000_000.0;
        if millions.fract().abs() < 1e-10 {
            let _ = write!(buf, "{millions:.0}M");
        } else {
            let _ = write!(buf, "{millions:.1}M");
        }
    } else if abs_value >= 1_000.0 {
        // Thousands: 1k, 10k, 100k, etc.
        let thousands = value / 1_000.0;
        if thousands.fract().abs() < 1e-10 {
            let _ = write!(buf, "{thousands:.0}k");
        } else {
            let _ = write!(buf, "{thousands:.1}k");
        }
    } else if abs_value >= 1.0 {
        // Regular values >= 1
        if value.fract().abs() < 1e-10 {
            let _ = write!(buf, "{value:.0}");
        } else {
            let _ = write!(buf, "{value:.1}");
        }
    } else {
        // Small values < 1
        let _ = write!(buf, "{value:.2}");
    }
}

/// Format tick labels for log scales with k/M suffixes.
pub(super) fn format_log_tick(value: f64) -> String {
    TICK_LABEL_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        format_log_tick_into(value, &mut buf);
        buf.clone()
    })
}

/// Maximum retained log-tick vectors.
const MAX_LOG_TICK_CACHE_ENTRIES: usize = 256;
/// Maximum retained bytes (tick payloads plus key/map overhead charge).
const MAX_LOG_TICK_CACHE_BYTES: usize = 256 * 1024;

/// Retained-byte and activity counters for the log-tick cache.
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LogTickCacheStats {
    pub entries: usize,
    pub retained_bytes: usize,
    pub high_water_bytes: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

/// Byte-budgeted LRU for generated log tick vectors. Previously the
/// cache cleared wholesale at 256 entries, so a working set just over
/// the cap re-generated every vector on every render.
#[derive(Default)]
struct LogTickCache {
    entries: HashMap<(u64, u64), Vec<f64>>,
    order: std::collections::VecDeque<(u64, u64)>,
    retained_bytes: usize,
    high_water_bytes: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
}

fn log_tick_charge(ticks: &[f64]) -> usize {
    // Key (16) + Vec header (24) + map slot slack (32) + payload.
    72 + std::mem::size_of_val(ticks)
}

impl LogTickCache {
    fn get(&mut self, key: &(u64, u64)) -> Option<Vec<f64>> {
        let ticks = self.entries.get(key)?.clone();
        self.hits = self.hits.saturating_add(1);
        self.order.retain(|candidate| candidate != key);
        self.order.push_back(*key);
        Some(ticks)
    }

    fn insert(&mut self, key: (u64, u64), ticks: Vec<f64>) {
        let charge = log_tick_charge(&ticks);
        if charge > MAX_LOG_TICK_CACHE_BYTES {
            self.evictions = self.evictions.saturating_add(1);
            return;
        }
        if let Some(old) = self.entries.remove(&key) {
            self.retained_bytes = self.retained_bytes.saturating_sub(log_tick_charge(&old));
            self.order.retain(|candidate| *candidate != key);
        }
        while !self.entries.is_empty()
            && (self.entries.len() >= MAX_LOG_TICK_CACHE_ENTRIES
                || self.retained_bytes.saturating_add(charge) > MAX_LOG_TICK_CACHE_BYTES)
        {
            let Some(victim) = self.order.pop_front() else {
                break;
            };
            if let Some(evicted) = self.entries.remove(&victim) {
                self.retained_bytes = self
                    .retained_bytes
                    .saturating_sub(log_tick_charge(&evicted));
                self.evictions = self.evictions.saturating_add(1);
            }
        }
        self.retained_bytes = self.retained_bytes.saturating_add(charge);
        self.high_water_bytes = self.high_water_bytes.max(self.retained_bytes);
        self.entries.insert(key, ticks);
        self.order.push_back(key);
    }

    #[cfg(test)]
    fn stats(&self) -> LogTickCacheStats {
        LogTickCacheStats {
            entries: self.entries.len(),
            retained_bytes: self.retained_bytes,
            high_water_bytes: self.high_water_bytes,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
        }
    }

    #[cfg(test)]
    fn trim(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.retained_bytes = 0;
        self.entries.shrink_to_fit();
        self.order.shrink_to_fit();
    }
}

thread_local! {
    /// Cache for generated log tick vectors keyed by (min_bits, max_bits).
    /// Log scales require strictly positive domains, so NaN/infinity are not
    /// expected here; the bit pattern is stable for finite positive values.
    static LOG_TICK_CACHE: RefCell<LogTickCache> = RefCell::new(LogTickCache::default());
}

/// Current retained-byte/activity counters of the log-tick cache.
#[cfg(test)]
pub(crate) fn log_tick_cache_stats() -> LogTickCacheStats {
    LOG_TICK_CACHE.with(|cache| cache.borrow().stats())
}

/// Release every retained log-tick vector (memory-pressure hook).
#[cfg(test)]
pub(crate) fn trim_log_tick_cache() {
    LOG_TICK_CACHE.with(|cache| cache.borrow_mut().trim());
}

/// Generate smart tick values for log scales to prevent label collision
/// Shows 1,2,3,4,5,10,20,30,40,50,100,... pattern
pub(super) fn generate_log_ticks(min: f64, max: f64) -> Vec<f64> {
    let key = (min.to_bits(), max.to_bits());
    LOG_TICK_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow_mut().get(&key) {
            return cached;
        }
        let ticks = generate_log_ticks_uncached(min, max);
        let mut cache = cache.borrow_mut();
        cache.misses = cache.misses.saturating_add(1);
        cache.insert(key, ticks.clone());
        ticks
    })
}

fn generate_log_ticks_uncached(min: f64, max: f64) -> Vec<f64> {
    let mut ticks = Vec::new();

    // Find the starting decade (power of 10)
    let start_exp = min.log10().floor() as i32;
    let end_exp = max.log10().ceil() as i32;

    for exp in start_exp..=end_exp {
        let base = 10_f64.powi(exp);

        // For each decade, show: 1, 2, 3, 4, 5, 10 (which becomes 1 of next decade)
        // This gives us: 1k, 2k, 3k, 4k, 5k, 10k, 20k, 30k, 40k, 50k, 100k, etc.
        for multiplier in [1.0, 2.0, 3.0, 4.0, 5.0] {
            let tick = base * multiplier;
            if tick >= min && tick <= max {
                ticks.push(tick);
            }
        }
    }

    // Add the final decade marker if we don't already have it
    let final_decade = 10_f64.powi(end_exp);
    if final_decade <= max && !ticks.contains(&final_decade) {
        ticks.push(final_decade);
    }

    ticks.sort_by(f64::total_cmp);
    ticks.dedup();
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_tick_cache_plateaus_and_keeps_recent_vectors() {
        trim_log_tick_cache();
        for i in 0..1000 {
            let _ = generate_log_ticks(1.0 + f64::from(i) * 0.001, 1000.0);
        }
        let stats = log_tick_cache_stats();
        assert!(stats.entries <= MAX_LOG_TICK_CACHE_ENTRIES, "{stats:?}");
        assert!(
            stats.retained_bytes <= MAX_LOG_TICK_CACHE_BYTES,
            "{stats:?}"
        );
        assert!(stats.evictions > 0, "{stats:?}");
        // The newest vector still hits (no wholesale-clear thrash).
        let hits = stats.hits;
        let _ = generate_log_ticks(1.0 + 999.0 * 0.001, 1000.0);
        assert_eq!(log_tick_cache_stats().hits, hits + 1);
        trim_log_tick_cache();
        assert_eq!(log_tick_cache_stats().entries, 0);
    }

    #[test]
    fn test_format_log_tick_zero() {
        assert_eq!(format_log_tick(0.0), "0");
    }

    #[test]
    fn test_format_log_tick_millions() {
        assert_eq!(format_log_tick(2_000_000.0), "2M");
        assert_eq!(format_log_tick(2_500_000.0), "2.5M");
    }

    #[test]
    fn test_format_log_tick_thousands() {
        assert_eq!(format_log_tick(3_000.0), "3k");
        assert_eq!(format_log_tick(3_500.0), "3.5k");
    }

    #[test]
    fn test_format_log_tick_regular() {
        assert_eq!(format_log_tick(42.0), "42");
        assert_eq!(format_log_tick(4.2), "4.2");
    }

    #[test]
    fn test_format_log_tick_small() {
        assert_eq!(format_log_tick(0.42), "0.42");
    }

    #[test]
    fn test_format_log_tick_into_matches_format_log_tick() {
        let values = [
            0.0,
            2_000_000.0,
            2_500_000.0,
            3_000.0,
            3_500.0,
            42.0,
            4.2,
            0.42,
        ];
        for &value in &values {
            let mut buf = String::new();
            format_log_tick_into(value, &mut buf);
            assert_eq!(buf, format_log_tick(value));
        }
    }

    #[test]
    fn test_generate_log_ticks_basic() {
        let ticks = generate_log_ticks(1.0, 100.0);
        assert!(ticks.contains(&1.0));
        assert!(ticks.contains(&10.0));
        assert!(ticks.contains(&100.0));
        assert!(ticks.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn test_generate_log_ticks_cache_reuse() {
        // Populate the cache.
        let first = generate_log_ticks(10.0, 1_000.0);
        // Second call should return a clone of the cached vector.
        let second = generate_log_ticks(10.0, 1_000.0);
        assert_eq!(first, second);
    }

    #[test]
    fn log_tick_cache_has_a_bounded_number_of_domains() {
        trim_log_tick_cache();

        for index in 0..=MAX_LOG_TICK_CACHE_ENTRIES {
            let min = 1.0 + index as f64;
            let _ = generate_log_ticks(min, min * 10.0);
        }

        assert!(log_tick_cache_stats().entries <= MAX_LOG_TICK_CACHE_ENTRIES);
    }
}
