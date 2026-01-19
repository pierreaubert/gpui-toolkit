use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use autoeq::read::{
    ContourPlotData, extract_cea2034_curves_original, fetch_available_speakers, fetch_contour_data,
    fetch_directivity_data, fetch_measurement_plot_data,
};
use autoeq::{Curve, DirectivityData};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::brush::{BrushSelection, BrushState};
use d3rs::color::D3Color;
use d3rs::contour::ContourGenerator;
use d3rs::gpu2d::{
    ContourConfig, HeatmapData, render_contour, render_contour_bands, render_heatmap,
};
use d3rs::grid::{GridConfig, render_grid};
use d3rs::prelude::{LinearScale, LogScale};
// Radial shape functions could be used in future - currently using canvas-based custom rendering
// use d3rs::shape::radial::{polar_grid_circles, polar_grid_rays, radial_line, RadialLineConfig, RadialPoint};
use d3rs::gpu3d::{
    Colormap as Surface3DColormap, Surface3DConfig, Surface3DElement, Surface3DState,
    SurfaceData as Surface3DData, SurfacePlotType,
};
use d3rs::text::{VectorFontConfig, render_vector_text};
use d3rs::zoom::ZoomState;
use gpui::prelude::*;
use gpui::{deferred, *};
use gpui_ui_kit::{SelectOption, Spinner, SpinnerSize};
use tokio::runtime::Runtime;

use super::render::render_freq_spl_plot;
use super::types::{
    BrushOverlay, ChartId, Colormap, ContourRenderMode, DirectivityPlane, LinePoint, LoadState,
    PlotCurve, PlotSection, SecondaryAxisConfig,
};

mod render_sphere;
use super::utils::{
    CEA2034_CURVES, cea2034_colors, format_frequency, get_angle_range, interpolate_spl_at_frequency,
};

/// Main application state
pub struct SpinoramaApp {
    pub runtime: Arc<Runtime>,
    // Speaker list
    pub speakers: Vec<String>,
    pub speakers_load_state: LoadState,
    // Version list for selected speaker
    pub versions: Vec<String>,
    pub versions_load_state: LoadState,
    // Selection state
    pub selected_speaker: Option<String>,
    pub selected_version: Option<String>,
    pub selected_measurement: String,
    // Data state
    pub cea2034_curves: HashMap<String, Curve>,
    pub directivity_data: Option<DirectivityData>,
    pub contour_data: Option<ContourPlotData>,
    pub data_load_state: LoadState,
    // UI state
    pub current_section: PlotSection,
    pub speaker_dropdown_open: bool,
    pub version_dropdown_open: bool,
    pub section_dropdown_open: bool,
    // Contour render mode for each plot (SPL Horizontal Contour, Directivity Contour)
    pub contour_mode_spl: ContourRenderMode,
    pub contour_mode_directivity: ContourRenderMode,
    // Colormap selection for contour plots
    pub contour_colormap: Colormap,
    // Zoom state for frequency/SPL plots (CEA2034, horizontal/vertical SPL)
    pub freq_spl_zoom: ZoomState,
    pub freq_spl_brush: BrushState,
    // Zoom state for SPL contour plot
    pub spl_contour_zoom: ZoomState,
    pub spl_contour_brush: BrushState,
    // Zoom state for directivity contour plot
    pub directivity_contour_zoom: ZoomState,
    pub directivity_contour_brush: BrushState,
    // Track which chart is currently being brushed (for event handling)
    pub active_brush_chart: Option<ChartId>,
    // Chart bounds for mouse position calculation (window-relative)
    // These are shared via Rc<RefCell> to allow capture in closures
    pub freq_spl_chart_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    pub spl_contour_chart_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    pub directivity_contour_chart_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,

    // Polar directivity plot state
    pub polar_selected_frequencies: Vec<f64>,
    pub polar_plane: DirectivityPlane,

    // 3D surface plot state
    pub surface_rotation_azimuth: f32,
    pub surface_rotation_elevation: f32,
    pub surface_state: Rc<RefCell<Surface3DState>>,
    pub surface_wireframe: bool,
    pub surface_isolines: bool,
    pub surface_opacity: f32,
    pub surface_plot_type: SurfacePlotType,
    pub surface_show_grid: bool,
    pub sphere_freq_idx: usize,

