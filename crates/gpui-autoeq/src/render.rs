//! RenderOnce implementation for the AutoEQ form.
//!
//! This is a direct port of the render implementation from gpui-ui-kit's autoeq module.
//! The render function is intentionally kept as a single large function to match the
//! original structure and avoid complex parameter threading through helper functions.

// Allow large render function - UI layout code is inherently verbose
#![allow(clippy::too_many_lines)]

use gpui::prelude::*;
use gpui::*;

use gpui_ui_kit::card::Card;
use gpui_ui_kit::number_input::{NumberInput, NumberInputSize};
use gpui_ui_kit::select::{Select, SelectOption};
use gpui_ui_kit::stack::{HStack, StackJustify, StackSpacing, VStack};
use gpui_ui_kit::text::{Text, TextSize, TextWeight};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::toggle::{Toggle, ToggleSize, ToggleTheme};

use crate::config::ParamLimits;
use crate::constants::*;
use crate::form::{AutoEqForm, AutoEqLayoutMode};
use crate::theme::AutoEqFormTheme;

impl RenderOnce for AutoEqForm {
    #[allow(clippy::cognitive_complexity)]
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
        let allowed_opt_modes = self.allowed_opt_modes;
        let hide_de_params = self.hide_de_params;
        let hide_smoothing = self.hide_smoothing;
        let hide_spacing = self.hide_spacing;
        let hide_tolerance = self.hide_tolerance;
        let hide_sample_rate = self.hide_sample_rate;
        let hide_phase_alignment = self.hide_phase_alignment;
        let hide_multi_seat = self.hide_multi_seat;
        let hide_scenario_a_text = self.hide_scenario_a_text;
        let hide_room_sections = self.hide_room_sections;
        let available_width = self.available_width;
        let layout_mode = self.layout_mode;

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
        let on_psychoacoustic_change_rc = self.on_psychoacoustic_change.map(std::rc::Rc::new);
        let on_asymmetric_loss_change_rc = self.on_asymmetric_loss_change.map(std::rc::Rc::new);
        let on_loss_type_change_rc = self.on_loss_type_change.map(std::rc::Rc::new);
        let on_loss_type_toggle_rc = self.on_loss_type_toggle.map(std::rc::Rc::new);
        let on_target_curve_change_rc = self.on_target_curve_change.map(std::rc::Rc::new);
        let on_target_curve_toggle_rc = self.on_target_curve_toggle.map(std::rc::Rc::new);
        let on_system_type_change_rc = self.on_system_type_change.map(std::rc::Rc::new);
        let on_system_type_toggle_rc = self.on_system_type_toggle.map(std::rc::Rc::new);

