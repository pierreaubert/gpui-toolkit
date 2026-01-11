//! AutoEQ Form Debug Example
//!
//! Interactive showcase for the AutoEQ Form component:
//! - Algorithm selection
//! - Number input fields with various parameters
//! - Compact vs standard layout
//! - Disabled state

use gpui::*;
use gpui_ui_kit::autoeq::{AutoEqConfig, AutoEqForm, AutoEqFormUiState};
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

/// Demo state
pub struct AutoEqFormDebug {
    /// Main form config
    config: AutoEqConfig,
    /// UI state for main form
    ui_state: AutoEqFormUiState,
    /// Entity reference
    entity: Entity<Self>,
}

impl AutoEqFormDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            config: AutoEqConfig::default(),
            ui_state: AutoEqFormUiState::default(),
            entity: cx.entity().clone(),
        }
    }
}

impl Render for AutoEqFormDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        div()
            .id("autoeq-form-debug-root")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_6()
            .flex()
            .flex_col()
            .gap_6()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h1("AutoEQ Form Component Debug"))
                    .child(
                        Text::new("Configure EQ optimization parameters. Click number inputs to edit, use dropdowns for selection.")
                            .muted(true),
                    ),
            )
            // i18n Status Bar - demonstrates language switching works
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("🌐 {}: ", cx.t(TranslationKey::MenuLanguage))).weight(TextWeight::Medium))
                    .child(Text::new(cx.language().native_name()).color(theme.accent))
                    .child(Text::new(" | "))
                    .child(Text::new(cx.t(TranslationKey::SectionFormControls)).color(theme.text_secondary)),
            )
            // Current Config Display
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Current Configuration")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .child(Text::new(format!("System: {}", self.config.system_type)).size(TextSize::Sm))
                            .child(Text::new(format!("Loss: {}", self.config.loss_type)).size(TextSize::Sm))
                            .child(Text::new(format!("Target: {}", self.config.target_curve)).size(TextSize::Sm)),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .child(Text::new(format!("Algorithm: {}", self.config.algo)).size(TextSize::Sm))
                            .child(Text::new(format!("Filters: {}", self.config.num_filters)).size(TextSize::Sm))
                            .child(Text::new(format!("Model: {}", self.config.peq_model)).size(TextSize::Sm))
                            .child(Text::new(format!("Q: {:.1} - {:.1}", self.config.min_q, self.config.max_q)).size(TextSize::Sm))
                            .child(Text::new(format!("dB: {:.1} - {:.1}", self.config.min_db, self.config.max_db)).size(TextSize::Sm)),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .child(Text::new(format!("Freq: {:.0} - {:.0} Hz", self.config.min_freq, self.config.max_freq)).size(TextSize::Sm))
                            .child(Text::new(format!("Iterations: {}", self.config.maxeval)).size(TextSize::Sm))
                            .child(Text::new(format!("Smooth: {} (n={})", self.config.smooth, self.config.smooth_n)).size(TextSize::Sm))
                            .child(Text::new(format!("Refine: {}", self.config.refine)).size(TextSize::Sm)),
                    ),
            )
            // Main Form
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Standard Form (Full)")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("All options visible, interactive editing").size(TextSize::Sm).muted(true))
                    .child({
                        let config = self.config.clone();
                        let ui_state = self.ui_state.clone();
                        AutoEqForm::new("main-form")
                            .config(config)
                            .ui_state(ui_state)
                            // Goals section callbacks
                            .on_system_type_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.system_type = val.to_string();
                                    });
                                }
                            })
                            .on_system_type_toggle({
                                let entity = entity.clone();
                                move |open, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.ui_state.system_type_open = open;
                                    });
                                }
                            })
                            .on_loss_type_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.loss_type = val.to_string();
                                    });
                                }
                            })
                            .on_loss_type_toggle({
                                let entity = entity.clone();
                                move |open, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.ui_state.loss_type_open = open;
                                    });
                                }
                            })
                            .on_target_curve_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.target_curve = val.to_string();
                                    });
                                }
                            })
                            .on_target_curve_toggle({
                                let entity = entity.clone();
                                move |open, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.ui_state.target_curve_open = open;
                                    });
                                }
                            })
                            // EQ Design callbacks
                            .on_opt_mode_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.opt_mode = val.to_string();
                                    });
                                }
                            })
                            .on_opt_mode_toggle({
                                let entity = entity.clone();
                                move |open, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.ui_state.opt_mode_open = open;
                                    });
                                }
                            })
                            .on_num_filters_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.num_filters = val;
                                    });
                                }
                            })
                            .on_sample_rate_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.sample_rate = val as u32;
                                    });
                                }
                            })
                            .on_min_db_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.min_db = val;
                                    });
                                }
                            })
                            .on_max_db_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.max_db = val;
                                    });
                                }
                            })
                            .on_min_q_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.min_q = val;
                                    });
                                }
                            })
                            .on_max_q_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.max_q = val;
                                    });
                                }
                            })
                            .on_min_freq_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.min_freq = val;
                                    });
                                }
                            })
                            .on_max_freq_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.max_freq = val;
                                    });
                                }
                            })
                            .on_peq_model_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.peq_model = val.to_string();
                                    });
                                }
                            })
                            .on_peq_model_toggle({
                                let entity = entity.clone();
                                move |open, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.ui_state.peq_model_open = open;
                                    });
                                }
                            })
                            .on_spacing_weight_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.spacing_weight = val;
                                    });
                                }
                            })
                            .on_min_spacing_oct_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.min_spacing_oct = val;
                                    });
                                }
                            })
                            // Optimization callbacks
                            .on_algo_change({
                                let entity = entity.clone();
                                move |alg, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.algo = alg.to_string();
                                    });
                                }
                            })
                            .on_algo_toggle({
                                let entity = entity.clone();
                                move |open, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.ui_state.algo_open = open;
                                    });
                                }
                            })
                            .on_population_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.population = val;
                                    });
                                }
                            })
                            .on_maxeval_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.maxeval = val;
                                    });
                                }
                            })
                            .on_tolerance_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.tolerance = val;
                                    });
                                }
                            })
                            .on_atolerance_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.atolerance = val;
                                    });
                                }
                            })
                            .on_strategy_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.strategy = val.to_string();
                                    });
                                }
                            })
                            .on_strategy_toggle({
                                let entity = entity.clone();
                                move |open, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.ui_state.strategy_open = open;
                                    });
                                }
                            })
                            .on_de_f_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.de_f = val;
                                    });
                                }
                            })
                            .on_de_cr_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.de_cr = val;
                                    });
                                }
                            })
                            .on_refine_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.refine = val;
                                    });
                                }
                            })
                            .on_local_algo_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.local_algo = val.to_string();
                                    });
                                }
                            })
                            .on_local_algo_toggle({
                                let entity = entity.clone();
                                move |open, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.ui_state.local_algo_open = open;
                                    });
                                }
                            })
                            .on_smooth_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.smooth = val;
                                    });
                                }
                            })
                            .on_smooth_n_change({
                                let entity = entity.clone();
                                move |val, _w, cx| {
                                    entity.update(cx, |this, _| {
                                        this.config.smooth_n = val;
                                    });
                                }
                            })
                    }),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("AutoEQ Form Debug")
            .size(800.0, 950.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(AutoEqFormDebug::new),
    );
}
