//! Tick generation utilities
//!
//! Provides functions for generating nice tick values for axes and scales.
//! Consolidates tick generation from scale/ticks.rs and adds additional utilities.

/// Generate nice linear ticks for a given domain.
///
/// Uses Wilkinson's algorithm to generate approximately `count` tick values
/// that are "nice" (round numbers).
///
/// # Example
///
/// ```
/// use d3rs::array::ticks;
///
/// let t = ticks(0.0, 100.0, 5);
/// assert!(t.len() >= 5);
/// assert!(t[0] <= 0.0);
/// assert!(*t.last().unwrap() >= 100.0);
/// ```
pub fn ticks(start: f64, stop: f64, count: usize) -> Vec<f64> {
    if count == 0 || start == stop {
        return vec![start];
    }

    let (start, stop, reverse) = if start > stop {
        (stop, start, true)
    } else {
        (start, stop, false)
    };

    let step = tick_step(start, stop, count);
    if step == 0.0 || !step.is_finite() {
        return vec![start];
    }

    let tick_start = (start / step).ceil() * step;
    let tick_stop = (stop / step).floor() * step;

    let n = ((tick_stop - tick_start) / step).round() as usize + 1;
    let mut result: Vec<f64> = (0..n).map(|i| tick_start + step * i as f64).collect();

    if reverse {
        result.reverse();
    }

    result
}

/// Compute the step size for generating approximately `count` ticks.
///
/// # Example
///
/// ```
/// use d3rs::array::tick_step;
///
/// let step = tick_step(0.0, 100.0, 10);
/// assert_eq!(step, 10.0);
/// ```
pub fn tick_step(start: f64, stop: f64, count: usize) -> f64 {
    if count == 0 {
        return 0.0;
    }

    let step0 = (stop - start).abs() / count as f64;
    let step1 = 10_f64.powf(step0.log10().floor());

    let error = step0 / step1;

    if error >= 10.0_f64.sqrt() * 5.0 {
        step1 * 10.0
    } else if error >= 10.0_f64.sqrt() * 2.0 {
        step1 * 5.0
    } else if error >= 10.0_f64.sqrt() {
        step1 * 2.0
    } else {
        step1
    }
}

/// Increment a nice step size to the next larger nice value.
///
/// Nice values are powers of 10, times 2 or 5.
pub fn tick_increment(start: f64, stop: f64, count: usize) -> f64 {
    let step = (stop - start) / count.max(1) as f64;
    let power = step.log10().floor();
    let error = step / 10_f64.powf(power);

    if error >= 10.0_f64.sqrt() * 5.0 {
        10_f64.powf(power + 1.0)
    } else if error >= 10.0_f64.sqrt() * 2.0 {
        5.0 * 10_f64.powf(power)
    } else if error >= 10.0_f64.sqrt() {
        2.0 * 10_f64.powf(power)
    } else {
        10_f64.powf(power)
    }
}

/// Extend the domain to nice round values.
///
/// Returns (nice_start, nice_stop) that encompass the original domain
/// and align with the tick step.
///
/// # Example
///
/// ```
/// use d3rs::array::nice;
///
/// let (start, stop) = nice(0.134, 0.867, 5);
/// assert!(start <= 0.134);
/// assert!(stop >= 0.867);
/// ```
pub fn nice(start: f64, stop: f64, count: usize) -> (f64, f64) {
    if start == stop {
        return (start, stop);
    }

    let (start, stop, reverse) = if start > stop {
        (stop, start, true)
    } else {
        (start, stop, false)
    };

    let step = tick_increment(start, stop, count);
    if step == 0.0 || !step.is_finite() {
        return if reverse {
            (stop, start)
        } else {
            (start, stop)
        };
    }

    let nice_start = (start / step).floor() * step;
    let nice_stop = (stop / step).ceil() * step;

    if reverse {
        (nice_stop, nice_start)
    } else {
        (nice_start, nice_stop)
    }
}

