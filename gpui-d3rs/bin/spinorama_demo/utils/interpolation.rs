use autoeq::DirectivityData;

/// Extract SPL values at a specific frequency from directivity data.
/// Uses linear interpolation in log-frequency space if exact frequency not found.
///
/// Returns a Vec of (angle, spl) pairs for the specified plane.
pub fn interpolate_spl_at_frequency(
    directivity_data: &DirectivityData,
    target_freq: f64,
    use_horizontal: bool,
) -> Vec<(f64, f64)> {
    let curves = if use_horizontal {
        &directivity_data.horizontal
    } else {
        &directivity_data.vertical
    };

    curves
        .iter()
        .map(|curve| {
            let spl = interpolate_frequency_value(
                curve.freq.as_slice().unwrap(),
                curve.spl.as_slice().unwrap(),
                target_freq,
            );
            (curve.angle, spl)
        })
        .collect()
}

/// Interpolate SPL value at a target frequency from frequency and SPL arrays.
/// Uses linear interpolation in log-frequency space.
fn interpolate_frequency_value(freq: &[f64], spl: &[f64], target_freq: f64) -> f64 {
    if freq.is_empty() {
        return 0.0;
    }

    // Handle edge cases
    if target_freq <= freq[0] {
        return spl[0];
    }
    if target_freq >= freq[freq.len() - 1] {
        return spl[spl.len() - 1];
    }

    // Find bracketing frequencies using log-space comparison
    let target_log = target_freq.ln();

    // Binary search for the insertion point
    let mut low = 0;
    let mut high = freq.len() - 1;

    while high - low > 1 {
        let mid = (low + high) / 2;
        if freq[mid] <= target_freq {
            low = mid;
        } else {
            high = mid;
        }
    }

    // Linear interpolation in log-frequency space
    let freq_low_log = freq[low].ln();
    let freq_high_log = freq[high].ln();

    let t = (target_log - freq_low_log) / (freq_high_log - freq_low_log);

    // Linear interpolation of SPL values (dB space is already linear-like)
    spl[low] + t * (spl[high] - spl[low])
}

/// Get the available angle range from directivity data
pub fn get_angle_range(directivity_data: &DirectivityData, use_horizontal: bool) -> (f64, f64) {
    let curves = if use_horizontal {
        &directivity_data.horizontal
    } else {
        &directivity_data.vertical
    };

    if curves.is_empty() {
        return (-90.0, 90.0);
    }

    let min_angle = curves.iter().map(|c| c.angle).fold(f64::INFINITY, f64::min);
    let max_angle = curves
        .iter()
        .map(|c| c.angle)
        .fold(f64::NEG_INFINITY, f64::max);

    (min_angle, max_angle)
}

/// Format frequency for display (e.g., 1000 -> "1k", 100 -> "100")
pub fn format_frequency(freq: f64) -> String {
    if freq >= 1000.0 {
        let khz = freq / 1000.0;
        if khz == khz.floor() {
            format!("{}k", khz as i32)
        } else {
            format!("{:.1}k", khz)
        }
    } else {
        format!("{}", freq as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_frequency_value_exact() {
        let freq = [100.0, 1000.0, 10000.0];
        let spl = [80.0, 85.0, 75.0];

        assert!((interpolate_frequency_value(&freq, &spl, 1000.0) - 85.0).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate_frequency_value_between() {
        let freq = [100.0, 1000.0, 10000.0];
        let spl = [80.0, 85.0, 75.0];

        // Midpoint in log space between 100 and 1000 is ~316 Hz
        let mid_log = (100.0_f64.ln() + 1000.0_f64.ln()) / 2.0;
        let mid_freq = mid_log.exp();

        let result = interpolate_frequency_value(&freq, &spl, mid_freq);
        // Should be midpoint of SPL values
        assert!((result - 82.5).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate_frequency_value_edge_low() {
        let freq = [100.0, 1000.0, 10000.0];
        let spl = [80.0, 85.0, 75.0];

        assert!((interpolate_frequency_value(&freq, &spl, 50.0) - 80.0).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate_frequency_value_edge_high() {
        let freq = [100.0, 1000.0, 10000.0];
        let spl = [80.0, 85.0, 75.0];

        assert!((interpolate_frequency_value(&freq, &spl, 20000.0) - 75.0).abs() < 1e-6);
    }

    #[test]
    fn test_format_frequency() {
        assert_eq!(format_frequency(100.0), "100");
        assert_eq!(format_frequency(1000.0), "1k");
        assert_eq!(format_frequency(2500.0), "2.5k");
        assert_eq!(format_frequency(10000.0), "10k");
    }
}
