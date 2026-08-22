use super::initial_lab_state::InitialLabState;
use super::lab_app_config::LabAppConfig;
use super::misc::alert_variant;
use super::misc::badge_variant;
use super::misc::boxplot_story_data;
use super::misc::button_variant;
use super::misc::clamp_f32;
use super::misc::color_scale;
use super::misc::confirm_dialog_variant;
use super::misc::design_for_theme_preset;
use super::misc::icon_button_variant;
use super::misc::lab_id;
use super::misc::live_reload_status;
use super::misc::notification_variant;
use super::misc::progress_variant;
use super::misc::prop_value_label;
use super::misc::scalar_field_data;
use super::misc::scatter_story_data;
use super::misc::showcase_section_for_story_id;
use super::misc::spectrum_axis_magnitudes;
use super::misc::spectrum_magnitudes;
use super::misc::surface_colormap;
use super::misc::tab_variant;
use super::misc::tag_variant;
use super::misc::tiling_method;
use super::misc::toast_variant;
use super::misc::treemap_story_data;
use super::misc::ui_kit_exported_component_story_id;
use super::number::number_prop;
use super::number::number_step;
use super::preview_align::PreviewAlign;
use super::preview_align::apply_preview_builder_style;
use super::preview_layout_constraints::PreviewLayoutConstraints;
use super::preview_overflow::PreviewOverflow;
use super::preview_sizing::PreviewSizing;
use super::preview_surface::PreviewSurface;
use super::render::render_chart_error;
use super::render::render_chart_result;
use super::sample::sample_wizard_steps;
use super::sample::sample_workflow_graph;
use super::story::bool_prop;
use super::story::choice_prop;
use super::story::story_file_name;
use super::story::text_prop;
use super::types::area_story_data;
use super::types::bar_story_data;
use super::types::line_story_data;
use crate::{
    ComponentStory, LivePreviewReload, MotionPreset, ResponsivePreviewMatrix, StoryDocument,
    StoryProp, StoryPropValue, StoryRegistry, StoryRendererRegistry, ThemePreset, ViewportPreset,
    builtin_story_registry, builtin_story_renderers, latest_story_or_token_modified,
    load_story_documents, reload_live_preview_state,
};
use anyhow::{Context as AnyhowContext, Result};
use d3rs::mesh::{
    ContourLevels, CoordinateAxis, RevolveSpec, ScalarAssociation, ScalarField, TriangleMesh,
};
use gpui::prelude::*;
use gpui::{
    AnyElement, Context, Entity, IntoElement, MouseButton, Pixels, Render, SharedString, Size,
    WeakEntity, Window, div, px, relative,
};
use gpui_audio_kit::{
    AudioScale, HorizontalMeterTheme, LevelMeterElement, Potentiometer, PotentiometerSize,
    SpectrumAxisTheme, SpectrumElement, TickConfig, VerticalSlider, VerticalSliderSize, VolumeKnob,
    render_horizontal_meter_bar, render_spectrum_db_axis, render_spectrum_frequency_axis,
    render_tick_row,
};
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_px::{
    ColorScale, LegendPosition, MeshPlotPick, MeshPlotView, PlotInteractions, StrokeDashArray,
    area, bar, boxplot, contour, donut, heatmap, isoline, line, mesh_plot, pie, scatter, surface3d,
    treemap,
};
use gpui_showcase::showcase::{Showcase, ShowcaseSection};
use gpui_ui_kit::qr::AnimatedQrCode;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::{
    Accordion, AccordionItem, Alert, Avatar, AvatarGroup, AvatarShape, AvatarSize, AvatarStatus,
    Badge, BadgeDot, BadgeSize, BadgeVariant, BreadcrumbItem, Breadcrumbs, Button, ButtonSet,
    ButtonSetOption, ButtonSize, ButtonVariant, Card, Checkbox, CheckboxSize, CircularProgress,
    Code, Color, ColorPickerView, Column, CommandItem, CommandPalette, ConfirmDialog, ContextMenu,
    DesignSystem, Dialog, DialogSize, Divider, DragItem, DragList, EmptyState, FocusDirection,
    FocusGroup, HStack, Heading, IconButton, IconButtonSize, ImageView, InlineAlert, Input,
    InputSize, KeyboardShortcutLabel, KeyboardShortcutSize, Link, LoadingDots, LoadingOverlay,
    Menu, MenuBar, MenuBarItem, MenuItem, Notification, NumberInput, NumberInputSize, PaneDivider,
    Popover, Port, PortDirection, Position, Progress, ProgressSize, QrCode, SearchBar,
    SearchBarSize, Select, SelectOption, SelectSize, SettingsForm, SettingsRow, Sidebar, Slider,
    Spacer, Spinner, SpinnerSize, SplitDirection, SplitPane, StatusBar, StepIndicator,
    StepIndicatorSize, StepItem, StepItemStatus, StepOrientation, StepStatus, TabItem, Table, Tabs,
    Tag, Text, TextSize, TextWeight, Toast, ToastContainer, ToastPosition, Toggle, ToggleSize,
    ToggleStyle, Toolbar, ToolbarItem, Tooltip, TreeNode, TreeView, VStack, WithTooltip, Wizard,
    WizardHeader, WizardNavigation, WizardVariant, WorkflowCanvas, WorkflowNode, WorkflowNodeData,
};
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};

fn mesh_plot_square_mesh(id: &str, with_ids: bool) -> TriangleMesh {
    static POSITIONS: OnceLock<Arc<[[f64; 3]]>> = OnceLock::new();
    static TRIANGLES: OnceLock<Arc<[[u32; 3]]>> = OnceLock::new();
    static VERTEX_IDS: OnceLock<Arc<[u64]>> = OnceLock::new();
    static CELL_IDS: OnceLock<Arc<[u64]>> = OnceLock::new();
    TriangleMesh {
        id: id.into(),
        positions: POSITIONS
            .get_or_init(|| {
                Arc::from([
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0],
                ])
            })
            .clone(),
        triangles: TRIANGLES
            .get_or_init(|| Arc::from([[0, 1, 2], [0, 2, 3]]))
            .clone(),
        vertex_ids: with_ids.then(|| {
            VERTEX_IDS
                .get_or_init(|| Arc::from([100, 101, 102, 103]))
                .clone()
        }),
        cell_ids: with_ids.then(|| CELL_IDS.get_or_init(|| Arc::from([2000, 2001])).clone()),
    }
}

