//! AutoEQ form component - struct definition and builder methods.

use gpui::*;

use crate::config::AutoEqConfig;
use crate::constants::OptimizationType;
use crate::theme::AutoEqFormTheme;
use crate::ui_state::AutoEqFormUiState;

/// Callback type for string parameter changes
pub(crate) type StringCallback = Box<dyn Fn(&str, &mut Window, &mut App) + 'static>;
/// Callback type for f64 parameter changes
pub(crate) type F64Callback = Box<dyn Fn(f64, &mut Window, &mut App) + 'static>;
/// Callback type for usize parameter changes
pub(crate) type UsizeCallback = Box<dyn Fn(usize, &mut Window, &mut App) + 'static>;
/// Callback type for bool parameter changes
pub(crate) type BoolCallback = Box<dyn Fn(bool, &mut Window, &mut App) + 'static>;
/// Callback type for dropdown toggle
pub(crate) type ToggleCallback = Box<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// Layout mode for the AutoEQ form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoEqLayoutMode {
    /// Original card-based layout (headphone EQ, spinorama EQ)
    #[default]
    Default,
    /// Room EQ layout: 3 sections (Optimisation Mode, Room Configuration, Optimiser Configuration)
    RoomEq,
}

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
    pub(crate) layout_mode: AutoEqLayoutMode,
    pub(crate) id: ElementId,
    pub(crate) config: AutoEqConfig,
    pub(crate) ui_state: AutoEqFormUiState,
    pub(crate) disabled: bool,
    pub(crate) show_goals: bool,
    pub(crate) show_eq_design: bool,
    pub(crate) show_optimization_tuning: bool,
    pub(crate) theme: Option<AutoEqFormTheme>,
    pub(crate) allowed_opt_modes: Option<Vec<String>>,
    /// Type of optimization (Speaker or Headphone) - affects which options are shown
    pub(crate) optimization_type: OptimizationType,
    /// Available spinorama curves for speaker mode (e.g., ["ON", "LW", "PIR"])
    pub(crate) available_spinorama_curves: Vec<String>,

    // Visibility flags for hiding fields not relevant to certain contexts
    /// Hide DE-specific parameters (strategy, mutation F, crossover CR)
    pub(crate) hide_de_params: bool,
    /// Hide smoothing toggle and window size
    pub(crate) hide_smoothing: bool,
    /// Hide spacing weight and min spacing octaves
    pub(crate) hide_spacing: bool,
    /// Hide tolerance and absolute tolerance
    pub(crate) hide_tolerance: bool,
    /// Hide sample rate input
    pub(crate) hide_sample_rate: bool,
    /// Hide phase alignment in Advanced System Optimization
    pub(crate) hide_phase_alignment: bool,
    /// Hide multi-seat in Advanced System Optimization
    pub(crate) hide_multi_seat: bool,
    /// Hide the "Scenario A" subtitle text
    pub(crate) hide_scenario_a_text: bool,
    /// Hide room-specific sections (Advanced Room Correction, System Optimization, Advanced Tuning)
    pub(crate) hide_room_sections: bool,
    /// Available width in pixels for responsive layout
    pub(crate) available_width: f32,

    // EQ Design callbacks
    pub(crate) on_opt_mode_change: Option<StringCallback>,
    pub(crate) on_opt_mode_toggle: Option<ToggleCallback>,
    pub(crate) on_fir_taps_change: Option<UsizeCallback>,
    pub(crate) on_fir_phase_change: Option<StringCallback>,
    pub(crate) on_fir_phase_toggle: Option<ToggleCallback>,
    pub(crate) on_num_filters_change: Option<UsizeCallback>,
    pub(crate) on_sample_rate_change: Option<UsizeCallback>,
    pub(crate) on_min_db_change: Option<F64Callback>,
    pub(crate) on_max_db_change: Option<F64Callback>,
    pub(crate) on_min_q_change: Option<F64Callback>,
    pub(crate) on_max_q_change: Option<F64Callback>,
    pub(crate) on_min_freq_change: Option<F64Callback>,
    pub(crate) on_max_freq_change: Option<F64Callback>,
    pub(crate) on_peq_model_change: Option<StringCallback>,
    pub(crate) on_peq_model_toggle: Option<ToggleCallback>,
    pub(crate) on_spacing_weight_change: Option<F64Callback>,
    pub(crate) on_min_spacing_oct_change: Option<F64Callback>,

    // Optimization callbacks
    pub(crate) on_algo_change: Option<StringCallback>,
    pub(crate) on_algo_toggle: Option<ToggleCallback>,
    pub(crate) on_population_change: Option<UsizeCallback>,
    pub(crate) on_maxeval_change: Option<UsizeCallback>,
    pub(crate) on_tolerance_change: Option<F64Callback>,
    pub(crate) on_atolerance_change: Option<F64Callback>,
    pub(crate) on_de_f_change: Option<F64Callback>,
    pub(crate) on_de_cr_change: Option<F64Callback>,
    pub(crate) on_strategy_change: Option<StringCallback>,
    pub(crate) on_strategy_toggle: Option<ToggleCallback>,
    pub(crate) on_refine_change: Option<BoolCallback>,
    pub(crate) on_local_algo_change: Option<StringCallback>,
    pub(crate) on_local_algo_toggle: Option<ToggleCallback>,
    pub(crate) on_smooth_change: Option<BoolCallback>,
    pub(crate) on_smooth_n_change: Option<UsizeCallback>,
    pub(crate) on_psychoacoustic_change: Option<BoolCallback>,
    pub(crate) on_asymmetric_loss_change: Option<BoolCallback>,

    // Goals callbacks
    pub(crate) on_loss_type_change: Option<StringCallback>,
    pub(crate) on_loss_type_toggle: Option<ToggleCallback>,
    pub(crate) on_target_curve_change: Option<StringCallback>,
    pub(crate) on_target_curve_toggle: Option<ToggleCallback>,
    pub(crate) on_system_type_change: Option<StringCallback>,
    pub(crate) on_system_type_toggle: Option<ToggleCallback>,

    // Advanced callbacks
    pub(crate) on_use_target_tilt_change: Option<BoolCallback>,
    pub(crate) on_tilt_type_change: Option<StringCallback>,
    pub(crate) on_tilt_type_toggle: Option<ToggleCallback>,
    pub(crate) on_tilt_slope_change: Option<F64Callback>,
    pub(crate) on_tilt_reference_freq_change: Option<F64Callback>,
    pub(crate) on_tilt_bass_shelf_db_change: Option<F64Callback>,
    pub(crate) on_tilt_bass_shelf_freq_change: Option<F64Callback>,

    pub(crate) on_use_excursion_protection_change: Option<BoolCallback>,
    pub(crate) on_excursion_auto_detect_f3_change: Option<BoolCallback>,
    pub(crate) on_excursion_manual_f3_change: Option<F64Callback>,
    pub(crate) on_excursion_filter_order_change: Option<UsizeCallback>,
    pub(crate) on_excursion_filter_type_change: Option<StringCallback>,
    pub(crate) on_excursion_filter_type_toggle: Option<ToggleCallback>,
    pub(crate) on_excursion_margin_octaves_change: Option<F64Callback>,

    pub(crate) on_use_schroeder_split_change: Option<BoolCallback>,
    pub(crate) on_schroeder_freq_change: Option<F64Callback>,
    pub(crate) on_schroeder_low_max_q_change: Option<F64Callback>,
    pub(crate) on_schroeder_low_allow_boost_change: Option<BoolCallback>,
    pub(crate) on_schroeder_high_max_q_change: Option<F64Callback>,
    pub(crate) on_schroeder_high_shelving_only_change: Option<BoolCallback>,

    pub(crate) on_use_phase_alignment_change: Option<BoolCallback>,
    pub(crate) on_phase_min_freq_change: Option<F64Callback>,
    pub(crate) on_phase_max_freq_change: Option<F64Callback>,
    pub(crate) on_phase_optimize_polarity_change: Option<BoolCallback>,
    pub(crate) on_phase_max_delay_ms_change: Option<F64Callback>,

    pub(crate) on_use_multi_seat_change: Option<BoolCallback>,
    pub(crate) on_multi_seat_strategy_change: Option<StringCallback>,
    pub(crate) on_multi_seat_strategy_toggle: Option<ToggleCallback>,
    pub(crate) on_multi_seat_primary_seat_change: Option<UsizeCallback>,
    pub(crate) on_multi_seat_max_deviation_db_change: Option<F64Callback>,

    // v2 callbacks
    pub(crate) on_allow_delay_change: Option<BoolCallback>,
    pub(crate) on_seed_enabled_change: Option<BoolCallback>,
    pub(crate) on_seed_change: Option<UsizeCallback>,
    pub(crate) on_gd_opt_enabled_change: Option<BoolCallback>,
    pub(crate) on_gd_opt_target_ms_change: Option<F64Callback>,
    pub(crate) on_vog_enabled_change: Option<BoolCallback>,
    pub(crate) on_vog_reference_channel_change: Option<StringCallback>,
    pub(crate) on_vog_reference_channel_toggle: Option<ToggleCallback>,
    pub(crate) on_broadband_target_matching_change: Option<BoolCallback>,
    pub(crate) on_mixed_crossover_freq_change: Option<F64Callback>,
    pub(crate) on_mixed_crossover_type_change: Option<StringCallback>,
    pub(crate) on_mixed_crossover_type_toggle: Option<ToggleCallback>,
    pub(crate) on_mixed_fir_band_change: Option<StringCallback>,
    pub(crate) on_mixed_fir_band_toggle: Option<ToggleCallback>,
}

