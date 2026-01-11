//! AutoEQ Parameter Form
//!
//! A reusable form component for AutoEQ optimization parameters.
//! Used by Room EQ, Speaker EQ, Headphone EQ, and Group optimization screens.
//!
//! The form includes two sections:
//! 1. EQ Design Parameters:
//!    - Number of PEQ filters
//!    - Sample rate
//!    - dB range (min/max)
//!    - Q factor range (min/max)
//!    - Frequency range (min/max Hz)
//!    - PEQ model selection
//!
//! 2. Optimization Fine Tuning:
//!    - Algorithm selection
//!    - Population size
//!    - Max evaluations
//!    - DE-specific parameters (strategy, mutation F, recombination CR)
//!    - Local refinement toggle and algorithm
//!    - Smoothing toggle

use gpui::prelude::*;
use gpui::*;

use crate::ComponentTheme;
use crate::card::Card;
use crate::number_input::{NumberInput, NumberInputSize, NumberInputTheme};
use crate::select::{Select, SelectOption, SelectTheme};
use crate::stack::{HStack, StackJustify, StackSpacing, VStack};
use crate::text::{Text, TextSize, TextWeight};
use crate::theme::ThemeExt;
use crate::toggle::{Toggle, ToggleSize, ToggleTheme};

// ============================================================================
// Constants - Algorithm and Model Options
// ============================================================================

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

/// Local algorithm options for refinement
pub const LOCAL_ALGO_OPTIONS: &[(&str, &str)] = &[
    ("cobyla", "COBYLA"),
    ("bobyqa", "BOBYQA"),
    ("newuoa", "NEWUOA"),
];

// ============================================================================
// Parameter Limits
// ============================================================================

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
}

// ============================================================================
// AutoEQ Form State (external state management)
// ============================================================================

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

    // Goals & Configuration
    /// Loss function type (e.g., "flat", "score")
    pub loss_type: String,
    /// Target curve (e.g., "flat", "harman")
    pub target_curve: String,
    /// System type (e.g., "stereo", "multisub")
    pub system_type: String,
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
            loss_type: "flat".to_string(),
            target_curve: "flat".to_string(),
            system_type: "stereo".to_string(),
        }
    }
}

/// UI state for AutoEQ form dropdowns
#[derive(Debug, Clone, Default)]
pub struct AutoEqFormUiState {
    /// EQ Mode dropdown open state
    pub opt_mode_open: bool,
    /// FIR Phase dropdown open state
    pub fir_phase_open: bool,
    /// Algorithm dropdown open state
    pub algo_open: bool,
    /// PEQ model dropdown open state
    pub peq_model_open: bool,
    /// DE strategy dropdown open state
    pub strategy_open: bool,
    /// Local algorithm dropdown open state
    pub local_algo_open: bool,
    /// Loss type dropdown open state
    pub loss_type_open: bool,
    /// Target curve dropdown open state
    pub target_curve_open: bool,
    /// System type dropdown open state
    pub system_type_open: bool,
}

// ============================================================================
// AutoEQ Form Theme
// ============================================================================

/// Theme for the AutoEQ form
#[derive(Debug, Clone, ComponentTheme)]
pub struct AutoEqFormTheme {
    /// Card background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub card_bg: Rgba,
    /// Section header color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub header_color: Rgba,
    /// Label color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label_color: Rgba,
    /// Description color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub description_color: Rgba,
    /// Accent color
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Toggle theme colors
    #[theme(default = 0x007accff, from = accent)]
    pub toggle_checked_bg: Rgba,
    #[theme(default = 0x4a4a4aff, from = muted)]
    pub toggle_unchecked_bg: Rgba,
    #[theme(default = 0xffffffff, from = text_primary)]
    pub toggle_knob: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Text muted color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub text_muted: Rgba,
    /// NumberInput theme
    #[theme(
        default_expr = "NumberInputTheme::default()",
        from_expr = "NumberInputTheme::from(theme)"
    )]
    pub number_input_theme: NumberInputTheme,
    /// Select theme
    #[theme(
        default_expr = "SelectTheme::default()",
        from_expr = "SelectTheme::from(theme)"
    )]
    pub select_theme: SelectTheme,
}

// ============================================================================
// AutoEQ Form Component
// ============================================================================

/// Callback type for string parameter changes
type StringCallback = Box<dyn Fn(&str, &mut Window, &mut App) + 'static>;
/// Callback type for f64 parameter changes
type F64Callback = Box<dyn Fn(f64, &mut Window, &mut App) + 'static>;
/// Callback type for usize parameter changes
type UsizeCallback = Box<dyn Fn(usize, &mut Window, &mut App) + 'static>;
/// Callback type for bool parameter changes
type BoolCallback = Box<dyn Fn(bool, &mut Window, &mut App) + 'static>;
/// Callback type for dropdown toggle
type ToggleCallback = Box<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// A reusable form for AutoEQ optimization parameters.
///
/// Renders three sections:
/// 1. Goals & Configuration - system type, targets, and EQ mode
/// 2. EQ Design Parameters - filter characteristics and frequency ranges
/// 3. Optimization Fine Tuning - algorithm settings and DE parameters
///
/// The form adapts its options based on `optimization_type`:
/// - **Speaker**: Shows system type, target curves include flat, custom, and spinorama curves
/// - **Headphone**: Hides system type, target curves include Harman curves
#[derive(IntoElement)]
pub struct AutoEqForm {
    id: ElementId,
    config: AutoEqConfig,
    ui_state: AutoEqFormUiState,
    disabled: bool,
    show_goals: bool,
    show_eq_design: bool,
    show_optimization_tuning: bool,
    theme: Option<AutoEqFormTheme>,
    allowed_opt_modes: Option<Vec<String>>,
    /// Type of optimization (Speaker or Headphone) - affects which options are shown
    optimization_type: OptimizationType,
    /// Available spinorama curves for speaker mode (e.g., ["ON", "LW", "PIR"])
    available_spinorama_curves: Vec<String>,