    // Polar contour plot state
    pub polar_contour_freq_range: (f64, f64),
}
impl SpinoramaApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
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
            contour_data: None,
            data_load_state: LoadState::Idle,
            current_section: PlotSection::default(),
            speaker_dropdown_open: false,
            version_dropdown_open: false,
            section_dropdown_open: false,
            contour_mode_spl: ContourRenderMode::default(),
            contour_mode_directivity: ContourRenderMode::default(),
            contour_colormap: Colormap::default(),
            // Zoom for freq/SPL plots: X=20Hz-20kHz (log), Y=-40 to 10 dB (linear)
            freq_spl_zoom: ZoomState::new(20.0, 20000.0, -40.0, 10.0).with_log_x(true),
            freq_spl_brush: BrushState::new(),
            // Zoom for SPL contour: X=100Hz-20kHz (log), Y=-180 to 180 deg (linear)
            spl_contour_zoom: ZoomState::new(100.0, 20000.0, -180.0, 180.0).with_log_x(true),
            spl_contour_brush: BrushState::new(),
            // Zoom for directivity contour: same ranges
            directivity_contour_zoom: ZoomState::new(100.0, 20000.0, -180.0, 180.0)
                .with_log_x(true),
            directivity_contour_brush: BrushState::new(),
            active_brush_chart: None,
            // Initialize chart bounds as None - will be captured during render
            freq_spl_chart_bounds: Rc::new(RefCell::new(None)),
            spl_contour_chart_bounds: Rc::new(RefCell::new(None)),
            directivity_contour_chart_bounds: Rc::new(RefCell::new(None)),

            // Polar directivity plot state - default frequencies at octaves
            polar_selected_frequencies: vec![100.0, 1000.0, 10000.0],
            polar_plane: DirectivityPlane::default(),

            // 3D surface plot state
            surface_rotation_azimuth: 45.0,
            surface_rotation_elevation: 30.0,
            surface_state: Rc::new(RefCell::new(Surface3DState::new(3.5, 45.0, 30.0))),
            surface_wireframe: false,
            surface_isolines: false,
            surface_opacity: 1.0,
            surface_show_grid: true,

            // Polar contour frequency range (20Hz - 20kHz)
            surface_plot_type: SurfacePlotType::Cartesian,
            sphere_freq_idx: 0,
            polar_contour_freq_range: (20.0, 20000.0),
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
            // Fetch CEA2034 data
            let cea2034_result: Result<HashMap<String, Curve>, String> = runtime
                .spawn({
                    let speaker = speaker.clone();
                    let version = version.clone();
                    let measurement = measurement.clone();
                    async move {
                        let plot_data =
                            fetch_measurement_plot_data(&speaker, &version, &measurement)
                                .await
                                .map_err(|e| e.to_string())?;
                        extract_cea2034_curves_original(&plot_data, &measurement)
                            .map_err(|e| e.to_string())
                    }
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);

            // Fetch directivity data
            let directivity_result: Option<DirectivityData> = runtime
                .spawn({
                    let speaker = speaker.clone();
                    let version = version.clone();
                    async move { fetch_directivity_data(&speaker, &version).await.ok() }
                })
                .await
                .ok()
                .flatten();

            // Fetch contour data (SPL Horizontal Contour)
            let contour_result: Option<ContourPlotData> = runtime
                .spawn({
                    let speaker = speaker.clone();
                    let version = version.clone();
                    async move {
                        fetch_contour_data(&speaker, &version, "horizontal")
                            .await
                            .ok()
                    }
                })
                .await
                .ok()
                .flatten();

            match cea2034_result {
                Ok(curves) => {
                    let _ = this.update(cx, |app, cx| {
                        app.cea2034_curves = curves;
                        app.directivity_data = directivity_result;
                        app.contour_data = contour_result;
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

    pub fn render_header(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
            .when(has_speaker, |el| {
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
            // Loading indicator
            .when(is_loading_data, |el| {
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
                    .child(div().text_xs().text_color(rgb(0x666666)).child("▼"))
                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                        println!("Speaker dropdown clicked!");
                        entity_for_toggle.update(cx, |this, cx| {
                            this.speaker_dropdown_open = !this.speaker_dropdown_open;
                            this.version_dropdown_open = false;
                            this.section_dropdown_open = false;
                            println!(
                                "Speaker dropdown open: {}, speakers count: {}",
                                this.speaker_dropdown_open,
                                this.speakers.len()
                            );
                            cx.notify();
                        });
                    }),
            )
            .when(is_open, |el| {
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
                                    .when(is_selected, |el| {
                                        el.bg(rgb(0x007acc)).text_color(rgb(0xffffff))
                                    })
                                    .when(!is_selected, |el| {
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
                                            this.contour_data = None;
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
                    .child(div().text_xs().text_color(rgb(0x666666)).child("▼"))
                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                        entity_for_toggle.update(cx, |this, cx| {
                            this.version_dropdown_open = !this.version_dropdown_open;
                            this.speaker_dropdown_open = false;
                            this.section_dropdown_open = false;
                            cx.notify();
                        });
                    }),
            )
            .when(is_open, |el| {
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
                                    .when(is_selected, |el| {
                                        el.bg(rgb(0x007acc)).text_color(rgb(0xffffff))
                                    })
                                    .when(!is_selected, |el| {
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
                    .child(div().text_xs().text_color(rgb(0x666666)).child("▼"))
                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                        entity_for_toggle.update(cx, |this, cx| {
                            this.section_dropdown_open = !this.section_dropdown_open;
                            this.speaker_dropdown_open = false;
                            this.version_dropdown_open = false;
                            cx.notify();
                        });
                    }),
            )
            .when(is_open, |el| {
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
                                    .when(is_selected, |el| {
                                        el.bg(rgb(0x007acc)).text_color(rgb(0xffffff))
                                    })
                                    .when(!is_selected, |el| {
                                        el.text_color(rgb(0xcccccc)).hover(|s| s.bg(rgb(0x3a3a3a)))
                                    })
                                    .child(opt.label)
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        let section = PlotSection::all()
                                            .into_iter()
                                            .find(|s| s.label() == label_str)
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

    pub fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let content: Div = match self.data_load_state {
            LoadState::Idle => self.render_welcome(),
            LoadState::Loading => self.render_loading(),
            LoadState::Error(ref e) => self.render_error(e),
            LoadState::Loaded => match self.current_section {
                PlotSection::CEA2034 => self.render_cea2034_plot(cx),
                PlotSection::HorizontalSPL => self.render_directivity_plot("horizontal", cx),
                PlotSection::VerticalSPL => self.render_directivity_plot("vertical", cx),
                PlotSection::Contour => self.render_contour_plot(cx),
                PlotSection::PolarDirectivity => self.render_polar_directivity_plot(cx),
                PlotSection::Surface3D => self.render_surface_3d_plot(cx),
                PlotSection::SurfaceSphere => self.render_sphere_plot(cx),
                PlotSection::PolarContour => self.render_polar_contour_plot(cx),
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
                this.section_dropdown_open = false;
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
                    .child("Spinorama Viewer"),
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .max_w(px(400.0))
                    .text_center()
                    .child("Select a speaker from the dropdown above to view its frequency response measurements from spinorama.org."),
            )
    }

    /// Wrap a chart element with interactive mouse handlers for pan/drag and zoom
    pub fn wrap_freq_spl_chart_interactive(
        &self,
        chart: Div,
        chart_id: ChartId,
        chart_width: f32,
        chart_height: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let entity = cx.entity().clone();
        let entity2 = entity.clone();
        let entity3 = entity.clone();
        let entity4 = entity.clone();

        // Get the chart bounds Rc for this chart type
        let chart_bounds = match chart_id {
            ChartId::FreqSpl => self.freq_spl_chart_bounds.clone(),
            ChartId::SplContour => self.spl_contour_chart_bounds.clone(),
            ChartId::DirectivityContour => self.directivity_contour_chart_bounds.clone(),
        };
        let chart_bounds_for_move = chart_bounds.clone();
        let chart_bounds_for_prepaint = chart_bounds.clone();

        // Left axis spacer width (from render_freq_spl_plot)
        let left_margin = 80.0_f32;

        // Track drag start position for pan
        let drag_start: Rc<RefCell<Option<(f32, f32)>>> = Rc::new(RefCell::new(None));
        let drag_start_down = drag_start.clone();
        let drag_start_move = drag_start.clone();
        let drag_start_up = drag_start.clone();

        // Helper to convert window position to chart-relative coordinates
        // using the stored chart bounds
        fn to_chart_coords(
            pos: Point<Pixels>,
            bounds: &Option<Bounds<Pixels>>,
            left_margin: f32,
            chart_width: f32,
            chart_height: f32,
        ) -> (f32, f32) {
            if let Some(b) = bounds {
                // Subtract wrapper origin to get element-relative coordinates
                let rel_x = f32::from(pos.x) - f32::from(b.origin.x) - left_margin;
                let rel_y = f32::from(pos.y) - f32::from(b.origin.y);
                (
                    rel_x.max(0.0).min(chart_width),
                    rel_y.max(0.0).min(chart_height),
                )
            } else {
                // Fallback: no bounds captured yet, use raw position with just X margin
                let chart_x = (f32::from(pos.x) - left_margin).max(0.0).min(chart_width);
                let chart_y = f32::from(pos.y).max(0.0).min(chart_height);
                (chart_x, chart_y)
            }
        }

        // Outer wrapper that captures bounds via on_children_prepainted
        div()
            .on_children_prepainted(move |children_bounds, _window, _cx| {
                // The first child is our inner interactive div - store its bounds
                if let Some(inner_bounds) = children_bounds.first() {
                    *chart_bounds_for_prepaint.borrow_mut() = Some(*inner_bounds);
                }
            })
            .child(
                // Inner div with mouse handlers and id
                div()
                    .id(match chart_id {
                        ChartId::FreqSpl => "freq-spl-chart",
                        ChartId::SplContour => "spl-contour-chart",
                        ChartId::DirectivityContour => "directivity-contour-chart",
                    })
                    .relative()
                    .cursor_grab()
                    .child(chart)
                    // Mouse down to start pan
                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                        let pos = event.position;
                        let bounds = chart_bounds.borrow();
                        let (chart_x, chart_y) =
                            to_chart_coords(pos, &bounds, left_margin, chart_width, chart_height);

                        *drag_start_down.borrow_mut() = Some((chart_x, chart_y));

                        entity.update(cx, |this, _cx| {
                            this.active_brush_chart = Some(chart_id);
                        });
                    })
                    // Mouse move to pan during drag
                    .on_mouse_move(move |event, _window, cx| {
                        if let Some((start_x, start_y)) = *drag_start_move.borrow() {
                            let pos = event.position;
                            let bounds = chart_bounds_for_move.borrow();
                            let (chart_x, chart_y) = to_chart_coords(
                                pos,
                                &bounds,
                                left_margin,
                                chart_width,
                                chart_height,
                            );

                            let dx = chart_x - start_x;
                            let dy = chart_y - start_y;

                            if dx.abs() > 1.0 || dy.abs() > 1.0 {
                                entity2.update(cx, |this, cx| {
                                    this.apply_pan(chart_id, dx, dy, chart_width, chart_height);
                                    cx.notify();
                                });
                                // Update drag start for continuous panning
                                *drag_start_move.borrow_mut() = Some((chart_x, chart_y));
                            }
                        }
                    })
                    // Mouse up to end pan
                    .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                        *drag_start_up.borrow_mut() = None;
                        entity3.update(cx, |this, _cx| {
                            this.active_brush_chart = None;
                        });
                    })
                    // Double click to reset zoom
                    .on_click(move |event, _window, cx| {
                        // Check for double-click (click_count >= 2)
                        if event.click_count() >= 2 {
                            entity4.update(cx, |this, cx| {
                                match chart_id {
                                    ChartId::FreqSpl => this.freq_spl_zoom.reset(),
                                    ChartId::SplContour => this.spl_contour_zoom.reset(),
                                    ChartId::DirectivityContour => {
                                        this.directivity_contour_zoom.reset()
                                    }
                                }
                                cx.notify();
                            });
                        }
                    }),
            )
    }

    /// Convert pixel selection to domain coordinates and apply zoom
    fn apply_zoom_from_selection(
        &mut self,
        chart_id: ChartId,
        sel: BrushSelection,
        chart_width: f32,
        chart_height: f32,
    ) {
        let (zoom_state, is_log_x) = match chart_id {
            ChartId::FreqSpl => (&mut self.freq_spl_zoom, true),
            ChartId::SplContour => (&mut self.spl_contour_zoom, true),
            ChartId::DirectivityContour => (&mut self.directivity_contour_zoom, true),
        };

        // Get current domain for scale inversion
        let x_domain = zoom_state.x_domain();
        let y_domain = zoom_state.y_domain();

        // Y-axis uses inverted range: pixel 0 = top = domain max, pixel height = bottom = domain min
        // This matches screen coordinates where Y increases downward but domain values increase upward
        let y_scale = LinearScale::new()
            .domain(y_domain.0, y_domain.1)
            .range(chart_height as f64, 0.0);

        // Convert pixel coordinates to domain
        // The brush module now does direct inversion without Y-swap
        let (x0, x1, y0, y1) = if is_log_x {
            let x_scale = LogScale::new()
                .domain(x_domain.0, x_domain.1)
                .range(0.0, chart_width as f64);
            let domain_sel = sel.to_domain(&x_scale, &y_scale);
            (domain_sel.x0, domain_sel.x1, domain_sel.y0, domain_sel.y1)
        } else {
            let x_scale = LinearScale::new()
                .domain(x_domain.0, x_domain.1)
                .range(0.0, chart_width as f64);
            let domain_sel = sel.to_domain(&x_scale, &y_scale);
            (domain_sel.x0, domain_sel.x1, domain_sel.y0, domain_sel.y1)
        };

        // Ensure x0 < x1 and y0 < y1 for zoom_to
        let (x_min, x_max) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };

        // Apply zoom
        zoom_state.zoom_to(x_min, x_max, y_min, y_max);
    }

    /// Apply pan/drag by converting pixel delta to domain delta
    fn apply_pan(
        &mut self,
        chart_id: ChartId,
        dx: f32,
        dy: f32,
        chart_width: f32,
        chart_height: f32,
    ) {
        let (zoom_state, is_log_x) = match chart_id {
            ChartId::FreqSpl => (&mut self.freq_spl_zoom, true),
            ChartId::SplContour => (&mut self.spl_contour_zoom, true),
            ChartId::DirectivityContour => (&mut self.directivity_contour_zoom, true),
        };

        let (x_min, x_max) = zoom_state.x_domain();
        let (y_min, y_max) = zoom_state.y_domain();

        // Convert pixel delta to domain delta
        let (new_x_min, new_x_max) = if is_log_x {
            // For log scale, pan in log space
            let log_min = x_min.ln();
            let log_max = x_max.ln();
            let log_range = log_max - log_min;
            let log_delta = -(dx as f64) * log_range / (chart_width as f64);
            (
                (log_min + log_delta).exp(),
                (log_max + log_delta).exp(),
            )
        } else {
            let x_range = x_max - x_min;
            let domain_dx = -(dx as f64) * x_range / (chart_width as f64);
            (x_min + domain_dx, x_max + domain_dx)
        };

        // Y is linear, and inverted (screen Y increases downward)
        let y_range = y_max - y_min;
        let domain_dy = (dy as f64) * y_range / (chart_height as f64);
        let new_y_min = y_min + domain_dy;
        let new_y_max = y_max + domain_dy;

        // Apply the pan by zooming to the new domain
        zoom_state.zoom_to(new_x_min, new_x_max, new_y_min, new_y_max);
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

include!("render_cea2034.rs");
include!("render_directivity.rs");
include!("render_contour.rs");
include!("render_contour_2d.rs");
include!("render_contour_3d.rs");
include!("render_contour_old.rs");
include!("render_contour_polar.rs");
include!("render_legend.rs");
include!("render_mode_toggle.rs");
include!("render_polar.rs");
