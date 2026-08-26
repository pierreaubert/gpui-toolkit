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
            let _ = write!(buf, "{:.0}M", millions);
        } else {
            let _ = write!(buf, "{:.1}M", millions);
        }
    } else if abs_value >= 1_000.0 {
        // Thousands: 1k, 10k, 100k, etc.
        let thousands = value / 1_000.0;
        if thousands.fract().abs() < 1e-10 {
            let _ = write!(buf, "{:.0}k", thousands);
        } else {
            let _ = write!(buf, "{:.1}k", thousands);
        }
    } else if abs_value >= 1.0 {
        // Regular values >= 1
        if value.fract().abs() < 1e-10 {
            let _ = write!(buf, "{:.0}", value);
        } else {
            let _ = write!(buf, "{:.1}", value);
        }
    } else {
        // Small values < 1
        let _ = write!(buf, "{:.2}", value);
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

thread_local! {
    /// Cache for generated log tick vectors keyed by (min_bits, max_bits).
    /// Log scales require strictly positive domains, so NaN/infinity are not
    /// expected here; the bit pattern is stable for finite positive values.
    static LOG_TICK_CACHE: RefCell<HashMap<(u64, u64), Vec<f64>>> = RefCell::new(HashMap::new());
}

const MAX_LOG_TICK_CACHE_ENTRIES: usize = 256;

/// Generate smart tick values for log scales to prevent label collision
/// Shows 1,2,3,4,5,10,20,30,40,50,100,... pattern
pub(super) fn generate_log_ticks(min: f64, max: f64) -> Vec<f64> {
    let key = (min.to_bits(), max.to_bits());
    LOG_TICK_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow().get(&key) {
            return cached.clone();
        }

        let ticks = generate_log_ticks_uncached(min, max);
        let mut cache = cache.borrow_mut();
        if cache.len() >= MAX_LOG_TICK_CACHE_ENTRIES {
            cache.clear();
        }
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

    ticks.sort_by(|a, b| a.total_cmp(b));
    ticks.dedup();
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

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
        LOG_TICK_CACHE.with(|cache| cache.borrow_mut().clear());

        for index in 0..=MAX_LOG_TICK_CACHE_ENTRIES {
            let min = 1.0 + index as f64;
            let _ = generate_log_ticks(min, min * 10.0);
        }

        LOG_TICK_CACHE.with(|cache| {
            assert!(cache.borrow().len() <= MAX_LOG_TICK_CACHE_ENTRIES);
        });
    }
}