/// Find a "nice" number approximately equal to the range.
///
/// Nice numbers are 1, 2, or 5 times a power of 10.
/// If `round` is true, rounds the number, otherwise takes the ceiling.
///
/// # Example
///
/// ```
/// use d3rs::array::nice_number;
///
/// assert_eq!(nice_number(10.0, false), 10.0);
/// assert_eq!(nice_number(15.0, false), 20.0);
/// assert_eq!(nice_number(25.0, false), 50.0);
/// ```
pub fn nice_number(range: f64, round: bool) -> f64 {
    if range == 0.0 {
        return 0.0;
    }

    let exponent = range.abs().log10().floor();
    let fraction = range.abs() / 10_f64.powf(exponent);

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

    nice_fraction * 10_f64.powf(exponent) * range.signum()
}

/// Generate nice logarithmic ticks for a given domain.
///
/// Generates ticks at powers of the base, with optional subdivisions.
///
/// # Example
///
/// ```
/// use d3rs::array::log_ticks;
///
/// let t = log_ticks(1.0, 1000.0, 10.0, false);
/// assert_eq!(t, vec![1.0, 10.0, 100.0, 1000.0]);
/// ```
pub fn log_ticks(min: f64, max: f64, base: f64, subdivisions: bool) -> Vec<f64> {
    if min <= 0.0 || max <= 0.0 || base <= 1.0 {
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

/// Generate ticks at specific intervals (e.g., every 5, 10, etc.).
///
/// # Example
///
/// ```
/// use d3rs::array::ticks_interval;
///
/// let t = ticks_interval(0.0, 100.0, 20.0);
/// assert_eq!(t, vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
/// ```
pub fn ticks_interval(start: f64, stop: f64, interval: f64) -> Vec<f64> {
    if interval <= 0.0 || start == stop {
        return vec![start];
    }

    let tick_start = (start / interval).ceil() * interval;
    let tick_stop = (stop / interval).floor() * interval;

    let n = ((tick_stop - tick_start) / interval).round() as usize + 1;
    (0..n).map(|i| tick_start + interval * i as f64).collect()
}

/// Generate date/time ticks (placeholder for future implementation).
///
/// For now, this just returns ticks for numeric timestamps.
pub fn time_ticks(start: f64, stop: f64, count: usize) -> Vec<f64> {
    ticks(start, stop, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticks() {
        let t = ticks(0.0, 100.0, 5);
        assert!(t.len() >= 5);
        assert!(t[0] <= 0.0);
        assert!(*t.last().unwrap() >= 100.0);
    }

    #[test]
    fn test_tick_step() {
        assert_eq!(tick_step(0.0, 100.0, 10), 10.0);
        assert_eq!(tick_step(0.0, 1.0, 10), 0.1);
    }

    #[test]
    fn test_nice() {
        let (start, stop) = nice(0.134, 0.867, 5);
        assert!(start <= 0.134);
        assert!(stop >= 0.867);
    }

    #[test]
    fn test_nice_number() {
        assert_eq!(nice_number(10.0, false), 10.0);
        assert_eq!(nice_number(15.0, false), 20.0);
        assert_eq!(nice_number(25.0, false), 50.0);
        assert_eq!(nice_number(75.0, false), 100.0);
    }

    #[test]
    fn test_log_ticks() {
        let t = log_ticks(1.0, 1000.0, 10.0, false);
        assert_eq!(t, vec![1.0, 10.0, 100.0, 1000.0]);
    }

    #[test]
    fn test_log_ticks_subdivisions() {
        let t = log_ticks(10.0, 100.0, 10.0, true);
        assert!(t.contains(&10.0));
        assert!(t.contains(&20.0));
        assert!(t.contains(&50.0));
        assert!(t.contains(&100.0));
    }

    #[test]
    fn test_ticks_interval() {
        let t = ticks_interval(0.0, 100.0, 20.0);
        assert_eq!(t, vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
    }

    #[test]
    fn test_ticks_reverse() {
        let t = ticks(100.0, 0.0, 5);
        assert!(t[0] >= t[1]); // Descending order
    }
}