fn mesh_plot_square_vertex_field(id: &str) -> ScalarField {
    static VALUES: OnceLock<Arc<[f64]>> = OnceLock::new();
    ScalarField {
        id: id.into(),
        label: "Response".into(),
        unit: Some("dB".into()),
        values: VALUES
            .get_or_init(|| Arc::from([0.0, 1.0, 2.0, 0.5]))
            .clone(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn mesh_plot_square_cell_field(id: &str) -> ScalarField {
    static VALUES: OnceLock<Arc<[f64]>> = OnceLock::new();
    ScalarField {
        id: id.into(),
        label: "Cell response".into(),
        unit: Some("dB".into()),
        values: VALUES.get_or_init(|| Arc::from([0.25, 1.25])).clone(),
        association: ScalarAssociation::Cell,
        valid: None,
    }
}

fn mesh_plot_saddle_mesh(id: &str) -> TriangleMesh {
    static GEOMETRY: OnceLock<(Arc<[[f64; 3]]>, Arc<[[u32; 3]]>)> = OnceLock::new();
    let (positions, triangles) = GEOMETRY.get_or_init(|| {
        (
            Arc::from([
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.5, 0.5, 0.0],
            ]),
            Arc::from([[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]]),
        )
    });
    TriangleMesh {
        id: id.into(),
        positions: positions.clone(),
        triangles: triangles.clone(),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn mesh_plot_saddle_field(id: &str) -> ScalarField {
    static VALUES: OnceLock<Arc<[f64]>> = OnceLock::new();
    ScalarField {
        id: id.into(),
        label: "Saddle".into(),
        unit: None,
        values: VALUES
            .get_or_init(|| Arc::from([-1.0, 1.0, -1.0, 1.0, 0.0]))
            .clone(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn mesh_plot_annulus_mesh(id: &str) -> TriangleMesh {
    static GEOMETRY: OnceLock<(Arc<[[f64; 3]]>, Arc<[[u32; 3]]>)> = OnceLock::new();
    let (positions, triangles) = GEOMETRY.get_or_init(|| {
        (
            Arc::from([
                [0.35, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.35, 0.0, 1.0],
            ]),
            Arc::from([[0, 1, 2], [0, 2, 3]]),
        )
    });
    TriangleMesh {
        id: id.into(),
        // The X/Z plane is interpreted as radial/axial by the axisymmetric
        // stories. The inner radius stays positive so the revolve fixture is
        // an annulus rather than a degenerate disk.
        positions: positions.clone(),
        triangles: triangles.clone(),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn mesh_plot_annulus_field(id: &str) -> ScalarField {
    static VALUES: OnceLock<Arc<[f64]>> = OnceLock::new();
    ScalarField {
        id: id.into(),
        label: "Radial response".into(),
        unit: Some("dB".into()),
        values: VALUES
            .get_or_init(|| Arc::from([0.1, 0.9, 1.4, 0.4]))
            .clone(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn mesh_plot_surface_mesh(id: &str) -> TriangleMesh {
    static GEOMETRY: OnceLock<(Arc<[[f64; 3]]>, Arc<[[u32; 3]]>)> = OnceLock::new();
    let (positions, triangles) = GEOMETRY.get_or_init(|| {
        (
            Arc::from([
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.35],
                [1.0, 1.0, 0.9],
                [0.0, 1.0, 0.25],
                [0.5, 0.5, 0.8],
            ]),
            Arc::from([[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]]),
        )
    });
    TriangleMesh {
        id: id.into(),
        positions: positions.clone(),
        triangles: triangles.clone(),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn mesh_plot_surface_field(id: &str) -> ScalarField {
    static VALUES: OnceLock<Arc<[f64]>> = OnceLock::new();
    ScalarField {
        id: id.into(),
        label: "Surface response".into(),
        unit: Some("dB".into()),
        values: VALUES
            .get_or_init(|| Arc::from([0.0, 0.8, 1.6, 0.3, 1.1]))
            .clone(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn mesh_plot_large_mesh(id: &str) -> TriangleMesh {
    const GRID: usize = 128;
    static GEOMETRY: OnceLock<(Arc<[[f64; 3]]>, Arc<[[u32; 3]]>)> = OnceLock::new();
    let (positions, triangles) = GEOMETRY.get_or_init(|| {
        let vertex_count = (GRID + 1) * (GRID + 1);
        let mut positions = Vec::with_capacity(vertex_count);
        let mut triangles = Vec::with_capacity(GRID * GRID * 2);

        for y in 0..=GRID {
            let v = y as f64 / GRID as f64;
            for x in 0..=GRID {
                let u = x as f64 / GRID as f64;
                let dx = u - 0.5;
                let dy = v - 0.5;
                let z = 0.22 * (dx * 18.0).sin() * (dy * 18.0).cos()
                    + 0.12 * (dx * dx + dy * dy).sqrt().cos();
                positions.push([u, v, z]);
            }
        }

        for y in 0..GRID {
            for x in 0..GRID {
                let top_left = y * (GRID + 1) + x;
                let top_right = top_left + 1;
                let bottom_left = (y + 1) * (GRID + 1) + x;
                let bottom_right = bottom_left + 1;
                triangles.push([top_left as u32, top_right as u32, bottom_right as u32]);
                triangles.push([top_left as u32, bottom_right as u32, bottom_left as u32]);
            }
        }
        (positions.into(), triangles.into())
    });

    TriangleMesh {
        id: id.into(),
        positions: positions.clone(),
        triangles: triangles.clone(),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn mesh_plot_large_field(id: &str) -> ScalarField {
    const GRID: usize = 128;
    static VALUES: OnceLock<Arc<[f64]>> = OnceLock::new();
    let values = VALUES.get_or_init(|| {
        let mut values = Vec::with_capacity((GRID + 1) * (GRID + 1));
        for y in 0..=GRID {
            let v = y as f64 / GRID as f64;
            for x in 0..=GRID {
                let u = x as f64 / GRID as f64;
                values.push((u * 14.0).sin() * (v * 11.0).cos());
            }
        }
        values.into()
    });

    ScalarField {
        id: id.into(),
        label: "Large mesh response".into(),
        unit: Some("a.u.".into()),
        values: values.clone(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

/// Launch the interactive GPUI component lab.
pub fn run_lab_app(config: LabAppConfig) -> Result<()> {
    let app_config = MiniAppConfig::new("gpui-component-lab")
        .size(1440.0, 920.0)
        .with_theme(true)
        .scrollable(false);
    MiniApp::run(app_config, move |cx| {
        let config = config.clone();
        cx.new(|cx| ComponentLab::new(config, cx))
    });
    Ok(())
}

/// Interactive storybook/designer view.
pub struct ComponentLab {
    pub(super) registry: StoryRegistry,
    pub(super) renderers: StoryRendererRegistry,
    pub(super) documents: BTreeMap<String, StoryDocument>,
    pub(super) story_ids: Vec<String>,
    pub(super) ui_showcases: BTreeMap<String, Entity<Showcase>>,
    pub(super) selected_story_id: String,
    pub(super) selected_viewport_id: String,
    pub(super) selected_theme_id: String,
    pub(super) selected_motion_id: String,
    pub(super) matrix_mode: bool,
    pub(super) layout_constraints: PreviewLayoutConstraints,
    layout_state_dirty: bool,
    pub(super) save_status: Option<SharedString>,
    pub(super) live_status: Option<SharedString>,
    pub(super) live_preview: bool,
    pub(super) last_live_modified: SystemTime,
    pub(super) stories_dir: PathBuf,
    pub(super) token_paths: Vec<PathBuf>,
    pub(super) entity: Entity<Self>,
    pub(super) cached_matrix: ResponsivePreviewMatrix,
    pub(super) sidebar_labels: BTreeMap<String, SharedString>,
    // Persistent child render entities to avoid rebuilding stable UI every frame.
    sidebar_entity: Entity<LabSidebar>,
    toolbar_entity: Entity<LabToolbar>,
    controls_panel_entity: Entity<LabControlsPanel>,
    preview_area_entity: Entity<LabPreviewArea>,
    // Allocation probe for tracking heap allocations during interactive events.
    alloc_probe: gpui_profiler::AllocProbe,
    last_render_alloc: gpui_profiler::AllocSnapshot,
    last_mouse_move_alloc: gpui_profiler::AllocSnapshot,
    last_sample: Option<(&'static str, gpui_profiler::AllocSnapshot)>,
    last_window_size: Option<Size<Pixels>>,
    visual_capture_mode: bool,
}

impl ComponentLab {
    pub(super) fn new(config: LabAppConfig, cx: &mut Context<Self>) -> Self {
        let registry = builtin_story_registry().expect("builtin story registry");
        let renderers = builtin_story_renderers().expect("builtin story renderers");
        let mut documents: BTreeMap<String, StoryDocument> = registry
            .stories()
            .cloned()
            .map(|story| (story.id.clone(), StoryDocument::new(story)))
            .collect();

        if let Ok(loaded_docs) = load_story_documents(&config.stories_dir) {
            for doc in loaded_docs {
                documents.insert(doc.story.id.clone(), doc);
            }
        }

        let story_ids: Vec<String> = registry.stories().map(|story| story.id.clone()).collect();
        let visual_capture = config.visual_capture.clone();
        let selected_story_id = visual_capture
            .as_ref()
            .map(|capture| capture.story_id.clone())
            .filter(|story_id| documents.contains_key(story_id))
            .or_else(|| story_ids.first().cloned())
            .unwrap_or_default();
        // A showcase owns a full component-demo tree. Retain only the selected one at
        // startup; subsequent showcase stories are initialized on first selection.
        let ui_showcases = build_ui_showcase_entities(std::slice::from_ref(&selected_story_id), cx);
        let selected_document = documents
            .get(&selected_story_id)
            .expect("selected story exists");
        let initial_state = InitialLabState::from_document(selected_document);

        let last_live_modified =
            latest_story_or_token_modified(&config.stories_dir, &config.token_paths)
                .unwrap_or(SystemTime::UNIX_EPOCH);
        let cached_matrix =
            ResponsivePreviewMatrix::for_story(&documents.get(&selected_story_id).unwrap().story);
        let sidebar_labels = Self::build_sidebar_labels(&documents, &story_ids);

        let entity = cx.entity().clone();
        let parent = entity.downgrade();
        let sidebar_entity = cx.new(|_cx| LabSidebar::new(parent.clone()));
        let toolbar_entity = cx.new(|_cx| LabToolbar::new(parent.clone()));
        let controls_panel_entity = cx.new(|_cx| LabControlsPanel::new(parent.clone()));
        let preview_area_entity = cx.new(|_cx| LabPreviewArea::new(parent.clone()));

        let mut lab = Self {
            registry,
            renderers,
            documents,
            story_ids,
            ui_showcases,
            selected_story_id,
            selected_viewport_id: initial_state.viewport_id,
            selected_theme_id: initial_state.theme_id,
            selected_motion_id: initial_state.motion_id,
            matrix_mode: initial_state.matrix_mode,
            layout_constraints: initial_state.layout_constraints,
            layout_state_dirty: false,
            save_status: None,
            live_status: config.watch.then(|| "Live preview enabled".into()),
            live_preview: config.watch,
            last_live_modified,
            stories_dir: config.stories_dir,
            token_paths: config.token_paths,
            entity,
            cached_matrix,
            sidebar_labels,
            sidebar_entity,
            toolbar_entity,
            controls_panel_entity,
            preview_area_entity,
            alloc_probe: gpui_profiler::AllocProbe::new(),
            last_render_alloc: gpui_profiler::AllocSnapshot::default(),
            last_mouse_move_alloc: gpui_profiler::AllocSnapshot::default(),
            last_sample: None,
            last_window_size: None,
            visual_capture_mode: visual_capture.is_some(),
        };
        if let Some(capture) = visual_capture {
            lab.select_story(capture.story_id, cx);
            lab.set_viewport(capture.viewport_id);
            lab.set_theme(capture.theme_id);
            lab.set_motion(if capture.reduced_motion {
                "reduced"
            } else {
                "system"
            });
            lab.matrix_mode = false;
        }
        if lab.live_preview {
            lab.start_live_preview(cx);
        }
        lab
    }

    pub(super) fn build_sidebar_labels(
        documents: &BTreeMap<String, StoryDocument>,
        story_ids: &[String],
    ) -> BTreeMap<String, SharedString> {
        story_ids
            .iter()
            .filter_map(|story_id| {
                documents.get(story_id).map(|doc| {
                    let label = SharedString::new(format!(
                        "{} / {}",
                        doc.story.crate_name, doc.story.title
                    ));
                    (story_id.clone(), label)
                })
            })
            .collect()
    }

    fn rebuild_derived_state(&mut self) {
        self.cached_matrix = ResponsivePreviewMatrix::for_story(
            &self.documents.get(&self.selected_story_id).unwrap().story,
        );
    }

    fn rebuild_sidebar_labels(&mut self) {
        self.sidebar_labels = Self::build_sidebar_labels(&self.documents, &self.story_ids);
    }

    pub(super) fn start_live_preview(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(750))
                    .await;
                let Ok((stories_dir, token_paths, last_seen)) = this.update(cx, |lab, _cx| {
                    (
                        lab.stories_dir.clone(),
                        lab.token_paths.clone(),
                        lab.last_live_modified,
                    )
                }) else {
                    break;
                };

                let (reload, error_latest) = cx
                    .background_executor()
                    .spawn(async move {
                        let reload =
                            reload_live_preview_state(&stories_dir, &token_paths, last_seen);
                        let error_latest = reload.as_ref().err().and_then(|_| {
                            latest_story_or_token_modified(&stories_dir, &token_paths).ok()
                        });
                        (reload, error_latest)
                    })
                    .await;

                if this
                    .update(cx, |lab, cx| {
                        lab.apply_live_preview_result(reload, error_latest, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn apply_live_preview_result(
        &mut self,
        reload: Result<Option<LivePreviewReload>>,
        error_latest: Option<SystemTime>,
        cx: &mut Context<Self>,
    ) {
        match reload {
            Ok(Some(reload)) => {
                self.apply_live_reload(reload);
                cx.notify();
            }
            Ok(None) => {}
            Err(err) => {
                if let Some(latest) = error_latest {
                    self.last_live_modified = latest;
                }
                self.live_status = Some(format!("Live reload failed: {err}").into());
                cx.notify();
            }
        }
    }

    pub(super) fn apply_live_reload(&mut self, reload: LivePreviewReload) {
        let selected_reloaded = reload
            .story_documents
            .iter()
            .any(|doc| doc.story.id == self.selected_story_id);
        let story_count = reload.story_documents.len();

        for doc in reload.story_documents {
            self.documents.insert(doc.story.id.clone(), doc);
        }
        self.rebuild_derived_state();
        self.rebuild_sidebar_labels();

        if selected_reloaded && let Some(document) = self.documents.get(&self.selected_story_id) {
            let state = InitialLabState::from_document(document);
            self.selected_viewport_id = state.viewport_id;
            self.selected_theme_id = state.theme_id;
            self.selected_motion_id = state.motion_id;
            self.matrix_mode = state.matrix_mode;
            self.layout_constraints = state.layout_constraints;
        }

        self.last_live_modified = reload.latest_modified;
        self.live_status = Some(live_reload_status(story_count, &reload.token_reports).into());
    }

    pub(super) fn selected_document(&self) -> &StoryDocument {
        self.documents
            .get(&self.selected_story_id)
            .expect("selected story document")
    }

    pub(super) fn selected_story(&self) -> &ComponentStory {
        &self.selected_document().story
    }

    pub(super) fn selected_viewport(&self) -> &ViewportPreset {
        self.selected_story()
            .viewports
            .iter()
            .find(|viewport| viewport.id == self.selected_viewport_id)
            .or_else(|| self.selected_story().viewports.first())
            .unwrap_or_else(|| {
                static FALLBACK: std::sync::OnceLock<ViewportPreset> = std::sync::OnceLock::new();
                FALLBACK.get_or_init(|| ViewportPreset::new("desktop", "Desktop", 1280.0, 800.0))
            })
    }

    pub(super) fn selected_theme_preset(&self) -> &ThemePreset {
        self.selected_story()
            .themes
            .iter()
            .find(|theme| theme.id == self.selected_theme_id)
            .or_else(|| self.selected_story().themes.first())
            .unwrap_or_else(|| {
                static FALLBACK: std::sync::OnceLock<ThemePreset> = std::sync::OnceLock::new();
                FALLBACK.get_or_init(|| ThemePreset::new("neutral", "Neutral", "neutral", false))
            })
    }

    pub(super) fn selected_motion_preset(&self) -> &MotionPreset {
        self.selected_story()
            .motions
            .iter()
            .find(|motion| motion.id == self.selected_motion_id)
            .or_else(|| self.selected_story().motions.first())
            .unwrap_or_else(|| {
                static FALLBACK: std::sync::OnceLock<MotionPreset> = std::sync::OnceLock::new();
                FALLBACK.get_or_init(|| MotionPreset::new("system", "System", false))
            })
    }

    pub(super) fn select_story(&mut self, story_id: String, cx: &mut Context<Self>) {
        self.sync_layout_state();
        if self.documents.contains_key(&story_id) {
            let state = InitialLabState::from_document(&self.documents[&story_id]);
            self.ensure_ui_showcase(&story_id, cx);
            self.selected_story_id = story_id;
            self.selected_viewport_id = state.viewport_id;
            self.selected_theme_id = state.theme_id;
            self.selected_motion_id = state.motion_id;
            self.matrix_mode = state.matrix_mode;
            self.layout_constraints = state.layout_constraints;
            self.save_status = None;
            self.rebuild_derived_state();
        }
    }

    fn ensure_ui_showcase(&mut self, story_id: &str, cx: &mut Context<Self>) {
        if self.ui_showcases.contains_key(story_id) {
            return;
        }

        if let Some(section) = showcase_section_for_story_id(story_id) {
            let showcase = cx.new(|cx| Showcase::embedded_section(section, cx));
            self.ui_showcases.insert(story_id.to_owned(), showcase);
        }
    }

    pub(super) fn set_prop(&mut self, story_id: &str, prop_name: &str, value: StoryPropValue) {
        if let Some(doc) = self.documents.get_mut(story_id)
            && doc.set_prop_value(prop_name, value).is_ok()
        {
            self.record_sample("prop-change");
            self.save_status = Some("Unsaved changes".into());
        }
    }

    /// Sample allocations and remember the result as the most recent sample.
    fn record_sample(&mut self, label: &'static str) -> gpui_profiler::AllocSnapshot {
        let delta = self.alloc_probe.sample(label);
        self.last_sample = Some((label, delta));
        delta
    }

    #[cfg(feature = "profiler")]
    pub(super) fn last_allocation_sample(
        &self,
    ) -> Option<(&'static str, gpui_profiler::AllocSnapshot)> {
        self.last_sample
    }

    #[cfg(feature = "profiler")]
    pub(super) fn last_render_allocation_sample(&self) -> gpui_profiler::AllocSnapshot {
        self.last_render_alloc
    }

    pub(super) fn set_viewport(&mut self, viewport_id: impl Into<String>) {
        self.selected_viewport_id = viewport_id.into();
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_theme(&mut self, theme_id: impl Into<String>) {
        self.selected_theme_id = theme_id.into();
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_motion(&mut self, motion_id: impl Into<String>) {
        self.selected_motion_id = motion_id.into();
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_sizing(&mut self, sizing: PreviewSizing) {
        self.layout_constraints.sizing = sizing;
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_min_width(&mut self, width: f64) {
        self.layout_constraints.min_width = clamp_f32(width, 160.0, 1600.0);
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_min_height(&mut self, height: f64) {
        self.layout_constraints.min_height = clamp_f32(height, 120.0, 1200.0);
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_aspect_ratio(&mut self, aspect_ratio: f64) {
        self.layout_constraints.aspect_ratio = clamp_f32(aspect_ratio, 0.5, 3.0);
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_padding(&mut self, padding: f64) {
        self.layout_constraints.padding = clamp_f32(padding, 0.0, 80.0);
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_horizontal_align(&mut self, align: PreviewAlign) {
        self.layout_constraints.horizontal_align = align;
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_vertical_align(&mut self, align: PreviewAlign) {
        self.layout_constraints.vertical_align = align;
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_overflow(&mut self, overflow: PreviewOverflow) {
        self.layout_constraints.overflow = overflow;
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_surface(&mut self, surface: PreviewSurface) {
        self.layout_constraints.surface = surface;
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_gap(&mut self, gap: f64) {
        self.layout_constraints.gap = clamp_f32(gap, 0.0, 80.0);
        self.mark_layout_state_dirty();
    }

    pub(super) fn set_layout_border(&mut self, border: bool) {
        self.layout_constraints.border = border;
        self.mark_layout_state_dirty();
    }

    pub(super) fn toggle_matrix(&mut self) {
        self.matrix_mode = !self.matrix_mode;
        self.mark_layout_state_dirty();
    }

    fn mark_layout_state_dirty(&mut self) {
        self.layout_state_dirty = true;
        self.save_status = Some("Unsaved changes".into());
    }

    pub(super) fn sync_layout_state(&mut self) {
        if !self.layout_state_dirty {
            return;
        }

        if let Some(doc) = self.documents.get_mut(&self.selected_story_id) {
            doc.layout = json!({
                "viewport": self.selected_viewport_id,
                "theme": self.selected_theme_id,
                "motion": self.selected_motion_id,
                "matrix": self.matrix_mode,
                "constraints": self.layout_constraints.as_json(),
                "builder": self.layout_constraints.builder_json(),
            });
        }
        self.save_status = Some("Unsaved changes".into());
        self.layout_state_dirty = false;
    }

    pub(super) fn save_selected(&mut self) {
        self.sync_layout_state();
        let result = self.try_save_selected();
        self.save_status = Some(match result {
            Ok(path) => format!("Saved {}", path.display()).into(),
            Err(err) => format!("Save failed: {err}").into(),
        });
    }

    pub(super) fn try_save_selected(&self) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.stories_dir)
            .with_context(|| format!("create {}", self.stories_dir.display()))?;
        let path = self
            .stories_dir
            .join(story_file_name(&self.selected_story_id));
        self.selected_document().save_story_json(&path)?;
        Ok(path)
    }

    pub(super) fn reload_documents(&mut self) {
        match load_story_documents(&self.stories_dir) {
            Ok(docs) => {
                for doc in docs {
                    self.documents.insert(doc.story.id.clone(), doc);
                }
                self.rebuild_derived_state();
                self.rebuild_sidebar_labels();
                self.save_status = Some("Reloaded story JSON".into());
            }
            Err(err) => {
                self.save_status = Some(format!("Reload failed: {err}").into());
            }
        }
    }

    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let mut list = div().flex().flex_col().gap_1();

        for story_id in &self.story_ids {
            let selected = *story_id == self.selected_story_id;
            let story_id_for_click = story_id.clone();
            let label = self
                .sidebar_labels
                .get(story_id)
                .cloned()
                .unwrap_or_else(|| SharedString::new(story_id.clone()));
            let entity = self.entity.clone();
            list = list.child(
                Button::new(lab_id(&["story", story_id]), label)
                    .variant(if selected {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(ButtonSize::Sm)
                    .full_width(true)
                    .on_click(move |_window, cx| {
                        entity.update(cx, |this, cx| {
                            this.select_story(story_id_for_click.clone(), cx)
                        });
                    }),
            );
        }

        div()
            .w(px(300.0))
            .h_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(theme.surface)
            .border_r_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h3("Component Lab"))
                    .child(Text::new(format!("{} stories", self.registry.len())).muted(true)),
            )
            .child(list)
            .child(self.render_token_status(cx))
            .into_any_element()
    }

    pub(super) fn render_token_status(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let token_label = if self.token_paths.is_empty() {
            "No token JSON watched".to_string()
        } else {
            format!("Watching {} token file(s)", self.token_paths.len())
        };
        let live_label = if self.live_preview {
            "Live preview on"
        } else {
            "Live preview off"
        };
        div()
            .mt_auto()
            .p_3()
            .rounded_md()
            .bg(theme.surface_hover)
            .border_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .gap_1()
            .child(
                Text::new(live_label)
                    .size(TextSize::Xs)
                    .color(theme.text_secondary),
            )
            .child(Text::new(token_label).size(TextSize::Xs).muted(true))
            .when_some(self.live_status.clone(), |el, status| {
                el.child(
                    Text::new(status)
                        .size(TextSize::Xs)
                        .color(theme.text_secondary),
                )
            })
            .into_any_element()
    }

    pub(super) fn render_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let entity = self.entity.clone();
        let story = self.selected_story();
        let viewport = self.selected_viewport();
        let theme_preset = self.selected_theme_preset();

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .pb_4()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h2(story.title.clone()))
                    .child(
                        Text::new(format!(
                            "{} | {} x {} | {}",
                            story.crate_name,
                            viewport.width.round(),
                            viewport.height.round(),
                            theme_preset.label
                        ))
                        .size(TextSize::Sm)
                        .color(theme.text_secondary),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new(
                            "lab-toggle-matrix",
                            if self.matrix_mode {
                                "Preview"
                            } else {
                                "Matrix"
                            },
                        )
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Sm)
                        .on_click(move |_window, cx| {
                            entity.update(cx, |this, _| this.toggle_matrix());
                        }),
                    )
                    .child({
                        let entity = self.entity.clone();
                        Button::new("lab-reload", "Reload")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm)
                            .on_click(move |_window, cx| {
                                entity.update(cx, |this, _| this.reload_documents());
                            })
                    })
                    .child({
                        let entity = self.entity.clone();
                        Button::new("lab-save", "Save")
                            .variant(ButtonVariant::Primary)
                            .size(ButtonSize::Sm)
                            .on_click(move |_window, cx| {
                                entity.update(cx, |this, _| this.save_selected());
                            })
                    }),
            )
            .into_any_element()
    }

    pub(super) fn render_controls_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let story = self.selected_story();

        let mut props = div().flex().flex_col().gap_3();
        for prop in &story.props {
            props = props.child(self.render_prop_editor(story, prop, cx));
        }

        div()
            .w(px(340.0))
            .h_full()
            .flex()
            .flex_col()
            .gap_5()
            .p_4()
            .bg(theme.surface)
            .border_l_1()
            .border_color(theme.border)
            .child(self.render_story_metadata(story, cx))
            .child(self.render_story_renderer(story, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h3("Props"))
                    .child(
                        Text::new(story.description.clone())
                            .size(TextSize::Sm)
                            .muted(true),
                    ),
            )
            .child(props)
            .child(self.render_layout_controls(cx))
            .when_some(self.save_status.clone(), |el, status| {
                el.child(
                    div()
                        .p_3()
                        .rounded_md()
                        .bg(theme.surface_hover)
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            Text::new(status)
                                .size(TextSize::Xs)
                                .color(theme.text_secondary),
                        ),
                )
            })
            .into_any_element()
    }

    pub(super) fn render_story_renderer(
        &self,
        story: &ComponentStory,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let mut rows = div().flex().flex_col().gap_2();

        if let Some(renderer) = self.renderers.renderer(&story.id) {
            for (label, value) in [
                ("Kind", renderer.kind.label().to_string()),
                ("Interactive", renderer.interactive.to_string()),
                ("Matrix", renderer.matrix_preview.to_string()),
            ] {
                rows = rows.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(Text::new(label).size(TextSize::Xs).color(theme.text_muted))
                        .child(
                            Text::new(value)
                                .size(TextSize::Xs)
                                .weight(TextWeight::Medium)
                                .color(theme.text_secondary),
                        ),
                );
            }
        } else {
            rows = rows.child(
                Text::new("No interactive renderer registered")
                    .size(TextSize::Xs)
                    .color(theme.text_secondary),
            );
        }

        div()
            .p_3()
            .rounded_md()
            .bg(theme.surface_hover)
            .border_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .gap_2()
            .child(Heading::h3("Renderer"))
            .child(rows)
            .into_any_element()
    }

    pub(super) fn render_story_metadata(
        &self,
        story: &ComponentStory,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let mut rows = div().flex().flex_col().gap_2();
        for item in &story.metadata {
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        Text::new(item.label.clone())
                            .size(TextSize::Xs)
                            .color(theme.text_muted),
                    )
                    .child(
                        Text::new(item.value.clone())
                            .size(TextSize::Xs)
                            .weight(TextWeight::Medium)
                            .color(theme.text_secondary),
                    ),
            );
        }

        div()
            .p_3()
            .rounded_md()
            .bg(theme.surface_hover)
            .border_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .gap_2()
            .child(Heading::h3("Metadata"))
            .child(rows)
            .into_any_element()
    }

    pub(super) fn render_prop_editor(
        &self,
        story: &ComponentStory,
        prop: &StoryProp,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let story_id = SharedString::new(story.id.clone());
        let prop_name = SharedString::new(prop.name.clone());
        let prop_label = SharedString::new(prop.label.clone());
        let entity = self.entity.clone();

        let control = match &prop.value {
            StoryPropValue::Bool(value) => {
                let story_id = story_id.clone();
                let prop_name = prop_name.clone();
                Toggle::new(lab_id(&["prop-bool", &story.id, &prop.name]))
                    .checked(*value)
                    .label(if *value { "On" } else { "Off" })
                    .size(ToggleSize::Sm)
                    .style(ToggleStyle::Sliding)
                    .on_change(move |checked, _window, cx| {
                        entity.update(cx, |this, _| {
                            this.set_prop(
                                story_id.as_str(),
                                prop_name.as_str(),
                                StoryPropValue::Bool(checked),
                            );
                        });
                    })
                    .into_any_element()
            }
            StoryPropValue::Number(value) => {
                let story_id = story_id.clone();
                let prop_name = prop_name.clone();
                NumberInput::new(lab_id(&["prop-number", &story.id, &prop.name]))
                    .value(*value)
                    .step(number_step(&prop.name))
                    .decimals(2)
                    .width(150.0)
                    .size(NumberInputSize::Sm)
                    .on_change(move |number, _window, cx| {
                        entity.update(cx, |this, _| {
                            this.set_prop(
                                story_id.as_str(),
                                prop_name.as_str(),
                                StoryPropValue::Number(number),
                            );
                        });
                    })
                    .into_any_element()
            }
            StoryPropValue::Text(value) | StoryPropValue::Color(value) => {
                let current_value = value.clone();
                let is_color = matches!(prop.value, StoryPropValue::Color(_));
                let story_id = story_id.clone();
                let prop_name = prop_name.clone();
                Input::new(lab_id(&["prop-text", &story.id, &prop.name]))
                    .value(current_value)
                    .size(InputSize::Sm)
                    .placeholder(prop_label)
                    .on_text_change(move |text, _window, cx| {
                        entity.update(cx, |this, _| {
                            let value = if is_color {
                                StoryPropValue::Color(SharedString::new(text))
                            } else {
                                StoryPropValue::Text(SharedString::new(text))
                            };
                            this.set_prop(story_id.as_str(), prop_name.as_str(), value);
                        });
                    })
                    .into_any_element()
            }
            StoryPropValue::Choice(value) => {
                let mut row = div().flex().flex_wrap().gap_1();
                for option in &prop.options {
                    let option_label = SharedString::new(option.clone());
                    let story_id = story_id.clone();
                    let prop_name = prop_name.clone();
                    let entity = self.entity.clone();
                    row = row.child(
                        Button::new(
                            lab_id(&["prop-choice", &story.id, &prop.name, option]),
                            option_label.clone(),
                        )
                        .variant(if option == value {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::Xs)
                        .on_click(move |_window, cx| {
                            entity.update(cx, |this, _| {
                                this.set_prop(
                                    story_id.as_str(),
                                    prop_name.as_str(),
                                    StoryPropValue::Choice(option_label.clone()),
                                );
                            });
                        }),
                    );
                }
                row.into_any_element()
            }
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        Text::new(prop.label.clone())
                            .size(TextSize::Sm)
                            .weight(TextWeight::Medium),
                    )
                    .child(
                        Text::new(prop_value_label(&prop.value))
                            .size(TextSize::Xs)
                            .color(theme.text_muted),
                    ),
            )
            .child(control)
            .into_any_element()
    }

    pub(super) fn render_layout_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let story = self.selected_story();

        let mut viewport_row = div().flex().flex_wrap().gap_1();
        for viewport in &story.viewports {
            let viewport_id = viewport.id.clone();
            let entity = self.entity.clone();
            viewport_row = viewport_row.child(
                Button::new(lab_id(&["viewport", &viewport.id]), viewport.label.clone())
                    .variant(if viewport.id == self.selected_viewport_id {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(ButtonSize::Xs)
                    .on_click(move |_window, cx| {
                        entity.update(cx, |this, _| this.set_viewport(viewport_id.clone()));
                    }),
            );
        }

        let mut theme_row = div().flex().flex_wrap().gap_1();
        for theme_preset in &story.themes {
            let theme_id = theme_preset.id.clone();
            let entity = self.entity.clone();
            theme_row = theme_row.child(
                Button::new(
                    lab_id(&["theme", &theme_preset.id]),
                    theme_preset.label.clone(),
                )
                .variant(if theme_preset.id == self.selected_theme_id {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Ghost
                })
                .size(ButtonSize::Xs)
                .on_click(move |_window, cx| {
                    entity.update(cx, |this, _| this.set_theme(theme_id.clone()));
                }),
            );
        }

        let mut motion_row = div().flex().flex_wrap().gap_1();
        for motion in &story.motions {
            let motion_id = motion.id.clone();
            let entity = self.entity.clone();
            motion_row = motion_row.child(
                Button::new(lab_id(&["motion", &motion.id]), motion.label.clone())
                    .variant(if motion.id == self.selected_motion_id {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(ButtonSize::Xs)
                    .on_click(move |_window, cx| {
                        entity.update(cx, |this, _| this.set_motion(motion_id.clone()));
                    }),
            );
        }

        let mut sizing_row = div().flex().flex_wrap().gap_1();
        for sizing in PreviewSizing::ALL {
            let entity = self.entity.clone();
            sizing_row = sizing_row.child(
                Button::new(lab_id(&["layout-sizing", sizing.as_str()]), sizing.label())
                    .variant(if sizing == self.layout_constraints.sizing {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(ButtonSize::Xs)
                    .on_click(move |_window, cx| {
                        entity.update(cx, |this, _| this.set_layout_sizing(sizing));
                    }),
            );
        }

        let mut horizontal_align_row = div().flex().flex_wrap().gap_1();
        for align in PreviewAlign::ALL {
            let entity = self.entity.clone();
            horizontal_align_row = horizontal_align_row.child(
                Button::new(lab_id(&["layout-h-align", align.as_str()]), align.label())
                    .variant(if align == self.layout_constraints.horizontal_align {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(ButtonSize::Xs)
                    .on_click(move |_window, cx| {
                        entity.update(cx, |this, _| this.set_layout_horizontal_align(align));
                    }),
            );
        }

        let mut vertical_align_row = div().flex().flex_wrap().gap_1();
        for align in PreviewAlign::ALL {
            let entity = self.entity.clone();
            vertical_align_row = vertical_align_row.child(
                Button::new(lab_id(&["layout-v-align", align.as_str()]), align.label())
                    .variant(if align == self.layout_constraints.vertical_align {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(ButtonSize::Xs)
                    .on_click(move |_window, cx| {
                        entity.update(cx, |this, _| this.set_layout_vertical_align(align));
                    }),
            );
        }

        let mut overflow_row = div().flex().flex_wrap().gap_1();
        for overflow in PreviewOverflow::ALL {
            let entity = self.entity.clone();
            overflow_row = overflow_row.child(
                Button::new(
                    lab_id(&["layout-overflow", overflow.as_str()]),
                    overflow.label(),
                )
                .variant(if overflow == self.layout_constraints.overflow {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Ghost
                })
                .size(ButtonSize::Xs)
                .on_click(move |_window, cx| {
                    entity.update(cx, |this, _| this.set_layout_overflow(overflow));
                }),
            );
        }

        let mut surface_row = div().flex().flex_wrap().gap_1();
        for surface in PreviewSurface::ALL {
            let entity = self.entity.clone();
            surface_row = surface_row.child(
                Button::new(
                    lab_id(&["layout-surface", surface.as_str()]),
                    surface.label(),
                )
                .variant(if surface == self.layout_constraints.surface {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Ghost
                })
                .size(ButtonSize::Xs)
                .on_click(move |_window, cx| {
                    entity.update(cx, |this, _| this.set_layout_surface(surface));
                }),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(Heading::h3("Layout"))
            .child(
                Text::new("Viewport")
                    .size(TextSize::Sm)
                    .weight(TextWeight::Medium),
            )
            .child(viewport_row)
            .child(
                Text::new("Design")
                    .size(TextSize::Sm)
                    .weight(TextWeight::Medium),
            )
            .child(theme_row)
            .child(
                Text::new("Motion")
                    .size(TextSize::Sm)
                    .weight(TextWeight::Medium),
            )
            .child(motion_row)
            .child(
                Text::new("Sizing")
                    .size(TextSize::Sm)
                    .weight(TextWeight::Medium),
            )
            .child(sizing_row)
            .child(
                Text::new("Horizontal Align")
                    .size(TextSize::Sm)
                    .weight(TextWeight::Medium),
            )
            .child(horizontal_align_row)
            .child(
                Text::new("Vertical Align")
                    .size(TextSize::Sm)
                    .weight(TextWeight::Medium),
            )
            .child(vertical_align_row)
            .child(
                Text::new("Overflow")
                    .size(TextSize::Sm)
                    .weight(TextWeight::Medium),
            )
            .child(overflow_row)
            .child(
                Text::new("Surface")
                    .size(TextSize::Sm)
                    .weight(TextWeight::Medium),
            )
            .child(surface_row)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        NumberInput::new("layout-min-width")
                            .label("Min W")
                            .value(self.layout_constraints.min_width as f64)
                            .range(160.0, 1600.0)
                            .step(20.0)
                            .decimals(0)
                            .unit("px")
                            .width(104.0)
                            .size(NumberInputSize::Sm)
                            .on_change({
                                let entity = self.entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _| this.set_layout_min_width(value));
                                }
                            }),
                    )
                    .child(
                        NumberInput::new("layout-min-height")
                            .label("Min H")
                            .value(self.layout_constraints.min_height as f64)
                            .range(120.0, 1200.0)
                            .step(20.0)
                            .decimals(0)
                            .unit("px")
                            .width(104.0)
                            .size(NumberInputSize::Sm)
                            .on_change({
                                let entity = self.entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _| this.set_layout_min_height(value));
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        NumberInput::new("layout-aspect-ratio")
                            .label("Ratio")
                            .value(self.layout_constraints.aspect_ratio as f64)
                            .range(0.5, 3.0)
                            .step(0.1)
                            .decimals(2)
                            .width(104.0)
                            .size(NumberInputSize::Sm)
                            .on_change({
                                let entity = self.entity.clone();
                                move |value, _window, cx| {
                                    entity
                                        .update(cx, |this, _| this.set_layout_aspect_ratio(value));
                                }
                            }),
                    )
                    .child(
                        NumberInput::new("layout-padding")
                            .label("Padding")
                            .value(self.layout_constraints.padding as f64)
                            .range(0.0, 80.0)
                            .step(4.0)
                            .decimals(0)
                            .unit("px")
                            .width(104.0)
                            .size(NumberInputSize::Sm)
                            .on_change({
                                let entity = self.entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _| this.set_layout_padding(value));
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        NumberInput::new("layout-gap")
                            .label("Gap")
                            .value(self.layout_constraints.gap as f64)
                            .range(0.0, 80.0)
                            .step(4.0)
                            .decimals(0)
                            .unit("px")
                            .width(104.0)
                            .size(NumberInputSize::Sm)
                            .on_change({
                                let entity = self.entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _| this.set_layout_gap(value));
                                }
                            }),
                    )
                    .child(
                        Toggle::new("layout-border")
                            .checked(self.layout_constraints.border)
                            .label("Border")
                            .size(ToggleSize::Sm)
                            .on_change({
                                let entity = self.entity.clone();
                                move |checked, _window, cx| {
                                    entity.update(cx, |this, _| this.set_layout_border(checked));
                                }
                            }),
                    ),
            )
            .child(
                Toggle::new("matrix-mode")
                    .checked(self.matrix_mode)
                    .label("Responsive matrix")
                    .size(ToggleSize::Sm)
                    .on_change({
                        let entity = self.entity.clone();
                        move |_checked, _window, cx| {
                            entity.update(cx, |this, _| this.toggle_matrix());
                        }
                    }),
            )
            .into_any_element()
    }

    pub(super) fn render_preview_area(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.matrix_mode {
            self.render_matrix(cx)
        } else {
            self.render_single_preview(cx)
        }
    }

    pub(super) fn render_single_preview(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let story = self.selected_story();
        let viewport = self.selected_viewport();
        let theme_preset = self.selected_theme_preset();
        let motion_preset = self.selected_motion_preset();
        let preview_design = design_for_theme_preset(theme_preset);
        let constraints = self.layout_constraints;
        let (frame_width, frame_height) = constraints.frame_dimensions(viewport);
        let scope = story.id.as_str();

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .child(
                div()
                    .w(px(frame_width))
                    .h(px(frame_height))
                    .max_w_full()
                    .max_h_full()
                    .flex()
                    .flex_col()
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_2()
                            .bg(theme.surface)
                            .border_b_1()
                            .border_color(theme.border)
                            .child(
                                Text::new(format!("{} preview", viewport.label)).size(TextSize::Xs),
                            )
                            .child(
                                Text::new(format!(
                                    "{} / {} / {}",
                                    theme_preset.label,
                                    motion_preset.label,
                                    constraints.sizing.label()
                                ))
                                .size(TextSize::Xs)
                                .muted(true),
                            ),
                    )
                    .child(
                        apply_preview_builder_style(
                            div()
                                .id("lab-preview-builder-surface")
                                .flex_1()
                                .flex()
                                .gap(px(constraints.gap)),
                            constraints,
                            theme,
                        )
                        .p(px(constraints.padding))
                        .child(self.render_story_preview(
                            story,
                            scope,
                            true,
                            preview_design,
                            cx,
                        )),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn render_matrix(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let story = self.selected_story();
        let motion_preset = self.selected_motion_preset();
        let matrix = &self.cached_matrix;

        let mut grid = div().flex().flex_wrap().gap_3().items_start();
        for (index, cell) in matrix.cells.iter().enumerate() {
            let scope = format!("matrix-{index}");
            let preview_design = design_for_theme_preset(&cell.theme);
            grid = grid.child(
                div()
                    .w(px(260.0))
                    .h(px(210.0))
                    .flex()
                    .flex_col()
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .overflow_hidden()
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .bg(theme.surface)
                            .border_b_1()
                            .border_color(theme.border)
                            .child(
                                Text::new(format!(
                                    "{} / {} x {}",
                                    cell.viewport.label,
                                    cell.viewport.width.round(),
                                    cell.viewport.height.round()
                                ))
                                .size(TextSize::Xs),
                            )
                            .child(
                                Text::new(cell.theme.label.clone())
                                    .size(TextSize::Xs)
                                    .muted(true),
                            )
                            .child(
                                Text::new(motion_preset.label.clone())
                                    .size(TextSize::Xs)
                                    .muted(true),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .p_3()
                            .child(self.render_story_preview(
                                story,
                                &scope,
                                false,
                                preview_design,
                                cx,
                            )),
                    ),
            );
        }

        div().size_full().p_6().child(grid).into_any_element()
    }

    pub(super) fn render_story_preview(
        &self,
        story: &ComponentStory,
        scope: &str,
        interactive: bool,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match story.id.as_str() {
            "ui-kit.button" => self.render_button_story(story, scope, interactive, design, cx),
            "ui-kit.form" => self.render_form_story(story, scope, interactive, design, cx),
            "ui-kit.status" => self.render_status_story(story, scope, design, cx),
            "ui-kit.navigation" => self.render_navigation_story(story, scope, design, cx),
            "ui-kit.feedback" => self.render_feedback_story(story, scope, design, cx),
            "ui-kit.card" => self.render_card_story(story, scope, design, cx),
            story_id if ui_kit_exported_component_story_id(story_id) => {
                self.render_exported_ui_kit_component_story(story, scope, interactive, design, cx)
            }
            story_id if self.ui_showcases.contains_key(story_id) => {
                self.render_ui_kit_showcase_story(story, scope, cx)
            }
            "audio-kit.potentiometer" => {
                self.render_potentiometer_story(story, scope, interactive, design, cx)
            }
            "audio-kit.vertical-slider" => {
                self.render_vertical_slider_story(story, scope, interactive, design, cx)
            }
            "audio-kit.volume-knob" => {
                self.render_volume_knob_story(story, scope, interactive, design, cx)
            }
            "audio-kit.meter" => self.render_meter_story(story, scope, design, cx),
            "audio-kit.horizontal-meter" => {
                self.render_horizontal_meter_story(story, scope, design, cx)
            }
            "audio-kit.spectrum" => self.render_spectrum_story(story, scope, design, cx),
            "audio-kit.spectrum-axis" => self.render_spectrum_axis_story(story, scope, design, cx),
            "px.line" => self.render_line_chart_story(story, scope, design, cx),
            "px.bar" => self.render_bar_chart_story(story, scope, design, cx),
            "px.scatter" => self.render_scatter_chart_story(story, scope, design, cx),
            "px.area" => self.render_area_chart_story(story, scope, design, cx),
            "px.heatmap" => self.render_heatmap_chart_story(story, scope, design, cx),
            "px.contour" => self.render_contour_chart_story(story, scope, design, cx),
            "px.isoline" => self.render_isoline_chart_story(story, scope, design, cx),
            "px.pie" => self.render_pie_chart_story(story, scope, design, cx),
            "px.donut" => self.render_donut_chart_story(story, scope, design, cx),
            "px.boxplot" => self.render_boxplot_chart_story(story, scope, design, cx),
            "px.treemap" => self.render_treemap_chart_story(story, scope, design, cx),
            "px.surface3d" => self.render_surface3d_chart_story(story, scope, design, cx),
            "px.mesh_plot" => self.render_mesh_plot_story(story, scope, design, cx),
            story_id if story_id.starts_with("px.mesh_plot.") => {
                self.render_mesh_plot_story(story, scope, design, cx)
            }
            _ if self.renderers.contains(&story.id) => div()
                .child(
                    Text::new("Renderer metadata exists, but no preview handler is wired")
                        .muted(true),
                )
                .into_any_element(),
            _ => div()
                .child(Text::new("No renderer registered").muted(true))
                .into_any_element(),
        }
    }

    pub(super) fn render_exported_ui_kit_component_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        _interactive: bool,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let label = text_prop(story, "label", story.title.as_str());
        let value = number_prop(story, "value", 0.64).clamp(0.0, 1.0);
        let disabled = bool_prop(story, "disabled", false);
        let selected = bool_prop(story, "selected", true);
        let open = bool_prop(story, "open", true);
        let variant_name = choice_prop(story, "variant", "default");
        let story_id = story.id.as_str();
        let scoped = |name: &str| lab_id(&[name, scope]);

        let element = match story_id {
            "ui-kit.button-set" => ButtonSet::new(scoped("button-set"))
                .options(vec![
                    ButtonSetOption::new("mix", "Mix"),
                    ButtonSetOption::new("edit", "Edit"),
                    ButtonSetOption::new("ship", "Ship"),
                ])
                .selected("edit")
                .disabled(disabled)
                .into_any_element(),
            "ui-kit.icon-button" => IconButton::new(scoped("icon-button"), "✦")
                .variant(icon_button_variant(&variant_name))
                .size(IconButtonSize::Lg)
                .selected(selected)
                .disabled(disabled)
                .aria_label(label)
                .into_any_element(),
            "ui-kit.alert" => Alert::new(scoped("alert"), label)
                .title("Alert")
                .variant(alert_variant(&variant_name))
                .closeable(open)
                .into_any_element(),
            "ui-kit.inline-alert" => InlineAlert::new(label)
                .variant(alert_variant(&variant_name))
                .into_any_element(),
            "ui-kit.toast" => Toast::new(scoped("toast"), label)
                .title("Toast")
                .variant(toast_variant(&variant_name))
                .closeable(open)
                .into_any_element(),
            "ui-kit.toast-container" => div()
                .relative()
                .w(px(360.0))
                .h(px(160.0))
                .child(
                    ToastContainer::new(ToastPosition::TopRight)
                        .toast(Toast::new(scoped("toast-container-item"), label).title("Toast")),
                )
                .into_any_element(),
            "ui-kit.checkbox" => Checkbox::new(scoped("checkbox"))
                .label(label)
                .checked(selected)
                .disabled(disabled)
                .size(CheckboxSize::Md)
                .design(design)
                .into_any_element(),
            "ui-kit.color-picker" => cx
                .new(|_| ColorPickerView::new(label, Color::from_hex(0x3b82f6)))
                .into_any_element(),
            "ui-kit.input" => Input::new(scoped("input"))
                .label("Label")
                .value(label)
                .placeholder("Type text")
                .size(InputSize::Md)
                .disabled(disabled)
                .into_any_element(),
            "ui-kit.number-input" => NumberInput::new(scoped("number-input"))
                .label("Value")
                .value(value)
                .width(160.0)
                .size(NumberInputSize::Md)
                .disabled(disabled)
                .into_any_element(),
            "ui-kit.select" => Select::new(scoped("select"))
                .label("Mode")
                .options(vec![
                    SelectOption::new("design", "Design"),
                    SelectOption::new("build", "Build"),
                    SelectOption::new("verify", "Verify"),
                ])
                .selected("build")
                .placeholder("Choose")
                .size(SelectSize::Md)
                .disabled(disabled)
                .is_open(open)
                .into_any_element(),
            "ui-kit.slider" => Slider::new(scoped("slider"))
                .label(label)
                .range(0.0, 1.0)
                .value(value as f32)
                .show_value(true)
                .width(260.0)
                .disabled(disabled)
                .design(design)
                .into_any_element(),
            "ui-kit.toggle" => Toggle::new(scoped("toggle"))
                .label(label)
                .checked(selected)
                .disabled(disabled)
                .size(ToggleSize::Md)
                .style(ToggleStyle::Sliding)
                .into_any_element(),
            "ui-kit.avatar" => Avatar::new()
                .name(label)
                .size(AvatarSize::Lg)
                .shape(AvatarShape::Circle)
                .status(AvatarStatus::Online)
                .into_any_element(),
            "ui-kit.avatar-group" => AvatarGroup::new()
                .avatars(vec![
                    Avatar::new().name("Ada Lovelace"),
                    Avatar::new().name("Grace Hopper"),
                    Avatar::new().name("Katherine Johnson"),
                ])
                .max_display(3)
                .size(AvatarSize::Md)
                .into_any_element(),
            "ui-kit.badge" => Badge::new(label)
                .variant(badge_variant(&variant_name))
                .size(BadgeSize::Lg)
                .rounded(true)
                .into_any_element(),
            "ui-kit.badge-dot" => BadgeDot::new()
                .variant(badge_variant(&variant_name))
                .size(px(12.0))
                .into_any_element(),
            "ui-kit.empty-state-component" => EmptyState::new(label)
                .description("No matching items")
                .action(Button::new(scoped("empty-action"), "Create"))
                .into_any_element(),
            "ui-kit.image-view-component" => ImageView::new(scoped("image-view"))
                .size(px(160.0))
                .placeholder_icon("image")
                .into_any_element(),
            "ui-kit.keyboard-shortcut-label" => KeyboardShortcutLabel::new("⌘ K")
                .size(KeyboardShortcutSize::Md)
                .into_any_element(),
            "ui-kit.progress-bar" => Progress::new(value as f32)
                .variant(progress_variant(&variant_name))
                .size(ProgressSize::Lg)
                .show_label(true)
                .into_any_element(),
            "ui-kit.circular-progress" => CircularProgress::new(value as f32)
                .variant(progress_variant(&variant_name))
                .size(px(64.0))
                .show_label(true)
                .into_any_element(),
            "ui-kit.qr-code-component" => QrCode::new("https://sotf.dev")
                .size(px(128.0))
                .into_any_element(),
            "ui-kit.animated-qr-code" => cx
                .new(|cx| AnimatedQrCode::new("https://sotf.dev/lab", px(48.0), cx))
                .into_any_element(),
            "ui-kit.spinner" => Spinner::new()
                .size(SpinnerSize::Lg)
                .label(label)
                .into_any_element(),
            "ui-kit.loading-dots" => LoadingDots::new().size(SpinnerSize::Lg).into_any_element(),
            "ui-kit.step-indicator-component" => StepIndicator::new(
                scoped("step-indicator"),
                vec![
                    StepItem::new("Props").status(StepItemStatus::Completed),
                    StepItem::new("Preview").status(StepItemStatus::Active),
                    StepItem::new("Ship").status(StepItemStatus::NotVisited),
                ],
            )
            .orientation(StepOrientation::Horizontal)
            .size(StepIndicatorSize::Md)
            .into_any_element(),
            "ui-kit.text-component" => Text::new(label).size(TextSize::Lg).into_any_element(),
            "ui-kit.heading" => Heading::new(label).level(2).into_any_element(),
            "ui-kit.code" => Code::new("ComponentStory::new(...)").into_any_element(),
            "ui-kit.link" => Link::new(scoped("link"), label)
                .href("https://sotf.dev")
                .external(true)
                .into_any_element(),
            "ui-kit.search-bar-component" => SearchBar::new(scoped("search-bar"))
                .value(label)
                .placeholder("Search stories")
                .size(SearchBarSize::Md)
                .show_clear(true)
                .into_any_element(),
            "ui-kit.tooltip-component" => Tooltip::new(label).into_any_element(),
            "ui-kit.with-tooltip" => WithTooltip::new(
                Button::new(scoped("with-tooltip-button"), "Hover target"),
                label,
            )
            .into_any_element(),
            "ui-kit.loading-overlay-component" => div()
                .relative()
                .w(px(300.0))
                .h(px(180.0))
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .child(LoadingOverlay::new(scoped("loading-overlay")).message(label))
                .into_any_element(),
            "ui-kit.pane-divider" => div()
                .h(px(120.0))
                .child(
                    PaneDivider::vertical(
                        scoped("pane-divider"),
                        gpui_ui_kit::CollapseDirection::Left,
                    )
                    .label(label),
                )
                .into_any_element(),
            "ui-kit.settings-row" => {
                let row = SettingsRow::new(label)
                    .description("A reusable settings row")
                    .control(Toggle::new(scoped("settings-row-toggle")).checked(selected));
                SettingsForm::new(scoped("settings-row-form"))
                    .row(row)
                    .into_any_element()
            }
            "ui-kit.settings-form-component" => SettingsForm::new(scoped("settings-form"))
                .section("Audio")
                .row(
                    SettingsRow::new(label)
                        .description("Design-token aware setting")
                        .control(Toggle::new(scoped("settings-form-toggle")).checked(selected)),
                )
                .into_any_element(),
            "ui-kit.sidebar-component" => Sidebar::new(scoped("sidebar"))
                .content(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(Heading::h3(label))
                        .child(Text::new("Sidebar content").muted(true)),
                )
                .design(design)
                .into_any_element(),
            "ui-kit.split-pane-component" => div()
                .w(px(420.0))
                .h(px(180.0))
                .child(
                    SplitPane::new(scoped("split-pane"))
                        .direction(SplitDirection::Horizontal)
                        .first(div().p_3().child("Left"))
                        .second(div().p_3().child("Right"))
                        .design(design),
                )
                .into_any_element(),
            "ui-kit.vstack" => VStack::new()
                .child(Text::new(label.clone()))
                .child(Button::new(scoped("vstack-button"), "Action"))
                .into_any_element(),
            "ui-kit.hstack" => HStack::new()
                .child(Text::new(label.clone()))
                .child(Badge::new("Live").variant(BadgeVariant::Success))
                .into_any_element(),
            "ui-kit.spacer" => HStack::new()
                .child(Text::new("Start"))
                .child(Spacer::new())
                .child(Text::new("End"))
                .into_any_element(),
            "ui-kit.divider" => VStack::new()
                .child(Text::new("Above"))
                .child(Divider::new())
                .child(Text::new("Below"))
                .into_any_element(),
            "ui-kit.status-bar-component" => StatusBar::new(scoped("status-bar"))
                .left(Text::new(label))
                .center(Badge::new("Ready").variant(BadgeVariant::Success))
                .right(Text::new("42ms"))
                .into_any_element(),
            "ui-kit.accordion-component" => Accordion::new()
                .items(vec![
                    AccordionItem::new("one", label).content(Text::new("Expanded content")),
                    AccordionItem::new("two", "Details").content(Text::new("Second panel")),
                ])
                .into_any_element(),
            "ui-kit.breadcrumbs-component" => Breadcrumbs::new()
                .items(vec![
                    BreadcrumbItem::new("home", "Home"),
                    BreadcrumbItem::new("lab", "Lab"),
                    BreadcrumbItem::new("story", label),
                ])
                .into_any_element(),
            "ui-kit.menu-component" => Menu::new(
                scoped("menu"),
                vec![
                    MenuItem::new("copy", "Copy"),
                    MenuItem::new("paste", "Paste").disabled(disabled),
                ],
            )
            .into_any_element(),
            "ui-kit.menu-bar" => MenuBar::new(vec![
                MenuBarItem::new("file", "File").with_items(vec![
                    MenuItem::new("new", "New"),
                    MenuItem::new("save", "Save"),
                ]),
                MenuBarItem::new("view", "View")
                    .with_items(vec![MenuItem::new("matrix", "Matrix")]),
            ])
            .into_any_element(),
            "ui-kit.dialog-component" => Dialog::new(scoped("dialog"))
                .title(label)
                .size(DialogSize::Md)
                .content(Text::new("Dialog content"))
                .into_any_element(),
            "ui-kit.confirm-dialog-component" => ConfirmDialog::new(scoped("confirm-dialog"))
                .title(label)
                .message("This action can be reviewed before it runs.")
                .variant(confirm_dialog_variant(&variant_name))
                .into_any_element(),
            "ui-kit.popover-component" => Popover::new(scoped("popover"))
                .content(div().p_3().child(label))
                .width(px(220.0))
                .into_any_element(),
            "ui-kit.context-menu-component" => ContextMenu::new(
                scoped("context-menu"),
                vec![
                    MenuItem::new("inspect", "Inspect"),
                    MenuItem::new("copy", "Copy"),
                ],
            )
            .into_any_element(),
            "ui-kit.tabs-component" => Tabs::new(scoped("tabs-component"))
                .tabs(vec![
                    TabItem::new("props", "Props"),
                    TabItem::new("preview", "Preview").badge("2"),
                    TabItem::new("qa", "QA"),
                ])
                .selected_index(1)
                .variant(tab_variant(&variant_name))
                .into_any_element(),
            "ui-kit.wizard-component" => Wizard::new()
                .steps(sample_wizard_steps())
                .variant(WizardVariant::Horizontal)
                .into_any_element(),
            "ui-kit.wizard-header" => WizardHeader::new()
                .title(label)
                .steps(sample_wizard_steps())
                .step_statuses(vec![
                    StepStatus::Completed,
                    StepStatus::Active,
                    StepStatus::NotVisited,
                ])
                .current_step(1)
                .into_any_element(),
            "ui-kit.wizard-navigation" => WizardNavigation::new(1, 3)
                .progress(value as f32)
                .status_message(label)
                .show_cancel(true)
                .into_any_element(),
            "ui-kit.command-palette-component" => CommandPalette::new(
                scoped("command-palette"),
                vec![
                    CommandItem::new("open", "Open Story").shortcut("⌘O"),
                    CommandItem::new("save", "Save Story").shortcut("⌘S"),
                    CommandItem::new("qa", "Run Conformance").category("QA"),
                ],
            )
            .query("story")
            .selected_index(0)
            .into_any_element(),
            "ui-kit.drag-list-component" => DragList::new(
                scoped("drag-list"),
                vec![
                    DragItem::new("one", Text::new("Props")),
                    DragItem::new("two", Text::new("Preview")),
                    DragItem::new("three", Text::new("QA")),
                ],
            )
            .into_any_element(),
            "ui-kit.notification-component" => Notification::new(scoped("notification"), label)
                .description("Conformance report passed")
                .variant(notification_variant(&variant_name))
                .dismissible(open)
                .into_any_element(),
            "ui-kit.tag-component" => Tag::new(scoped("tag"), label)
                .variant(tag_variant(&variant_name))
                .removable(open)
                .into_any_element(),
            "ui-kit.toolbar-component" => Toolbar::new(scoped("toolbar"))
                .item(ToolbarItem::button(scoped("toolbar-save"), "Save").active(selected))
                .separator()
                .item(ToolbarItem::button(scoped("toolbar-run"), "Run").disabled(disabled))
                .design(design)
                .into_any_element(),
            "ui-kit.tree-view-component" => {
                let mut expanded = HashSet::new();
                expanded.insert(SharedString::from("root"));
                TreeView::new(
                    scoped("tree-view"),
                    vec![TreeNode::new("root", label).children(vec![
                        TreeNode::new("child-props", "Props").leaf(true),
                        TreeNode::new("child-renderer", "Renderer").leaf(true),
                    ])],
                )
                .expanded(expanded)
                .selected("child-renderer")
                .into_any_element()
            }
            "ui-kit.table-component" => {
                let rows = vec![
                    ("Button".to_string(), "interactive".to_string()),
                    ("Chart".to_string(), "responsive".to_string()),
                ];
                Table::new(scoped("table"), rows)
                    .columns(vec![
                        Column::new("component", "Component").cell_render(
                            |row: &(String, String), _, _, _| Text::new(row.0.clone()),
                        ),
                        Column::new("status", "Status").cell_render(
                            |row: &(String, String), _, _, _| Badge::new(row.1.clone()),
                        ),
                    ])
                    .design(design)
                    .into_any_element()
            }
            "ui-kit.workflow-node" => WorkflowNode::new(
                scoped("workflow-node"),
                WorkflowNodeData::new(label, Position::new(0.0, 0.0)).with_ports(2, 1),
            )
            .selected(selected)
            .into_any_element(),
            "ui-kit.focus-group" => FocusGroup::new(scoped("focus-group"))
                .direction(FocusDirection::Horizontal)
                .wraparound(open)
                .child(Button::new(scoped("focus-first"), label.clone()).disabled(disabled))
                .child(Button::new(scoped("focus-second"), "Second"))
                .child(Input::new(scoped("focus-input")).placeholder("Focusable input"))
                .into_any_element(),
            "ui-kit.workflow-port" => HStack::new()
                .child(
                    Port::new(scoped("workflow-port-in"), PortDirection::Input, 0)
                        .connected(selected),
                )
                .child(Text::new(label.clone()).size(TextSize::Sm))
                .child(
                    Port::new(scoped("workflow-port-out"), PortDirection::Output, 0)
                        .connected(open)
                        .valid_target(Some(!disabled)),
                )
                .into_any_element(),
            "ui-kit.workflow-canvas" => div()
                .w(px(420.0))
                .h(px(260.0))
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .overflow_hidden()
                .child(cx.new(|cx| WorkflowCanvas::with_graph(sample_workflow_graph(label), cx)))
                .into_any_element(),
            "ui-kit.showcase-component" => div()
                .id(scoped("showcase-component"))
                .w(px(420.0))
                .max_h(px(300.0))
                .overflow_y_scroll()
                .child(cx.new(|cx| Showcase::embedded_section(ShowcaseSection::Buttons, cx)))
                .into_any_element(),
            _ => div()
                .child(Text::new("No exported component renderer registered").muted(true))
                .into_any_element(),
        };

        div()
            .max_w_full()
            .flex()
            .items_center()
            .justify_center()
            .child(element)
            .into_any_element()
    }

    pub(super) fn render_button_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        interactive: bool,
        design: Arc<DesignSystem>,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = text_prop(story, "label", "Save");
        let variant = button_variant(&choice_prop(story, "variant", "primary"));
        let disabled = bool_prop(story, "disabled", false);
        let entity = self.entity.clone();
        let mut button = Button::new(lab_id(&["preview-button", scope]), label)
            .variant(variant)
            .size(ButtonSize::Lg)
            .design(design)
            .disabled(disabled);
        if interactive {
            button = button.on_click(move |_window, cx| {
                entity.update(cx, |this, _| {
                    this.save_status = Some("Preview button clicked".into());
                });
            });
        }
        div().child(button).into_any_element()
    }

    pub(super) fn render_form_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        interactive: bool,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let label = text_prop(story, "label", "Gain");
        let value = number_prop(story, "value", 0.5).clamp(0.0, 1.0);
        let story_id = story.id.clone();
        let entity = self.entity.clone();

        let mut slider = Slider::new(lab_id(&["form-slider", scope]))
            .label(label.clone())
            .range(0.0, 1.0)
            .value(value as f32)
            .show_value(true)
            .design(design)
            .width(220.0);
        if interactive {
            slider = slider.on_change(move |new_value, _window, cx| {
                entity.update(cx, |this, _| {
                    this.set_prop(&story_id, "value", StoryPropValue::Number(new_value as f64));
                });
            });
        }

        div()
            .w(px(320.0))
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(
                Input::new(lab_id(&["form-input", scope]))
                    .value(label.clone())
                    .label("Label")
                    .readonly(true),
            )
            .child(slider)
            .child(
                Toggle::new(lab_id(&["form-toggle", scope]))
                    .checked(value > 0.5)
                    .label("Above midpoint")
                    .disabled(true),
            )
            .into_any_element()
    }

    pub(super) fn render_status_story(
        &self,
        story: &ComponentStory,
        _scope: &str,
        _design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let label = text_prop(story, "label", "Ready");
        let variant = badge_variant(&choice_prop(story, "variant", "success"));
        let progress_variant = progress_variant(&choice_prop(story, "variant", "success"));
        let value = number_prop(story, "value", 0.72).clamp(0.0, 1.0) as f32;

        div()
            .w(px(360.0))
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Text::new("Build Status").size(TextSize::Sm))
                    .child(
                        Badge::new(label)
                            .variant(variant)
                            .size(BadgeSize::Lg)
                            .rounded(true),
                    ),
            )
            .child(
                Progress::new(value)
                    .variant(progress_variant)
                    .size(ProgressSize::Lg)
                    .show_label(true)
                    .aria_label("Story progress"),
            )
            .into_any_element()
    }

    pub(super) fn render_navigation_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        _design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let selected = number_prop(story, "selected", 1.0).round().clamp(0.0, 2.0) as usize;
        let variant = tab_variant(&choice_prop(story, "variant", "pills"));
        let tabs = Tabs::new(lab_id(&["preview-tabs", scope]))
            .tabs(vec![
                TabItem::new("overview", "Overview"),
                TabItem::new("tokens", "Tokens").badge("4"),
                TabItem::new("motion", "Motion"),
            ])
            .selected_index(selected)
            .variant(variant)
            .aria_label("Component lab navigation");

        div()
            .w(px(420.0))
            .p_4()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(tabs)
            .into_any_element()
    }

    pub(super) fn render_feedback_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        _design: Arc<DesignSystem>,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let variant = alert_variant(&choice_prop(story, "variant", "info"));
        let message = text_prop(story, "message", "Design tokens validated");
        Alert::new(lab_id(&["preview-alert", scope]), message)
            .title("Conformance")
            .variant(variant)
            .closeable(false)
            .into_any_element()
    }

    pub(super) fn render_card_story(
        &self,
        story: &ComponentStory,
        _scope: &str,
        _design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title = text_prop(story, "title", "Preview");
        let content = text_prop(story, "content", "Responsive component composition");
        let theme = cx.theme();

        Card::new()
            .header(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Heading::new(title).level(3))
                    .child(Badge::new("Lab").variant(BadgeVariant::Info).rounded(true)),
            )
            .content(
                div()
                    .w(px(360.0))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new(content).size(TextSize::Sm).muted(true))
                    .child(
                        div()
                            .h(px(8.0))
                            .rounded_full()
                            .bg(theme.accent_muted)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(0.62))
                                    .rounded_full()
                                    .bg(theme.accent),
                            ),
                    ),
            )
            .footer(
                Text::new("Theme-aware slots")
                    .size(TextSize::Xs)
                    .muted(true),
            )
            .into_any_element()
    }

    pub(super) fn render_ui_kit_showcase_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        if scope.starts_with("matrix-") {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .gap_2()
                .p_4()
                .child(
                    Badge::new("Showcase")
                        .variant(BadgeVariant::Info)
                        .rounded(true),
                )
                .child(Text::new(story.title.clone()).size(TextSize::Sm))
                .child(
                    Text::new(story.description.clone())
                        .size(TextSize::Xs)
                        .color(theme.text_secondary),
                )
                .into_any_element();
        }

        self.ui_showcases
            .get(&story.id)
            .cloned()
            .map(|showcase| {
                div()
                    .size_full()
                    .min_w_0()
                    .min_h_0()
                    .child(showcase)
                    .into_any_element()
            })
            .unwrap_or_else(|| {
                div()
                    .child(Text::new("Showcase section unavailable").muted(true))
                    .into_any_element()
            })
    }

    pub(super) fn render_potentiometer_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        interactive: bool,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let label = text_prop(story, "label", "Frequency");
        let value = number_prop(story, "value", 1000.0).clamp(20.0, 20_000.0);
        let scale = if choice_prop(story, "scale", "logarithmic") == "logarithmic" {
            AudioScale::Logarithmic
        } else {
            AudioScale::Linear
        };
        let story_id = story.id.clone();
        let entity = self.entity.clone();

        let mut knob = Potentiometer::new(lab_id(&["preview-pot", scope]))
            .label(label)
            .value(value)
            .min(20.0)
            .max(20_000.0)
            .unit("Hz")
            .scale(scale)
            .design(design)
            .size(PotentiometerSize::Lg);
        if interactive {
            knob = knob.on_change(move |new_value, _window, cx| {
                entity.update(cx, |this, _| {
                    this.set_prop(&story_id, "value", StoryPropValue::Number(new_value));
                });
            });
        }

        div()
            .p_6()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(knob)
            .into_any_element()
    }

    pub(super) fn render_vertical_slider_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        interactive: bool,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let label = text_prop(story, "label", "Gain");
        let min = number_prop(story, "min", -60.0);
        let max = number_prop(story, "max", 6.0).max(min + 0.001);
        let value = number_prop(story, "value", -6.0).clamp(min, max);
        let peak = number_prop(story, "peak", -1.5).clamp(min, max);
        let scale = if choice_prop(story, "scale", "linear") == "logarithmic" {
            AudioScale::Logarithmic
        } else {
            AudioScale::Linear
        };
        let story_id = story.id.clone();
        let entity = self.entity.clone();

        let mut slider = VerticalSlider::new(lab_id(&["preview-vertical-slider", scope]))
            .label(label)
            .value(value)
            .min(min)
            .max(max)
            .unit("dB")
            .peak(Some(peak))
            .scale(scale)
            .size(VerticalSliderSize::Lg)
            .height(170.0)
            .selected(interactive)
            .design(design);

        if bool_prop(story, "ticks", true) {
            slider = slider.with_ticks();
        }

        if interactive {
            slider = slider.on_change(move |new_value, _window, cx| {
                entity.update(cx, |this, _| {
                    this.set_prop(&story_id, "value", StoryPropValue::Number(new_value));
                });
            });
        }

        div()
            .min_w(px(140.0))
            .h(px(260.0))
            .flex()
            .items_center()
            .justify_center()
            .p_5()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(slider)
            .into_any_element()
    }

    pub(super) fn render_volume_knob_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        interactive: bool,
        _design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let label = text_prop(story, "label", "Output");
        let value = number_prop(story, "value", 0.72).clamp(0.0, 1.0) as f32;
        let muted = bool_prop(story, "muted", false);
        let story_id = story.id.clone();
        let mute_story_id = story.id.clone();
        let entity = self.entity.clone();
        let mute_entity = self.entity.clone();

        let mut knob = VolumeKnob::new()
            .id(lab_id(&["preview-volume-knob", scope]))
            .label(label)
            .value(value)
            .muted(muted)
            .size(px(if scope.starts_with("matrix-") {
                52.0
            } else {
                72.0
            }))
            .accent_color(theme.accent)
            .bg_color(theme.background)
            .text_color(theme.text_primary)
            .muted_color(theme.text_muted);

        if interactive {
            knob = knob
                .on_change(move |new_value, _window, cx| {
                    entity.update(cx, |this, _| {
                        this.set_prop(&story_id, "value", StoryPropValue::Number(new_value as f64));
                    });
                })
                .on_mute_toggle(move |new_muted, _window, cx| {
                    mute_entity.update(cx, |this, _| {
                        this.set_prop(&mute_story_id, "muted", StoryPropValue::Bool(new_muted));
                    });
                });
        }

        div()
            .min_w(px(150.0))
            .h(px(170.0))
            .flex()
            .items_center()
            .justify_center()
            .p_5()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(knob)
            .into_any_element()
    }

    pub(super) fn render_meter_story(
        &self,
        story: &ComponentStory,
        _scope: &str,
        _design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let level_db = number_prop(story, "level_db", -12.0);
        let peak_db = number_prop(story, "peak_db", -3.0);

        div()
            .w(px(140.0))
            .h(px(180.0))
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .p_4()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(
                div().h(px(120.0)).flex().items_stretch().child(
                    LevelMeterElement::new(level_db, "L")
                        .peak(peak_db)
                        .width(px(24.0)),
                ),
            )
            .child(Text::new(format!("{level_db:.1} dB")).size(TextSize::Sm))
            .into_any_element()
    }

    pub(super) fn render_horizontal_meter_story(
        &self,
        story: &ComponentStory,
        _scope: &str,
        _design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let kind = choice_prop(story, "kind", "lufs");
        let label = text_prop(story, "label", "LUFS");
        let raw_value = number_prop(story, "value", -18.0);
        let mut tick_config = match kind.as_str() {
            "stereo_width" => TickConfig::stereo_width(),
            "peak_spread" => TickConfig::peak_spread(),
            _ => TickConfig::lufs(),
        };
        tick_config.tick_color = theme.border_hover;
        let value = raw_value.clamp(tick_config.min, tick_config.max);
        let meter_theme = HorizontalMeterTheme {
            color_normal: theme.success,
            color_warning: theme.warning,
            color_critical: theme.error,
            color_info: theme.info,
            color_background: theme.background,
            color_border: theme.border,
            color_text: theme.text_secondary,
            use_gradient: bool_prop(story, "gradient", true),
            ..HorizontalMeterTheme::default()
        };

        div()
            .w(px(430.0))
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(render_horizontal_meter_bar(
                label,
                value,
                &tick_config,
                meter_theme.clone(),
            ))
            .child(render_tick_row(
                &tick_config,
                meter_theme.label_width,
                meter_theme.value_width,
            ))
            .into_any_element()
    }

    pub(super) fn render_spectrum_story(
        &self,
        story: &ComponentStory,
        _scope: &str,
        _design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let bins = number_prop(story, "bins", 64.0).clamp(8.0, 128.0).round() as usize;
        let magnitudes = spectrum_magnitudes(bins);

        div()
            .w(px(360.0))
            .p_4()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(SpectrumElement::new(magnitudes).height(px(150.0)))
            .into_any_element()
    }

    pub(super) fn render_spectrum_axis_story(
        &self,
        story: &ComponentStory,
        _scope: &str,
        _design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let min_freq = number_prop(story, "min_freq", 20.0).clamp(1.0, 96_000.0) as f32;
        let max_freq =
            number_prop(story, "max_freq", 20_000.0).clamp(min_freq as f64 + 1.0, 192_000.0) as f32;
        let axis_theme = SpectrumAxisTheme {
            text_color: theme.text_secondary,
            ..SpectrumAxisTheme::default()
        };
        let db_axis_width = axis_theme.db_axis_width;
        let magnitudes = spectrum_axis_magnitudes();

        div()
            .w(px(460.0))
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(
                div()
                    .h(px(170.0))
                    .flex()
                    .gap_1()
                    .child(render_spectrum_db_axis(axis_theme.clone()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_hidden()
                            .rounded_sm()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .child(
                                SpectrumElement::new(magnitudes)
                                    .frequency_range(min_freq, max_freq)
                                    .height(px(170.0)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(div().w(px(db_axis_width)))
                    .child(render_spectrum_frequency_axis(
                        min_freq, max_freq, axis_theme,
                    )),
            )
            .into_any_element()
    }

    pub(super) fn render_line_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let compact = scope.starts_with("matrix-");
        let (min_width, min_height) = self.chart_minimum(scope);
        let data = line_story_data(&choice_prop(story, "series", "sine"));

        let mut chart = line(&data.x, &data.y)
            .title(data.title)
            .x_label(data.x_label)
            .y_label(data.y_label)
            .label(data.primary_label)
            .color(0x2563eb)
            .stroke_width(if compact { 2.0 } else { 2.5 })
            .show_points(!compact)
            .x_scale(data.x_scale)
            .design(design)
            .legend_position(if compact {
                LegendPosition::Hidden
            } else {
                LegendPosition::Bottom
            });

        if let Some((min, max)) = data.y_range {
            chart = chart.y_range(min, max);
        }

        if let Some(extra) = data.comparison_y {
            chart = chart
                .add_series(&extra, Some(data.comparison_label), 0xf97316, 1.75, 0.9)
                .series_dash_array(StrokeDashArray::Dashed);
        }

        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        match chart.build() {
            Ok(chart) => div()
                .w_full()
                .h_full()
                .min_w_0()
                .min_h_0()
                .child(chart)
                .into_any_element(),
            Err(err) => render_chart_error(err, theme),
        }
    }

    pub(super) fn render_bar_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let compact = scope.starts_with("matrix-");
        let (min_width, min_height) = self.chart_minimum(scope);
        let bar_count = number_prop(story, "bars", 8.0).round().clamp(3.0, 12.0) as usize;
        let data = bar_story_data(bar_count);

        let mut chart = bar(&data.categories, &data.values)
            .title("Category Mix")
            .label("Current")
            .color(0x2563eb)
            .bar_gap(if compact { 2.0 } else { 4.0 })
            .border_radius(if compact { 2.0 } else { 4.0 })
            .add_series(&data.comparison_values, Some("Target"), 0xf97316, 0.76)
            .design(design)
            .legend_position(if compact {
                LegendPosition::Hidden
            } else {
                LegendPosition::Bottom
            });

        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        match chart.build() {
            Ok(chart) => div()
                .w_full()
                .h_full()
                .min_w_0()
                .min_h_0()
                .child(chart)
                .into_any_element(),
            Err(err) => render_chart_error(err, theme),
        }
    }

    pub(super) fn render_scatter_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let compact = scope.starts_with("matrix-");
        let (min_width, min_height) = self.chart_minimum(scope);
        let count = number_prop(story, "points", 48.0).round().clamp(12.0, 96.0) as usize;
        let (x, y) = scatter_story_data(count);

        let mut chart = scatter(&x, &y)
            .title("Correlation")
            .color(0x2563eb)
            .point_radius(if compact { 3.0 } else { 4.5 })
            .opacity(0.78)
            .design(design);

        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_area_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let (min_width, min_height) = self.chart_minimum(scope);
        let data = area_story_data(&choice_prop(story, "series", "envelope"));

        let mut chart = area(&data.x, &data.y)
            .title(data.title)
            .color(0x14b8a6)
            .opacity(0.58)
            .design(design);
        if let Some(y0) = data.y0 {
            chart = chart.y0(&y0);
        }
        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_heatmap_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let (min_width, min_height) = self.chart_minimum(scope);
        let size = number_prop(story, "size", 18.0).round().clamp(8.0, 32.0) as usize;
        let z = scalar_field_data(size, size);
        let mut chart = heatmap(&z, size, size)
            .title("Response Field")
            .color_scale(color_scale(&choice_prop(story, "scale", "viridis")))
            .design(design);
        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_mesh_plot_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let (min_width, min_height) = self.chart_minimum(scope);
        let story_id = story.id.as_str();
        let (mesh, field, view, mode, title, selection, color_scale) = match story_id {
            "px.mesh_plot.mesh_only" => (
                mesh_plot_square_mesh("component-lab-mesh-only", false),
                None,
                MeshPlotView::Planar {
                    horizontal: CoordinateAxis::X,
                    vertical: CoordinateAxis::Y,
                },
                gpui_px::MeshRenderMode::Mesh,
                "Mesh only",
                None,
                ColorScale::Greys,
            ),
            "px.mesh_plot.smooth_fill" => (
                mesh_plot_square_mesh("component-lab-smooth-fill", false),
                Some(mesh_plot_square_vertex_field("component-lab-smooth-field")),
                MeshPlotView::Planar {
                    horizontal: CoordinateAxis::X,
                    vertical: CoordinateAxis::Y,
                },
                gpui_px::MeshRenderMode::ScalarFill {
                    interpolation: gpui_px::FieldInterpolation::Smooth,
                },
                "Smooth scalar fill",
                None,
                ColorScale::Viridis,
            ),
            "px.mesh_plot.flat_fill" => (
                mesh_plot_square_mesh("component-lab-flat-fill", false),
                Some(mesh_plot_square_cell_field("component-lab-flat-field")),
                MeshPlotView::Planar {
                    horizontal: CoordinateAxis::X,
                    vertical: CoordinateAxis::Y,
                },
                gpui_px::MeshRenderMode::ScalarFill {
                    interpolation: gpui_px::FieldInterpolation::Flat,
                },
                "Flat cell fill",
                None,
                ColorScale::Plasma,
            ),
            "px.mesh_plot.filled_contours" => (
                mesh_plot_saddle_mesh("component-lab-filled-contours"),
                Some(mesh_plot_saddle_field("component-lab-contour-field")),
                MeshPlotView::Planar {
                    horizontal: CoordinateAxis::X,
                    vertical: CoordinateAxis::Y,
                },
                gpui_px::MeshRenderMode::FilledContours {
                    levels: ContourLevels::Count(6),
                },
                "Filled contours",
                None,
                ColorScale::Coolwarm,
            ),
            "px.mesh_plot.isolines" => (
                mesh_plot_saddle_mesh("component-lab-isolines"),
                Some(mesh_plot_saddle_field("component-lab-isoline-field")),
                MeshPlotView::Planar {
                    horizontal: CoordinateAxis::X,
                    vertical: CoordinateAxis::Y,
                },
                gpui_px::MeshRenderMode::Isolines {
                    levels: ContourLevels::Count(6),
                },
                "Isolines",
                None,
                ColorScale::Coolwarm,
            ),
            "px.mesh_plot.combined" => (
                mesh_plot_saddle_mesh("component-lab-combined"),
                Some(mesh_plot_saddle_field("component-lab-combined-field")),
                MeshPlotView::Planar {
                    horizontal: CoordinateAxis::X,
                    vertical: CoordinateAxis::Y,
                },
                gpui_px::MeshRenderMode::FillAndIsolines {
                    levels: ContourLevels::Count(6),
                },
                "Combined scalar mesh",
                None,
                ColorScale::Viridis,
            ),
            "px.mesh_plot.axisymmetric_section" => (
                mesh_plot_annulus_mesh("component-lab-axisymmetric-section"),
                Some(mesh_plot_annulus_field("component-lab-axisymmetric-field")),
                MeshPlotView::AxisymmetricSection {
                    radial: CoordinateAxis::X,
                    axial: CoordinateAxis::Z,
                },
                gpui_px::MeshRenderMode::ScalarFill {
                    interpolation: gpui_px::FieldInterpolation::Smooth,
                },
                "Axisymmetric r-z section",
                None,
                ColorScale::Viridis,
            ),
            "px.mesh_plot.revolve" => (
                mesh_plot_annulus_mesh("component-lab-revolve-profile"),
                Some(mesh_plot_annulus_field("component-lab-revolve-field")),
                MeshPlotView::AxisymmetricRevolve(RevolveSpec {
                    radial: CoordinateAxis::X,
                    axial: CoordinateAxis::Z,
                    start_angle: 0.0,
                    sweep_angle: std::f64::consts::TAU,
                    segments: 12,
                    end_caps: false,
                }),
                gpui_px::MeshRenderMode::ScalarFill {
                    interpolation: gpui_px::FieldInterpolation::Smooth,
                },
                "Axisymmetric revolve",
                None,
                ColorScale::Plasma,
            ),
            "px.mesh_plot.surface3d" => (
                mesh_plot_surface_mesh("component-lab-surface3d"),
                Some(mesh_plot_surface_field("component-lab-surface-field")),
                MeshPlotView::Surface3d,
                gpui_px::MeshRenderMode::ScalarFill {
                    interpolation: gpui_px::FieldInterpolation::Smooth,
                },
                "Unstructured surface 3D",
                None,
                ColorScale::Viridis,
            ),
            "px.mesh_plot.large_mesh" => (
                mesh_plot_large_mesh("component-lab-large-mesh"),
                Some(mesh_plot_large_field("component-lab-large-field")),
                MeshPlotView::Surface3d,
                gpui_px::MeshRenderMode::ScalarFill {
                    interpolation: gpui_px::FieldInterpolation::Smooth,
                },
                "Large unstructured surface 3D",
                None,
                ColorScale::Viridis,
            ),
            "px.mesh_plot.picking" => {
                let mesh = mesh_plot_square_mesh("component-lab-picking-mesh", true);
                let field = mesh_plot_square_vertex_field("component-lab-picking-field");
                (
                    mesh,
                    Some(field),
                    MeshPlotView::Planar {
                        horizontal: CoordinateAxis::X,
                        vertical: CoordinateAxis::Y,
                    },
                    gpui_px::MeshRenderMode::ScalarFill {
                        interpolation: gpui_px::FieldInterpolation::Smooth,
                    },
                    "Mesh picking",
                    Some(MeshPlotPick {
                        plot_id: "component-lab-picking".into(),
                        mesh_id: "component-lab-picking-mesh".into(),
                        cell_index: 1,
                        cell_id: Some(2001),
                        nearest_vertex_index: Some(2),
                        vertex_id: Some(102),
                        world_position: [0.72, 0.72, 0.0],
                        displayed_value: Some(1.35),
                        field_id: Some("component-lab-picking-field".into()),
                    }),
                    ColorScale::Viridis,
                )
            }
            _ => {
                let mode = match choice_prop(story, "mode", "combined").as_ref() {
                    "mesh" => gpui_px::MeshRenderMode::Mesh,
                    "smooth_fill" => gpui_px::MeshRenderMode::ScalarFill {
                        interpolation: gpui_px::FieldInterpolation::Smooth,
                    },
                    "flat_fill" => gpui_px::MeshRenderMode::ScalarFill {
                        interpolation: gpui_px::FieldInterpolation::Flat,
                    },
                    "filled_contours" => gpui_px::MeshRenderMode::FilledContours {
                        levels: ContourLevels::Count(6),
                    },
                    "isolines" => gpui_px::MeshRenderMode::Isolines {
                        levels: ContourLevels::Count(6),
                    },
                    _ => gpui_px::MeshRenderMode::FillAndIsolines {
                        levels: ContourLevels::Count(6),
                    },
                };
                (
                    mesh_plot_square_mesh("component-lab-mesh", false),
                    Some(mesh_plot_square_vertex_field("component-lab-field")),
                    MeshPlotView::Planar {
                        horizontal: CoordinateAxis::X,
                        vertical: CoordinateAxis::Y,
                    },
                    mode,
                    "Mesh plot",
                    None,
                    ColorScale::Viridis,
                )
            }
        };

        let mut chart = mesh_plot(mesh)
            .view(view)
            .mode(mode)
            .color_scale(color_scale)
            .title(title)
            .design(design)
            .interactions(PlotInteractions::inspect_and_navigate())
            .wireframe(if bool_prop(story, "wireframe", true) {
                gpui_px::Wireframe::overlay()
            } else {
                gpui_px::Wireframe::hidden()
            });
        if let Some(field) = field {
            chart = chart.field(field);
        }
        if let Some(selection) = selection {
            chart = chart.selection(selection);
        }
        chart = if bool_prop(story, "fill", true) {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };
        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_contour_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let (min_width, min_height) = self.chart_minimum(scope);
        let size = number_prop(story, "size", 24.0).round().clamp(12.0, 40.0) as usize;
        let z = scalar_field_data(size, size);
        let mut chart = contour(&z, size, size)
            .title("Density Bands")
            .thresholds(vec![-0.8, -0.4, 0.0, 0.4, 0.8])
            .color_scale(ColorScale::Plasma)
            .design(design);
        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_isoline_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let (min_width, min_height) = self.chart_minimum(scope);
        let size = number_prop(story, "size", 24.0).round().clamp(12.0, 40.0) as usize;
        let z = scalar_field_data(size, size);
        let mut chart = isoline(&z, size, size)
            .title("Level Curves")
            .levels(vec![-0.6, -0.2, 0.2, 0.6])
            .color(0x334155)
            .stroke_width(1.5)
            .design(design);
        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_pie_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.render_pie_like_chart_story(story, scope, design, cx, bool_prop(story, "donut", false))
    }

    pub(super) fn render_donut_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.render_pie_like_chart_story(story, scope, design, cx, true)
    }

    pub(super) fn render_pie_like_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
        donut_chart: bool,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let (min_width, min_height) = self.chart_minimum(scope);
        let count = number_prop(story, "slices", 5.0).round().clamp(3.0, 8.0) as usize;
        let labels = (0..count)
            .map(|index| format!("S{}", index + 1))
            .collect::<Vec<_>>();
        let values = (0..count)
            .map(|index| 12.0 + (index as f64 * 1.7).sin().abs() * 36.0 + index as f64 * 4.0)
            .collect::<Vec<_>>();
        let mut chart = if donut_chart {
            donut(&values)
        } else {
            pie(&values)
        }
        .labels(&labels)
        .title(if donut_chart { "Share" } else { "Mix" })
        .design(design);

        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_boxplot_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let (min_width, min_height) = self.chart_minimum(scope);
        let groups = number_prop(story, "groups", 5.0).round().clamp(3.0, 8.0) as usize;
        let (x, y) = boxplot_story_data(groups);
        let mut chart = boxplot(&x, &y)
            .title("Distribution")
            .bins(groups)
            .box_color(0x2563eb)
            .median_color(0xf97316)
            .design(design);

        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_treemap_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let (min_width, min_height) = self.chart_minimum(scope);
        let root = treemap_story_data();
        let mut chart = treemap(&root)
            .title("Toolkit Surface")
            .tiling_method(tiling_method(&choice_prop(story, "tiling", "squarify")))
            .padding(2.0)
            .design(design);

        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn render_surface3d_chart_story(
        &self,
        story: &ComponentStory,
        scope: &str,
        design: Arc<DesignSystem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let fill = bool_prop(story, "fill", true);
        let (min_width, min_height) = self.chart_minimum(scope);
        let size = number_prop(story, "size", 22.0).round().clamp(8.0, 34.0) as usize;
        let z = scalar_field_data(size, size);
        let mut chart = surface3d(&z, size, size)
            .title("Surface Response")
            .colormap(surface_colormap(&choice_prop(story, "colormap", "viridis")))
            .wireframe(bool_prop(story, "wireframe", false))
            .design(design);

        chart = if fill {
            chart
                .fill()
                .min_size(min_width, min_height)
                .aspect_ratio(self.layout_constraints.aspect_ratio)
        } else {
            chart.size(min_width, min_height)
        };

        render_chart_result(chart.build(), theme)
    }

    pub(super) fn chart_minimum(&self, scope: &str) -> (f32, f32) {
        if scope.starts_with("matrix-") {
            (220.0, 145.0)
        } else {
            (
                self.layout_constraints.min_width,
                self.layout_constraints.min_height,
            )
        }
    }

    #[cfg(feature = "visual-capture")]
    pub(super) fn release_visual_capture_resources(&mut self, cx: &mut Context<Self>) {
        for showcase in self.ui_showcases.values() {
            showcase.update(cx, |showcase, _cx| {
                showcase.release_entity_handle();
            });
        }
        self.ui_showcases.clear();
    }
}

fn build_ui_showcase_entities(
    story_ids: &[String],
    cx: &mut Context<ComponentLab>,
) -> BTreeMap<String, Entity<Showcase>> {
    let mut showcases = BTreeMap::new();
    for story_id in story_ids {
        if let Some(section) = showcase_section_for_story_id(story_id) {
            let showcase = cx.new(|cx| Showcase::embedded_section(section, cx));
            showcases.insert(story_id.clone(), showcase);
        }
    }
    showcases
}

impl ComponentLab {
    /// Small in-UI overlay showing the last measured allocation deltas.
    pub(super) fn render_alloc_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let render_ok = self.last_render_alloc.count == 0;
        let mouse_ok = self.last_mouse_move_alloc.count == 0;
        let last_ok = self.last_sample.as_ref().is_none_or(|(_, s)| s.count == 0);

        div()
            .id("alloc-overlay")
            .absolute()
            .top_4()
            .right_4()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(if render_ok && mouse_ok && last_ok {
                theme.surface
            } else {
                theme.error
            })
            .text_color(if render_ok && mouse_ok && last_ok {
                theme.text_primary
            } else {
                theme.text_on_accent
            })
            .shadow_lg()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                Text::new(format!(
                    "render: {} bytes / {} allocs",
                    self.last_render_alloc.bytes, self.last_render_alloc.count
                ))
                .size(TextSize::Xs),
            )
            .child(
                Text::new(format!(
                    "mouse: {} bytes / {} allocs",
                    self.last_mouse_move_alloc.bytes, self.last_mouse_move_alloc.count
                ))
                .size(TextSize::Xs),
            )
            .when_some(self.last_sample.as_ref(), |el, (label, snapshot)| {
                el.child(
                    Text::new(format!(
                        "last ({label}): {} bytes / {} allocs",
                        snapshot.bytes, snapshot.count
                    ))
                    .size(TextSize::Xs),
                )
            })
            .into_any_element()
    }
}

impl Render for ComponentLab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        // Detect window resize and sample allocations triggered by it.
        let current_size = window.bounds().size;
        let resized = self
            .last_window_size
            .is_some_and(|last| last != current_size);
        self.last_window_size = Some(current_size);
        if resized {
            self.record_sample("resize");
        }

        if self.visual_capture_mode {
            let story = self.selected_story();
            let design = design_for_theme_preset(self.selected_theme_preset());
            let constraints = self.layout_constraints;
            let result = div()
                .id("gpui-component-lab-visual-capture")
                .size_full()
                .flex()
                .bg(theme.background)
                .text_color(theme.text_primary)
                .child(
                    apply_preview_builder_style(
                        div()
                            .id("gpui-component-lab-visual-capture-surface")
                            .size_full()
                            .flex()
                            .gap(px(constraints.gap)),
                        constraints,
                        theme,
                    )
                    .p(px(constraints.padding))
                    .child(self.render_story_preview(
                        story,
                        "visual-capture",
                        false,
                        design,
                        cx,
                    )),
                )
                .into_any_element();
            self.last_render_alloc = self.record_sample("visual-capture-render");
            return result;
        }

        // Sync child entity state only when it changes so stable subtrees are
        // not marked dirty on every frame.
        {
            let sidebar = self.sidebar_entity.read(cx);
            let live_status = self.live_status.clone();
            if sidebar.selected_story_id != self.selected_story_id
                || sidebar.live_status != live_status
                || sidebar.registry_len != self.registry.len()
            {
                self.sidebar_entity.update(cx, |sidebar, _cx| {
                    sidebar.selected_story_id = self.selected_story_id.clone();
                    sidebar.live_status = live_status;
                    sidebar.registry_len = self.registry.len();
                });
            }
        }

        {
            let toolbar = self.toolbar_entity.read(cx);
            let story = self.selected_story();
            let viewport = self.selected_viewport();
            let theme_preset = self.selected_theme_preset();
            if toolbar.story_id != story.id
                || toolbar.viewport_id != viewport.id.as_ref()
                || toolbar.theme_id != theme_preset.id.as_ref()
                || toolbar.matrix_mode != self.matrix_mode
            {
                self.toolbar_entity.update(cx, |toolbar, _cx| {
                    toolbar.story_id = story.id.clone();
                    toolbar.viewport_id = viewport.id.to_string();
                    toolbar.theme_id = theme_preset.id.to_string();
                    toolbar.matrix_mode = self.matrix_mode;
                });
            }
        }

        {
            let controls = self.controls_panel_entity.read(cx);
            let story_id = self.selected_story().id.clone();
            let constraints = self.layout_constraints;
            let save_status = self.save_status.clone();
            if controls.story_id != story_id
                || controls.layout_constraints != constraints
                || controls.save_status != save_status
            {
                let story = self.selected_story().clone();
                self.controls_panel_entity.update(cx, |controls, _cx| {
                    controls.story_id = story_id;
                    controls.story = story;
                    controls.layout_constraints = constraints;
                    controls.save_status = save_status;
                });
            }
        }

        {
            let preview = self.preview_area_entity.read(cx);
            let story_id = self.selected_story().id.clone();
            let viewport_id = self.selected_viewport().id.clone();
            let theme_id = self.selected_theme_preset().id.clone();
            let motion_id = self.selected_motion_preset().id.clone();
            let constraints = self.layout_constraints;
            if preview.story_id != story_id
                || preview.viewport.id != viewport_id
                || preview.theme.id != theme_id
                || preview.motion.id != motion_id
                || preview.layout_constraints != constraints
                || preview.matrix_mode != self.matrix_mode
            {
                let story = self.selected_story().clone();
                let viewport = self.selected_viewport().clone();
                let theme = self.selected_theme_preset().clone();
                let motion = self.selected_motion_preset().clone();
                let matrix = self.cached_matrix.clone();
                self.preview_area_entity.update(cx, |preview, _cx| {
                    preview.story_id = story_id;
                    preview.story = story;
                    preview.viewport = viewport;
                    preview.theme = theme;
                    preview.motion = motion;
                    preview.layout_constraints = constraints;
                    preview.matrix_mode = self.matrix_mode;
                    preview.cached_matrix = matrix;
                });
            }
        }

        let result = div()
            .id("gpui-component-lab-root")
            .relative()
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .flex()
            .when(cfg!(feature = "profiler"), |el| {
                let entity = self.entity.clone();
                el.on_mouse_move({
                    let entity = entity.clone();
                    move |_event, _window, cx| {
                        entity.update(cx, |this, _cx| {
                            this.last_mouse_move_alloc = this.record_sample("mouse-move");
                        });
                    }
                })
                .on_mouse_down(MouseButton::Left, {
                    let entity = entity.clone();
                    move |_event, _window, cx| {
                        entity.update(cx, |this, _cx| {
                            this.record_sample("mouse-down");
                        });
                    }
                })
                .on_mouse_up(MouseButton::Left, {
                    let entity = entity.clone();
                    move |_event, _window, cx| {
                        entity.update(cx, |this, _cx| {
                            this.record_sample("mouse-up");
                        });
                    }
                })
                .on_scroll_wheel({
                    let entity = entity.clone();
                    move |_event, _window, cx| {
                        entity.update(cx, |this, _cx| {
                            this.record_sample("scroll");
                        });
                    }
                })
            })
            .child(self.sidebar_entity.clone())
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .p_5()
                    .child(self.toolbar_entity.clone())
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h_full()
                                    .child(self.preview_area_entity.clone()),
                            )
                            .child(self.controls_panel_entity.clone()),
                    ),
            )
            .child(self.render_alloc_overlay(cx));

        self.last_render_alloc = self.record_sample("render");
        result.into_any_element()
    }
}

// ---------------------------------------------------------------------------
// Persistent child render entities
// ---------------------------------------------------------------------------

/// Persistent sidebar for the component lab.
///
/// Only re-renders when the selected story or live-reload status changes.
struct LabSidebar {
    selected_story_id: String,
    live_status: Option<SharedString>,
    registry_len: usize,
    parent: WeakEntity<ComponentLab>,
}

impl LabSidebar {
    fn new(parent: WeakEntity<ComponentLab>) -> Self {
        Self {
            selected_story_id: String::new(),
            live_status: None,
            registry_len: 0,
            parent,
        }
    }
}

impl Render for LabSidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self
            .parent
            .update(cx, |parent, cx| parent.render_sidebar(cx))
        {
            Ok(element) => element,
            Err(_) => div().into_any_element(),
        }
    }
}

/// Persistent toolbar for the component lab.
///
/// Only re-renders when the active story, viewport, theme or matrix mode
/// changes.
struct LabToolbar {
    story_id: String,
    viewport_id: String,
    theme_id: String,
    matrix_mode: bool,
    parent: WeakEntity<ComponentLab>,
}

impl LabToolbar {
    fn new(parent: WeakEntity<ComponentLab>) -> Self {
        Self {
            story_id: String::new(),
            viewport_id: String::new(),
            theme_id: String::new(),
            matrix_mode: false,
            parent,
        }
    }
}

impl Render for LabToolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self
            .parent
            .update(cx, |parent, cx| parent.render_toolbar(cx))
        {
            Ok(element) => element,
            Err(_) => div().into_any_element(),
        }
    }
}

/// Persistent controls panel for the component lab.
///
/// Only re-renders when the active story, layout constraints or save status
/// changes.
struct LabControlsPanel {
    story_id: String,
    story: ComponentStory,
    layout_constraints: PreviewLayoutConstraints,
    save_status: Option<SharedString>,
    parent: WeakEntity<ComponentLab>,
}

impl LabControlsPanel {
    fn new(parent: WeakEntity<ComponentLab>) -> Self {
        Self {
            story_id: String::new(),
            story: ComponentStory::new("", "", "", ""),
            layout_constraints: PreviewLayoutConstraints::default(),
            save_status: None,
            parent,
        }
    }
}

impl Render for LabControlsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self
            .parent
            .update(cx, |parent, cx| parent.render_controls_panel(cx))
        {
            Ok(element) => element,
            Err(_) => div().into_any_element(),
        }
    }
}

/// Persistent preview area for the component lab.
///
/// Only re-renders when the active story, viewport, theme, motion, layout
/// constraints or matrix mode changes.
struct LabPreviewArea {
    story_id: String,
    story: ComponentStory,
    viewport: ViewportPreset,
    theme: ThemePreset,
    motion: MotionPreset,
    layout_constraints: PreviewLayoutConstraints,
    matrix_mode: bool,
    cached_matrix: ResponsivePreviewMatrix,
    parent: WeakEntity<ComponentLab>,
}

impl LabPreviewArea {
    fn new(parent: WeakEntity<ComponentLab>) -> Self {
        Self {
            story_id: String::new(),
            story: ComponentStory::new("", "", "", ""),
            viewport: ViewportPreset::new("", "", 0.0, 0.0),
            theme: ThemePreset::new("", "", "", false),
            motion: MotionPreset::new("", "", false),
            layout_constraints: PreviewLayoutConstraints::default(),
            matrix_mode: false,
            cached_matrix: ResponsivePreviewMatrix { cells: Vec::new() },
            parent,
        }
    }
}

impl Render for LabPreviewArea {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self
            .parent
            .update(cx, |parent, cx| parent.render_preview_area(cx))
        {
            Ok(element) => element,
            Err(_) => div().into_any_element(),
        }
    }
}
