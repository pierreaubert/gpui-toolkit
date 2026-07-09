//! Chord Diagram Layout (d3-chord)
//!
//! Visualizes relationships or flows between a set of nodes using a circular layout.
//! `ChordLayout::try_compute` validates user-provided matrices before layout,
//! while `ChordLayout::compute` keeps the older panic-on-invalid-input behavior.

use std::f64::consts::PI;
use std::fmt;

use crate::util::scratch::path_to_string;

/// A chord representing a flow between source and target
#[derive(Debug, Clone)]
pub struct Chord {
    pub source: ChordSubgroup,
    pub target: ChordSubgroup,
}

/// A subgroup within a chord (one end of the flow)
#[derive(Debug, Clone)]
pub struct ChordSubgroup {
    pub index: usize,
    pub start_angle: f64,
    pub end_angle: f64,
    pub value: f64,
}

/// A group (arc) representing a node
#[derive(Debug, Clone)]
pub struct ChordGroup {
    pub index: usize,
    pub start_angle: f64,
    pub end_angle: f64,
    pub value: f64,
}

/// Chord layout configuration
#[derive(Debug, Clone)]
pub struct ChordLayout {
    pub pad_angle: f64,
    pub sort_groups: Option<fn(f64, f64) -> std::cmp::Ordering>,
    pub sort_subgroups: Option<fn(f64, f64) -> std::cmp::Ordering>,
    pub sort_chords: Option<fn(f64, f64) -> std::cmp::Ordering>,
}

impl Default for ChordLayout {
    fn default() -> Self {
        Self {
            pad_angle: 0.0,
            sort_groups: None,
            sort_subgroups: None,
            sort_chords: None,
        }
    }
}

#[derive(Debug)]
pub struct ChordResult {
    pub chords: Vec<Chord>,
    pub groups: Vec<ChordGroup>,
}

/// Recoverable errors for chord layout input validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ChordLayoutError {
    /// The matrix must be square: every row must have the same length as the
    /// number of rows.
    NonSquareMatrix {
        row: usize,
        expected: usize,
        actual: usize,
    },
    /// Chord values must be finite.
    NonFiniteValue {
        row: usize,
        column: usize,
        value: f64,
    },
    /// Chord values must be zero or positive.
    NegativeValue {
        row: usize,
        column: usize,
        value: f64,
    },
}

impl fmt::Display for ChordLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonSquareMatrix {
                row,
                expected,
                actual,
            } => write!(
                f,
                "chord matrix row {row} has length {actual}; expected {expected}"
            ),
            Self::NonFiniteValue { row, column, value } => write!(
                f,
                "chord matrix value at row {row}, column {column} is not finite: {value}"
            ),
            Self::NegativeValue { row, column, value } => write!(
                f,
                "chord matrix value at row {row}, column {column} is negative: {value}"
            ),
        }
    }
}

impl std::error::Error for ChordLayoutError {}