        // Advanced callbacks Rc
        let on_use_target_tilt_change_rc = self.on_use_target_tilt_change.map(std::rc::Rc::new);
        let on_tilt_type_change_rc = self.on_tilt_type_change.map(std::rc::Rc::new);
        let on_tilt_type_toggle_rc = self.on_tilt_type_toggle.map(std::rc::Rc::new);
        let on_tilt_slope_change_rc = self.on_tilt_slope_change.map(std::rc::Rc::new);
        let on_tilt_reference_freq_change_rc =
            self.on_tilt_reference_freq_change.map(std::rc::Rc::new);
        let on_tilt_bass_shelf_db_change_rc =
            self.on_tilt_bass_shelf_db_change.map(std::rc::Rc::new);
        let on_tilt_bass_shelf_freq_change_rc =
            self.on_tilt_bass_shelf_freq_change.map(std::rc::Rc::new);
        let on_use_excursion_protection_change_rc = self
            .on_use_excursion_protection_change
            .map(std::rc::Rc::new);
        let on_excursion_auto_detect_f3_change_rc = self
            .on_excursion_auto_detect_f3_change
            .map(std::rc::Rc::new);
        let on_excursion_manual_f3_change_rc =
            self.on_excursion_manual_f3_change.map(std::rc::Rc::new);
        let on_excursion_filter_order_change_rc =
            self.on_excursion_filter_order_change.map(std::rc::Rc::new);
        let on_excursion_filter_type_change_rc =
            self.on_excursion_filter_type_change.map(std::rc::Rc::new);
        let on_excursion_filter_type_toggle_rc =
            self.on_excursion_filter_type_toggle.map(std::rc::Rc::new);
        let on_excursion_margin_octaves_change_rc = self
            .on_excursion_margin_octaves_change
            .map(std::rc::Rc::new);
        let on_use_schroeder_split_change_rc =
            self.on_use_schroeder_split_change.map(std::rc::Rc::new);
        let on_schroeder_freq_change_rc = self.on_schroeder_freq_change.map(std::rc::Rc::new);
        let on_schroeder_low_max_q_change_rc =
            self.on_schroeder_low_max_q_change.map(std::rc::Rc::new);
        let on_schroeder_low_allow_boost_change_rc = self
            .on_schroeder_low_allow_boost_change
            .map(std::rc::Rc::new);
        let on_schroeder_high_max_q_change_rc =
            self.on_schroeder_high_max_q_change.map(std::rc::Rc::new);
        let on_schroeder_high_shelving_only_change_rc = self
            .on_schroeder_high_shelving_only_change
            .map(std::rc::Rc::new);
        let on_use_phase_alignment_change_rc =
            self.on_use_phase_alignment_change.map(std::rc::Rc::new);
        let on_phase_min_freq_change_rc = self.on_phase_min_freq_change.map(std::rc::Rc::new);
        let on_phase_max_freq_change_rc = self.on_phase_max_freq_change.map(std::rc::Rc::new);
        let on_phase_optimize_polarity_change_rc =
            self.on_phase_optimize_polarity_change.map(std::rc::Rc::new);
        let on_phase_max_delay_ms_change_rc =
            self.on_phase_max_delay_ms_change.map(std::rc::Rc::new);
        let on_use_multi_seat_change_rc = self.on_use_multi_seat_change.map(std::rc::Rc::new);
        let on_multi_seat_strategy_change_rc =
            self.on_multi_seat_strategy_change.map(std::rc::Rc::new);
        let on_multi_seat_strategy_toggle_rc =
            self.on_multi_seat_strategy_toggle.map(std::rc::Rc::new);
        let on_multi_seat_primary_seat_change_rc =
            self.on_multi_seat_primary_seat_change.map(std::rc::Rc::new);
        let on_multi_seat_max_deviation_db_change_rc = self
            .on_multi_seat_max_deviation_db_change
            .map(std::rc::Rc::new);

        // v2 callbacks Rc
        let on_allow_delay_change_rc = self.on_allow_delay_change.map(std::rc::Rc::new);
        let on_seed_enabled_change_rc = self.on_seed_enabled_change.map(std::rc::Rc::new);
        let on_seed_change_rc = self.on_seed_change.map(std::rc::Rc::new);
        let on_gd_opt_enabled_change_rc = self.on_gd_opt_enabled_change.map(std::rc::Rc::new);
        let on_gd_opt_target_ms_change_rc = self.on_gd_opt_target_ms_change.map(std::rc::Rc::new);
        let on_vog_enabled_change_rc = self.on_vog_enabled_change.map(std::rc::Rc::new);
        let on_vog_reference_channel_change_rc =
            self.on_vog_reference_channel_change.map(std::rc::Rc::new);
        let on_vog_reference_channel_toggle_rc =
            self.on_vog_reference_channel_toggle.map(std::rc::Rc::new);
        let on_broadband_target_matching_change_rc = self
            .on_broadband_target_matching_change
            .map(std::rc::Rc::new);
        let on_mixed_crossover_freq_change_rc =
            self.on_mixed_crossover_freq_change.map(std::rc::Rc::new);
        let on_mixed_crossover_type_change_rc =
            self.on_mixed_crossover_type_change.map(std::rc::Rc::new);
        let on_mixed_crossover_type_toggle_rc =
            self.on_mixed_crossover_type_toggle.map(std::rc::Rc::new);
        let on_mixed_fir_band_change_rc = self.on_mixed_fir_band_change.map(std::rc::Rc::new);
        let on_mixed_fir_band_toggle_rc = self.on_mixed_fir_band_toggle.map(std::rc::Rc::new);

        // Include the render body from the separate file
        match layout_mode {
            AutoEqLayoutMode::Default => {
                include!("render_body.rs")
            }
            AutoEqLayoutMode::RoomEq => {
                include!("render_body_room_eq.rs")
            }
        }
    }
}
