//! Spinorama Demo - Speaker frequency response viewer using gpui-px charts.
//!
//! This demo fetches speaker data from spinorama.org and displays CEA2034 plots
//! using the high-level gpui-px charting API.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use autoeq::read::{
    extract_cea2034_curves_original, fetch_available_speakers, fetch_directivity_data,
    fetch_measurement_plot_data,
};
use autoeq::{Curve, DirectivityData};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_px::interaction::{InteractiveChart, InteractiveChartConfig, InteractiveChartState};
use gpui_px::{
    ColorScale, Colormap, LegendPosition, ScaleType, Surface3DState, heatmap, line, surface3d,
};
use gpui_ui_kit::{MiniApp, MiniAppConfig, SelectOption, Spinner, SpinnerSize};
use tokio::runtime::Runtime;

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Spinorama Viewer (gpui-px)").size(1200.0, 800.0),
        |cx| cx.new(SpinoramaApp::new),
    );
}

// ============================================================================
// Load States
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum LoadState {
    Idle,
    Loading,
    Loaded,
    Error(String),
}

// ============================================================================
// Plot Sections
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PlotSection {
    #[default]
    CEA2034,
    HorizontalSPL,
    VerticalSPL,
    Contour,
    Surface3D,
}

impl PlotSection {
    fn all() -> &'static [PlotSection] {
        &[
            PlotSection::CEA2034,
            PlotSection::HorizontalSPL,
            PlotSection::VerticalSPL,
            PlotSection::Contour,
            PlotSection::Surface3D,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            PlotSection::CEA2034 => "CEA2034",
            PlotSection::HorizontalSPL => "Horizontal SPL",
            PlotSection::VerticalSPL => "Vertical SPL",
            PlotSection::Contour => "Contour",
            PlotSection::Surface3D => "Surface 3D",
        }
    }
}

// ============================================================================
// CEA2034 Colors (matching spinorama.org)
// ============================================================================

fn cea2034_colors() -> HashMap<&'static str, u32> {
    let mut colors = HashMap::new();
    colors.insert("On Axis", 0x1f77b4); // Blue
    colors.insert("Listening Window", 0xff7f0e); // Orange
    colors.insert("Early Reflections", 0x2ca02c); // Green
    colors.insert("Sound Power", 0xd62728); // Red
    colors.insert("Early Reflections DI", 0x9467bd); // Purple
    colors.insert("Sound Power DI", 0x8c564b); // Brown
    colors
}

// ============================================================================
// Main Application
// ============================================================================

struct SpinoramaApp {
    runtime: Arc<Runtime>,
    // Speaker list
    speakers: Vec<String>,
    speakers_load_state: LoadState,
    // Version list for selected speaker
    versions: Vec<String>,
    versions_load_state: LoadState,
    // Selection state
    selected_speaker: Option<String>,
    selected_version: Option<String>,
    selected_measurement: String,
    // Data state
    cea2034_curves: HashMap<String, Curve>,
    directivity_data: Option<DirectivityData>,
    data_load_state: LoadState,
    // UI state
    current_section: PlotSection,
    speaker_dropdown_open: bool,
    version_dropdown_open: bool,
    section_dropdown_open: bool,
    // 3D Surface interaction state
    surface3d_state: Rc<RefCell<Surface3DState>>,
    surface3d_dragging: bool,
    surface3d_last_mouse: Option<Point<Pixels>>,
    // Colormap selection
    selected_colormap: Colormap,
    colormap_dropdown_open: bool,
    // Interactive chart state for frequency/SPL plots (CEA2034, horizontal/vertical SPL)
    freq_spl_chart_state: InteractiveChartState,
    // Interactive chart state for contour plot
    contour_chart_state: InteractiveChartState,
    // Hidden series for CEA2034 chart (toggled via legend clicks)
    cea2034_hidden_series: Rc<RefCell<HashSet<usize>>>,
}

