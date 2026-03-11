//! UI state for AutoEQ form dropdowns.

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

    /// Tilt type dropdown open state
    pub tilt_type_open: bool,
    /// Excursion filter type dropdown open state
    pub excursion_filter_type_open: bool,
    /// Multi-seat strategy dropdown open state
    pub multi_seat_strategy_open: bool,

    // v2 dropdown states
    /// Mixed crossover type dropdown open state
    pub mixed_crossover_type_open: bool,
    /// Mixed FIR band dropdown open state
    pub mixed_fir_band_open: bool,
    /// VoG reference channel dropdown open state
    pub vog_reference_channel_open: bool,
}