impl ChordLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pad_angle(mut self, angle: f64) -> Self {
        self.pad_angle = angle;
        self
    }

    pub fn sort_groups(mut self, f: fn(f64, f64) -> std::cmp::Ordering) -> Self {
        self.sort_groups = Some(f);
        self
    }

    pub fn sort_subgroups(mut self, f: fn(f64, f64) -> std::cmp::Ordering) -> Self {
        self.sort_subgroups = Some(f);
        self
    }

    pub fn sort_chords(mut self, f: fn(f64, f64) -> std::cmp::Ordering) -> Self {
        self.sort_chords = Some(f);
        self
    }

    pub fn compute(&self, matrix: &[Vec<f64>]) -> ChordResult {
        self.try_compute(matrix)
            .expect("ChordLayout::compute requires a square matrix with finite non-negative values")
    }

    pub fn try_compute(&self, matrix: &[Vec<f64>]) -> Result<ChordResult, ChordLayoutError> {
        let n = matrix.len();
        if n == 0 {
            return Ok(ChordResult {
                chords: vec![],
                groups: vec![],
            });
        }

        validate_matrix(matrix)?;

        // 1. Compute group values
        let mut group_values = vec![0.0; n];
        let mut total_value = 0.0;

        for (i, row) in matrix.iter().enumerate().take(n) {
            for v in row.iter().take(n) {
                group_values[i] += v;
                total_value += v;
            }
        }

        // 2. Compute group angles
        let transform_k = if total_value > 0.0 {
            (2.0 * PI - self.pad_angle * n as f64) / total_value
        } else {
            0.0
        };

        let mut groups = Vec::with_capacity(n);
        let mut current_angle = 0.0;
        let mut group_order: Vec<usize> = (0..n).collect();

        if let Some(cmp_fn) = self.sort_groups {
            group_order.sort_by(|&a, &b| cmp_fn(group_values[a], group_values[b]));
        }

        for &i in &group_order {
            let value = group_values[i];
            let start_angle = current_angle;
            let end_angle = start_angle + value * transform_k;
            groups.push(ChordGroup {
                index: i,
                start_angle,
                end_angle,
                value,
            });
            current_angle = end_angle + self.pad_angle;
        }
        groups.sort_by_key(|group| group.index);

        // 3. Determine subgroup ordering per group
        // For each group i, build the order of target indices j
        let subgroup_orders: Vec<Vec<usize>> = (0..n)
            .map(|i| {
                let mut indices: Vec<usize> = (0..n).collect();
                if let Some(cmp_fn) = self.sort_subgroups {
                    indices.sort_by(|&a, &b| cmp_fn(matrix[i][a], matrix[i][b]));
                }
                indices
            })
            .collect();

        // 4. Pre-compute subgroup angles using the (possibly sorted) ordering
        // subgroup_angles[i][j] = (start_angle, end_angle) for the subgroup of group i targeting j
        let mut subgroup_angles: Vec<Vec<(f64, f64)>> = vec![vec![(0.0, 0.0); n]; n];
        let mut group_angular_positions = groups.iter().map(|g| g.start_angle).collect::<Vec<_>>();

        for i in 0..n {
            for &j in &subgroup_orders[i] {
                let value = matrix[i][j];
                let start = group_angular_positions[i];
                let end = start + value * transform_k;
                subgroup_angles[i][j] = (start, end);
                group_angular_positions[i] = end;
            }
        }

        // 5. Build chords by pairing (i,j) with j >= i
        let mut chords = Vec::new();

        for i in 0..n {
            for j in i..n {
                let v_ij = matrix[i][j];
                let v_ji = matrix[j][i];

                if v_ij > 0.0 || v_ji > 0.0 {
                    let (start_i, end_i) = subgroup_angles[i][j];
                    let (start_j, end_j) = subgroup_angles[j][i];

                    let source = ChordSubgroup {
                        index: i,
                        start_angle: start_i,
                        end_angle: end_i,
                        value: v_ij,
                    };

                    let target = ChordSubgroup {
                        index: j,
                        start_angle: start_j,
                        end_angle: end_j,
                        value: v_ji,
                    };

                    chords.push(Chord { source, target });
                }
            }
        }

        // 6. Apply sort_chords if present
        if let Some(cmp_fn) = self.sort_chords {
            chords.sort_by(|a, b| {
                let sum_a = a.source.value + a.target.value;
                let sum_b = b.source.value + b.target.value;
                cmp_fn(sum_a, sum_b)
            });
        }

        Ok(ChordResult { chords, groups })
    }
}

fn validate_matrix(matrix: &[Vec<f64>]) -> Result<(), ChordLayoutError> {
    let n = matrix.len();
    for (row_index, row) in matrix.iter().enumerate() {
        if row.len() != n {
            return Err(ChordLayoutError::NonSquareMatrix {
                row: row_index,
                expected: n,
                actual: row.len(),
            });
        }

        for (column_index, value) in row.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(ChordLayoutError::NonFiniteValue {
                    row: row_index,
                    column: column_index,
                    value,
                });
            }
            if value < 0.0 {
                return Err(ChordLayoutError::NegativeValue {
                    row: row_index,
                    column: column_index,
                    value,
                });
            }
        }
    }

    Ok(())
}

/// Generates SVG path data for a ribbon (chord)
pub struct RibbonGenerator {
    pub radius: f64,
    pub center_x: f64,
    pub center_y: f64,
}

impl RibbonGenerator {
    pub fn new(radius: f64) -> Self {
        Self {
            radius,
            center_x: 0.0,
            center_y: 0.0,
        }
    }

    pub fn center(mut self, x: f64, y: f64) -> Self {
        self.center_x = x;
        self.center_y = y;
        self
    }

