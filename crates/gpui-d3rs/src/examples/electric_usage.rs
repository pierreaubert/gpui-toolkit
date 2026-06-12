//! Electric Usage 2019 — <https://observablehq.com/@mbostock/electric-usage-2019>
//!
//! Heatmap of hourly electricity usage: 24 hours × 365 days.
//! Color encodes usage intensity (sequential warm scale).

#[derive(Debug, Clone)]
pub struct UsageCell {
    pub date: String, // YYYY-MM-DD
    pub hour: usize,
    pub usage: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub struct ElectricUsageResult {
    pub width: f64,
    pub height: f64,
    pub cells: Vec<UsageCell>,
    pub cell_width: f64,
    pub cell_height: f64,
    pub usage_max: f64,
    pub unique_dates: usize,
}

/// Parse pge-electric-data.csv: date,usage
pub fn load_csv(csv_str: &str) -> Vec<(String, usize, f64)> {
    csv_str
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 2 {
                return None;
            }
            let date_str = cols[0].trim();
            let usage: f64 = cols[1].trim().parse().ok()?;
            // Extract date (YYYY-MM-DD) and hour from ISO datetime
            let date = &date_str[..10];
            // Parse hour from the datetime
            let hour: usize = if date_str.len() > 11 {
                date_str[11..13].parse().unwrap_or(0)
            } else {
                0
            };
            Some((date.to_string(), hour, usage))
        })
        .collect()
}

/// Compute the heatmap layout.
pub fn compute(data: &[(String, usize, f64)]) -> ElectricUsageResult {
    let width = 928.0;
    let margin_left = 80.0;
    let margin_top = 30.0;
    let margin_right = 10.0;

    // Collect unique dates
    let mut dates: Vec<String> = data.iter().map(|(d, _, _)| d.clone()).collect();
    dates.sort();
    dates.dedup();
    let n_dates = dates.len();

    let cell_width = (width - margin_left - margin_right) / 24.0;
    let cell_height = 2.5; // compact rows
    let height = margin_top + n_dates as f64 * cell_height + 10.0;

    let usage_max = data.iter().map(|(_, _, u)| *u).fold(0.0f64, f64::max);

    // Build date→row index map
    let date_index: std::collections::HashMap<&str, usize> = dates
        .iter()
        .enumerate()
        .map(|(i, d)| (d.as_str(), i))
        .collect();

    let cells: Vec<UsageCell> = data
        .iter()
        .filter_map(|(date, hour, usage)| {
            let row = *date_index.get(date.as_str())?;
            let x = margin_left + *hour as f64 * cell_width;
            let y = margin_top + row as f64 * cell_height;
            Some(UsageCell {
                date: date.clone(),
                hour: *hour,
                usage: *usage,
                x,
                y,
            })
        })
        .collect();

    ElectricUsageResult {
        width,
        height,
        cells,
        cell_width,
        cell_height,
        usage_max,
        unique_dates: n_dates,
    }
}
