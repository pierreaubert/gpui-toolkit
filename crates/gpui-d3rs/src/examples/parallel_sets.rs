//! Parallel Sets — <https://observablehq.com/@d3/parallel-sets>
//!
//! Demonstrates: `SankeyLayout` applied to categorical flow data (Titanic).
//! Groups passengers by Class → Sex → Survived and visualizes the flow.

use crate::sankey::{SankeyLayout, SankeyLinkInput, SankeyResult};
use std::collections::HashMap;

/// Build parallel sets from Titanic CSV data.
///
/// The CSV has columns: Survived, Sex, Age, Class, value
/// We aggregate into flows: Class → Sex → Survived
pub fn load_csv(csv_str: &str) -> (Vec<String>, Vec<SankeyLinkInput>) {
    // Parse CSV into (class, sex, survived, value) tuples
    let rows: Vec<(&str, &str, &str, f64)> = csv_str
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 5 {
                return None;
            }
            let survived = cols[0].trim();
            let sex = cols[1].trim();
            let class = cols[3].trim();
            let value: f64 = cols[4].trim().parse().ok()?;
            Some((class, sex, survived, value))
        })
        .collect();

    // Aggregate flows: Class→Sex and Sex→Survived
    let mut class_sex: HashMap<(String, String), f64> = HashMap::new();
    let mut sex_survived: HashMap<(String, String), f64> = HashMap::new();

    for (class, sex, survived, value) in &rows {
        *class_sex
            .entry((class.to_string(), sex.to_string()))
            .or_default() += value;
        *sex_survived
            .entry((sex.to_string(), survived.to_string()))
            .or_default() += value;
    }

    // Collect unique node names (order: classes, then sexes, then outcomes)
    let mut classes: Vec<String> = rows.iter().map(|(c, _, _, _)| c.to_string()).collect();
    classes.sort();
    classes.dedup();
    let mut sexes: Vec<String> = rows.iter().map(|(_, s, _, _)| s.to_string()).collect();
    sexes.sort();
    sexes.dedup();
    let mut outcomes: Vec<String> = rows.iter().map(|(_, _, o, _)| o.to_string()).collect();
    outcomes.sort();
    outcomes.dedup();

    let mut names = Vec::new();
    names.extend(classes);
    names.extend(sexes);
    names.extend(outcomes);

    // Build links
    let mut links = Vec::new();
    for ((class, sex), value) in &class_sex {
        links.push(SankeyLinkInput {
            source: class.clone(),
            target: sex.clone(),
            value: *value,
        });
    }
    for ((sex, survived), value) in &sex_survived {
        links.push(SankeyLinkInput {
            source: sex.clone(),
            target: survived.clone(),
            value: *value,
        });
    }

    (names, links)
}

/// Compute parallel sets layout from Titanic data.
pub fn compute(names: &[String], links: &[SankeyLinkInput]) -> SankeyResult {
    SankeyLayout::new()
        .width(928.0)
        .height(600.0)
        .margins(5.0, 1.0, 5.0, 1.0)
        .node_width(15.0)
        .node_padding(10.0)
        .compute(names, links)
}
