//! Tick generation utilities
//!
//! Implements Wilkinson's algorithm for generating nice tick values

/// Generate nice linear ticks for a given domain
///
/// Uses a simplified version of Wilkinson's algorithm to generate
/// approximately `count` tick values that are "nice" (round numbers).
/// Ticks are constrained to start at or after `min` and end at or before `max`.
pub fn generate_linear_ticks(min: f64, max: f64, count: usize) -> Vec<f64> {
    if count == 0 || min == max {
        return vec![min];
    }

    let range = max - min;
    let rough_step = range / (count as f64);
    let nice_step = nice_number(rough_step, false);

    // Start at the first nice tick >= min
    let tick_min = (min / nice_step).ceil() * nice_step;
    // End at the last nice tick <= max
    let tick_max = (max / nice_step).floor() * nice_step;

    let mut ticks = Vec::new();
    let mut tick = tick_min;

    // Small epsilon for floating point comparison
    let epsilon = nice_step * 1e-10;

    while tick <= tick_max + epsilon {
        ticks.push(tick);
        tick += nice_step;
    }

    // Ensure we have at least the domain bounds if no ticks were generated
    if ticks.is_empty() {
        ticks.push(min);
        ticks.push(max);
    }

    ticks
}

/// Find a "nice" number approximately equal to the range
///
/// Nice numbers are 1, 2, or 5 times a power of 10.
///
/// If `round` is true, rounds the number, otherwise takes the ceiling.
pub fn nice_number(range: f64, round: bool) -> f64 {
    let exponent = range.log10().floor();
    let fraction = range / 10_f64.powf(exponent);
    let nice_fraction = if round {
        if fraction < 1.5 {
            1.0
        } else if fraction < 3.0 {
            2.0
        } else if fraction < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice_fraction * 10_f64.powf(exponent)
}

/// Generate nice logarithmic ticks for a given domain
///
/// Generates ticks at powers of the base, with optional subdivisions.
pub fn generate_log_ticks(min: f64, max: f64, base: f64, subdivisions: bool) -> Vec<f64> {
    if min <= 0.0 || max <= 0.0 {
        return vec![];
    }

    let log_min = min.log(base).floor();
    let log_max = max.log(base).ceil();

    let mut ticks = Vec::new();

    let mut exp = log_min;
    while exp <= log_max {
        let tick = base.powf(exp);

        if tick >= min && tick <= max {
            ticks.push(tick);
        }

        // Add subdivisions (e.g., 20, 30, ..., 90 for base 10)
        if subdivisions && exp < log_max {
            for i in 2..base as i32 {
                let sub_tick = tick * i as f64;
                if sub_tick >= min && sub_tick <= max {
                    ticks.push(sub_tick);
                }
            }
        }

        exp += 1.0;
    }

    ticks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_nice_number() {
        assert_relative_eq!(nice_number(10.0, false), 10.0);
        assert_relative_eq!(nice_number(15.0, false), 20.0);
        assert_relative_eq!(nice_number(25.0, false), 50.0);
        assert_relative_eq!(nice_number(75.0, false), 100.0);
    }

    #[test]
    fn test_nice_number_round() {
        assert_relative_eq!(nice_number(10.0, true), 10.0);
        assert_relative_eq!(nice_number(12.0, true), 10.0);
        assert_relative_eq!(nice_number(18.0, true), 20.0);
        assert_relative_eq!(nice_number(35.0, true), 50.0);
    }

    #[test]
    fn test_generate_linear_ticks_basic() {
        let ticks = generate_linear_ticks(0.0, 100.0, 5);

        // Should generate nice round numbers
        assert!(ticks.len() >= 3);
        assert!(ticks.len() <= 11); // Reasonable upper bound
        // First tick should be >= min
        assert!(ticks[0] >= 0.0 - 1e-10);
        // Last tick should be <= max
        assert!(ticks[ticks.len() - 1] <= 100.0 + 1e-10);
    }

    #[test]
    fn test_generate_linear_ticks_range() {
        let ticks = generate_linear_ticks(-24.0, 24.0, 8);

        // Ticks should be within the domain bounds
        // First tick should be at or after min
        assert!(ticks[0] >= -24.0 - 1e-10);
        // Last tick should be at or before max
        assert!(ticks[ticks.len() - 1] <= 24.0 + 1e-10);

        // Ticks should be evenly spaced
        if ticks.len() >= 2 {
            let step = ticks[1] - ticks[0];
            for i in 1..ticks.len() {
                assert_relative_eq!(ticks[i] - ticks[i - 1], step, epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_generate_linear_ticks_symmetric() {
        // For -180 to 180, we want ticks like -150, -100, -50, 0, 50, 100, 150
        // or -180, -150, -120, ..., 180 depending on count
        let ticks = generate_linear_ticks(-180.0, 180.0, 12);

        // Verify ticks are within bounds
        assert!(ticks[0] >= -180.0 - 1e-10);
        assert!(ticks[ticks.len() - 1] <= 180.0 + 1e-10);

        // Verify we don't get -200 or 200
        for &tick in &ticks {
            assert!(tick >= -180.0 - 1e-10, "Tick {} is below -180", tick);
            assert!(tick <= 180.0 + 1e-10, "Tick {} is above 180", tick);
        }
    }

    #[test]
    fn test_generate_log_ticks() {
        let ticks = generate_log_ticks(1.0, 1000.0, 10.0, false);

        assert_eq!(ticks.len(), 4); // 1, 10, 100, 1000
        assert_relative_eq!(ticks[0], 1.0);
        assert_relative_eq!(ticks[1], 10.0);
        assert_relative_eq!(ticks[2], 100.0);
        assert_relative_eq!(ticks[3], 1000.0);
    }

    #[test]
    fn test_generate_log_ticks_with_subdivisions() {
        let ticks = generate_log_ticks(10.0, 100.0, 10.0, true);

        // Should include 10, 20, 30, ..., 90, 100
        assert!(ticks.len() > 4);
        assert_relative_eq!(ticks[0], 10.0);
        assert_relative_eq!(ticks[ticks.len() - 1], 100.0);

        // Check that subdivisions are included
        assert!(ticks.contains(&20.0));
        assert!(ticks.contains(&50.0));
    }

    #[test]
    fn test_generate_log_ticks_frequency_range() {
        let ticks = generate_log_ticks(20.0, 20000.0, 10.0, false);

        // Should include major frequency points
        assert!(ticks.iter().any(|&t| (t - 100.0).abs() < 1e-6));
        assert!(ticks.iter().any(|&t| (t - 1000.0).abs() < 1e-6));
        assert!(ticks.iter().any(|&t| (t - 10000.0).abs() < 1e-6));
    }
}
