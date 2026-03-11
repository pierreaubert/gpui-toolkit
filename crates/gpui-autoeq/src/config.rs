//! AutoEQ configuration types - parameter limits and optimization config.

/// Limits for optimization parameters
#[derive(Debug, Clone, Copy)]
pub struct ParamLimits {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl ParamLimits {
    pub const NUM_FILTERS: Self = Self {
        min: 1.0,
        max: 20.0,
        step: 1.0,
    };
    pub const SAMPLE_RATE: Self = Self {
        min: 8000.0,
        max: 192000.0,
        step: 1000.0,
    };
    pub const DB: Self = Self {
        min: -25.0,
        max: 25.0,
        step: 0.5,
    };
    pub const Q: Self = Self {
        min: 0.1,
        max: 10.0,
        step: 0.1,
    };
    pub const FREQUENCY: Self = Self {
        min: 20.0,
        max: 20000.0,
        step: 10.0,
    };
    pub const FIR_TAPS: Self = Self {
        min: 256.0,
        max: 65536.0,
        step: 256.0,
    };
    pub const POPULATION: Self = Self {
        min: 10.0,
        max: 10000.0,
        step: 10.0,
    };
    pub const MAXEVAL: Self = Self {
        min: 100.0,
        max: 100000.0,
        step: 100.0,
    };
    pub const DE_FACTOR: Self = Self {
        min: 0.0,
        max: 2.0,
        step: 0.1,
    };
    pub const DE_CR: Self = Self {
        min: 0.0,
        max: 1.0,
        step: 0.1,
    };
    pub const SMOOTH_N: Self = Self {
        min: 1.0,
        max: 24.0,
        step: 1.0,
    };
    pub const TOLERANCE: Self = Self {
        min: 0.0,
        max: 1.0,
        step: 0.000001,
    };
    pub const SPACING_WEIGHT: Self = Self {
        min: 0.0,
        max: 1000.0,
        step: 0.1,
    };
    pub const MIN_SPACING_OCT: Self = Self {
        min: 0.01,
        max: 1.0,
        step: 0.01,
    };
    pub const TILT_SLOPE: Self = Self {
        min: -3.0,
        max: 3.0,
        step: 0.1,
    };
    pub const BASS_SHELF: Self = Self {
        min: 0.0,
        max: 12.0,
        step: 0.5,
    };
    pub const SCHROEDER_FREQ: Self = Self {
        min: 50.0,
        max: 1000.0,
        step: 10.0,
    };
    pub const DELAY_MS: Self = Self {
        min: 0.0,
        max: 100.0,
        step: 0.1,
    };
    pub const SEED: Self = Self {
        min: 0.0,
        max: 999999.0,
        step: 1.0,
    };
    pub const GD_TARGET_MS: Self = Self {
        min: 0.0,
        max: 50.0,
        step: 0.1,
    };
    pub const MIXED_CROSSOVER_FREQ: Self = Self {
        min: 50.0,
        max: 2000.0,
        step: 10.0,
    };
}

/// AutoEQ optimization configuration - matches OptimizationParams from sotf-audio-player
#[derive(Debug, Clone)]
pub struct AutoEqConfig {
    // EQ Design Parameters
    /// Optimization mode (IIR, FIR, Mixed)
    pub opt_mode: String,
    /// Number of FIR taps (for FIR/Mixed mode)
    pub fir_taps: usize,
    /// FIR phase type (for FIR/Mixed mode)
    pub fir_phase: String,
    /// Number of PEQ filters
    pub num_filters: usize,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Minimum gain in dB
    pub min_db: f64,
    /// Maximum gain in dB
    pub max_db: f64,
    /// Minimum Q factor
    pub min_q: f64,
    /// Maximum Q factor
    pub max_q: f64,
    /// Minimum frequency in Hz
    pub min_freq: f64,
    /// Maximum frequency in Hz
    pub max_freq: f64,
    /// PEQ model (e.g., "pk", "ls-pk-hs")
    pub peq_model: String,
    /// Spacing constraint weight (0-1000)
    pub spacing_weight: f64,
    /// Minimum spacing between filters in octaves (0.01-1.0)
    pub min_spacing_oct: f64,

    // Algorithm Parameters
    /// Optimization algorithm (e.g., "autoeq:de", "nlopt:cobyla")
    pub algo: String,
    /// Population size for evolutionary algorithms
    pub population: usize,
    /// Maximum function evaluations
    pub maxeval: usize,
    /// Relative tolerance for convergence
    pub tolerance: f64,
    /// Absolute tolerance for convergence
    pub atolerance: f64,

    // DE-specific Parameters
    /// Mutation factor (F) for DE
    pub de_f: f64,
    /// Crossover rate (CR) for DE
    pub de_cr: f64,
    /// DE strategy (e.g., "currenttobest1bin")
    pub strategy: String,

    // Refinement Parameters
    /// Enable local refinement after global optimization
    pub refine: bool,
    /// Local algorithm for refinement
    pub local_algo: String,

    // Smoothing Parameters
    /// Enable smoothing
    pub smooth: bool,
    /// Smoothing window size (1-24)
    pub smooth_n: usize,

    /// Enable psychoacoustic variable smoothing
    pub psychoacoustic: bool,
    /// Enable asymmetric loss weighting
    pub asymmetric_loss: bool,

    // Goals & Configuration
    /// Loss function type (e.g., "flat", "score")
    pub loss_type: String,
    /// Target curve (e.g., "flat", "harman")
    pub target_curve: String,
    /// System type (e.g., "stereo", "multisub")
    pub system_type: String,

