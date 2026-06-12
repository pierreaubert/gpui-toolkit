//! Ridgeline Plot — <https://observablehq.com/@d3/ridgeline-plot>
//!
//! Demonstrates: Multiple overlapping `Area` generators to show distribution
//! by category. Uses weather data grouped by month.

use crate::scale::{LinearScale, Scale};
use crate::shape::path::{Path, PathBuilder};

#[derive(Debug)]
pub struct RidgelineResult {
    pub width: f64,
    pub height: f64,
    /// One area path per category (month), rendered with overlap.
    pub area_paths: Vec<(String, Path)>,
    pub y_offsets: Vec<f64>,
    pub x_domain: [f64; 2],
}

/// Parse weather CSV into monthly temperature distributions.
/// Returns (month_name, sorted_values) pairs.
pub fn load_csv(csv_str: &str) -> Vec<(String, Vec<f64>)> {
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut monthly: Vec<Vec<f64>> = vec![Vec::new(); 12];

    for line in csv_str.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 2 {
            continue;
        }
        // Parse month from date "2011-09-30T22:00Z"
        let date_str = cols[0];
        if let Ok(month) = date_str[5..7].parse::<usize>()
            && (1..=12).contains(&month)
            && let Ok(val) = cols[1].parse::<f64>()
        {
            monthly[month - 1].push(val);
        }
    }

    monthly
        .into_iter()
        .enumerate()
        .filter(|(_, v)| !v.is_empty())
        .map(|(i, mut vals)| {
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            (month_names[i].to_string(), vals)
        })
        .collect()
}

/// Compute ridgeline plot: KDE-like area for each month.
pub fn compute(monthly_data: &[(String, Vec<f64>)]) -> RidgelineResult {
    let width = 928.0;
    let margin_left = 60.0;
    let margin_right = 20.0;
    let margin_top = 20.0;
    let row_height = 40.0;
    let overlap = 8.0; // overlap between adjacent rows

    let n_months = monthly_data.len();
    let height = margin_top + n_months as f64 * (row_height - overlap) + overlap + 10.0;

    // Global temperature range for x axis
    let all_min = monthly_data
        .iter()
        .flat_map(|(_, v)| v.iter())
        .fold(f64::INFINITY, |a, &b| a.min(b));
    let all_max = monthly_data
        .iter()
        .flat_map(|(_, v)| v.iter())
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    // Guard against degenerate domain (all values identical)
    let x_max = if all_max > all_min {
        all_max
    } else {
        all_min + 1.0
    };
    let x_scale = LinearScale::new()
        .domain(all_min, x_max)
        .range(margin_left, width - margin_right);

    // Build a simple histogram-based density for each month
    let n_bins = 50;
    // Guard against uniform data (all_max == all_min)
    let bin_width = if all_max > all_min {
        (all_max - all_min) / n_bins as f64
    } else {
        1.0
    };

    let mut area_paths = Vec::new();
    let mut y_offsets = Vec::new();

    for (mi, (name, values)) in monthly_data.iter().enumerate() {
        let y_base = margin_top + mi as f64 * (row_height - overlap) + row_height;
        y_offsets.push(y_base);

        // Bin the values
        let mut bins = vec![0usize; n_bins];
        for &v in values {
            let idx = ((v - all_min) / bin_width).floor() as usize;
            let idx = idx.min(n_bins - 1);
            bins[idx] += 1;
        }
        let max_count = *bins.iter().max().unwrap_or(&1) as f64;

        // Build area path: baseline at y_base, peaks upward
        let density_scale = LinearScale::new()
            .domain(0.0, max_count)
            .range(0.0, row_height * 0.9);

        let mut builder = PathBuilder::new();
        // Start at baseline left
        let x_start = x_scale.scale(all_min);
        builder = builder.move_to(x_start, y_base);

        // Top line (density curve)
        for (bi, &count) in bins.iter().enumerate() {
            let x_mid = all_min + (bi as f64 + 0.5) * bin_width;
            let x = x_scale.scale(x_mid);
            let h = density_scale.scale(count as f64);
            builder = builder.line_to(x, y_base - h);
        }

        // Close back to baseline
        let x_end = x_scale.scale(all_max);
        builder = builder.line_to(x_end, y_base);
        builder = builder.close_path();

        area_paths.push((name.clone(), builder.build()));
    }

    RidgelineResult {
        width,
        height,
        area_paths,
        y_offsets,
        x_domain: [all_min, x_max],
    }
}