impl AutoEqForm {
    /// Create a new AutoEQ form
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            layout_mode: AutoEqLayoutMode::Default,
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
            hide_de_params: false,
            hide_smoothing: false,
            hide_spacing: false,
            hide_tolerance: false,
            hide_sample_rate: false,
            hide_phase_alignment: false,
            hide_multi_seat: false,
            hide_scenario_a_text: false,
            hide_room_sections: false,
            available_width: 0.0,
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
            on_psychoacoustic_change: None,
            on_asymmetric_loss_change: None,
            on_use_target_tilt_change: None,
            on_tilt_type_change: None,
            on_tilt_type_toggle: None,
            on_tilt_slope_change: None,
            on_tilt_reference_freq_change: None,
            on_tilt_bass_shelf_db_change: None,
            on_tilt_bass_shelf_freq_change: None,
            on_use_excursion_protection_change: None,
            on_excursion_auto_detect_f3_change: None,
            on_excursion_manual_f3_change: None,
            on_excursion_filter_order_change: None,
            on_excursion_filter_type_change: None,
            on_excursion_filter_type_toggle: None,
            on_excursion_margin_octaves_change: None,
            on_use_schroeder_split_change: None,
            on_schroeder_freq_change: None,
            on_schroeder_low_max_q_change: None,
            on_schroeder_low_allow_boost_change: None,
            on_schroeder_high_max_q_change: None,
            on_schroeder_high_shelving_only_change: None,
            on_use_phase_alignment_change: None,
            on_phase_min_freq_change: None,
            on_phase_max_freq_change: None,
            on_phase_optimize_polarity_change: None,
            on_phase_max_delay_ms_change: None,
            on_use_multi_seat_change: None,
            on_multi_seat_strategy_change: None,
            on_multi_seat_strategy_toggle: None,
            on_multi_seat_primary_seat_change: None,
            on_multi_seat_max_deviation_db_change: None,
            on_allow_delay_change: None,
            on_seed_enabled_change: None,
            on_seed_change: None,
            on_gd_opt_enabled_change: None,
            on_gd_opt_target_ms_change: None,
            on_vog_enabled_change: None,
            on_vog_reference_channel_change: None,
            on_vog_reference_channel_toggle: None,
            on_broadband_target_matching_change: None,
            on_mixed_crossover_freq_change: None,
            on_mixed_crossover_type_change: None,
            on_mixed_crossover_type_toggle: None,
            on_mixed_fir_band_change: None,
            on_mixed_fir_band_toggle: None,
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
    pub fn optimization_type(mut self, opt_type: OptimizationType) -> Self {
        self.optimization_type = opt_type;
        self
    }

