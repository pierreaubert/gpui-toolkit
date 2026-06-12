//! Stacked Area Chart — <https://observablehq.com/@d3/stacked-area-chart>
//!
//! Demonstrates: `Stack` with none offset, `TimeScale` (scaleUtc) for x-axis,
//! `LinearScale` for y-axis, `Curve::monotone_x` for area interpolation.

use crate::scale::{LinearScale, Scale};
use crate::shape::area::Area;
use crate::shape::curve::Curve;
use crate::shape::stack::{Stack, StackOffset, StackOrder, StackSeries};
use crate::time::TimeScale;

/// A single dated data row with values per category.
#[derive(Debug, Clone)]
pub struct DateRow {
    /// Seconds since epoch (UTC)
    pub date: i64,
    pub values: Vec<f64>,
}

#[derive(Debug)]
pub struct StackedAreaResult {
    pub width: f64,
    pub height: f64,
    pub categories: Vec<String>,
    pub series: Vec<StackSeries>,
    /// Pre-computed d3rs area paths (key, Path)
    pub area_paths: Vec<(String, crate::shape::path::Path)>,
    pub x_domain: [i64; 2],
    pub y_domain: [f64; 2],
    pub x_ticks: Vec<i64>,
}

/// Load stacked area data from a long-format CSV (date,category,value).
///
/// Uses `d3rs::fetch::csv::parse_csv` for parsing, then pivots the long format
/// into wide format: one `DateRow` per unique date, with a value column per category.
///
/// Expected columns: "date" (YYYY-MM-DD), a category column, a value column.
/// Pass the column names for category and value.
pub fn load_csv(
    csv_str: &str,
    date_col: &str,
    category_col: &str,
    value_col: &str,
) -> (Vec<String>, Vec<DateRow>) {
    use crate::fetch::parse_csv;
    use std::collections::{BTreeMap, BTreeSet};

    let rows = parse_csv(csv_str).expect("valid stacked area CSV");

    let mut categories_set = BTreeSet::new();
    let mut date_map: BTreeMap<i64, BTreeMap<String, f64>> = BTreeMap::new();

    let date_key = date_col.to_string();
    let cat_key = category_col.to_string();
    let val_key = value_col.to_string();

    for row in &rows {
        let date_str = match row.get(&date_key) {
            Some(s) => s.as_str(),
            None => continue,
        };
        let category = match row.get(&cat_key) {
            Some(s) => s.clone(),
            None => continue,
        };
        let value: f64 = row
            .get(&val_key)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let epoch = parse_date_to_epoch(date_str);
        categories_set.insert(category.clone());
        date_map.entry(epoch).or_default().insert(category, value);
    }

    let categories: Vec<String> = categories_set.into_iter().collect();
    let date_rows: Vec<DateRow> = date_map
        .into_iter()
        .map(|(date, vals)| {
            let values: Vec<f64> = categories
                .iter()
                .map(|c| vals.get(c).copied().unwrap_or(0.0))
                .collect();
            DateRow { date, values }
        })
        .collect();

    (categories, date_rows)
}

/// Parse "YYYY-MM-DD" or bare "YYYY" to approximate epoch seconds (UTC).
fn parse_date_to_epoch(s: &str) -> i64 {
    let parts: Vec<&str> = s.split('-').collect();
    let year: i64 = parts[0].parse().unwrap_or(2000);
    let month: i64 = if parts.len() >= 2 {
        parts[1].parse().unwrap_or(1)
    } else {
        1
    };
    let day: i64 = if parts.len() >= 3 {
        parts[2].parse().unwrap_or(1)
    } else {
        1
    };
    let years = year - 1970;
    let leap_years = (year - 1969) / 4 - (year - 1901) / 100 + (year - 1601) / 400;
    let days_of_months: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mut days = years * 365 + leap_years + days_of_months[(month - 1) as usize] + (day - 1);
    if month > 2 && (year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)) {
        days += 1;
    }
    days * 86400
}

/// Default monthly spending data (4 categories, 12 months of 2024).
pub fn default_data() -> (Vec<String>, Vec<DateRow>) {
    let categories: Vec<String> = ["Electronics", "Clothing", "Food", "Transport"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Monthly dates: 2024-01-01 through 2024-12-01 as epoch seconds (UTC)
    let base_epoch = 1704067200_i64; // 2024-01-01T00:00:00Z
    let month_seconds = 30 * 24 * 3600_i64; // approximate

    let rows: Vec<DateRow> = (0..12)
        .map(|m| {
            let date = base_epoch + m as i64 * month_seconds;
            let values: Vec<f64> = (0..categories.len())
                .map(|ci| {
                    let base = 50.0 + ci as f64 * 20.0;
                    let val = base
                        + 15.0 * (m as f64 * 0.5 + ci as f64 * 1.2).sin()
                        + 5.0 * (m as f64 * 0.8 + ci as f64 * 0.5).cos();
                    (val * 100.0).round() / 100.0
                })
                .collect();
            DateRow { date, values }
        })
        .collect();

    (categories, rows)
}

/// Compute stacked area chart with TimeScale x-axis.
pub fn compute(categories: &[String], rows: &[DateRow]) -> StackedAreaResult {
    let width = 928.0;
    let height = 500.0;
    let margin_top = 20.0;
    let margin_right = 20.0;
    let margin_bottom = 30.0;
    let margin_left = 40.0;
    let n = rows.len();

    // Extract matrix for Stack
    let matrix: Vec<Vec<f64>> = rows.iter().map(|r| r.values.clone()).collect();

    let stack = Stack::new()
        .keys(categories.to_vec())
        .order(StackOrder::None)
        .offset(StackOffset::None);
    let series = stack.generate(&matrix);

    // X: TimeScale (scaleUtc equivalent)
    let x_domain = [rows[0].date, rows[n - 1].date];
    let x_scale = TimeScale::new()
        .domain(x_domain[0], x_domain[1])
        .range(margin_left, width - margin_right);

    let x_ticks = x_scale.time_ticks(6);

    // Y: LinearScale
    let y_max = series
        .iter()
        .flat_map(|s| (0..n).filter_map(|i| s.get(i).map(|v| v[1])))
        .fold(0.0f64, f64::max);
    let y_scale = LinearScale::new()
        .domain(0.0, y_max)
        .range(height - margin_bottom, margin_top);

    // Generate area paths using d3rs Area generator
    // Each data item is (row_index, [y0, y1]) for a given series
    let area_paths: Vec<(String, crate::shape::path::Path)> = series
        .iter()
        .map(|s| {
            let data: Vec<(usize, [f64; 2])> = (0..n)
                .map(|i| (i, s.get(i).unwrap_or([0.0, 0.0])))
                .collect();

            let dates: Vec<i64> = rows.iter().map(|r| r.date).collect();
            let area = Area::new()
                .x(move |d: &(usize, [f64; 2])| x_scale.scale(dates[d.0]))
                .y0(move |d: &(usize, [f64; 2])| y_scale.scale(d.1[0]))
                .y1(move |d: &(usize, [f64; 2])| y_scale.scale(d.1[1]))
                .curve(Curve::monotone_x());

            let path = area.generate(&data);
            (s.key.clone(), path)
        })
        .collect();

    StackedAreaResult {
        width,
        height,
        categories: categories.to_vec(),
        series,
        area_paths,
        x_domain,
        y_domain: [0.0, y_max],
        x_ticks,
    }
}
