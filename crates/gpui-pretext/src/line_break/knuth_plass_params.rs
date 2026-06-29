/// Tuning parameters for the Knuth-Plass algorithm.
#[derive(Debug, Clone, Copy)]
pub struct KnuthPlassParams {
    /// Penalty for a normal inter-word breakpoint (typically 0).
    pub line_penalty: f64,
    /// Penalty for a hyphenation break (typically 50).
    pub hyphen_penalty: f64,
    /// Extra demerits when two consecutive lines end with flagged breaks
    /// (e.g., both hyphenated). Typically 3000.
    pub flagged_demerits: f64,
    /// Extra demerits when adjacent lines differ by more than one fitness
    /// class. Typically 10000.
    pub fitness_demerits: f64,
    /// Tolerance: maximum allowed adjustment ratio before a breakpoint is
    /// considered infeasible. Typically 1.0–2.0 for body text.
    pub tolerance: f64,
    /// When true and no feasible solution exists within tolerance, increase
    /// tolerance and retry rather than failing.
    pub looseness_recovery: bool,
}

impl Default for KnuthPlassParams {
    fn default() -> Self {
        Self {
            line_penalty: 0.0,
            hyphen_penalty: 50.0,
            flagged_demerits: 3000.0,
            fitness_demerits: 10000.0,
            tolerance: 2.0,
            looseness_recovery: true,
        }
    }
}
