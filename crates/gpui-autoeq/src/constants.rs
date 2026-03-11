//! Constants - Algorithm and model option arrays for AutoEQ forms.

/// Optimization type - determines which options are shown in the form
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizationType {
    /// Speaker optimization - shows system type, speaker-specific target curves
    #[default]
    Speaker,
    /// Headphone optimization - hides system type, shows Harman target curves
    Headphone,
}

/// Optimization mode options
pub const OPT_MODE_OPTIONS: &[(&str, &str)] = &[
    ("iir", "IIR (PEQ)"),
    ("fir", "FIR (Convolution)"),
    ("mixed", "Mixed (IIR + FIR)"),
];

/// FIR Phase options
pub const FIR_PHASE_OPTIONS: &[(&str, &str)] = &[
    ("linear", "Linear Phase"),
    ("minimum", "Minimum Phase"),
    ("kirkeby", "Kirkeby Inverse"),
];

/// Loss Type options
pub const LOSS_TYPE_OPTIONS: &[(&str, &str)] =
    &[("flat", "Flat Response"), ("score", "Preference Score")];

/// Target curve options for headphones (Harman curves)
pub const HEADPHONE_TARGET_CURVE_OPTIONS: &[(&str, &str)] = &[
    ("flat", "Flat"),
    ("harman-over-ear-2018", "Harman Over-Ear 2018"),
    ("harman-over-ear-2015", "Harman Over-Ear 2015"),
    ("harman-over-ear-2013", "Harman Over-Ear 2013"),
    ("harman-in-ear-2019", "Harman In-Ear 2019"),
    ("custom", "Custom (File Path)"),
];

/// Base target curve options for speakers (always available)
pub const SPEAKER_TARGET_CURVE_OPTIONS: &[(&str, &str)] =
    &[("flat", "Flat (0 dB)"), ("custom", "Custom (Manual Entry)")];

/// Spinorama curve options for speakers (available when spinorama data is loaded)
pub const SPINORAMA_CURVE_OPTIONS: &[(&str, &str)] = &[
    ("ON", "On-Axis (ON)"),
    ("LW", "Listening Window (LW)"),
    ("ER", "Early Reflections (ER)"),
    ("SP", "Sound Power (SP)"),
    ("PIR", "Predicted In-Room (PIR)"),
];

/// System Type options
pub const SYSTEM_TYPE_OPTIONS: &[(&str, &str)] = &[
    ("stereo", "Stereo / Independent"),
    ("multisub", "Multi-Subwoofer"),
    ("dba", "Double Bass Array"),
];

/// Tilt Type options
pub const TILT_TYPE_OPTIONS: &[(&str, &str)] = &[
    ("flat", "Flat (None)"),
    ("harman", "Harman (-0.8 dB/oct)"),
    ("custom", "Custom Tilt"),
];

/// Highpass Filter options
pub const HIGHPASS_TYPE_OPTIONS: &[(&str, &str)] =
    &[("lr", "Linkwitz-Riley"), ("bw", "Butterworth")];

/// Multi-Seat Strategy options
pub const MULTI_SEAT_STRATEGY_OPTIONS: &[(&str, &str)] = &[
    ("variance", "Minimize Variance"),
    ("primary", "Primary + Constraints"),
    ("average", "Average Response"),
];

/// Algorithm options for optimization
pub const ALGORITHM_OPTIONS: &[(&str, &str)] = &[
    ("autoeq:de", "Auto DE (Recommended)"),
    ("mh:de", "MH Differential Evolution"),
    ("mh:pso", "MH Particle Swarm"),
    ("mh:rga", "MH Genetic Algorithm"),
    ("mh:tlbo", "MH TLBO"),
    ("mh:fa", "MH Firefly"),
    ("nlopt:isres", "NLOPT ISRES"),
    ("nlopt:ags", "NLOPT AGS"),
    ("nlopt:cobyla", "NLOPT COBYLA"),
    ("nlopt:bobyqa", "NLOPT BOBYQA"),
    ("nlopt:neldermead", "NLOPT Nelder-Mead"),
];

/// DE strategy options
pub const DE_STRATEGY_OPTIONS: &[(&str, &str)] = &[
    ("currenttobest1bin", "Current-to-Best/1/Bin (Recommended)"),
    ("rand1bin", "Rand/1/Bin"),
    ("best1bin", "Best/1/Bin"),
    ("rand2bin", "Rand/2/Bin"),
    ("randtobest1bin", "Rand-to-Best/1/Bin"),
    ("adaptivebin", "Adaptive/Bin (Experimental)"),
];

/// PEQ model options
pub const PEQ_MODEL_OPTIONS: &[(&str, &str)] = &[
    ("pk", "PK - All Peak Filters"),
    ("hp-pk", "HP+PK - Highpass + Peaks"),
    ("hp-pk-lp", "HP+PK+LP - Highpass + Peaks + Lowpass"),
    ("ls-pk", "LS+PK - Low Shelf + Peaks"),
    ("ls-pk-hs", "LS+PK+HS - Low Shelf + Peaks + High Shelf"),
    ("free-pk-free", "Free+PK+Free - Flexible ends, peaks middle"),
    ("free", "Free - All filters flexible"),
];

/// Mixed mode crossover type options
pub const MIXED_CROSSOVER_TYPE_OPTIONS: &[(&str, &str)] = &[
    ("LR24", "Linkwitz-Riley 24dB"),
    ("LR48", "Linkwitz-Riley 48dB"),
];

/// Mixed mode FIR band options
pub const MIXED_FIR_BAND_OPTIONS: &[(&str, &str)] = &[
    ("low", "Low Frequencies (Bass)"),
    ("high", "High Frequencies"),
];

/// Local algorithm options for refinement
pub const LOCAL_ALGO_OPTIONS: &[(&str, &str)] = &[
    ("cobyla", "COBYLA"),
    ("bobyqa", "BOBYQA"),
    ("newuoa", "NEWUOA"),
];
