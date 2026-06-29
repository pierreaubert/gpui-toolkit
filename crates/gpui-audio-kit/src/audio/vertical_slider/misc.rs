use gpui::SharedString;

/// Format a value with abbreviated suffix (1k, 10k, etc.)
pub(super) fn format_value_abbrev(value: f64) -> SharedString {
    let abs_value = value.abs();
    let sign = if value < 0.0 { "-" } else { "" };

    if abs_value >= 10000.0 {
        // 10000 -> 10k, 20000 -> 20k
        format!("{}{}k", sign, (abs_value / 1000.0).round() as i32).into()
    } else if abs_value >= 1000.0 {
        // 1000 -> 1k, 2500 -> 2.5k
        let k_value = abs_value / 1000.0;
        if (k_value.round() - k_value).abs() < 0.01 {
            format!("{}{}k", sign, k_value.round() as i32).into()
        } else {
            format!("{}{:.1}k", sign, k_value).into()
        }
    } else if abs_value >= 10.0 {
        // For values >= 10, show as integer
        format!("{}{}", sign, abs_value.round() as i32).into()
    } else if abs_value >= 1.0 {
        // Show one decimal if needed
        if (abs_value.round() - abs_value).abs() < 0.01 {
            format!("{}{}", sign, abs_value.round() as i32).into()
        } else {
            format!("{}{:.1}", sign, abs_value).into()
        }
    } else if abs_value >= 0.1 {
        format!("{}{:.1}", sign, abs_value).into()
    } else if abs_value > 0.0 {
        format!("{}{:.2}", sign, abs_value).into()
    } else {
        "0".into()
    }
}

/// Find a nice step size for linear scale
pub(super) fn find_nice_step(range: f64, target_ticks: usize) -> f64 {
    if range <= 0.0 || target_ticks < 2 {
        return range;
    }

    let rough_step = range / (target_ticks - 1) as f64;
    let magnitude = 10_f64.powf(rough_step.log10().floor());

    // Try nice multiples: 1, 2, 2.5, 5, 10
    let normalized = rough_step / magnitude;
    let nice_normalized = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 2.5 {
        2.5
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice_normalized * magnitude
}

#[cfg(test)]
mod tests {
    use super::{find_nice_step, format_value_abbrev};

    #[test]
    fn format_value_abbrev_covers_ranges() {
        assert_eq!(format_value_abbrev(0.0), "0");
        assert_eq!(format_value_abbrev(0.05), "0.05");
        assert_eq!(format_value_abbrev(0.5), "0.5");
        assert_eq!(format_value_abbrev(5.0), "5");
        assert_eq!(format_value_abbrev(10.0), "10");
        assert_eq!(format_value_abbrev(999.0), "999");
        assert_eq!(format_value_abbrev(1000.0), "1k");
        assert_eq!(format_value_abbrev(2500.0), "2.5k");
        assert_eq!(format_value_abbrev(20000.0), "20k");
        assert_eq!(format_value_abbrev(-2500.0), "-2.5k");
    }

    #[test]
    fn find_nice_step_falls_back_for_bad_inputs() {
        assert_eq!(find_nice_step(0.0, 5), 0.0);
        assert_eq!(find_nice_step(100.0, 1), 100.0);
    }

    #[test]
    fn find_nice_step_picks_expected_multiples() {
        assert_eq!(find_nice_step(100.0, 5), 25.0);
        assert_eq!(find_nice_step(100.0, 6), 20.0);
        assert_eq!(find_nice_step(100.0, 11), 10.0);
    }
}