    pub fn generate_path(&self, chord: &Chord) -> crate::shape::path::Path {
        use crate::shape::path::PathBuilder;
        use std::f64::consts::PI;

        let r = self.radius;
        let cx = self.center_x;
        let cy = self.center_y;

        let sa0 = chord.source.start_angle - PI / 2.0;
        let sa1 = chord.source.end_angle - PI / 2.0;

        let ta0 = chord.target.start_angle - PI / 2.0;
        let ta1 = chord.target.end_angle - PI / 2.0;

        let sx0 = cx + r * sa0.cos();
        let sy0 = cy + r * sa0.sin();

        let tx0 = cx + r * ta0.cos();
        let ty0 = cy + r * ta0.sin();

        PathBuilder::new()
            .move_to(sx0, sy0)
            .arc(cx, cy, r, sa0, sa1, true) // Wait, arc direction?
            // Arc(x, y, r, start, end, anticlockwise)
            // Start sa0, End sa1. Normalized angles.
            // Usually clockwise. So anticlockwise = false.
            // But d3-chord ribbons are complex. Simple Bezier approximation:
            .quadratic_curve_to(cx, cy, tx0, ty0)
            .arc(cx, cy, r, ta0, ta1, true)
            .quadratic_curve_to(cx, cy, sx0, sy0)
            .close_path()
            .build()
    }

    // Legacy String return for compatibility
    pub fn generate(&self, chord: &Chord) -> String {
        path_to_string(&self.generate_path(chord))
    }
}

#[cfg(test)]
mod tests {
    use super::{ChordLayout, ChordLayoutError};
    use std::cmp::Ordering;

    fn descending(a: f64, b: f64) -> Ordering {
        b.partial_cmp(&a).unwrap_or(Ordering::Equal)
    }

    #[test]
    fn sort_groups_orders_group_angles_without_reindexing_results() {
        let matrix = vec![
            vec![1.0, 1.0, 1.0],
            vec![4.0, 4.0, 4.0],
            vec![2.0, 2.0, 2.0],
        ];

        let result = ChordLayout::new()
            .sort_groups(descending)
            .try_compute(&matrix)
            .unwrap();

        assert_eq!(
            result.groups.iter().map(|g| g.index).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(result.groups[1].start_angle, 0.0);
        assert!(result.groups[2].start_angle > result.groups[1].start_angle);
        assert!(result.groups[0].start_angle > result.groups[2].start_angle);
    }

    #[test]
    fn sort_subgroups_uses_sorted_values_within_each_group() {
        let matrix = vec![
            vec![1.0, 3.0, 2.0],
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];

        let result = ChordLayout::new()
            .sort_subgroups(descending)
            .try_compute(&matrix)
            .unwrap();
        let source_to_one = result
            .chords
            .iter()
            .find(|chord| chord.source.index == 0 && chord.target.index == 1)
            .unwrap();
        let source_to_two = result
            .chords
            .iter()
            .find(|chord| chord.source.index == 0 && chord.target.index == 2)
            .unwrap();
        let source_to_zero = result
            .chords
            .iter()
            .find(|chord| chord.source.index == 0 && chord.target.index == 0)
            .unwrap();

        assert!(source_to_one.source.start_angle < source_to_two.source.start_angle);
        assert!(source_to_two.source.start_angle < source_to_zero.source.start_angle);
    }

    #[test]
    fn try_compute_rejects_ragged_matrices() {
        let err = ChordLayout::new()
            .try_compute(&[vec![1.0, 2.0], vec![3.0]])
            .unwrap_err();

        assert_eq!(
            err,
            ChordLayoutError::NonSquareMatrix {
                row: 1,
                expected: 2,
                actual: 1
            }
        );
    }

    #[test]
    fn try_compute_rejects_nan_and_negative_values() {
        match ChordLayout::new()
            .try_compute(&[vec![0.0, f64::NAN], vec![0.0, 0.0]])
            .unwrap_err()
        {
            ChordLayoutError::NonFiniteValue { row, column, value } => {
                assert_eq!(row, 0);
                assert_eq!(column, 1);
                assert!(value.is_nan());
            }
            other => panic!("unexpected error: {other:?}"),
        }

        assert_eq!(
            ChordLayout::new()
                .try_compute(&[vec![0.0, 0.0], vec![-1.0, 0.0]])
                .unwrap_err(),
            ChordLayoutError::NegativeValue {
                row: 1,
                column: 0,
                value: -1.0
            }
        );
    }
}
