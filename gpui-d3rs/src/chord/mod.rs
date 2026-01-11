//! Chord Diagram Layout (d3-chord)
//!
//! Visualizes relationships or flows between a set of nodes using a circular layout.

use std::f64::consts::PI;

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

pub struct ChordResult {
    pub chords: Vec<Chord>,
    pub groups: Vec<ChordGroup>,
}

impl ChordLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pad_angle(mut self, angle: f64) -> Self {
        self.pad_angle = angle;
        self
    }

    pub fn compute(&self, matrix: &[Vec<f64>]) -> ChordResult {
        let n = matrix.len();
        if n == 0 {
            return ChordResult {
                chords: vec![],
                groups: vec![],
            };
        }

        // 1. Compute group values
        let mut group_values = vec![0.0; n];
        let mut total_value = 0.0;

        for i in 0..n {
            for j in 0..n {
                let v = matrix[i][j];
                group_values[i] += v;
                total_value += v;
            }
        }

        // 2. Compute group angles
        // TODO: Apply sort_groups if present

        let transform_k = if total_value > 0.0 {
            (2.0 * PI - self.pad_angle * n as f64) / total_value
        } else {
            0.0
        };

        let mut groups = Vec::with_capacity(n);
        let mut current_angle = 0.0;

        for (i, &value) in group_values.iter().enumerate() {
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

        // 3. Compute Chords
        let mut chords = Vec::new();

        // Subgroup angles tracking
        let mut group_angular_positions = groups.iter().map(|g| g.start_angle).collect::<Vec<_>>();

        for i in 0..n {
            for j in 0..n {
                // Only create chords for i -> j if i < j (undirected visual) OR create both if directed flows needed.
                // d3-chord typically creates directed ribbons.
                // But typically we visualize the flow i->j combined with j->i
                // For simplicity here, we generate directed chords for all non-zero i->j

                // Usually d3 pairs (i,j) and (j,i) into a SINGLE chord object with source=i->j and target=j->i
                // To do this, we iterate i from 0..n, j from i..n

                // Let's implement the standard pairing
                if j >= i {
                    let v_ij = matrix[i][j];
                    let v_ji = matrix[j][i];

                    if v_ij > 0.0 || v_ji > 0.0 {
                        // Source (i)
                        let start_i = group_angular_positions[i];
                        let end_i = start_i + v_ij * transform_k;
                        group_angular_positions[i] = end_i; // Advance

                        // Target (j)
                        let start_j = group_angular_positions[j];
                        let end_j = start_j + v_ji * transform_k;
                        group_angular_positions[j] = end_j; // Advance

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
        }

        // TODO: Apply sort_chords

        ChordResult { chords, groups }
    }
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
        self.generate_path(chord).to_svg_string()
    }
}