    /// Set available spinorama curves for speaker mode
    pub fn available_spinorama_curves(mut self, curves: Vec<String>) -> Self {
        self.available_spinorama_curves = curves;
        self
    }

    /// Hide DE-specific parameters (strategy, mutation F, crossover CR)
    pub fn hide_de_params(mut self, hide: bool) -> Self {
        self.hide_de_params = hide;
        self
    }

    /// Hide smoothing toggle and window size
    pub fn hide_smoothing(mut self, hide: bool) -> Self {
        self.hide_smoothing = hide;
        self
    }

    /// Hide spacing weight and min spacing octaves
    pub fn hide_spacing(mut self, hide: bool) -> Self {
        self.hide_spacing = hide;
        self
    }

    /// Hide tolerance and absolute tolerance
    pub fn hide_tolerance(mut self, hide: bool) -> Self {
        self.hide_tolerance = hide;
        self
    }

    /// Hide sample rate input
    pub fn hide_sample_rate(mut self, hide: bool) -> Self {
        self.hide_sample_rate = hide;
        self
    }

    /// Hide phase alignment in Advanced System Optimization section
    pub fn hide_phase_alignment(mut self, hide: bool) -> Self {
        self.hide_phase_alignment = hide;
        self
    }

    /// Hide multi-seat in Advanced System Optimization section
    pub fn hide_multi_seat(mut self, hide: bool) -> Self {
        self.hide_multi_seat = hide;
        self
    }