impl SpinoramaApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
        );

        let mut app = Self {
            runtime,
            speakers: Vec::new(),
            speakers_load_state: LoadState::Idle,
            versions: Vec::new(),
            versions_load_state: LoadState::Idle,
            selected_speaker: None,
            selected_version: None,
            selected_measurement: "CEA2034".to_string(),
            cea2034_curves: HashMap::new(),
            directivity_data: None,
            data_load_state: LoadState::Idle,
            current_section: PlotSection::default(),
            speaker_dropdown_open: false,
            version_dropdown_open: false,
            section_dropdown_open: false,
            // Initialize 3D surface state with good defaults for spinorama viewing
            surface3d_state: Rc::new(RefCell::new(Surface3DState::new(3.5, 60.0, 25.0))),
            surface3d_dragging: false,
            surface3d_last_mouse: None,
            // Colormap selection (Turbo is good default for spinorama)
            selected_colormap: Colormap::Turbo,
            colormap_dropdown_open: false,
            // Interactive chart state for freq/SPL plots: X=20Hz-20kHz (log), Y=-40 to 10 dB
            freq_spl_chart_state: InteractiveChartState::new(20.0, 20000.0, -40.0, 10.0)
                .with_log_x(true)
                .with_size(900.0, 500.0)
                .with_config(
                    InteractiveChartConfig::new()
                        .with_left_margin(50.0)
                        .with_top_margin(30.0),
                ),
            // Interactive chart state for contour: X=100Hz-20kHz (log), Y=-60 to 60 deg
            contour_chart_state: InteractiveChartState::new(100.0, 20000.0, -60.0, 60.0)
                .with_log_x(true)
                .with_size(900.0, 500.0)
                .with_config(
                    InteractiveChartConfig::new()
                        .with_left_margin(50.0)
                        .with_top_margin(30.0),
                ),
            // Hidden series for legend toggle
            cea2034_hidden_series: Rc::new(RefCell::new(HashSet::new())),
        };

        // Start loading speakers list
        app.load_speakers(cx);
        app
    }

    fn load_speakers(&mut self, cx: &mut Context<Self>) {
        self.speakers_load_state = LoadState::Loading;
        let runtime = self.runtime.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result: Result<Vec<String>, String> = runtime
                .spawn(async { fetch_available_speakers().await.map_err(|e| e.to_string()) })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);

            match result {
                Ok(speakers) => {
                    println!("Loaded {} speakers", speakers.len());
                    let _ = this.update(cx, |app, cx| {
                        app.speakers = speakers;
                        app.speakers_load_state = LoadState::Loaded;
                        // Auto-select first speaker and load its versions
                        if let Some(first_speaker) = app.speakers.first().cloned() {
                            app.selected_speaker = Some(first_speaker);
                            app.load_versions(cx);
                        }
                        cx.notify();
                    });
                }
                Err(e) => {
                    println!("Error loading speakers: {}", e);
                    let _ = this.update(cx, |app, cx| {
                        app.speakers_load_state = LoadState::Error(e);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn load_versions(&mut self, cx: &mut Context<Self>) {
        let Some(speaker) = self.selected_speaker.clone() else {
            return;
        };

        self.versions_load_state = LoadState::Loading;
        self.versions.clear();
        self.selected_version = None;
        let runtime = self.runtime.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result: Result<Vec<String>, String> = runtime
                .spawn({
                    let speaker = speaker.clone();
                    async move {
                        let url = format!(
                            "https://api.spinorama.org/v1/speaker/{}/versions",
                            urlencoding::encode(&speaker)
                        );
                        let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
                        if !response.status().is_success() {
                            return Err(format!("Failed to fetch versions: {}", response.status()));
                        }
                        let versions: Vec<String> =
                            response.json().await.map_err(|e| e.to_string())?;
                        Ok(versions)
                    }
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);

            match result {
                Ok(versions) => {
                    println!("Loaded {} versions for speaker", versions.len());
                    let _ = this.update(cx, |app, cx| {
                        app.versions = versions;
                        app.versions_load_state = LoadState::Loaded;
                        // Auto-select first version and load speaker data
                        if let Some(first_version) = app.versions.first().cloned() {
                            app.selected_version = Some(first_version);
                            app.load_speaker_data(cx);
                        }
                        cx.notify();
                    });
                }
                Err(e) => {
                    println!("Error loading versions: {}", e);
                    let _ = this.update(cx, |app, cx| {
                        app.versions_load_state = LoadState::Error(e);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn load_speaker_data(&mut self, cx: &mut Context<Self>) {
        let Some(speaker) = self.selected_speaker.clone() else {
            return;
        };
        let Some(version) = self.selected_version.clone() else {
            return;
        };

        self.data_load_state = LoadState::Loading;
        let runtime = self.runtime.clone();
        let measurement = self.selected_measurement.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            // Fetch CEA2034 data and directivity data in parallel
            let cea2034_handle = runtime.spawn({
                let speaker = speaker.clone();
                let version = version.clone();
                let measurement = measurement.clone();
                async move {
                    let plot_data = fetch_measurement_plot_data(&speaker, &version, &measurement)
                        .await
                        .map_err(|e| e.to_string())?;
                    extract_cea2034_curves_original(&plot_data, &measurement)
                        .map_err(|e| e.to_string())
                }
            });

            let directivity_handle = runtime.spawn({
                let speaker = speaker.clone();
                let version = version.clone();
                async move {
                    fetch_directivity_data(&speaker, &version)
                        .await
                        .map_err(|e| e.to_string())
                }
            });

            // Wait for CEA2034 data (required)
            let cea2034_result: Result<HashMap<String, Curve>, String> = cea2034_handle
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);

            // Wait for directivity data (optional - don't fail if not available)
            let directivity_result: Option<DirectivityData> =
                directivity_handle.await.ok().and_then(|r| r.ok());

            match cea2034_result {
                Ok(curves) => {
                    let _ = this.update(cx, |app, cx| {
                        app.cea2034_curves = curves;
                        app.directivity_data = directivity_result;
                        app.data_load_state = LoadState::Loaded;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(cx, |app, cx| {
                        app.data_load_state = LoadState::Error(e);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn render_header(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let speaker_options: Vec<SelectOption> = self
            .speakers
            .iter()
            .map(|s| SelectOption::new(s.clone(), s.clone()))
            .collect();

        let version_options: Vec<SelectOption> = self
            .versions
            .iter()
            .map(|v| SelectOption::new(v.clone(), v.clone()))
            .collect();

        let section_options: Vec<SelectOption> = PlotSection::all()
            .iter()
            .map(|s| SelectOption::new(s.label(), s.label()))
            .collect();

        let current_speaker = self.selected_speaker.clone();
        let current_version = self.selected_version.clone();
        let current_section = self.current_section.label();
        let speaker_dropdown_open = self.speaker_dropdown_open;
        let version_dropdown_open = self.version_dropdown_open;
        let section_dropdown_open = self.section_dropdown_open;
        let is_loading_speakers = self.speakers_load_state == LoadState::Loading;
        let is_loading_versions = self.versions_load_state == LoadState::Loading;
        let is_loading_data = self.data_load_state == LoadState::Loading;
        let has_speaker = self.selected_speaker.is_some();

        div()
            .w_full()
            .min_h(px(60.0))
            .bg(rgb(0x1e1e1e))
            .border_b_1()
            .border_color(rgb(0x3c3c3c))
            .flex()
            .items_center()
            .px_4()
            .py_2()
            .gap_4()
            // Speaker select
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().text_color(rgb(0xcccccc)).child("Speaker:"))
                    .child(if is_loading_speakers {
                        div()
                            .id("speaker-loading")
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Spinner::new().size(SpinnerSize::Sm))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x888888))
                                    .child("Loading..."),
                            )
                    } else {
                        self.render_speaker_dropdown(
                            speaker_options,
                            current_speaker,
                            speaker_dropdown_open,
                            cx,
                        )
                    }),
            )
            // Version select (only show if speaker is selected)
            .when(has_speaker, |el: Div| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().text_sm().text_color(rgb(0xcccccc)).child("Version:"))
                        .child(if is_loading_versions {
                            div()
                                .id("version-loading")
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(Spinner::new().size(SpinnerSize::Sm))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x888888))
                                        .child("Loading..."),
                                )
                        } else {
                            self.render_version_dropdown(
                                version_options,
                                current_version,
                                version_dropdown_open,
                                cx,
                            )
                        }),
                )
            })
            // Plot type select
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().text_color(rgb(0xcccccc)).child("Plot:"))
                    .child(self.render_section_dropdown(
                        section_options,
                        current_section,
                        section_dropdown_open,
                        cx,
                    )),
            )
            // Colormap select (only for Surface3D and Contour)
            .when(
                self.current_section == PlotSection::Surface3D
                    || self.current_section == PlotSection::Contour,
                |el: Div| {
                    el.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().text_color(rgb(0xcccccc)).child("Colormap:"))
                            .child(self.render_colormap_dropdown(cx)),
                    )
                },
            )
            // Loading indicator
            .when(is_loading_data, |el: Div| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .ml_auto()
                        .child(Spinner::new().size(SpinnerSize::Sm))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x888888))
                                .child("Loading data..."),
                        ),
                )
            })
    }

    fn render_speaker_dropdown(
        &mut self,
        options: Vec<SelectOption>,
        current: Option<String>,
        is_open: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let entity = cx.entity().clone();
        let entity_for_toggle = cx.entity().clone();

        div()
            .relative()
            .id("speaker-dropdown-container")
            .child(
                div()
                    .id("speaker-select")
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .min_w(px(200.0))
                    .bg(rgb(0x2a2a2a))
                    .border_1()
                    .border_color(rgb(0x3a3a3a))
                    .rounded_md()
                    .cursor_pointer()
                    .text_sm()
                    .hover(|s| s.border_color(rgb(0x007acc)))
                    .child(
                        div()
                            .text_color(if current.is_some() {
                                rgb(0xffffff)
                            } else {
                                rgb(0x666666)
                            })
                            .child(
                                current
                                    .clone()
                                    .unwrap_or_else(|| "Select speaker...".into()),
                            ),
                    )
                    .child(div().text_xs().text_color(rgb(0x666666)).child("v"))
                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                        entity_for_toggle.update(cx, |this, cx| {
                            this.speaker_dropdown_open = !this.speaker_dropdown_open;
                            this.version_dropdown_open = false;
                            this.section_dropdown_open = false;
                            cx.notify();
                        });
                    }),
            )
            .when(is_open, |el: Stateful<Div>| {
                el.child(
                    deferred(
                        div()
                            .id("speaker-dropdown")
                            .absolute()
                            .top_full()
                            .left_0()
                            .mt_1()
                            .w(px(300.0))
                            .max_h(px(400.0))
                            .overflow_y_scroll()
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x3a3a3a))
                            .rounded_md()
                            .shadow_lg()
                            .py_1()
                            .children(options.into_iter().enumerate().map(|(i, opt)| {
                                let is_selected = current.as_ref() == Some(&opt.value.to_string());
                                let value = opt.value.to_string();
                                let entity = entity.clone();

                                div()
                                    .id(ElementId::NamedInteger("speaker-opt".into(), i as u64))
                                    .px_3()
                                    .py(px(6.0))
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_selected, |el: Stateful<Div>| {
                                        el.bg(rgb(0x007acc)).text_color(rgb(0xffffff))
                                    })
                                    .when(!is_selected, |el: Stateful<Div>| {
                                        el.text_color(rgb(0xcccccc)).hover(|s| s.bg(rgb(0x3a3a3a)))
                                    })
                                    .child(opt.label)
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        entity.update(cx, |this, cx| {
                                            this.selected_speaker = Some(value.clone());
                                            this.speaker_dropdown_open = false;
                                            // Clear previous data when changing speaker
                                            this.cea2034_curves.clear();
                                            this.directivity_data = None;
                                            this.data_load_state = LoadState::Idle;
                                            // Load versions for this speaker
                                            this.load_versions(cx);
                                        });
                                    })
                            })),
                    )
                    .with_priority(1),
                )
            })
    }

    fn render_version_dropdown(
        &mut self,
        options: Vec<SelectOption>,
        current: Option<String>,
        is_open: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let entity = cx.entity().clone();
        let entity_for_toggle = cx.entity().clone();

        div()
            .relative()
            .id("version-dropdown-container")
            .child(
                div()
                    .id("version-select")
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .min_w(px(120.0))
                    .bg(rgb(0x2a2a2a))
                    .border_1()
                    .border_color(rgb(0x3a3a3a))
                    .rounded_md()
                    .cursor_pointer()
                    .text_sm()
                    .hover(|s| s.border_color(rgb(0x007acc)))
                    .child(
                        div()
                            .text_color(if current.is_some() {
                                rgb(0xffffff)
                            } else {
                                rgb(0x666666)
                            })
                            .child(
                                current
                                    .clone()
                                    .unwrap_or_else(|| "Select version...".into()),
                            ),
                    )
                    .child(div().text_xs().text_color(rgb(0x666666)).child("v"))
                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                        entity_for_toggle.update(cx, |this, cx| {
                            this.version_dropdown_open = !this.version_dropdown_open;
                            this.speaker_dropdown_open = false;
                            this.section_dropdown_open = false;
                            cx.notify();
                        });
                    }),
            )
            .when(is_open, |el: Stateful<Div>| {
                el.child(
                    deferred(
                        div()
                            .id("version-dropdown")
                            .absolute()
                            .top_full()
                            .left_0()
                            .mt_1()
                            .w(px(150.0))
                            .max_h(px(300.0))
                            .overflow_y_scroll()
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x3a3a3a))
                            .rounded_md()
                            .shadow_lg()
                            .py_1()
                            .children(options.into_iter().enumerate().map(|(i, opt)| {
                                let is_selected = current.as_ref() == Some(&opt.value.to_string());
                                let value = opt.value.to_string();
                                let entity = entity.clone();

                                div()
                                    .id(ElementId::NamedInteger("version-opt".into(), i as u64))
                                    .px_3()
                                    .py(px(6.0))
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_selected, |el: Stateful<Div>| {
                                        el.bg(rgb(0x007acc)).text_color(rgb(0xffffff))
                                    })
                                    .when(!is_selected, |el: Stateful<Div>| {
                                        el.text_color(rgb(0xcccccc)).hover(|s| s.bg(rgb(0x3a3a3a)))
                                    })
                                    .child(opt.label)
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        entity.update(cx, |this, cx| {
                                            this.selected_version = Some(value.clone());
                                            this.version_dropdown_open = false;
                                            // Load speaker data with the selected version
                                            this.load_speaker_data(cx);
                                        });
                                    })
                            })),
                    )
                    .with_priority(1),
                )
            })
    }

    fn render_section_dropdown(
        &mut self,
        options: Vec<SelectOption>,
        current: &'static str,
        is_open: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let entity = cx.entity().clone();
        let entity_for_toggle = cx.entity().clone();

        div()
            .relative()
            .id("section-dropdown-container")
            .child(
                div()
                    .id("section-select")
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .min_w(px(180.0))
                    .bg(rgb(0x2a2a2a))
                    .border_1()
                    .border_color(rgb(0x3a3a3a))
                    .rounded_md()
                    .cursor_pointer()
                    .text_sm()
                    .hover(|s| s.border_color(rgb(0x007acc)))
                    .child(div().text_color(rgb(0xffffff)).child(current))
                    .child(div().text_xs().text_color(rgb(0x666666)).child("v"))
                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                        entity_for_toggle.update(cx, |this, cx| {
                            this.section_dropdown_open = !this.section_dropdown_open;
                            this.speaker_dropdown_open = false;
                            this.version_dropdown_open = false;
                            cx.notify();
                        });
                    }),
            )
            .when(is_open, |el: Stateful<Div>| {
                el.child(
                    deferred(
                        div()
                            .id("section-dropdown")
                            .absolute()
                            .top_full()
                            .left_0()
                            .mt_1()
                            .w(px(200.0))
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x3a3a3a))
                            .rounded_md()
                            .shadow_lg()
                            .py_1()
                            .children(options.into_iter().enumerate().map(|(i, opt)| {
                                let is_selected = current == opt.value.as_ref();
                                let label_str = opt.label.to_string();
                                let entity = entity.clone();

                                div()
                                    .id(ElementId::NamedInteger("section-opt".into(), i as u64))
                                    .px_3()
                                    .py(px(6.0))
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_selected, |el: Stateful<Div>| {
                                        el.bg(rgb(0x007acc)).text_color(rgb(0xffffff))
                                    })
                                    .when(!is_selected, |el: Stateful<Div>| {
                                        el.text_color(rgb(0xcccccc)).hover(|s| s.bg(rgb(0x3a3a3a)))
                                    })
                                    .child(opt.label)
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        let section = PlotSection::all()
                                            .iter()
                                            .find(|s| s.label() == label_str)
                                            .copied()
                                            .unwrap_or_default();
                                        entity.update(cx, |this, _| {
                                            this.current_section = section;
                                            this.section_dropdown_open = false;
                                        });
                                    })
                            })),
                    )
                    .with_priority(1),
                )
            })
    }

    fn render_colormap_dropdown(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let entity = cx.entity().clone();
        let entity_for_toggle = cx.entity().clone();
        let is_open = self.colormap_dropdown_open;

        let colormaps = [
            (Colormap::Turbo, "Turbo"),
            (Colormap::Viridis, "Viridis"),
            (Colormap::Plasma, "Plasma"),
            (Colormap::Inferno, "Inferno"),
            (Colormap::CoolWarm, "CoolWarm"),
        ];

        let current_label = colormaps
            .iter()
            .find(|(c, _)| *c == self.selected_colormap)
            .map(|(_, l)| *l)
            .unwrap_or("Turbo");

        div()
            .relative()
            .id("colormap-dropdown-container")
            .child(
                div()
                    .id("colormap-select")
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .min_w(px(120.0))
                    .bg(rgb(0x2a2a2a))
                    .border_1()
                    .border_color(rgb(0x3a3a3a))
                    .rounded_md()
                    .cursor_pointer()
                    .text_sm()
                    .hover(|s| s.border_color(rgb(0x007acc)))
                    .child(div().text_color(rgb(0xffffff)).child(current_label))
                    .child(div().text_xs().text_color(rgb(0x666666)).child("v"))
                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                        entity_for_toggle.update(cx, |this, cx| {
                            this.colormap_dropdown_open = !this.colormap_dropdown_open;
                            this.speaker_dropdown_open = false;
                            this.version_dropdown_open = false;
                            this.section_dropdown_open = false;
                            cx.notify();
                        });
                    }),
            )
            .when(is_open, |el: Stateful<Div>| {
                el.child(
                    deferred(
                        div()
                            .id("colormap-dropdown")
                            .absolute()
                            .top_full()
                            .left_0()
                            .mt_1()
                            .w(px(140.0))
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x3a3a3a))
                            .rounded_md()
                            .shadow_lg()
                            .py_1()
                            .children(colormaps.into_iter().enumerate().map(
                                |(i, (cmap, label))| {
                                    let is_selected = self.selected_colormap == cmap;
                                    let entity = entity.clone();

                                    div()
                                        .id(ElementId::NamedInteger(
                                            "colormap-opt".into(),
                                            i as u64,
                                        ))
                                        .px_3()
                                        .py(px(6.0))
                                        .cursor_pointer()
                                        .text_sm()
                                        .when(is_selected, |el: Stateful<Div>| {
                                            el.bg(rgb(0x007acc)).text_color(rgb(0xffffff))
                                        })
                                        .when(!is_selected, |el: Stateful<Div>| {
                                            el.text_color(rgb(0xcccccc))
                                                .hover(|s| s.bg(rgb(0x3a3a3a)))
                                        })
                                        .child(label)
                                        .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.selected_colormap = cmap;
                                                this.colormap_dropdown_open = false;
                                                cx.notify();
                                            });
                                        })
                                },
                            )),
                    )
                    .with_priority(1),
                )
            })
    }

    fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let content: Div = match self.data_load_state {
            LoadState::Idle => self.render_welcome(),
            LoadState::Loading => self.render_loading(),
            LoadState::Error(ref e) => self.render_error(e),
            LoadState::Loaded => match self.current_section {
                PlotSection::CEA2034 => self.render_cea2034_plot(cx),
                PlotSection::HorizontalSPL => self.render_directivity_plot("horizontal", cx),
                PlotSection::VerticalSPL => self.render_directivity_plot("vertical", cx),
                PlotSection::Contour => self.render_contour_plot(cx),
                PlotSection::Surface3D => self.render_surface3d_plot(cx),
            },
        };

        div()
            .id("content-scroll")
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .bg(rgb(0xffffff))
            .p_8()
            .child(content)
            // Close dropdowns when clicking on content area
            .on_click(cx.listener(|this, _, _window, _cx| {
                this.speaker_dropdown_open = false;
                this.version_dropdown_open = false;
                this.section_dropdown_open = false;
                this.colormap_dropdown_open = false;
            }))
    }

    fn render_welcome(&self) -> Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h_full()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x333333))
                    .child("Spinorama Viewer (gpui-px)"),
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .max_w(px(400.0))
                    .text_center()
                    .child("Select a speaker from the dropdown above to view its frequency response measurements from spinorama.org."),
            )
            .child(
                div()
                    .mt_4()
                    .text_sm()
                    .text_color(rgb(0x999999))
                    .child("Using high-level gpui-px charting API"),
            )
    }

    fn render_loading(&self) -> Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h_full()
            .gap_4()
            .child(Spinner::new().size(SpinnerSize::Xl))
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("Loading speaker data..."),
            )
    }

    fn render_error(&self, error: &str) -> Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h_full()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xd32f2f))
                    .child("Error Loading Data"),
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .max_w(px(400.0))
                    .text_center()
                    .child(error.to_string()),
            )
    }

    fn render_cea2034_plot(&self, _cx: &mut Context<Self>) -> Div {
        let colors = cea2034_colors();

        // SPL curves (primary axis)
        let spl_curve_names = [
            "On Axis",
            "Listening Window",
            "Early Reflections",
            "Sound Power",
        ];

        // DI curves (secondary axis)
        let di_curve_names = ["Early Reflections DI", "Sound Power DI"];

        // Get the first curve to use as the base
        let first_curve_name = spl_curve_names
            .iter()
            .find(|&name| self.cea2034_curves.contains_key(*name));

        let Some(&first_name) = first_curve_name else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No CEA2034 data available for this speaker"),
            );
        };

        let first_curve = &self.cea2034_curves[first_name];
        let first_color = colors.get(first_name).copied().unwrap_or(0x888888);

        // Filter frequency data to 20Hz-20kHz range
        let freq_indices: Vec<usize> = first_curve
            .freq
            .iter()
            .enumerate()
            .filter(|&(_, f)| (20.0..=20000.0).contains(f))
            .map(|(i, _)| i)
            .collect();

        let freq: Vec<f64> = freq_indices.iter().map(|&i| first_curve.freq[i]).collect();
        let first_spl: Vec<f64> = freq_indices.iter().map(|&i| first_curve.spl[i]).collect();

        // Get zoom state from interactive chart state
        let (x_min, x_max) = self.freq_spl_chart_state.x_domain();
        let (y_min, y_max) = self.freq_spl_chart_state.y_domain();
        let is_zoomed = self.freq_spl_chart_state.is_zoomed();

        // Chart dimensions
        let chart_width = 900.0_f32;
        let chart_height = 500.0_f32;

        // Get hidden series for legend toggle
        let hidden_series_set = self.cea2034_hidden_series.borrow().clone();
        let hidden_indices: Vec<usize> = hidden_series_set.iter().copied().collect();

        // Clone for the callback
        let hidden_series_ref = self.cea2034_hidden_series.clone();

        // Start building the line chart with zoom-adjusted ranges
        let mut chart = line(&freq, &first_spl)
            .title(format!(
                "CEA2034 - {}{}",
                self.selected_speaker.as_deref().unwrap_or("Unknown"),
                if is_zoomed { " (zoomed)" } else { "" }
            ))
            .x_label("Frequency (Hz)")
            .y_label("SPL (dB)")
            .label(first_name)
            .color(first_color)
            .x_scale(ScaleType::Log)
            .x_range(x_min, x_max)
            .y_range(y_min, y_max)
            .size(chart_width, chart_height)
            .stroke_width(2.0)
            .legend_position(LegendPosition::Bottom)
            .hidden_series(&hidden_indices)
            .on_legend_click(move |series_idx, window, _cx| {
                // Toggle visibility of the clicked series
                let mut hidden = hidden_series_ref.borrow_mut();
                if hidden.contains(&series_idx) {
                    hidden.remove(&series_idx);
                } else {
                    hidden.insert(series_idx);
                }
                window.refresh();
            });

        // Add remaining SPL curves
        for &name in spl_curve_names.iter().skip(1) {
            if let Some(curve) = self.cea2034_curves.get(name) {
                let color = colors.get(name).copied().unwrap_or(0x888888);
                let spl: Vec<f64> = freq_indices
                    .iter()
                    .filter_map(|&i| curve.spl.get(i).copied())
                    .collect();

                if spl.len() == freq.len() {
                    chart = chart.add_series(&spl, Some(name), color, 2.0, 1.0);
                }
            }
        }

        // Configure secondary axis for DI curves
        chart = chart.y2_label("DI (dB)").y2_range(-5.0, 45.0);

        // Add DI curves on secondary axis
        for &name in &di_curve_names {
            if let Some(curve) = self.cea2034_curves.get(name) {
                let color = colors.get(name).copied().unwrap_or(0x888888);
                let spl: Vec<f64> = freq_indices
                    .iter()
                    .filter_map(|&i| curve.spl.get(i).copied())
                    .collect();

                if spl.len() == freq.len() {
                    chart = chart.add_series_y2(&spl, Some(name), color, 2.0, 1.0);
                }
            }
        }

        match chart.build() {
            Ok(element) => {
                // Wrap chart with interactive handlers using gpui-px InteractiveChart
                let interactive_chart = InteractiveChart::new(
                    "cea2034-chart",
                    element,
                    self.freq_spl_chart_state.clone(),
                )
                .build();

                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(interactive_chart)
                    .child(
                        div().mt_4().p_4().bg(rgb(0xf5f5f5)).rounded_md().child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x666666))
                                .child("Drag to pan • Scroll to zoom • Double-click to reset • Click legend to toggle"),
                        ),
                    )
            }
            Err(e) => div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0xd32f2f))
                    .child(format!("Chart error: {}", e)),
            ),
        }
    }

    fn render_directivity_plot(&self, plane: &str, _cx: &mut Context<Self>) -> Div {
        // Viridis color palette for different angles
        let viridis_colors: [(f32, f32, f32); 6] = [
            (0.267, 0.004, 0.329), // Dark purple
            (0.255, 0.267, 0.529), // Purple-blue
            (0.165, 0.471, 0.557), // Teal
            (0.133, 0.659, 0.518), // Green-teal
            (0.478, 0.820, 0.319), // Light green
            (0.992, 0.906, 0.145), // Yellow
        ];

        let Some(ref directivity) = self.directivity_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No directivity data available for this speaker."),
            );
        };

        let curves = if plane == "horizontal" {
            &directivity.horizontal
        } else {
            &directivity.vertical
        };

        if curves.is_empty() {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child(format!("No {} directivity data available.", plane)),
            );
        }

        // Get the first curve to use as the base for frequency data
        let first_curve = &curves[0];

        // Filter frequency data to 20Hz-20kHz range
        let freq_indices: Vec<usize> = first_curve
            .freq
            .iter()
            .enumerate()
            .filter(|&(_, f)| (20.0..=20000.0).contains(f))
            .map(|(i, _)| i)
            .collect();

        let freq: Vec<f64> = freq_indices.iter().map(|&i| first_curve.freq[i]).collect();

        if freq.is_empty() {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No frequency data in valid range."),
            );
        }

        let num_curves = curves.len();

        // Interpolate color for an index
        let interpolate_color = |index: usize| -> u32 {
            let t = index as f32 / (num_curves.max(1) - 1).max(1) as f32;
            let t_scaled = t * 5.0;
            let segment = (t_scaled as usize).min(4);
            let local_t = t_scaled - segment as f32;

            let (r1, g1, b1) = viridis_colors[segment];
            let (r2, g2, b2) = viridis_colors[(segment + 1).min(5)];

            let r = ((r1 + (r2 - r1) * local_t) * 255.0) as u32;
            let g = ((g1 + (g2 - g1) * local_t) * 255.0) as u32;
            let b = ((b1 + (b2 - b1) * local_t) * 255.0) as u32;

            (r << 16) | (g << 8) | b
        };

        // Build chart with first curve
        let first_spl: Vec<f64> = freq_indices
            .iter()
            .filter_map(|&i| first_curve.spl.get(i).copied())
            .collect();

        let first_color = interpolate_color(0);
        let first_label = format!("{:.0}°", first_curve.angle);

        // Get zoom state from interactive chart state
        let (x_min, x_max) = self.freq_spl_chart_state.x_domain();
        let (y_min, y_max) = self.freq_spl_chart_state.y_domain();
        let is_zoomed = self.freq_spl_chart_state.is_zoomed();

        let mut chart = line(&freq, &first_spl)
            .title(format!(
                "{} SPL - {}{}",
                if plane == "horizontal" {
                    "Horizontal"
                } else {
                    "Vertical"
                },
                self.selected_speaker.as_deref().unwrap_or("Unknown"),
                if is_zoomed { " (zoomed)" } else { "" }
            ))
            .x_label("Frequency (Hz)")
            .y_label("SPL (dB)")
            .label(&first_label)
            .color(first_color)
            .x_scale(ScaleType::Log)
            .x_range(x_min, x_max)
            .y_range(y_min, y_max)
            .size(900.0, 500.0)
            .stroke_width(1.5)
            .legend_position(LegendPosition::Hidden); // Too many curves for legend

        // Add remaining curves
        for (i, curve) in curves.iter().enumerate().skip(1) {
            let spl: Vec<f64> = freq_indices
                .iter()
                .filter_map(|&idx| curve.spl.get(idx).copied())
                .collect();

            if spl.len() == freq.len() {
                let color = interpolate_color(i);
                let label = format!("{:.0}°", curve.angle);
                chart = chart.add_series(&spl, Some(&label), color, 1.5, 1.0);
            }
        }

        // Get angle range for legend
        let angle_min = curves.first().map(|c| c.angle).unwrap_or(-60.0);
        let angle_max = curves.last().map(|c| c.angle).unwrap_or(60.0);

        match chart.build() {
            Ok(element) => {
                // Wrap chart with interactive handlers
                let interactive_chart = InteractiveChart::new(
                    format!("{}-spl-chart", plane),
                    element,
                    self.freq_spl_chart_state.clone(),
                )
                .build();

                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(interactive_chart)
                    // Angle color legend
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .p_4()
                            .bg(rgb(0xf5f5f5))
                            .rounded_md()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x666666))
                                    .child(format!("{:.0}°", angle_min)),
                            )
                            // Gradient strip
                            .children((0..6).map(|i| {
                                let (r, g, b) = viridis_colors[i];
                                let color = ((r * 255.0) as u32) << 16
                                    | ((g * 255.0) as u32) << 8
                                    | (b * 255.0) as u32;
                                div().flex_1().h(px(16.0)).bg(rgb(color))
                            }))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x666666))
                                    .child(format!("{:.0}°", angle_max)),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(rgb(0x888888))
                            .child(format!(
                                "{} curves from {:.0}° to {:.0}° • Drag to pan • Scroll to zoom • Double-click to reset",
                                num_curves, angle_min, angle_max
                            )),
                    )
            }
            Err(e) => div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0xd32f2f))
                    .child(format!("Chart error: {}", e)),
            ),
        }
    }

    fn render_contour_plot(&self, _cx: &mut Context<Self>) -> Div {
        let Some(ref directivity) = self.directivity_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No directivity data available for contour plot."),
            );
        };

        let curves = &directivity.horizontal;

        if curves.is_empty() {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No horizontal directivity data available for contour plot."),
            );
        }

        // Build a 2D grid from directivity data
        // X axis: frequency (log spaced samples)
        // Y axis: angle
        // Z: SPL values

        // Sample frequencies logarithmically from 100Hz to 20kHz
        let num_freq_samples = 100;
        let freq_min = 100.0_f64;
        let freq_max = 20000.0_f64;
        let log_min = freq_min.log10();
        let log_max = freq_max.log10();

        let sample_freqs: Vec<f64> = (0..num_freq_samples)
            .map(|i| {
                let t = i as f64 / (num_freq_samples - 1) as f64;
                10_f64.powf(log_min + t * (log_max - log_min))
            })
            .collect();

        // Get angle values from curves
        let angle_values: Vec<f64> = curves.iter().map(|c| c.angle).collect();
        let num_angles = curves.len();

        // Build the Z data grid (angles x frequencies)
        // Data is in row-major order: z[row * width + col] where row 0 is at the bottom
        let mut z_data: Vec<f64> = Vec::with_capacity(num_angles * num_freq_samples);

        for curve in curves {
            for &target_freq in &sample_freqs {
                // Find the closest frequency in the curve data and interpolate
                let spl = interpolate_spl_at_freq(
                    curve.freq.as_slice().unwrap(),
                    curve.spl.as_slice().unwrap(),
                    target_freq,
                );
                z_data.push(spl);
            }
        }

        // Find SPL range for color scale
        let spl_min = z_data.iter().copied().fold(f64::INFINITY, f64::min);
        let spl_max = z_data.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        let angle_min = curves.first().map(|c| c.angle).unwrap_or(-60.0);
        let angle_max = curves.last().map(|c| c.angle).unwrap_or(60.0);

        // Build heatmap with proper axis configuration
        match heatmap(&z_data, num_freq_samples, num_angles)
            .title(format!(
                "Horizontal Contour - {}",
                self.selected_speaker.as_deref().unwrap_or("Unknown")
            ))
            .x(&sample_freqs)
            .y(&angle_values)
            .x_scale(ScaleType::Log)
            .color_scale(colormap_to_color_scale(self.selected_colormap))
            .size(900.0, 500.0)
            .build()
        {
            Ok(element) => {
                // Wrap chart with interactive handlers
                let interactive_chart = InteractiveChart::new(
                    "contour-chart",
                    element,
                    self.contour_chart_state.clone(),
                )
                .build();

                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(interactive_chart)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_4()
                            .p_4()
                            .bg(rgb(0xf5f5f5))
                            .rounded_md()
                            .child(div().text_sm().text_color(rgb(0x666666)).child(format!(
                                "X: Frequency ({:.0} Hz - {:.0} Hz, log scale)",
                                freq_min, freq_max
                            )))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x666666))
                                    .child(format!(
                                        "Y: Angle ({:.0}° to {:.0}°)",
                                        angle_min, angle_max
                                    )),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x666666))
                                    .child(format!(
                                        "SPL range: {:.1} dB to {:.1} dB",
                                        spl_min, spl_max
                                    )),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x888888))
                                    .child("Drag to pan • Scroll to zoom • Double-click to reset"),
                            ),
                    )
            }
            Err(e) => div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0xd32f2f))
                    .child(format!("Chart error: {}", e)),
            ),
        }
    }

    fn render_surface3d_plot(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(ref directivity) = self.directivity_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No directivity data available for 3D surface plot."),
            );
        };

        let curves = &directivity.horizontal;

        if curves.is_empty() {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No horizontal directivity data available for 3D surface plot."),
            );
        }

        // Build a 2D grid from directivity data
        // X axis: frequency (log spaced samples)
        // Y axis: angle
        // Z: SPL values

        // Sample frequencies logarithmically from 100Hz to 20kHz
        let num_freq_samples = 80;
        let freq_min = 100.0_f64;
        let freq_max = 20000.0_f64;
        let log_min = freq_min.log10();
        let log_max = freq_max.log10();

        let sample_freqs: Vec<f64> = (0..num_freq_samples)
            .map(|i| {
                let t = i as f64 / (num_freq_samples - 1) as f64;
                10_f64.powf(log_min + t * (log_max - log_min))
            })
            .collect();

        // Get angle values from curves
        let angle_values: Vec<f64> = curves.iter().map(|c| c.angle).collect();
        let num_angles = curves.len();

        // Build the Z data grid (angles x frequencies)
        // Data is in row-major order: z[row * width + col] where row 0 is at the bottom
        let mut z_data: Vec<f64> = Vec::with_capacity(num_angles * num_freq_samples);

        for curve in curves {
            for &target_freq in &sample_freqs {
                // Find the closest frequency in the curve data and interpolate
                let spl = interpolate_spl_at_freq(
                    curve.freq.as_slice().unwrap(),
                    curve.spl.as_slice().unwrap(),
                    target_freq,
                );
                z_data.push(spl);
            }
        }

        // Find SPL range for info display
        let spl_min = z_data.iter().copied().fold(f64::INFINITY, f64::min);
        let spl_max = z_data.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        let angle_min = curves.first().map(|c| c.angle).unwrap_or(-60.0);
        let angle_max = curves.last().map(|c| c.angle).unwrap_or(60.0);

        // Build 3D surface with proper axis configuration and shared state for interaction
        match surface3d(&z_data, num_freq_samples, num_angles)
            .title(format!(
                "Horizontal 3D Surface - {}",
                self.selected_speaker.as_deref().unwrap_or("Unknown")
            ))
            .x(&sample_freqs)
            .y(&angle_values)
            .x_log(true)
            .x_label("Frequency (Hz)")
            .y_label("Angle (°)")
            .z_label("SPL (dB)")
            .colormap(self.selected_colormap)
            .wireframe(false)
            .size(900.0, 600.0)
            .with_state(self.surface3d_state.clone())
            .build()
        {
            Ok(element) => {
                let state = self.surface3d_state.clone();

                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(
                        // Wrap the 3D surface in a container with mouse event handlers
                        div()
                            .id("surface3d-container")
                            .cursor(CursorStyle::PointingHand)
                            .child(element)
                            // Mouse drag for rotation
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |view, event: &MouseDownEvent, _window, _cx| {
                                    if event.click_count == 2 {
                                        // Double click - reset view
                                        let mut state = view.surface3d_state.borrow_mut();
                                        state.controls.reset();
                                        state.update_camera();
                                    } else {
                                        view.surface3d_dragging = true;
                                        view.surface3d_last_mouse = Some(event.position);
                                    }
                                }),
                            )
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|view, _event: &MouseUpEvent, _window, _cx| {
                                    view.surface3d_dragging = false;
                                }),
                            )
                            .on_mouse_move(cx.listener(
                                move |view, event: &MouseMoveEvent, _window, cx| {
                                    if view.surface3d_dragging {
                                        if let Some(last) = view.surface3d_last_mouse {
                                            let dx: f32 = (event.position.x - last.x).into();
                                            let dy: f32 = (event.position.y - last.y).into();

                                            let mut state = view.surface3d_state.borrow_mut();
                                            state.controls.rotate(dx, dy);
                                            state.update_camera();
                                            cx.notify();
                                        }
                                        view.surface3d_last_mouse = Some(event.position);
                                    }
                                },
                            ))
                            // Scroll wheel for zoom
                            .on_scroll_wheel(cx.listener({
                                let state = state.clone();
                                move |_view, event: &ScrollWheelEvent, _window, cx| {
                                    let delta = match event.delta {
                                        ScrollDelta::Lines(lines) => lines.y * 0.5,
                                        ScrollDelta::Pixels(pixels) => {
                                            let py: f32 = pixels.y.into();
                                            py * 0.01
                                        }
                                    };
                                    let mut state = state.borrow_mut();
                                    state.controls.zoom(delta);
                                    state.update_camera();
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_4()
                            .p_4()
                            .bg(rgb(0xf5f5f5))
                            .rounded_md()
                            .child(div().text_sm().text_color(rgb(0x666666)).child(format!(
                                "X: Frequency ({:.0} Hz - {:.0} Hz, log scale)",
                                freq_min, freq_max
                            )))
                            .child(div().text_sm().text_color(rgb(0x666666)).child(format!(
                                "Y: Angle ({:.0}° to {:.0}°)",
                                angle_min, angle_max
                            )))
                            .child(div().text_sm().text_color(rgb(0x666666)).child(format!(
                                "Z: SPL range ({:.1} dB to {:.1} dB)",
                                spl_min, spl_max
                            )))
                            .child(
                                div().text_sm().text_color(rgb(0x888888)).child(
                                    "Drag to rotate • Scroll to zoom • Double-click to reset",
                                ),
                            ),
                    )
            }
            Err(e) => div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0xd32f2f))
                    .child(format!("Chart error: {}", e)),
            ),
        }
    }
}