    // EQ Design callbacks
    on_opt_mode_change: Option<StringCallback>,
    on_opt_mode_toggle: Option<ToggleCallback>,
    on_fir_taps_change: Option<UsizeCallback>,
    on_fir_phase_change: Option<StringCallback>,
    on_fir_phase_toggle: Option<ToggleCallback>,
    on_num_filters_change: Option<UsizeCallback>,
    on_sample_rate_change: Option<UsizeCallback>,
    on_min_db_change: Option<F64Callback>,
    on_max_db_change: Option<F64Callback>,
    on_min_q_change: Option<F64Callback>,
    on_max_q_change: Option<F64Callback>,
    on_min_freq_change: Option<F64Callback>,
    on_max_freq_change: Option<F64Callback>,
    on_peq_model_change: Option<StringCallback>,
    on_peq_model_toggle: Option<ToggleCallback>,
    on_spacing_weight_change: Option<F64Callback>,
    on_min_spacing_oct_change: Option<F64Callback>,

    // Optimization callbacks
    on_algo_change: Option<StringCallback>,
    on_algo_toggle: Option<ToggleCallback>,
    on_population_change: Option<UsizeCallback>,
    on_maxeval_change: Option<UsizeCallback>,
    on_tolerance_change: Option<F64Callback>,
    on_atolerance_change: Option<F64Callback>,
    on_de_f_change: Option<F64Callback>,
    on_de_cr_change: Option<F64Callback>,
    on_strategy_change: Option<StringCallback>,
    on_strategy_toggle: Option<ToggleCallback>,
    on_refine_change: Option<BoolCallback>,
    on_local_algo_change: Option<StringCallback>,
    on_local_algo_toggle: Option<ToggleCallback>,
    on_smooth_change: Option<BoolCallback>,
    on_smooth_n_change: Option<UsizeCallback>,

    // Goals callbacks
    on_loss_type_change: Option<StringCallback>,
    on_loss_type_toggle: Option<ToggleCallback>,
    on_target_curve_change: Option<StringCallback>,
    on_target_curve_toggle: Option<ToggleCallback>,
    on_system_type_change: Option<StringCallback>,
    on_system_type_toggle: Option<ToggleCallback>,
}