    /// Hide the "Scenario A" subtitle text
    pub fn hide_scenario_a_text(mut self, hide: bool) -> Self {
        self.hide_scenario_a_text = hide;
        self
    }

    /// Hide room-specific sections (Advanced Room Correction, System Optimization, Advanced Tuning)
    pub fn hide_room_sections(mut self, hide: bool) -> Self {
        self.hide_room_sections = hide;
        self
    }

    /// Set available width for responsive layout
    pub fn available_width(mut self, width: f32) -> Self {
        self.available_width = width;
        self
    }

    /// Set the layout mode (Default or RoomEq)
    pub fn layout_mode(mut self, mode: AutoEqLayoutMode) -> Self {
        self.layout_mode = mode;
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

    /// Set psychoacoustic toggle handler
    pub fn on_psychoacoustic_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_psychoacoustic_change = Some(Box::new(handler));
        self
    }

    /// Set asymmetric loss toggle handler
    pub fn on_asymmetric_loss_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_asymmetric_loss_change = Some(Box::new(handler));
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

    // Advanced Room Correction (Scenario B) callbacks

    pub fn on_use_target_tilt_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_use_target_tilt_change = Some(Box::new(handler));
        self
    }

    pub fn on_tilt_type_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tilt_type_change = Some(Box::new(handler));
        self
    }

    pub fn on_tilt_type_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tilt_type_toggle = Some(Box::new(handler));
        self
    }

    pub fn on_tilt_slope_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tilt_slope_change = Some(Box::new(handler));
        self
    }

    pub fn on_tilt_reference_freq_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tilt_reference_freq_change = Some(Box::new(handler));
        self
    }

    pub fn on_tilt_bass_shelf_db_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tilt_bass_shelf_db_change = Some(Box::new(handler));
        self
    }

    pub fn on_tilt_bass_shelf_freq_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tilt_bass_shelf_freq_change = Some(Box::new(handler));
        self
    }

    pub fn on_use_excursion_protection_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_use_excursion_protection_change = Some(Box::new(handler));
        self
    }

    pub fn on_excursion_auto_detect_f3_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_excursion_auto_detect_f3_change = Some(Box::new(handler));
        self
    }

    pub fn on_excursion_manual_f3_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_excursion_manual_f3_change = Some(Box::new(handler));
        self
    }

    pub fn on_excursion_filter_order_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_excursion_filter_order_change = Some(Box::new(handler));
        self
    }

    pub fn on_excursion_filter_type_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_excursion_filter_type_change = Some(Box::new(handler));
        self
    }

    pub fn on_excursion_filter_type_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_excursion_filter_type_toggle = Some(Box::new(handler));
        self
    }

    pub fn on_excursion_margin_octaves_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_excursion_margin_octaves_change = Some(Box::new(handler));
        self
    }

    pub fn on_use_schroeder_split_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_use_schroeder_split_change = Some(Box::new(handler));
        self
    }

    pub fn on_schroeder_freq_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_schroeder_freq_change = Some(Box::new(handler));
        self
    }

    pub fn on_schroeder_low_max_q_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_schroeder_low_max_q_change = Some(Box::new(handler));
        self
    }

    pub fn on_schroeder_low_allow_boost_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_schroeder_low_allow_boost_change = Some(Box::new(handler));
        self
    }

    pub fn on_schroeder_high_max_q_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_schroeder_high_max_q_change = Some(Box::new(handler));
        self
    }

    pub fn on_schroeder_high_shelving_only_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_schroeder_high_shelving_only_change = Some(Box::new(handler));
        self
    }

    // Advanced System Optimization (Scenario A) callbacks

    pub fn on_use_phase_alignment_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_use_phase_alignment_change = Some(Box::new(handler));
        self
    }

    pub fn on_phase_min_freq_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_phase_min_freq_change = Some(Box::new(handler));
        self
    }

    pub fn on_phase_max_freq_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_phase_max_freq_change = Some(Box::new(handler));
        self
    }

    pub fn on_phase_optimize_polarity_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_phase_optimize_polarity_change = Some(Box::new(handler));
        self
    }

    pub fn on_phase_max_delay_ms_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_phase_max_delay_ms_change = Some(Box::new(handler));
        self
    }

    pub fn on_use_multi_seat_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_use_multi_seat_change = Some(Box::new(handler));
        self
    }

    pub fn on_multi_seat_strategy_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_multi_seat_strategy_change = Some(Box::new(handler));
        self
    }

    pub fn on_multi_seat_strategy_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_multi_seat_strategy_toggle = Some(Box::new(handler));
        self
    }

    pub fn on_multi_seat_primary_seat_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_multi_seat_primary_seat_change = Some(Box::new(handler));
        self
    }

    pub fn on_multi_seat_max_deviation_db_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_multi_seat_max_deviation_db_change = Some(Box::new(handler));
        self
    }

    // v2 callbacks

    pub fn on_allow_delay_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_allow_delay_change = Some(Box::new(handler));
        self
    }

    pub fn on_seed_enabled_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_seed_enabled_change = Some(Box::new(handler));
        self
    }

    pub fn on_seed_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_seed_change = Some(Box::new(handler));
        self
    }

    pub fn on_gd_opt_enabled_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_gd_opt_enabled_change = Some(Box::new(handler));
        self
    }

    pub fn on_gd_opt_target_ms_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_gd_opt_target_ms_change = Some(Box::new(handler));
        self
    }

    pub fn on_vog_enabled_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_vog_enabled_change = Some(Box::new(handler));
        self
    }

    pub fn on_vog_reference_channel_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_vog_reference_channel_change = Some(Box::new(handler));
        self
    }

    pub fn on_vog_reference_channel_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_vog_reference_channel_toggle = Some(Box::new(handler));
        self
    }

    pub fn on_broadband_target_matching_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_broadband_target_matching_change = Some(Box::new(handler));
        self
    }

    pub fn on_mixed_crossover_freq_change(
        mut self,
        handler: impl Fn(f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mixed_crossover_freq_change = Some(Box::new(handler));
        self
    }

    pub fn on_mixed_crossover_type_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mixed_crossover_type_change = Some(Box::new(handler));
        self
    }

    pub fn on_mixed_crossover_type_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mixed_crossover_type_toggle = Some(Box::new(handler));
        self
    }

    pub fn on_mixed_fir_band_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mixed_fir_band_change = Some(Box::new(handler));
        self
    }

    pub fn on_mixed_fir_band_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mixed_fir_band_toggle = Some(Box::new(handler));
        self
    }
}