    // --- Advanced Room Correction (Scenario B) ---
    /// Enable target tilt
    pub use_target_tilt: bool,
    /// Tilt type: flat, harman, custom
    pub tilt_type: String,
    /// Tilt slope in dB/octave
    pub tilt_slope: f64,
    /// Tilt reference frequency in Hz
    pub tilt_reference_freq: f64,
    /// Bass shelf boost in dB
    pub tilt_bass_shelf_db: f64,
    /// Bass shelf frequency in Hz
    pub tilt_bass_shelf_freq: f64,

    /// Enable excursion protection
    pub use_excursion_protection: bool,
    /// Auto-detect F3 from measurement
    pub excursion_auto_detect_f3: bool,
    /// Manual F3 override in Hz
    pub excursion_manual_f3: f64,
    /// HPF filter order (2 or 4)
    pub excursion_filter_order: usize,
    /// HPF filter type (lr, bw)
    pub excursion_filter_type: String,
    /// HPF margin in octaves
    pub excursion_margin_octaves: f64,

    /// Enable Schroeder split
    pub use_schroeder_split: bool,
    /// Schroeder frequency in Hz
    pub schroeder_freq: f64,
    /// Low freq max Q
    pub schroeder_low_max_q: f64,
    /// Low freq allow boost
    pub schroeder_low_allow_boost: bool,
    /// High freq max Q
    pub schroeder_high_max_q: f64,
    /// High freq shelving only
    pub schroeder_high_shelving_only: bool,

    // --- v2 fields ---
    /// Allow inter-speaker delay optimization
    pub allow_delay: bool,
    /// Enable seed for reproducible results
    pub seed_enabled: bool,
    /// Random seed value
    pub seed: u64,
    /// Enable group delay optimization
    pub gd_opt_enabled: bool,
    /// Group delay target in ms
    pub gd_opt_target_ms: f64,
    /// Enable Voice of God (timbre matching)
    pub vog_enabled: bool,
    /// VoG reference channel name
    pub vog_reference_channel: String,
    /// Enable broadband target matching
    pub broadband_target_matching: bool,
    /// Mixed mode crossover frequency
    pub mixed_crossover_freq: f64,
    /// Mixed mode crossover type ("LR24", "LR48")
    pub mixed_crossover_type: String,
    /// Mixed mode FIR band ("low" or "high")
    pub mixed_fir_band: String,

    // --- Advanced System Optimization (Scenario A) ---
    /// Enable phase alignment
    pub use_phase_alignment: bool,
    /// Phase alignment min freq
    pub phase_min_freq: f64,
    /// Phase alignment max freq
    pub phase_max_freq: f64,
    /// Optimize polarity
    pub phase_optimize_polarity: bool,
    /// Maximum delay in ms
    pub phase_max_delay_ms: f64,

    /// Enable multi-seat optimization
    pub use_multi_seat: bool,
    /// Multi-seat strategy
    pub multi_seat_strategy: String,
    /// Primary seat index
    pub multi_seat_primary_seat: usize,
    /// Max deviation in dB
    pub multi_seat_max_deviation_db: f64,
}

impl Default for AutoEqConfig {
    fn default() -> Self {
        Self {
            opt_mode: "iir".to_string(),
            fir_taps: 4096,
            fir_phase: "kirkeby".to_string(),
            num_filters: 10,
            sample_rate: 48000,
            min_db: -12.0,
            max_db: 6.0,
            min_q: 0.5,
            max_q: 10.0,
            min_freq: 20.0,
            max_freq: 20000.0,
            peq_model: "pk".to_string(),
            spacing_weight: 1.0,
            min_spacing_oct: 0.08,
            algo: "autoeq:de".to_string(),
            population: 100,
            maxeval: 10000,
            tolerance: 0.00001,
            atolerance: 0.00001,
            de_f: 0.8,
            de_cr: 0.9,
            strategy: "currenttobest1bin".to_string(),
            refine: true,
            local_algo: "cobyla".to_string(),
            smooth: false,
            smooth_n: 6,
            psychoacoustic: true,
            asymmetric_loss: true,
            loss_type: "flat".to_string(),
            target_curve: "flat".to_string(),
            system_type: "stereo".to_string(),

            // Scenario B defaults
            use_target_tilt: false,
            tilt_type: "flat".to_string(),
            tilt_slope: -0.8,
            tilt_reference_freq: 1000.0,
            tilt_bass_shelf_db: 0.0,
            tilt_bass_shelf_freq: 200.0,

            use_excursion_protection: false,
            excursion_auto_detect_f3: true,
            excursion_manual_f3: 40.0,
            excursion_filter_order: 4,
            excursion_filter_type: "lr".to_string(),
            excursion_margin_octaves: 0.25,

            use_schroeder_split: false,
            schroeder_freq: 300.0,
            schroeder_low_max_q: 10.0,
            schroeder_low_allow_boost: false,
            schroeder_high_max_q: 1.0,
            schroeder_high_shelving_only: false,

            // v2 defaults
            allow_delay: false,
            seed_enabled: false,
            seed: 42,
            gd_opt_enabled: false,
            gd_opt_target_ms: 0.0,
            vog_enabled: false,
            vog_reference_channel: "C".to_string(),
            broadband_target_matching: false,
            mixed_crossover_freq: 300.0,
            mixed_crossover_type: "LR24".to_string(),
            mixed_fir_band: "low".to_string(),

            // Scenario A defaults
            use_phase_alignment: false,
            phase_min_freq: 60.0,
            phase_max_freq: 100.0,
            phase_optimize_polarity: true,
            phase_max_delay_ms: 30.0,

            use_multi_seat: false,
            multi_seat_strategy: "variance".to_string(),
            multi_seat_primary_seat: 0,
            multi_seat_max_deviation_db: 6.0,
        }
    }
}