impl AutoEqForm {
    /// Create a new AutoEQ form
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            config: AutoEqConfig::default(),
            ui_state: AutoEqFormUiState::default(),
            disabled: false,
            show_goals: true,
            show_eq_design: true,
            show_optimization_tuning: true,
            theme: None,
            allowed_opt_modes: None,
            optimization_type: OptimizationType::default(),
            available_spinorama_curves: Vec::new(),
            on_opt_mode_change: None,
            on_opt_mode_toggle: None,
            on_fir_taps_change: None,
            on_fir_phase_change: None,
            on_fir_phase_toggle: None,
            on_num_filters_change: None,
            on_sample_rate_change: None,
            on_min_db_change: None,
            on_max_db_change: None,
            on_min_q_change: None,
            on_max_q_change: None,
            on_min_freq_change: None,
            on_max_freq_change: None,
            on_peq_model_change: None,
            on_peq_model_toggle: None,
            on_spacing_weight_change: None,
            on_min_spacing_oct_change: None,
            on_algo_change: None,
            on_algo_toggle: None,
            on_population_change: None,
            on_maxeval_change: None,
            on_tolerance_change: None,
            on_atolerance_change: None,
            on_de_f_change: None,
            on_de_cr_change: None,
            on_strategy_change: None,
            on_strategy_toggle: None,
            on_refine_change: None,
            on_local_algo_change: None,
            on_local_algo_toggle: None,
            on_smooth_change: None,
            on_smooth_n_change: None,
            on_loss_type_change: None,
            on_loss_type_toggle: None,
            on_target_curve_change: None,
            on_target_curve_toggle: None,
            on_system_type_change: None,
            on_system_type_toggle: None,
        }
    }

    /// Set the configuration values
    pub fn config(mut self, config: AutoEqConfig) -> Self {
        self.config = config;
        self
    }

    /// Set UI state
    pub fn ui_state(mut self, ui_state: AutoEqFormUiState) -> Self {
        self.ui_state = ui_state;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show/hide Goals section
    pub fn show_goals(mut self, show: bool) -> Self {
        self.show_goals = show;
        self
    }

    /// Show/hide EQ Design section
    pub fn show_eq_design(mut self, show: bool) -> Self {
        self.show_eq_design = show;
        self
    }

    /// Show/hide Optimization Tuning section
    pub fn show_optimization_tuning(mut self, show: bool) -> Self {
        self.show_optimization_tuning = show;
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: AutoEqFormTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set allowed optimization modes (e.g., vec!["iir".to_string(), "fir".to_string()])
    pub fn allowed_opt_modes(mut self, modes: Vec<String>) -> Self {
        self.allowed_opt_modes = Some(modes);
        self
    }

    /// Set the optimization type (Speaker or Headphone)
    ///
    /// This affects which options are shown in the Goals section:
    /// - **Speaker**: Shows system type dropdown, target curves include flat, custom, and spinorama curves
    /// - **Headphone**: Hides system type dropdown, target curves include Harman curves
    pub fn optimization_type(mut self, opt_type: OptimizationType) -> Self {
        self.optimization_type = opt_type;
        self
    }

    /// Set available spinorama curves for speaker mode
    ///
    /// Only curves in this list will be shown in the target curve dropdown.
    /// Common values: "ON", "LW", "ER", "SP", "PIR"
    pub fn available_spinorama_curves(mut self, curves: Vec<String>) -> Self {
        self.available_spinorama_curves = curves;
        self
    }

    // EQ Design callbacks

    /// Set optim mode change handler
    pub fn on_opt_mode_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_opt_mode_change = Some(Box::new(handler));
        self
    }

    /// Set optim mode dropdown toggle handler
    pub fn on_opt_mode_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_opt_mode_toggle = Some(Box::new(handler));
        self
    }

    /// Set FIR taps change handler
    pub fn on_fir_taps_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_fir_taps_change = Some(Box::new(handler));
        self
    }

    /// Set FIR phase change handler
    pub fn on_fir_phase_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_fir_phase_change = Some(Box::new(handler));
        self
    }

    /// Set FIR phase dropdown toggle handler
    pub fn on_fir_phase_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_fir_phase_toggle = Some(Box::new(handler));
        self
    }

    /// Set number of filters change handler
    pub fn on_num_filters_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_num_filters_change = Some(Box::new(handler));
        self
    }

    /// Set sample rate change handler
    pub fn on_sample_rate_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_sample_rate_change = Some(Box::new(handler));
        self
    }

    /// Set min dB change handler
    pub fn on_min_db_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_min_db_change = Some(Box::new(handler));
        self
    }

    /// Set max dB change handler
    pub fn on_max_db_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_max_db_change = Some(Box::new(handler));
        self
    }

    /// Set min Q change handler
    pub fn on_min_q_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_min_q_change = Some(Box::new(handler));
        self
    }

    /// Set max Q change handler
    pub fn on_max_q_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_max_q_change = Some(Box::new(handler));
        self
    }

    /// Set min frequency change handler
    pub fn on_min_freq_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_min_freq_change = Some(Box::new(handler));
        self
    }

    /// Set max frequency change handler
    pub fn on_max_freq_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_max_freq_change = Some(Box::new(handler));
        self
    }

    /// Set PEQ model change handler
    pub fn on_peq_model_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_peq_model_change = Some(Box::new(handler));
        self
    }

    /// Set PEQ model dropdown toggle handler
    pub fn on_peq_model_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_peq_model_toggle = Some(Box::new(handler));
        self
    }

    /// Set spacing weight change handler
    pub fn on_spacing_weight_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_spacing_weight_change = Some(Box::new(handler));
        self
    }

    /// Set min spacing octaves change handler
    pub fn on_min_spacing_oct_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_min_spacing_oct_change = Some(Box::new(handler));
        self
    }

    // Optimization callbacks

    /// Set algorithm change handler
    pub fn on_algo_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_algo_change = Some(Box::new(handler));
        self
    }

    /// Set algorithm dropdown toggle handler
    pub fn on_algo_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_algo_toggle = Some(Box::new(handler));
        self
    }

    /// Set population change handler
    pub fn on_population_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_population_change = Some(Box::new(handler));
        self
    }

    /// Set maxeval change handler
    pub fn on_maxeval_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_maxeval_change = Some(Box::new(handler));
        self
    }

    /// Set relative tolerance change handler
    pub fn on_tolerance_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tolerance_change = Some(Box::new(handler));
        self
    }

    /// Set absolute tolerance change handler
    pub fn on_atolerance_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_atolerance_change = Some(Box::new(handler));
        self
    }

    /// Set DE mutation factor (F) change handler
    pub fn on_de_f_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_de_f_change = Some(Box::new(handler));
        self
    }

    /// Set DE crossover rate (CR) change handler
    pub fn on_de_cr_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_de_cr_change = Some(Box::new(handler));
        self
    }

    /// Set DE strategy change handler
    pub fn on_strategy_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_strategy_change = Some(Box::new(handler));
        self
    }

    /// Set DE strategy dropdown toggle handler
    pub fn on_strategy_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_strategy_toggle = Some(Box::new(handler));
        self
    }

    /// Set local refinement toggle handler
    pub fn on_refine_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_refine_change = Some(Box::new(handler));
        self
    }

    /// Set local algorithm change handler
    pub fn on_local_algo_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_local_algo_change = Some(Box::new(handler));
        self
    }

    /// Set local algorithm dropdown toggle handler
    pub fn on_local_algo_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_local_algo_toggle = Some(Box::new(handler));
        self
    }

    /// Set smoothing toggle handler
    pub fn on_smooth_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_smooth_change = Some(Box::new(handler));
        self
    }

    /// Set smoothing window size change handler
    pub fn on_smooth_n_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_smooth_n_change = Some(Box::new(handler));
        self
    }

    // Goals callbacks

    /// Set loss type change handler
    pub fn on_loss_type_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_loss_type_change = Some(Box::new(handler));
        self
    }

    /// Set loss type dropdown toggle handler
    pub fn on_loss_type_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_loss_type_toggle = Some(Box::new(handler));
        self
    }

    /// Set target curve change handler
    pub fn on_target_curve_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_target_curve_change = Some(Box::new(handler));
        self
    }

    /// Set target curve dropdown toggle handler
    pub fn on_target_curve_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_target_curve_toggle = Some(Box::new(handler));
        self
    }

    /// Set system type change handler
    pub fn on_system_type_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_system_type_change = Some(Box::new(handler));
        self
    }

    /// Set system type dropdown toggle handler
    pub fn on_system_type_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_system_type_toggle = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for AutoEqForm {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| AutoEqFormTheme::from(&global_theme));

        let id = self.id;
        let config = self.config;
        let ui_state = self.ui_state;
        let disabled = self.disabled;
        let show_goals = self.show_goals;
        let show_eq_design = self.show_eq_design;
        let show_optimization_tuning = self.show_optimization_tuning;
        let optimization_type = self.optimization_type;
        let available_spinorama_curves = self.available_spinorama_curves;

        // Wrap callbacks in Rc for sharing
        let on_opt_mode_change_rc = self.on_opt_mode_change.map(std::rc::Rc::new);
        let on_opt_mode_toggle_rc = self.on_opt_mode_toggle.map(std::rc::Rc::new);
        let on_fir_taps_change_rc = self.on_fir_taps_change.map(std::rc::Rc::new);
        let on_fir_phase_change_rc = self.on_fir_phase_change.map(std::rc::Rc::new);
        let on_fir_phase_toggle_rc = self.on_fir_phase_toggle.map(std::rc::Rc::new);
        let on_num_filters_change_rc = self.on_num_filters_change.map(std::rc::Rc::new);
        let on_sample_rate_change_rc = self.on_sample_rate_change.map(std::rc::Rc::new);
        let on_min_db_change_rc = self.on_min_db_change.map(std::rc::Rc::new);
        let on_max_db_change_rc = self.on_max_db_change.map(std::rc::Rc::new);
        let on_min_q_change_rc = self.on_min_q_change.map(std::rc::Rc::new);
        let on_max_q_change_rc = self.on_max_q_change.map(std::rc::Rc::new);
        let on_min_freq_change_rc = self.on_min_freq_change.map(std::rc::Rc::new);
        let on_max_freq_change_rc = self.on_max_freq_change.map(std::rc::Rc::new);
        let on_peq_model_change_rc = self.on_peq_model_change.map(std::rc::Rc::new);
        let on_peq_model_toggle_rc = self.on_peq_model_toggle.map(std::rc::Rc::new);
        let on_spacing_weight_change_rc = self.on_spacing_weight_change.map(std::rc::Rc::new);
        let on_min_spacing_oct_change_rc = self.on_min_spacing_oct_change.map(std::rc::Rc::new);
        let on_algo_change_rc = self.on_algo_change.map(std::rc::Rc::new);
        let on_algo_toggle_rc = self.on_algo_toggle.map(std::rc::Rc::new);
        let on_population_change_rc = self.on_population_change.map(std::rc::Rc::new);
        let on_maxeval_change_rc = self.on_maxeval_change.map(std::rc::Rc::new);
        let on_tolerance_change_rc = self.on_tolerance_change.map(std::rc::Rc::new);
        let on_atolerance_change_rc = self.on_atolerance_change.map(std::rc::Rc::new);
        let on_de_f_change_rc = self.on_de_f_change.map(std::rc::Rc::new);
        let on_de_cr_change_rc = self.on_de_cr_change.map(std::rc::Rc::new);
        let on_strategy_change_rc = self.on_strategy_change.map(std::rc::Rc::new);
        let on_strategy_toggle_rc = self.on_strategy_toggle.map(std::rc::Rc::new);
        let on_refine_change_rc = self.on_refine_change.map(std::rc::Rc::new);
        let on_local_algo_change_rc = self.on_local_algo_change.map(std::rc::Rc::new);
        let on_local_algo_toggle_rc = self.on_local_algo_toggle.map(std::rc::Rc::new);
        let on_smooth_change_rc = self.on_smooth_change.map(std::rc::Rc::new);
        let on_smooth_n_change_rc = self.on_smooth_n_change.map(std::rc::Rc::new);
        let on_loss_type_change_rc = self.on_loss_type_change.map(std::rc::Rc::new);
        let on_loss_type_toggle_rc = self.on_loss_type_toggle.map(std::rc::Rc::new);
        let on_target_curve_change_rc = self.on_target_curve_change.map(std::rc::Rc::new);
        let on_target_curve_toggle_rc = self.on_target_curve_toggle.map(std::rc::Rc::new);
        let on_system_type_change_rc = self.on_system_type_change.map(std::rc::Rc::new);
        let on_system_type_toggle_rc = self.on_system_type_toggle.map(std::rc::Rc::new);

        let mut form = VStack::new().spacing(StackSpacing::Lg);

        // ========================================
        // Goals & Configuration Section
        // ========================================
        if show_goals {
            let mut goals_content = VStack::new().spacing(StackSpacing::Sm);

            // Header
            goals_content = goals_content.child(
                VStack::new()
                    .spacing(StackSpacing::None)
                    .child(
                        Text::new("Goals & Configuration")
                            .size(TextSize::Sm)
                            .weight(TextWeight::Semibold)
                            .color(theme.header_color),
                    )
                    .child(
                        Text::new("Optimization goals, system type, and targets")
                            .size(TextSize::Xs)
                            .color(theme.description_color),
                    ),
            );

            // System Type dropdown - only shown for Speaker optimization
            if optimization_type == OptimizationType::Speaker {
                let system_type_options: Vec<SelectOption> = SYSTEM_TYPE_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut system_type_select = Select::new("autoeq-system-type")
                    .label("System Type")
                    .options(system_type_options)
                    .selected(&config.system_type)
                    .is_open(ui_state.system_type_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_system_type_toggle_rc {
                    let h = handler.clone();
                    system_type_select =
                        system_type_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_system_type_change_rc {
                    let h = handler.clone();
                    system_type_select =
                        system_type_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
                }

                goals_content = goals_content.child(system_type_select);
            }

            // Loss Type dropdown
            let loss_type_options: Vec<SelectOption> = LOSS_TYPE_OPTIONS
                .iter()
                .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                .collect();

            let mut loss_type_select = Select::new("autoeq-loss-type")
                .label("Loss Function")
                .options(loss_type_options)
                .selected(&config.loss_type)
                .is_open(ui_state.loss_type_open)
                .disabled(disabled)
                .theme(theme.select_theme.clone());

            if let Some(ref handler) = on_loss_type_toggle_rc {
                let h = handler.clone();
                loss_type_select = loss_type_select.on_toggle(move |open, w, cx| h(open, w, cx));
            }

            if let Some(ref handler) = on_loss_type_change_rc {
                let h = handler.clone();
                loss_type_select =
                    loss_type_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
            }

            goals_content = goals_content.child(loss_type_select);

            // Target Curve dropdown - options depend on optimization type
            let target_curve_options: Vec<SelectOption> = match optimization_type {
                OptimizationType::Headphone => {
                    // Headphone: Harman curves
                    HEADPHONE_TARGET_CURVE_OPTIONS
                        .iter()
                        .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                        .collect()
                }
                OptimizationType::Speaker => {
                    // Speaker: flat, custom, plus available spinorama curves
                    let mut options: Vec<SelectOption> = SPEAKER_TARGET_CURVE_OPTIONS
                        .iter()
                        .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                        .collect();

                    // Add available spinorama curves
                    for (val, lbl) in SPINORAMA_CURVE_OPTIONS {
                        if available_spinorama_curves.iter().any(|c| c == *val) {
                            options.push(SelectOption::new(*val, *lbl));
                        }
                    }

                    options
                }
            };

            let mut target_curve_select = Select::new("autoeq-target-curve")
                .label("Target Curve")
                .options(target_curve_options)
                .selected(&config.target_curve)
                .is_open(ui_state.target_curve_open) // Assuming target_curve_open exists in ui_state
                .disabled(disabled)
                .theme(theme.select_theme.clone());

            if let Some(ref handler) = on_target_curve_change_rc {
                let h = handler.clone();
                target_curve_select =
                    target_curve_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
            }

            if let Some(ref handler) = on_target_curve_toggle_rc {
                let h = handler.clone();
                target_curve_select =
                    target_curve_select.on_toggle(move |open, w, cx| h(open, w, cx));
            }

            goals_content = goals_content.child(target_curve_select);

            form = form.child(Card::new().content(goals_content));
        }

        // ========================================
        // EQ Design Parameters Section
        // ========================================
        if show_eq_design {
            let mut eq_design_content = VStack::new().spacing(StackSpacing::Sm);

            // Header
            eq_design_content = eq_design_content.child(
                VStack::new()
                    .spacing(StackSpacing::None)
                    .child(
                        Text::new("EQ Design Parameters")
                            .size(TextSize::Sm)
                            .weight(TextWeight::Semibold)
                            .color(theme.header_color),
                    )
                    .child(
                        Text::new("Configure filter characteristics and frequency ranges")
                            .size(TextSize::Xs)
                            .color(theme.description_color),
                    ),
            );

            // EQ Mode dropdown
            let opt_mode_options: Vec<SelectOption> = OPT_MODE_OPTIONS
                .iter()
                .filter(|(val, _)| {
                    if let Some(allowed) = &self.allowed_opt_modes {
                        allowed.contains(&val.to_string())
                    } else {
                        true
                    }
                })
                .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                .collect();

            let mut opt_mode_select = Select::new("autoeq-opt-mode")
                .label("EQ Mode")
                .options(opt_mode_options)
                .selected(&config.opt_mode)
                .is_open(ui_state.opt_mode_open)
                .disabled(disabled)
                .theme(theme.select_theme.clone());

            if let Some(ref handler) = on_opt_mode_toggle_rc {
                let h = handler.clone();
                opt_mode_select = opt_mode_select.on_toggle(move |open, w, cx| h(open, w, cx));
            }

            if let Some(ref handler) = on_opt_mode_change_rc {
                let h = handler.clone();
                opt_mode_select =
                    opt_mode_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
            }

            eq_design_content = eq_design_content.child(opt_mode_select);

            // Conditional fields based on Mode
            let is_fir = config.opt_mode == "fir" || config.opt_mode == "mixed";
            let is_iir = config.opt_mode == "iir" || config.opt_mode == "mixed";

            if is_fir {
                // FIR Taps and Phase
                let mut fir_taps_input = NumberInput::new("autoeq-fir-taps")
                    .value(config.fir_taps as f64)
                    .min(ParamLimits::FIR_TAPS.min)
                    .max(ParamLimits::FIR_TAPS.max)
                    .step(ParamLimits::FIR_TAPS.step)
                    .decimals(0)
                    .label("FIR Taps")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_fir_taps_change_rc {
                    let h = handler.clone();
                    fir_taps_input =
                        fir_taps_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                }

                // FIR Phase dropdown
                let fir_phase_options: Vec<SelectOption> = FIR_PHASE_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut fir_phase_select = Select::new("autoeq-fir-phase")
                    .label("Phase")
                    .options(fir_phase_options)
                    .selected(&config.fir_phase)
                    .is_open(ui_state.fir_phase_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_fir_phase_toggle_rc {
                    let h = handler.clone();
                    fir_phase_select =
                        fir_phase_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_fir_phase_change_rc {
                    let h = handler.clone();
                    fir_phase_select =
                        fir_phase_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
                }

                eq_design_content = eq_design_content.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(fir_taps_input)
                        .child(fir_phase_select),
                );
            }

            // Common params (Sample Rate) + Filters (if IIR)
            let mut sample_rate_input = NumberInput::new("autoeq-sample-rate")
                .value(config.sample_rate as f64)
                .min(ParamLimits::SAMPLE_RATE.min)
                .max(ParamLimits::SAMPLE_RATE.max)
                .step(ParamLimits::SAMPLE_RATE.step)
                .decimals(0)
                .label("Sample Rate")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_sample_rate_change_rc {
                let h = handler.clone();
                sample_rate_input =
                    sample_rate_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
            }

            if is_iir {
                let mut num_filters_input = NumberInput::new("autoeq-num-filters")
                    .value(config.num_filters as f64)
                    .min(ParamLimits::NUM_FILTERS.min)
                    .max(ParamLimits::NUM_FILTERS.max)
                    .step(ParamLimits::NUM_FILTERS.step)
                    .decimals(0)
                    .label("Filters")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_num_filters_change_rc {
                    let h = handler.clone();
                    num_filters_input =
                        num_filters_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                }

                eq_design_content = eq_design_content.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(num_filters_input)
                        .child(sample_rate_input),
                );
            } else {
                // FIR only - just show sample rate
                eq_design_content = eq_design_content.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(sample_rate_input),
                );
            }

            // dB Range row
            let mut min_db_input = NumberInput::new("autoeq-min-db")
                .value(config.min_db)
                .min(ParamLimits::DB.min)
                .max(ParamLimits::DB.max)
                .step(ParamLimits::DB.step)
                .decimals(1)
                .label("Min dB")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_min_db_change_rc {
                let h = handler.clone();
                min_db_input = min_db_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            let mut max_db_input = NumberInput::new("autoeq-max-db")
                .value(config.max_db)
                .min(ParamLimits::DB.min)
                .max(ParamLimits::DB.max)
                .step(ParamLimits::DB.step)
                .decimals(1)
                .label("Max dB")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_max_db_change_rc {
                let h = handler.clone();
                max_db_input = max_db_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            eq_design_content = eq_design_content.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(min_db_input)
                    .child(max_db_input),
            );

            // Q Range row (IIR only)
            if is_iir {
                let mut min_q_input = NumberInput::new("autoeq-min-q")
                    .value(config.min_q)
                    .min(ParamLimits::Q.min)
                    .max(ParamLimits::Q.max)
                    .step(ParamLimits::Q.step)
                    .decimals(1)
                    .label("Min Q")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_min_q_change_rc {
                    let h = handler.clone();
                    min_q_input = min_q_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                let mut max_q_input = NumberInput::new("autoeq-max-q")
                    .value(config.max_q)
                    .min(ParamLimits::Q.min)
                    .max(ParamLimits::Q.max)
                    .step(ParamLimits::Q.step)
                    .decimals(1)
                    .label("Max Q")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_max_q_change_rc {
                    let h = handler.clone();
                    max_q_input = max_q_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                eq_design_content = eq_design_content.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(min_q_input)
                        .child(max_q_input),
                );
            }

            // Frequency Range row
            let mut min_freq_input = NumberInput::new("autoeq-min-freq")
                .value(config.min_freq)
                .min(ParamLimits::FREQUENCY.min)
                .max(ParamLimits::FREQUENCY.max)
                .step(ParamLimits::FREQUENCY.step)
                .decimals(0)
                .label("Min Freq")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_min_freq_change_rc {
                let h = handler.clone();
                min_freq_input = min_freq_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            let mut max_freq_input = NumberInput::new("autoeq-max-freq")
                .value(config.max_freq)
                .min(ParamLimits::FREQUENCY.min)
                .max(ParamLimits::FREQUENCY.max)
                .step(ParamLimits::FREQUENCY.step)
                .decimals(0)
                .label("Max Freq")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_max_freq_change_rc {
                let h = handler.clone();
                max_freq_input = max_freq_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            eq_design_content = eq_design_content.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(min_freq_input)
                    .child(max_freq_input),
            );

            // PEQ Model dropdown (IIR only)
            if is_iir {
                let peq_model_options: Vec<SelectOption> = PEQ_MODEL_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut peq_model_select = Select::new("autoeq-peq-model")
                    .label("PEQ Model")
                    .options(peq_model_options)
                    .selected(&config.peq_model)
                    .is_open(ui_state.peq_model_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_peq_model_toggle_rc {
                    let h = handler.clone();
                    peq_model_select =
                        peq_model_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_peq_model_change_rc {
                    let h = handler.clone();
                    peq_model_select =
                        peq_model_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
                }

                eq_design_content = eq_design_content.child(peq_model_select);

                // Spacing constraint row
                let mut spacing_weight_input = NumberInput::new("autoeq-spacing-weight")
                    .value(config.spacing_weight)
                    .min(ParamLimits::SPACING_WEIGHT.min)
                    .max(ParamLimits::SPACING_WEIGHT.max)
                    .step(ParamLimits::SPACING_WEIGHT.step)
                    .decimals(1)
                    .label("Spacing Weight")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_spacing_weight_change_rc {
                    let h = handler.clone();
                    spacing_weight_input =
                        spacing_weight_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                let mut min_spacing_oct_input = NumberInput::new("autoeq-min-spacing-oct")
                    .value(config.min_spacing_oct)
                    .min(ParamLimits::MIN_SPACING_OCT.min)
                    .max(ParamLimits::MIN_SPACING_OCT.max)
                    .step(ParamLimits::MIN_SPACING_OCT.step)
                    .decimals(2)
                    .label("Min Spacing (oct)")
                    .size(NumberInputSize::Sm)
                    .width(120.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_min_spacing_oct_change_rc {
                    let h = handler.clone();
                    min_spacing_oct_input =
                        min_spacing_oct_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                eq_design_content = eq_design_content.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(spacing_weight_input)
                        .child(min_spacing_oct_input),
                );
            }

            form = form.child(Card::new().content(eq_design_content));
        }

        // ========================================
        // Optimization Fine Tuning Section
        // ========================================
        if show_optimization_tuning {
            let mut opt_tuning_content = VStack::new().spacing(StackSpacing::Sm);

            // Header
            opt_tuning_content = opt_tuning_content.child(
                VStack::new()
                    .spacing(StackSpacing::None)
                    .child(
                        Text::new("Optimization Fine Tuning")
                            .size(TextSize::Sm)
                            .weight(TextWeight::Semibold)
                            .color(theme.header_color),
                    )
                    .child(
                        Text::new("Advanced optimization algorithm settings")
                            .size(TextSize::Xs)
                            .color(theme.description_color),
                    ),
            );

            // Algorithm dropdown
            let algo_options: Vec<SelectOption> = ALGORITHM_OPTIONS
                .iter()
                .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                .collect();

            let mut algo_select = Select::new("autoeq-algo")
                .label("Algorithm")
                .options(algo_options)
                .selected(&config.algo)
                .is_open(ui_state.algo_open)
                .disabled(disabled)
                .theme(theme.select_theme.clone());

            if let Some(ref handler) = on_algo_toggle_rc {
                let h = handler.clone();
                algo_select = algo_select.on_toggle(move |open, w, cx| h(open, w, cx));
            }

            if let Some(ref handler) = on_algo_change_rc {
                let h = handler.clone();
                algo_select = algo_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
            }

            opt_tuning_content = opt_tuning_content.child(algo_select);

            // Population and MaxEval row
            let mut population_input = NumberInput::new("autoeq-population")
                .value(config.population as f64)
                .min(ParamLimits::POPULATION.min)
                .max(ParamLimits::POPULATION.max)
                .step(ParamLimits::POPULATION.step)
                .decimals(0)
                .label("Population")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_population_change_rc {
                let h = handler.clone();
                population_input =
                    population_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
            }

            let mut maxeval_input = NumberInput::new("autoeq-maxeval")
                .value(config.maxeval as f64)
                .min(ParamLimits::MAXEVAL.min)
                .max(ParamLimits::MAXEVAL.max)
                .step(ParamLimits::MAXEVAL.step)
                .decimals(0)
                .label("Max Evals")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_maxeval_change_rc {
                let h = handler.clone();
                maxeval_input =
                    maxeval_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
            }

            opt_tuning_content = opt_tuning_content.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(population_input)
                    .child(maxeval_input),
            );

            // Tolerance row
            let mut tolerance_input = NumberInput::new("autoeq-tolerance")
                .value(config.tolerance)
                .min(ParamLimits::TOLERANCE.min)
                .max(ParamLimits::TOLERANCE.max)
                .step(ParamLimits::TOLERANCE.step)
                .decimals(6)
                .label("Tolerance")
                .size(NumberInputSize::Sm)
                .width(120.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_tolerance_change_rc {
                let h = handler.clone();
                tolerance_input = tolerance_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            let mut atolerance_input = NumberInput::new("autoeq-atolerance")
                .value(config.atolerance)
                .min(ParamLimits::TOLERANCE.min)
                .max(ParamLimits::TOLERANCE.max)
                .step(ParamLimits::TOLERANCE.step)
                .decimals(6)
                .label("Abs Tolerance")
                .size(NumberInputSize::Sm)
                .width(120.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_atolerance_change_rc {
                let h = handler.clone();
                atolerance_input = atolerance_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            opt_tuning_content = opt_tuning_content.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(tolerance_input)
                    .child(atolerance_input),
            );

            // DE-specific settings (only show when DE algorithm selected)
            if config.algo.contains("de") || config.algo.contains("mh:") {
                // Show params for DE and Metaheuristics
                // Note: Not all MH algos use DE params but usually population/maxeval are common.
                // DE strategy is specific to DE.

                if config.algo.contains(":de") {
                    // DE Strategy dropdown
                    let strategy_options: Vec<SelectOption> = DE_STRATEGY_OPTIONS
                        .iter()
                        .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                        .collect();

                    let mut strategy_select = Select::new("autoeq-strategy")
                        .label("DE Strategy")
                        .options(strategy_options)
                        .selected(&config.strategy)
                        .is_open(ui_state.strategy_open)
                        .disabled(disabled)
                        .theme(theme.select_theme.clone());

                    if let Some(ref handler) = on_strategy_toggle_rc {
                        let h = handler.clone();
                        strategy_select =
                            strategy_select.on_toggle(move |open, w, cx| h(open, w, cx));
                    }

                    if let Some(ref handler) = on_strategy_change_rc {
                        let h = handler.clone();
                        strategy_select =
                            strategy_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
                    }

                    opt_tuning_content = opt_tuning_content.child(strategy_select);

                    // DE F and CR row
                    let mut de_f_input = NumberInput::new("autoeq-de-f")
                        .value(config.de_f)
                        .min(ParamLimits::DE_FACTOR.min)
                        .max(ParamLimits::DE_FACTOR.max)
                        .step(ParamLimits::DE_FACTOR.step)
                        .decimals(1)
                        .label("Mutation (F)")
                        .size(NumberInputSize::Sm)
                        .width(100.0)
                        .disabled(disabled)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref handler) = on_de_f_change_rc {
                        let h = handler.clone();
                        de_f_input = de_f_input.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    let mut de_cr_input = NumberInput::new("autoeq-de-cr")
                        .value(config.de_cr)
                        .min(ParamLimits::DE_CR.min)
                        .max(ParamLimits::DE_CR.max)
                        .step(ParamLimits::DE_CR.step)
                        .decimals(1)
                        .label("Recomb (CR)")
                        .size(NumberInputSize::Sm)
                        .width(100.0)
                        .disabled(disabled)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref handler) = on_de_cr_change_rc {
                        let h = handler.clone();
                        de_cr_input = de_cr_input.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    opt_tuning_content = opt_tuning_content.child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(de_f_input)
                            .child(de_cr_input),
                    );
                }
            }

            // Local Refinement toggle
            let toggle_theme = ToggleTheme {
                checked_bg: theme.toggle_checked_bg,
                unchecked_bg: theme.toggle_unchecked_bg,
                knob: theme.toggle_knob,
                knob_on_checked: theme.card_bg,
                track_border: theme.border,
                label: theme.label_color,
                accent: theme.accent,
                accent_muted: theme.accent,
                success: theme.accent,
                border: theme.border,
                text_on_accent: theme.toggle_knob,
                text_muted: theme.text_muted,
                text_primary: theme.header_color,
                surface_hover: theme.toggle_unchecked_bg,
                background: theme.card_bg,
            };

            let mut refine_toggle = Toggle::new("autoeq-refine")
                .size(ToggleSize::Sm)
                .checked(config.refine)
                .theme(toggle_theme.clone());

            if let Some(ref handler) = on_refine_change_rc {
                let h = handler.clone();
                refine_toggle = refine_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            opt_tuning_content = opt_tuning_content.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        Text::new("Local Refinement")
                            .size(TextSize::Xs)
                            .color(theme.label_color),
                    )
                    .child(refine_toggle),
            );

            // Local algorithm dropdown (only when refine is enabled)
            if config.refine {
                let local_algo_options: Vec<SelectOption> = LOCAL_ALGO_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut local_algo_select = Select::new("autoeq-local-algo")
                    .label("Local Algo")
                    .options(local_algo_options)
                    .selected(&config.local_algo)
                    .is_open(ui_state.local_algo_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_local_algo_toggle_rc {
                    let h = handler.clone();
                    local_algo_select =
                        local_algo_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_local_algo_change_rc {
                    let h = handler.clone();
                    local_algo_select =
                        local_algo_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
                }

                opt_tuning_content = opt_tuning_content.child(local_algo_select);
            }

            // Smoothing toggle
            let mut smooth_toggle = Toggle::new("autoeq-smooth")
                .size(ToggleSize::Sm)
                .checked(config.smooth)
                .theme(toggle_theme);

            if let Some(ref handler) = on_smooth_change_rc {
                let h = handler.clone();
                smooth_toggle = smooth_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            opt_tuning_content = opt_tuning_content.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        Text::new("Smoothing")
                            .size(TextSize::Xs)
                            .color(theme.label_color),
                    )
                    .child(smooth_toggle),
            );

            // Smoothing window size (only when smooth is enabled)
            if config.smooth {
                let mut smooth_n_input = NumberInput::new("autoeq-smooth-n")
                    .value(config.smooth_n as f64)
                    .min(ParamLimits::SMOOTH_N.min)
                    .max(ParamLimits::SMOOTH_N.max)
                    .step(ParamLimits::SMOOTH_N.step)
                    .decimals(0)
                    .label("Smooth Window")
                    .size(NumberInputSize::Sm)
                    .width(120.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_smooth_n_change_rc {
                    let h = handler.clone();
                    smooth_n_input =
                        smooth_n_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                }

                opt_tuning_content = opt_tuning_content.child(smooth_n_input);
            }

            form = form.child(Card::new().content(opt_tuning_content));
        }

        div().id(id).child(form)
    }
}
