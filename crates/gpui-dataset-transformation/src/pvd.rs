//! PVD collection files: time series of VTU parts.
//!
//! A `.pvd` document lists `DataSet` entries with `file`, `timestep`,
//! and `part` attributes. Entries sharing a timestep are parallel pieces
//! of one step and are appended into a single dataset; distinct
//! timesteps form the series steps. File contents resolve through a
//! caller-supplied function, so no filesystem is assumed.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::append_datasets;
use crate::vtu::{read_vtu_bytes, write_vtu};
use gpui_mesh::{DataSet, TimeSeries};
use std::fmt::Write as _;

/// One PVD entry: a file, its physical time, and its part index.
#[derive(Debug, Clone, PartialEq)]
pub struct PvdEntry {
    /// VTU file name as written in the document.
    pub file: String,
    /// Physical time of the step this entry belongs to.
    pub timestep: f64,
    /// Part index within the step (parallel pieces).
    pub part: usize,
}

/// Serialises PVD entries as a collection document.
#[must_use]
pub fn write_pvd(entries: &[PvdEntry]) -> String {
    let mut out = String::from(
        "<VTKFile type=\"Collection\" version=\"0.1\" byte_order=\"LittleEndian\">\n<Collection>\n",
    );
    for entry in entries {
        let _ = writeln!(
            out,
            "<DataSet timestep=\"{}\" group=\"\" part=\"{}\" file=\"{}\"/>",
            entry.timestep,
            entry.part,
            entry.file.replace('"', "&quot;")
        );
    }
    out.push_str("</Collection>\n</VTKFile>\n");
    out
}

/// Parses the entries of a PVD document.
///
/// # Errors
///
/// Returns an error when an entry lacks a file or has a bad timestep.
pub fn parse_pvd(xml: &str) -> Result<Vec<PvdEntry>, String> {
    let mut entries = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<DataSet") {
        let after = &rest[start..];
        let Some(tag_end) = after.find('>') else {
            break;
        };
        let tag = &after[..tag_end];
        let attr = |key: &str| -> Option<String> {
            let pattern = format!("{key}=\"");
            let idx = tag.find(&pattern)? + pattern.len();
            let end = tag[idx..].find('"')? + idx;
            Some(tag[idx..end].to_string())
        };
        let file = attr("file").ok_or("PVD DataSet entry without file attribute")?;
        let timestep: f64 = attr("timestep")
            .unwrap_or_else(|| "0".into())
            .parse()
            .map_err(|_| "PVD entry with bad timestep")?;
        let part: usize = attr("part")
            .unwrap_or_else(|| "0".into())
            .parse()
            .map_err(|_| "PVD entry with bad part")?;
        entries.push(PvdEntry {
            file,
            timestep,
            part,
        });
        rest = &after[tag_end..];
    }
    if entries.is_empty() {
        return Err("PVD document has no DataSet entries".into());
    }
    Ok(entries)
}

/// Loads a PVD collection as a time series, resolving files via `resolve`.
///
/// Parts of one timestep merge with [`append_datasets`] in part order;
/// steps sort by timestep.
///
/// # Errors
///
/// Returns an error when a file cannot be resolved or parsed, parts of
/// one step disagree, or timesteps repeat inconsistently.
pub fn read_pvd(
    xml: &str,
    resolve: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<TimeSeries, String> {
    let mut entries = parse_pvd(xml)?;
    entries.sort_by(|a, b| {
        a.timestep
            .partial_cmp(&b.timestep)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.part.cmp(&b.part))
    });
    let mut steps: Vec<DataSet> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        let timestep = entries[index].timestep;
        let mut merged: Option<DataSet> = None;
        while index < entries.len() && entries[index].timestep == timestep {
            let entry = &entries[index];
            let bytes = resolve(&entry.file)
                .ok_or_else(|| format!("cannot resolve PVD file '{}'", entry.file))?;
            let dataset = read_vtu_bytes(&bytes)?;
            merged = Some(match merged {
                None => dataset,
                Some(acc) => {
                    append_datasets(&acc, &dataset).map_err(|e: TransformError| e.to_string())?
                }
            });
            index += 1;
        }
        steps.push(merged.ok_or("empty PVD step")?);
        times.push(timestep);
    }
    TimeSeries::new(steps, times).map_err(|e| e.to_string())
}

/// Convenience: the VTU file bytes for one series step, named for PVD.
#[must_use]
pub fn pvd_part_bytes(dataset: &DataSet) -> Vec<u8> {
    write_vtu(dataset).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pvd_round_trip_with_parallel_parts() {
        let a = DataSet::structured_grid("a", [2, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        let b = DataSet::structured_grid("b", [2, 2, 2], [5.0, 0.0, 0.0], [1.0; 3]).unwrap();
        let entries = vec![
            PvdEntry {
                file: "s0_p0.vtu".into(),
                timestep: 0.0,
                part: 0,
            },
            PvdEntry {
                file: "s0_p1.vtu".into(),
                timestep: 0.0,
                part: 1,
            },
            PvdEntry {
                file: "s1_p0.vtu".into(),
                timestep: 1.5,
                part: 0,
            },
            PvdEntry {
                file: "s1_p1.vtu".into(),
                timestep: 1.5,
                part: 1,
            },
        ];
        let xml = write_pvd(&entries);
        let files: std::collections::HashMap<&str, Vec<u8>> = [
            ("s0_p0.vtu", pvd_part_bytes(&a)),
            ("s0_p1.vtu", pvd_part_bytes(&b)),
            ("s1_p0.vtu", pvd_part_bytes(&a)),
            ("s1_p1.vtu", pvd_part_bytes(&b)),
        ]
        .into_iter()
        .collect();
        let series = read_pvd(&xml, |name| files.get(name).cloned()).unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series.times, vec![0.0, 1.5]);
        // Step 0 merged two 1-cell parts into 2 cells.
        assert_eq!(series.steps[0].cell_count(), 2);
        assert_eq!(series.steps[1].cell_count(), 2);
    }
}
