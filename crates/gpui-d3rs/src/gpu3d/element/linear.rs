pub(super) fn linear_step_ticks(min: f64, max: f64, step: f64) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() || !step.is_finite() || step <= 0.0 {
        return Vec::new();
    }
    let start = (min / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut value = start;
    while value <= max + step * 1e-3 {
        ticks.push(value);
        value += step;
    }
    ticks
}

/// d3-style "nice" step for `target` ticks across `[min, max]`: a
/// 1/2/5/10 power-of-ten multiple just under `range / target`.
pub(super) fn linear_nice_step(min: f64, max: f64, target: usize) -> f64 {
    let range = max - min;
    if !range.is_finite() || range <= 0.0 {
        return 1.0;
    }
    let raw = range / target.max(1) as f64;
    let magnitude = 10f64.powf(raw.log10().floor());
    let error = raw / magnitude;
    let factor = if error >= 50f64.sqrt() {
        10.0
    } else if error >= 10f64.sqrt() {
        5.0
    } else if error >= 2f64.sqrt() {
        2.0
    } else {
        1.0
    };
    magnitude * factor
}

/// d3-style tick values across `[min, max]` aiming at `target` ticks.
/// Degenerate ranges yield the single endpoint; non-finite input yields
/// nothing.
pub(super) fn linear_nice_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() {
        return Vec::new();
    }
    if min >= max {
        return vec![min];
    }
    linear_step_ticks(min, max, linear_nice_step(min, max, target))
}

/// Plain tick label: integers without decimals, fractions trimmed to four
/// decimals, `-0` normalized to `0`. Never scientific: values too small for
/// four decimals fall back to Rust's shortest representation.
pub(super) fn format_plain_tick(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let trimmed = format!("{:.4}", value);
    let trimmed = trimmed.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "0" || trimmed == "-0" {
        format!("{}", value)
    } else {
        trimmed.to_string()
    }
}

pub(super) fn linear_subdivision_ticks(min: f64, max: f64, divisions: usize) -> Vec<f64> {
    if divisions == 0 || !min.is_finite() || !max.is_finite() || min >= max {
        return Vec::new();
    }
    (0..=divisions)
        .map(|i| min + (max - min) * i as f64 / divisions as f64)
        .collect()
}
