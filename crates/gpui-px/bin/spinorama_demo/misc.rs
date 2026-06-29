use gpui::*;
use gpui_px::{ColorScale, Colormap};
use std::collections::HashMap;

pub(super) fn cea2034_colors() -> HashMap<&'static str, u32> {
    let mut colors = HashMap::new();
    colors.insert("On Axis", 0x1f77b4); // Blue
    colors.insert("Listening Window", 0xff7f0e); // Orange
    colors.insert("Early Reflections", 0x2ca02c); // Green
    colors.insert("Sound Power", 0xd62728); // Red
    colors.insert("Early Reflections DI", 0x9467bd); // Purple
    colors.insert("Sound Power DI", 0x8c564b); // Brown
    colors
}

/// Convert Colormap (3D) to ColorScale (2D heatmap)
pub(super) fn colormap_to_color_scale(colormap: Colormap) -> ColorScale {
    match colormap {
        Colormap::Viridis => ColorScale::Viridis,
        Colormap::Plasma => ColorScale::Plasma,
        Colormap::Inferno => ColorScale::Inferno,
        Colormap::Turbo => ColorScale::Inferno, // Turbo not available in ColorScale, use Inferno
        Colormap::CoolWarm => ColorScale::Coolwarm,
    }
}

/// Interpolate SPL value at a specific frequency from curve data
pub(super) fn interpolate_spl_at_freq(freqs: &[f64], spls: &[f64], target_freq: f64) -> f64 {
    if freqs.is_empty() || spls.is_empty() {
        return 0.0;
    }

    // Find the two frequencies that bracket the target
    let mut lower_idx = 0;
    for (i, &f) in freqs.iter().enumerate() {
        if f <= target_freq {
            lower_idx = i;
        } else {
            break;
        }
    }

    if lower_idx >= freqs.len() - 1 {
        return spls.last().copied().unwrap_or(0.0);
    }

    let f1 = freqs[lower_idx];
    let f2 = freqs[lower_idx + 1];
    let s1 = spls[lower_idx];
    let s2 = spls[lower_idx + 1];

    // Linear interpolation in log-frequency space
    if f2 <= f1 {
        return s1;
    }

    let log_f1 = f1.log10();
    let log_f2 = f2.log10();
    let log_target = target_freq.log10();

    let t = (log_target - log_f1) / (log_f2 - log_f1);
    s1 + t * (s2 - s1)
}