/// Convert Colormap (3D) to ColorScale (2D heatmap)
fn colormap_to_color_scale(colormap: Colormap) -> ColorScale {
    match colormap {
        Colormap::Viridis => ColorScale::Viridis,
        Colormap::Plasma => ColorScale::Plasma,
        Colormap::Inferno => ColorScale::Inferno,
        Colormap::Turbo => ColorScale::Inferno, // Turbo not available in ColorScale, use Inferno
        Colormap::CoolWarm => ColorScale::Coolwarm,
    }
}

/// Interpolate SPL value at a specific frequency from curve data
fn interpolate_spl_at_freq(freqs: &[f64], spls: &[f64], target_freq: f64) -> f64 {
    if freqs.is_empty() || spls.is_empty() {
        return 0.0;
    }

    // Find the two frequencies that bracket the target
    let mut lower_idx = 0;
    for (i, &f) in freqs.iter().enumerate() {
        if f <= target_freq {
            lower_idx = i;
        } else {
            break;
        }
    }

    if lower_idx >= freqs.len() - 1 {
        return spls.last().copied().unwrap_or(0.0);
    }

    let f1 = freqs[lower_idx];
    let f2 = freqs[lower_idx + 1];
    let s1 = spls[lower_idx];
    let s2 = spls[lower_idx + 1];

    // Linear interpolation in log-frequency space
    if f2 <= f1 {
        return s1;
    }

    let log_f1 = f1.log10();
    let log_f2 = f2.log10();
    let log_target = target_freq.log10();

    let t = (log_target - log_f1) / (log_f2 - log_f1);
    s1 + t * (s2 - s1)
}

impl Render for SpinoramaApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("main-container")
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .child(self.render_header(cx))
            .child(self.render_content(cx))
    }
}
