use super::host_state::PresentationStore;
use super::misc::apply_size;
use super::misc::badge_colors;
use super::misc::color_scale;
use super::misc::hex_color;
use super::misc::scale_type;
use super::misc::tone_color;
use super::types::StackDirection;
use d3rs::gpu3d::{Lines3DElement, Lines3DState, Surface3DElement, Surface3DState};
use d3rs::mesh::{
    ContourLevels, CoordinateAxis, MissingValuePolicy, ScalarAssociation, ScalarField,
    TriangleMesh, project_2d,
};
use gpui::prelude::*;
use gpui::*;
use gpui_audio_kit::{
    LevelMeterElement, Potentiometer, SpectrumElement, VerticalSlider, VolumeKnob,
};
use gpui_design::{DesignExt, DesignSystem};
use gpui_px::interaction::{InteractiveChartState, interactive};
use gpui_px::{
    AutoOrFixed, Axes2d, ColorRange, ColorScale, Colorbar, FieldInterpolation, MeshPlotPick,
    MeshPlotState, MeshPlotView, MeshRenderMode, PlotInteractions, Wireframe, area, bar, boxplot,
    contour, donut, heatmap, isoline, line, mesh_plot, pie, scatter, treemap,
};
use gpui_python_runtime::audio_stream::{AudioFrameKind, AudioFrameStore};
use gpui_python_runtime::gpui_adapter::{Gpui3DCache, GpuiMeshPlotCache};
use gpui_python_runtime::mesh_frames::{
    MeshFrame, MeshFrameKind, MeshFrameOutcome, MeshFrameStore,
};
use gpui_python_runtime::meshplot::{MeshPlotResourceError, MeshPlotSpec};
use gpui_python_runtime::native_mesh_plot::{
    decode_field as decode_mesh_field, decode_geometry as decode_mesh_geometry,
    decode_ids as decode_inline_ids,
};
use gpui_python_runtime::session::{
    HostMessage, JobLogLine, JobRegistry, JobState, JobUpdate, LogSeverity, Patch, PatchOp,
    PythonMessage, SessionState,
};
use gpui_python_runtime::spec_cache::TypedSpecCache;
use gpui_python_runtime::ui_ir::{
    AccordionNode, AlertNode, AudioControlNode, AudioMeterNode, AudioSpectrumNode, BadgeNode,
    BooleanInputNode, BreadcrumbsNode, ButtonNode, CardNode, ChartKind, ChartNode,
    ChartTreemapNode, ColorPickerNode, ConfirmDialogNode, ContextMenuNode, DialogNode,
    EmptyStateNode, FormNode, ListEditorNode, MenuBarNode, MenuItemNode, MenuNode, MeshPlotNode,
    MiniAppShellConfig, NumberInputNode, PathInputNode, PopoverNode, ProgressNode, PythonAppIr,
    Scene3dNode, SectionHeaderNode, SelectNode, SimpleNode, SliderNode, SpinnerNode, StackNode,
    StepperNode, TableNode, TabsNode, TextInputNode, TextNode, ThinkingOrbNode, ToastNode,
    TooltipNode, UiNode,
};
use gpui_ui_kit::color::Color;
use gpui_ui_kit::data_navigation::{DataNavigationAction, DataNavigationState};
use gpui_ui_kit::theme::{Theme, ThemeExt, ThemeState, ThemeVariant};
use gpui_ui_kit::thinking_orb::{engine as thinking_orb_engine, presets as thinking_orb_presets};
use gpui_ui_kit::{
    Alert, AlertVariant, BreadcrumbItem, BreadcrumbSeparator, Breadcrumbs, ColorPickerView,
    ConfirmDialog, ConfirmDialogVariant, ContextMenu, Dialog, DialogSize, DragItem, DragList,
    EmptyState, I18nState, Language, Menu, MenuBar, MenuBarItem, MenuItem, Popover,
    PopoverPlacement, Toast, ToastVariant, TooltipPlacement, WithTooltip,
    accordion::{Accordion, AccordionItem, AccordionMode},
    checkbox::Checkbox,
    input::Input,
    number_input::NumberInput,
    select::Select,
    slider::Slider,
    toggle::Toggle,
};
use gpui_ui_kit::{AriaProps, AriaRole, AriaState, apply_native_accessibility};
use gpui_ui_kit::{OrbSize, OrbState, ThinkingOrb};
use serde::Deserialize;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

struct ElementIdHasher(std::collections::hash_map::DefaultHasher);

impl fmt::Write for ElementIdHasher {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        value.hash(&mut self.0);
        Ok(())
    }
}

/// Build stable showcase IDs without allocating formatted names every render.
fn stable_element_id(arguments: fmt::Arguments<'_>) -> ElementId {
    let mut hasher = ElementIdHasher(Default::default());
    fmt::write(&mut hasher, arguments).expect("hash writer is infallible");
    ElementId::named_usize("python", hasher.0.finish() as usize)
}

fn select_wire_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

fn table_cell_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        value => value.to_string(),
    }
}

fn write_qa_json_artifact(variable: &str, value: &Value) {
    let Some(destination) = env::var_os(variable).map(PathBuf::from) else {
        return;
    };
    if let Some(parent) = destination.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        destination,
        serde_json::to_vec_pretty(value).unwrap_or_else(|_| b"{}\n".to_vec()),
    );
}

fn mesh_selection_payload(pick: &MeshPlotPick) -> Value {
    serde_json::json!({
        "plot_id": pick.plot_id,
        "mesh_id": pick.mesh_id,
        "cell_index": pick.cell_index,
        "cell_id": pick.cell_id,
        "nearest_vertex_index": pick.nearest_vertex_index,
        "vertex_id": pick.vertex_id,
        "world_position": pick.world_position,
        "displayed_value": pick.displayed_value,
        "field_id": pick.field_id,
    })
}

fn mesh_selection_event_payload(selection: Option<&MeshPlotPick>) -> Value {
    selection.map(mesh_selection_payload).unwrap_or(Value::Null)
}

fn cached_meshplot_fallback(
    cache: &GpuiMeshPlotCache,
    requested: &MeshPlotSpec,
) -> Option<MeshPlotSpec> {
    cache
        .get(&requested.id)
        .filter(|previous| *previous != requested)
        .cloned()
}

type NativeMeshPlotOptions = gpui_python_runtime::native_mesh_plot::NativeMeshPlotOptions;

fn finite_json_pair(value: &Value, name: &str) -> Result<[f64; 2], String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("mesh_plot {name} must be an array of two numbers"))?;
    if values.len() != 2 {
        return Err(format!("mesh_plot {name} must contain exactly two values"));
    }
    let min = values[0]
        .as_f64()
        .ok_or_else(|| format!("mesh_plot {name} values must be numbers"))?;
    let max = values[1]
        .as_f64()
        .ok_or_else(|| format!("mesh_plot {name} values must be numbers"))?;
    if !min.is_finite() || !max.is_finite() || min >= max {
        return Err(format!(
            "mesh_plot {name} values must be finite and increasing"
        ));
    }
    Ok([min, max])
}

fn native_mesh_plot_color_range(value: &Value) -> Result<ColorRange, String> {
    match value {
        Value::String(value) if value == "auto" => Ok(ColorRange::Auto),
        Value::Array(_) => {
            let [min, max] = finite_json_pair(value, "color_range")?;
            Ok(ColorRange::Fixed { min, max })
        }
        Value::Object(value) => {
            let symmetric = value
                .get("symmetric")
                .and_then(Value::as_object)
                .filter(|_| value.len() == 1)
                .ok_or("mesh_plot symmetric color_range must contain a symmetric object")?;
            let center = symmetric
                .get("center")
                .and_then(Value::as_f64)
                .filter(|center| center.is_finite())
                .ok_or("mesh_plot symmetric color_range center must be finite")?;
            let extent = match symmetric.get("extent") {
                Some(Value::String(value)) if value == "auto" => AutoOrFixed::Auto,
                Some(value) => AutoOrFixed::Fixed(
                    value
                        .as_f64()
                        .filter(|extent| extent.is_finite() && *extent > 0.0)
                        .ok_or("mesh_plot symmetric color_range extent must be 'auto' or positive finite")?,
                ),
                None => return Err("mesh_plot symmetric color_range requires extent".into()),
            };
            Ok(ColorRange::Symmetric { center, extent })
        }
        _ => Err("mesh_plot color_range must be 'auto', [min, max], or a symmetric range".into()),
    }
}

#[allow(dead_code)] // Retained temporarily while the resource-builder extraction is completed.
fn native_mesh_plot_contour_levels(value: Option<&Value>) -> Result<ContourLevels, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(ContourLevels::Count(8));
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot contour_levels must be an object")?;
    if let Some(count) = object.get("count") {
        let count = count
            .as_u64()
            .and_then(|count| u32::try_from(count).ok())
            .filter(|count| *count > 0)
            .ok_or("mesh_plot contour_levels.count must be a positive integer")?;
        return Ok(ContourLevels::Count(count));
    }
    let values = object
        .get("values")
        .and_then(Value::as_array)
        .ok_or("mesh_plot contour_levels requires count or values")?;
    let values = values
        .iter()
        .map(|value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or("mesh_plot contour level must be finite")
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() < 2 || values.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err("mesh_plot contour_levels.values must contain increasing finite values".into());
    }
    Ok(ContourLevels::Explicit(Arc::from(values)))
}

#[allow(dead_code)] // Retained temporarily while the resource-builder extraction is completed.
fn native_mesh_plot_missing_value_policy(value: &str) -> Result<MissingValuePolicy, String> {
    match value {
        "reject" => Ok(MissingValuePolicy::Reject),
        "mask_nan" => Ok(MissingValuePolicy::MaskNaN),
        value => Err(format!(
            "unsupported mesh_plot missing_value_policy {value:?}"
        )),
    }
}

#[allow(dead_code)] // Retained temporarily while the resource-builder extraction is completed.
fn native_mesh_plot_viewport(value: Option<&Value>) -> Result<Option<[f64; 4]>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot viewport must be an object")?;
    let [x_min, x_max] = finite_json_pair(
        object
            .get("x")
            .ok_or("mesh_plot viewport requires an x range")?,
        "viewport.x",
    )?;
    let [y_min, y_max] = finite_json_pair(
        object
            .get("y")
            .ok_or("mesh_plot viewport requires a y range")?,
        "viewport.y",
    )?;
    Ok(Some([x_min, x_max, y_min, y_max]))
}

#[allow(dead_code)] // Retained temporarily while the resource-builder extraction is completed.
fn native_mesh_plot_selection(
    value: Option<&Value>,
    plot_id: &str,
    mesh_id: &str,
) -> Result<Option<MeshPlotPick>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot selection must be an object or null")?;
    let cell_index = object
        .get("cell_index")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or("mesh_plot selection requires a u32 cell_index")?;
    let world_position = match object.get("world_position") {
        None => [0.0; 3],
        Some(value) => {
            let values = value
                .as_array()
                .ok_or("mesh_plot selection world_position must be an array")?;
            if values.len() != 3 {
                return Err("mesh_plot selection world_position must contain three values".into());
            }
            let values = values
                .iter()
                .map(|value| {
                    value
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .ok_or("mesh_plot selection world_position must be finite")
                })
                .collect::<Result<Vec<_>, _>>()?;
            [values[0], values[1], values[2]]
        }
    };
    let optional_u32 = |name: &str| -> Result<Option<u32>, String> {
        object
            .get(name)
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or_else(|| format!("mesh_plot selection {name} must be a u32"))
            })
            .transpose()
    };
    let optional_u64 = |name: &str| -> Result<Option<u64>, String> {
        object
            .get(name)
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_u64()
                    .ok_or_else(|| format!("mesh_plot selection {name} must be a u64"))
            })
            .transpose()
    };
    let displayed_value = object
        .get("displayed_value")
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or("mesh_plot selection displayed_value must be finite")
        })
        .transpose()?;
    let field_id = object
        .get("field_id")
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(Arc::from)
                .ok_or("mesh_plot selection field_id must be a non-empty string")
        })
        .transpose()?;
    Ok(Some(MeshPlotPick {
        plot_id: Arc::from(plot_id),
        mesh_id: Arc::from(mesh_id),
        cell_index,
        cell_id: optional_u64("cell_id")?,
        nearest_vertex_index: optional_u32("nearest_vertex_index")?,
        vertex_id: optional_u64("vertex_id")?,
        world_position,
        displayed_value,
        field_id,
    }))
}

fn native_mesh_plot_options(
    spec: &MeshPlotSpec,
    mesh_id: &str,
) -> Result<NativeMeshPlotOptions, String> {
    gpui_python_runtime::native_mesh_plot::options(spec, mesh_id)
}

#[cfg(all(test, target_os = "macos", feature = "native-qa"))]
mod native_mesh_plot_tests {
    use super::{
        Patch, PatchOp, PresentationStore, PythonIrShowcase, mesh_plot_resource_handles,
        mesh_selection_event_payload,
    };
    use gpui::{
        AnyWindowHandle, AppContext, Context, HeadlessAppContext, InputEvent, InteractiveElement,
        Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement, Platform, Render,
        Styled, Window, div, point, px, size,
    };
    use gpui_macos::metal_renderer::MetalHeadlessRenderer;
    use gpui_px::{MeshPlotPick, MeshPlotState};
    use gpui_python_runtime::mesh_frames::{
        MeshDtype, MeshFrame, MeshFrameKind, MeshFrameOutcome, MeshFrameStore,
    };
    use gpui_python_runtime::meshplot::MeshPlotSpec;
    use serde_json::Value;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, MutexGuard, atomic::Ordering};

    // TIS/TSM keyboard-layout initialization can abort macOS test processes
    // when multiple native GPUI platforms are created concurrently. Use the
    // tests only need Metal rendering, so use the deterministic no-op text
    // system and serialize the complete platform/window lifetime.
    static NATIVE_PLATFORM_LOCK: Mutex<()> = Mutex::new(());

    fn native_platform_lock() -> MutexGuard<'static, ()> {
        NATIVE_PLATFORM_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn native_text_system() -> Arc<dyn gpui::PlatformTextSystem> {
        Arc::new(gpui::NoopTextSystem::new())
    }

    fn native_metal_required() -> bool {
        matches!(std::env::var("QA_NATIVE_REQUIRED").as_deref(), Ok("1"))
            || matches!(std::env::var("QA_METAL_REQUIRED").as_deref(), Ok("1"))
    }

    fn native_metal_available() -> bool {
        if MetalHeadlessRenderer::try_new().is_some() {
            true
        } else if native_metal_required() {
            panic!("native Metal QA requires a compatible Metal device")
        } else {
            eprintln!("native Metal QA skipped: no compatible Metal device");
            false
        }
    }

    struct NativeResourceMeshPlotView {
        spec: MeshPlotSpec,
        frames: Rc<RefCell<MeshFrameStore>>,
        state: Rc<RefCell<MeshPlotState>>,
        last_valid_spec: Option<MeshPlotSpec>,
        selection: Rc<RefCell<Option<MeshPlotPick>>>,
        payload: Rc<RefCell<Value>>,
        nested: bool,
    }

    impl Render for NativeResourceMeshPlotView {
        fn render(
            &mut self,
            _window: &mut Window,
            _cx: &mut Context<Self>,
        ) -> impl gpui::IntoElement {
            let selection = self.selection.clone();
            let payload = self.payload.clone();
            let callback: Rc<dyn Fn(Option<MeshPlotPick>)> = Rc::new(move |pick| {
                *payload.borrow_mut() = mesh_selection_event_payload(pick.as_ref());
                *selection.borrow_mut() = pick;
            });
            let (plot, state) = PythonIrShowcase::build_native_mesh_plot(
                &self.spec,
                &self.frames.borrow(),
                Some(self.state.clone()),
                Some(callback.clone()),
            )
            .map(|result| {
                self.last_valid_spec = Some(self.spec.clone());
                result
            })
            .unwrap_or_else(|error| {
                // Match the showcase host's last-valid-frame policy: a
                // resource decode/build failure must keep the prior native
                // plot rather than replacing it with an error card.
                let previous = self
                    .last_valid_spec
                    .as_ref()
                    .expect("invalid patch requires an earlier valid native MeshPlot");
                PythonIrShowcase::build_native_mesh_plot(
                    previous,
                    &self.frames.borrow(),
                    Some(self.state.clone()),
                    Some(callback),
                )
                .unwrap_or_else(|fallback_error| {
                    panic!(
                        "invalid resource patch ({error}) must retain a buildable last-valid MeshPlot: {fallback_error}"
                    )
                })
            });
            self.state = state;
            let mut view = div()
                .id("python-resource-mesh-plot")
                .w(px(600.0))
                .h(px(400.0));
            if self.nested {
                view = view
                    .flex()
                    .flex_col()
                    .child(div().h(px(40.0)).child("Mesh plot"))
                    .child(div().h(px(40.0)).child("4 vertices · 2 triangles"))
                    .child(div().flex_1().size_full().child(plot));
            } else {
                view = view.child(plot);
            }
            view
        }
    }

    fn frame(
        resource_id: &str,
        kind: MeshFrameKind,
        dtype: MeshDtype,
        shape: &[u32],
        payload: Vec<u8>,
    ) -> MeshFrame {
        MeshFrame {
            resource_id: resource_id.into(),
            generation: 1,
            sequence: 0,
            chunk_count: 1,
            kind,
            dtype,
            shape: shape.to_vec(),
            payload,
        }
    }

    fn bytes<T: Copy, B: IntoIterator<Item = u8>>(
        values: &[T],
        encode: impl Fn(T) -> B,
    ) -> Vec<u8> {
        values.iter().copied().flat_map(encode).collect()
    }

    fn fixture() -> (MeshPlotSpec, Rc<RefCell<MeshFrameStore>>) {
        fixture_with_store(MeshFrameStore::new())
    }

    fn fixture_with_store(store: MeshFrameStore) -> (MeshPlotSpec, Rc<RefCell<MeshFrameStore>>) {
        let mut store = store;
        store
            .ingest(frame(
                "mesh-positions",
                MeshFrameKind::Geometry,
                MeshDtype::F64LE,
                &[4, 3],
                bytes(
                    &[
                        0.0_f64, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
                    ],
                    f64::to_le_bytes,
                ),
            ))
            .expect("position frame");
        store
            .ingest(frame(
                "mesh-triangles",
                MeshFrameKind::Geometry,
                MeshDtype::U32LE,
                &[2, 3],
                bytes(&[0_u32, 1, 2, 0, 2, 3], u32::to_le_bytes),
            ))
            .expect("triangle frame");
        store
            .ingest(frame(
                "mesh-vertex-ids",
                MeshFrameKind::Ids,
                MeshDtype::U64LE,
                &[4],
                bytes(&[101_u64, 102, 103, 104], u64::to_le_bytes),
            ))
            .expect("vertex id frame");
        store
            .ingest(frame(
                "mesh-cell-ids",
                MeshFrameKind::Ids,
                MeshDtype::U64LE,
                &[2],
                bytes(&[201_u64, 202], u64::to_le_bytes),
            ))
            .expect("cell id frame");
        store
            .ingest(frame(
                "mesh-pressure",
                MeshFrameKind::Field,
                MeshDtype::F64LE,
                &[4],
                bytes(&[0.0_f64, 0.5, 1.0, 0.25], f64::to_le_bytes),
            ))
            .expect("field frame");
        store
            .ingest(frame(
                "mesh-pressure-valid",
                MeshFrameKind::Mask,
                MeshDtype::BoolBytes,
                &[4],
                vec![1, 1, 1, 1],
            ))
            .expect("mask frame");
        let spec = MeshPlotSpec::from_value(serde_json::json!({
            "schema_version": 1,
            "id": "resource-pressure-plot",
            "geometry": {
                "id": "resource-baffle",
                "positions": {"resource_id": "mesh-positions", "generation": 1, "dtype": "f64le"},
                "triangles": {"resource_id": "mesh-triangles", "generation": 1, "dtype": "u32le"},
                "vertex_ids": {"resource_id": "mesh-vertex-ids", "generation": 1, "dtype": "u64le"},
                "cell_ids": {"resource_id": "mesh-cell-ids", "generation": 1, "dtype": "u64le"}
            },
            "field": {
                "id": "resource-pressure",
                "label": "Sound pressure level",
                "unit": "dB SPL",
                "resource_id": "mesh-pressure",
                "generation": 1,
                "association": "vertex",
                "valid": {"resource_id": "mesh-pressure-valid", "generation": 1, "dtype": "bool_bytes"}
            },
            "view": "planar",
            "mode": "scalar_fill",
            "color_scale": "viridis",
            "wireframe": true,
            "equal_aspect": true,
            "interactions": ["pan", "zoom", "inspect", "select", "reset", "fit"]
        }))
        .expect("resource-backed MeshPlot spec");
        (spec, Rc::new(RefCell::new(store)))
    }

    #[::core::prelude::v1::test]
    fn native_metal_python_resource_plot_renders_and_emits_typed_selection() {
        let _platform_guard = native_platform_lock();
        if !native_metal_available() {
            return;
        }
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let (spec, frames) = fixture();
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let payload = Rc::new(RefCell::new(Value::Null));
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                let payload = payload.clone();
                move |_window, app| {
                    app.new(|_cx| NativeResourceMeshPlotView {
                        spec,
                        frames,
                        state,
                        last_valid_spec: None,
                        selection,
                        payload,
                        nested: false,
                    })
                }
            })
            .expect("open native Python MeshPlot window");
        let any_window: AnyWindowHandle = window.into();
        cx.update_window(any_window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw resource-backed Python MeshPlot");

        let position = point(px(300.0), px(200.0));
        cx.update_window(any_window, |_, window, app| {
            window.dispatch_event(
                MouseDownEvent {
                    position,
                    modifiers: Modifiers::default(),
                    button: MouseButton::Left,
                    click_count: 1,
                    first_mouse: false,
                }
                .to_platform_input(),
                app,
            );
            window.dispatch_event(
                MouseUpEvent {
                    position,
                    modifiers: Modifiers::default(),
                    button: MouseButton::Left,
                    click_count: 1,
                }
                .to_platform_input(),
                app,
            );
        })
        .expect("dispatch Python MeshPlot selection click");
        cx.run_until_parked();

        let picked = selection.borrow().clone().expect("selection callback");
        assert!(matches!(picked.cell_id, Some(201 | 202)));
        assert!(matches!(picked.vertex_id, Some(101..=104)));
        assert_eq!(payload.borrow()["plot_id"], "resource-pressure-plot");
        assert_eq!(payload.borrow()["mesh_id"], "resource-baffle");
        assert!(matches!(
            payload.borrow()["cell_id"].as_u64(),
            Some(201 | 202)
        ));
        assert!(matches!(
            payload.borrow()["vertex_id"].as_u64(),
            Some(101..=104)
        ));

        let screenshot = cx
            .capture_screenshot(any_window)
            .expect("capture Python MeshPlot framebuffer");
        assert_eq!(screenshot.width(), 1200);
        assert_eq!(screenshot.height(), 800);
        let (min_luma, max_luma) = screenshot
            .pixels()
            .map(|pixel| u16::from(pixel.0[0]) + u16::from(pixel.0[1]) + u16::from(pixel.0[2]))
            .fold((u16::MAX, 0), |(min_luma, max_luma), luma| {
                (min_luma.min(luma), max_luma.max(luma))
            });
        assert!(
            max_luma > min_luma,
            "resource-backed MeshPlot framebuffer is blank"
        );
        cx.update_window(any_window, |_, window, _| window.remove_window())
            .expect("close native Python MeshPlot window");
        cx.run_until_parked();
    }

    #[::core::prelude::v1::test]
    fn nested_python_mesh_plot_layout_preserves_pointer_selection() {
        let _platform_guard = native_platform_lock();
        if !native_metal_available() {
            return;
        }
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let (spec, frames) = fixture();
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let payload = Rc::new(RefCell::new(Value::Null));
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                let payload = payload.clone();
                move |_window, app| {
                    app.new(|_cx| NativeResourceMeshPlotView {
                        spec,
                        frames,
                        state,
                        last_valid_spec: None,
                        selection,
                        payload,
                        nested: true,
                    })
                }
            })
            .expect("open nested native Python MeshPlot window");
        let any_window: AnyWindowHandle = window.into();
        cx.update_window(any_window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw nested resource-backed Python MeshPlot");

        let position = point(px(300.0), px(200.0));
        cx.update_window(any_window, |_, window, app| {
            window.dispatch_event(
                MouseDownEvent {
                    position,
                    modifiers: Modifiers::default(),
                    button: MouseButton::Left,
                    click_count: 1,
                    first_mouse: false,
                }
                .to_platform_input(),
                app,
            );
            window.dispatch_event(
                MouseUpEvent {
                    position,
                    modifiers: Modifiers::default(),
                    button: MouseButton::Left,
                    click_count: 1,
                }
                .to_platform_input(),
                app,
            );
        })
        .expect("dispatch nested Python MeshPlot selection click");
        cx.run_until_parked();

        let picked = selection
            .borrow()
            .clone()
            .expect("nested selection callback");
        assert!(matches!(picked.cell_id, Some(201 | 202)));
        assert!(matches!(picked.vertex_id, Some(101..=104)));
        assert_eq!(payload.borrow()["plot_id"], "resource-pressure-plot");
        cx.update_window(any_window, |_, window, _| window.remove_window())
            .expect("close nested native Python MeshPlot window");
        cx.run_until_parked();
    }

    #[test]
    fn native_metal_invalid_resource_patch_keeps_the_last_valid_meshplot_frame() {
        let _platform_guard = native_platform_lock();
        if !native_metal_available() {
            return;
        }
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let (spec, frames) = fixture();
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let payload = Rc::new(RefCell::new(Value::Null));
        let initial_spec = spec.clone();
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                let payload = payload.clone();
                move |_window, app| {
                    app.new(|_cx| NativeResourceMeshPlotView {
                        spec: initial_spec,
                        frames,
                        state,
                        last_valid_spec: None,
                        selection,
                        payload,
                        nested: false,
                    })
                }
            })
            .expect("open native resource-backed MeshPlot window");
        let view = cx
            .read_window(&window, |view, _| view)
            .expect("read native resource-backed MeshPlot view");
        let any_window: AnyWindowHandle = window.into();
        cx.update_window(any_window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw valid resource-backed MeshPlot");
        let before = cx
            .capture_screenshot(any_window)
            .expect("capture valid resource-backed MeshPlot frame");

        let mut invalid_field = spec.clone();
        invalid_field.revision = 2;
        invalid_field.field = Some(serde_json::json!({
            "id": "resource-pressure",
            "label": "Sound pressure level",
            "unit": "dB SPL",
            "resource_id": "missing-pressure",
            "generation": 2,
            "association": "vertex",
            "valid": {
                "resource_id": "mesh-pressure-valid",
                "generation": 1,
                "dtype": "bool_bytes"
            }
        }));
        cx.update_entity(&view, |view, cx| {
            view.spec = invalid_field;
            cx.notify();
        });
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("draw invalid field resource-backed patch fallback");
        let invalid_field_frame = cx
            .capture_screenshot(any_window)
            .expect("capture invalid field fallback frame");
        assert_eq!(
            invalid_field_frame.as_raw(),
            before.as_raw(),
            "an invalid field resource patch must preserve the last-valid native frame"
        );

        let mut invalid_mask = spec.clone();
        invalid_mask.revision = 3;
        invalid_mask
            .field
            .as_mut()
            .expect("fixture includes a field")["valid"] = serde_json::json!({
            "resource_id": "missing-mask",
            "generation": 2,
            "dtype": "bool_bytes"
        });
        cx.update_entity(&view, |view, cx| {
            view.spec = invalid_mask;
            cx.notify();
        });
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("draw invalid mask resource-backed patch fallback");
        let invalid_mask_frame = cx
            .capture_screenshot(any_window)
            .expect("capture invalid mask fallback frame");
        assert_eq!(
            invalid_mask_frame.as_raw(),
            before.as_raw(),
            "an invalid mask resource patch must preserve the last-valid native frame"
        );

        let mut invalid = spec.clone();
        invalid.revision = 4;
        invalid.geometry = serde_json::json!({
            "id": "resource-baffle",
            "positions": {"resource_id": "missing-positions", "generation": 2, "dtype": "f64le"},
            "triangles": {"resource_id": "missing-triangles", "generation": 2, "dtype": "u32le"}
        });
        cx.update_entity(&view, |view, cx| {
            view.spec = invalid;
            cx.notify();
        });
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("draw invalid resource-backed patch fallback");
        let after = cx
            .capture_screenshot(any_window)
            .expect("capture fallback resource-backed MeshPlot frame");
        assert_eq!(
            after.as_raw(),
            before.as_raw(),
            "an invalid resource-backed patch must preserve the complete last-valid native frame"
        );
        assert!(
            cx.read_entity(&view, |view, _| Rc::ptr_eq(&view.state, &state)),
            "invalid resource patch must keep the retained MeshPlotState owner"
        );
        drop(view);
        cx.update_window(any_window, |_, window, _| window.remove_window())
            .expect("close native resource-backed MeshPlot window");
        cx.run_until_parked();
    }

    #[test]
    fn native_metal_resource_patch_stream_keeps_the_newest_valid_frame() {
        let _platform_guard = native_platform_lock();
        if !native_metal_available() {
            return;
        }
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let (spec, frames) = fixture();
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let payload = Rc::new(RefCell::new(Value::Null));
        let initial_spec = spec.clone();
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                let payload = payload.clone();
                move |_window, app| {
                    app.new(|_cx| NativeResourceMeshPlotView {
                        spec: initial_spec,
                        frames,
                        state,
                        last_valid_spec: None,
                        selection,
                        payload,
                        nested: false,
                    })
                }
            })
            .expect("open native resource patch-stream MeshPlot window");
        let view = cx
            .read_window(&window, |view, _| view)
            .expect("read native resource patch-stream MeshPlot view");
        let any_window: AnyWindowHandle = window.into();
        cx.update_window(any_window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw initial resource-backed MeshPlot");
        let field_revision_before = state.borrow().field_revision;

        let mut replacement_field = frame(
            "mesh-pressure",
            MeshFrameKind::Field,
            MeshDtype::F64LE,
            &[4],
            bytes(&[1.0_f64, 0.0, 1.0, 0.0], f64::to_le_bytes),
        );
        replacement_field.generation = 2;
        cx.update_entity(&view, |view, _cx| {
            view.frames
                .borrow_mut()
                .ingest(replacement_field)
                .expect("ingest newer field resource generation");
            let mut updated = spec.clone();
            updated.revision = 2;
            updated.field.as_mut().expect("fixture includes a field")["generation"] =
                Value::from(2);
            view.spec = updated;
        });
        cx.update_entity(&view, |_view, cx| cx.notify());
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("draw valid resource-backed field patch");
        let updated = cx
            .capture_screenshot(any_window)
            .expect("capture valid resource-backed field patch");
        assert!(
            cx.read_entity(&view, |view, _| Rc::ptr_eq(&view.state, &state)),
            "valid field patch must preserve the retained MeshPlotState owner"
        );
        assert_eq!(
            state.borrow().field_revision,
            field_revision_before + 1,
            "a declarative resource-field replacement must advance only the retained field domain"
        );
        assert_eq!(
            cx.read_entity(&view, |view, _| {
                view.last_valid_spec
                    .as_ref()
                    .and_then(|spec| spec.field.as_ref())
                    .and_then(|field| field.get("generation"))
                    .and_then(Value::as_u64)
            }),
            Some(2),
            "the valid resource patch must become the retained last-valid specification"
        );

        let mut invalid = spec.clone();
        invalid.revision = 3;
        invalid.geometry = serde_json::json!({
            "id": "resource-baffle",
            "positions": {"resource_id": "missing-positions", "generation": 3, "dtype": "f64le"},
            "triangles": {"resource_id": "missing-triangles", "generation": 3, "dtype": "u32le"}
        });
        cx.update_entity(&view, |view, cx| {
            view.spec = invalid;
            cx.notify();
        });
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("draw invalid resource-backed patch after valid update");
        let fallback = cx
            .capture_screenshot(any_window)
            .expect("capture newest-valid resource-backed fallback frame");
        assert_eq!(
            fallback.as_raw(),
            updated.as_raw(),
            "an invalid patch must preserve the newest complete valid native frame"
        );
        assert!(
            cx.read_entity(&view, |view, _| Rc::ptr_eq(&view.state, &state)),
            "invalid patch must preserve the retained MeshPlotState owner"
        );
        drop(view);
        cx.update_window(any_window, |_, window, _| window.remove_window())
            .expect("close native resource patch-stream MeshPlot window");
        cx.run_until_parked();
    }

    #[test]
    fn native_metal_malformed_mesh_frame_recovers_after_corrected_generation() {
        let _platform_guard = native_platform_lock();
        if !native_metal_available() {
            return;
        }
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let (spec, frames) = fixture();
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let payload = Rc::new(RefCell::new(Value::Null));
        let initial_spec = spec.clone();
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                let payload = payload.clone();
                move |_window, app| {
                    app.new(|_cx| NativeResourceMeshPlotView {
                        spec: initial_spec,
                        frames,
                        state,
                        last_valid_spec: None,
                        selection,
                        payload,
                        nested: false,
                    })
                }
            })
            .expect("open native malformed-frame recovery MeshPlot window");
        let view = cx
            .read_window(&window, |view, _| view)
            .expect("read native malformed-frame recovery MeshPlot view");
        let any_window: AnyWindowHandle = window.into();
        cx.update_window(any_window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw initial resource-backed MeshPlot");
        let initial_frame = cx
            .capture_screenshot(any_window)
            .expect("capture initial resource-backed MeshPlot frame");
        let initial_field_revision = state.borrow().field_revision;

        let mut malformed = frame(
            "mesh-pressure",
            MeshFrameKind::Field,
            MeshDtype::F64LE,
            &[4],
            bytes(&[1.0_f64, 0.0, 1.0, 0.0], f64::to_le_bytes),
        );
        malformed.generation = 2;
        malformed.payload.pop();
        cx.update_entity(&view, |view, _cx| {
            assert!(
                view.frames.borrow_mut().ingest(malformed).is_err(),
                "a malformed binary MeshFrame must be rejected before it reaches rendering"
            );
        });
        cx.update_entity(&view, |_view, cx| cx.notify());
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("redraw after malformed resource frame");
        let fallback = cx
            .capture_screenshot(any_window)
            .expect("capture last-valid frame after malformed resource frame");
        assert_eq!(
            fallback.as_raw(),
            initial_frame.as_raw(),
            "a malformed binary frame must preserve the rendered last-valid frame"
        );
        assert_eq!(state.borrow().field_revision, initial_field_revision);
        assert_eq!(
            cx.read_entity(&view, |view, _| {
                view.last_valid_spec
                    .as_ref()
                    .and_then(|spec| spec.field.as_ref())
                    .and_then(|field| field.get("generation"))
                    .and_then(Value::as_u64)
            }),
            Some(1),
            "a malformed frame must not become the retained last-valid generation"
        );

        let mut corrected = frame(
            "mesh-pressure",
            MeshFrameKind::Field,
            MeshDtype::F64LE,
            &[4],
            bytes(&[1.0_f64, 0.0, 1.0, 0.0], f64::to_le_bytes),
        );
        corrected.generation = 2;
        cx.update_entity(&view, |view, _cx| {
            view.frames
                .borrow_mut()
                .ingest(corrected)
                .expect("corrected MeshFrame generation");
            let mut updated = spec.clone();
            updated.revision = 2;
            updated.field.as_mut().expect("fixture includes a field")["generation"] =
                Value::from(2);
            view.spec = updated;
        });
        cx.update_entity(&view, |_view, cx| cx.notify());
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("draw corrected resource-backed MeshPlot frame");
        let recovered = cx
            .capture_screenshot(any_window)
            .expect("capture corrected resource-backed MeshPlot frame");
        assert_ne!(
            recovered.as_raw(),
            initial_frame.as_raw(),
            "a corrected generation must replace the last-valid rendered field"
        );
        assert_eq!(state.borrow().field_revision, initial_field_revision + 1);
        assert_eq!(
            cx.read_entity(&view, |view, _| {
                view.last_valid_spec
                    .as_ref()
                    .and_then(|spec| spec.field.as_ref())
                    .and_then(|field| field.get("generation"))
                    .and_then(Value::as_u64)
            }),
            Some(2),
            "the corrected frame must become the retained last-valid generation"
        );

        drop(view);
        cx.update_window(any_window, |_, window, _| window.remove_window())
            .expect("close native malformed-frame recovery MeshPlot window");
        cx.run_until_parked();
    }

    #[test]
    fn native_metal_surface_resource_geometry_patch_replaces_only_the_retained_geometry_scene() {
        let _platform_guard = native_platform_lock();
        if !native_metal_available() {
            return;
        }
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let (mut spec, frames) = fixture();
        spec.view = "surface3d".into();
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let payload = Rc::new(RefCell::new(Value::Null));
        let initial_spec = spec.clone();
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                let payload = payload.clone();
                move |_window, app| {
                    app.new(|_cx| NativeResourceMeshPlotView {
                        spec: initial_spec,
                        frames,
                        state,
                        last_valid_spec: None,
                        selection,
                        payload,
                        nested: false,
                    })
                }
            })
            .expect("open native Surface3d resource MeshPlot window");
        let view = cx
            .read_window(&window, |view, _| view)
            .expect("read native Surface3d resource MeshPlot view");
        let any_window: AnyWindowHandle = window.into();
        cx.update_window(any_window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw initial resource-backed Surface3d MeshPlot");
        let initial_frame = cx
            .capture_screenshot(any_window)
            .expect("capture initial resource-backed Surface3d frame");
        let initial_stats = state
            .borrow()
            .retained_3d_stats()
            .expect("initial Surface3d frame creates a retained scene");
        let initial_field_revision = state.borrow().field_revision;

        // Decoding the same resource-backed spec creates fresh immutable Arc
        // buffers. It must still retain the existing scene and GPU upload.
        cx.update_entity(&view, |_view, cx| cx.notify());
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("redraw unchanged resource-backed Surface3d MeshPlot");
        assert_eq!(
            state.borrow().retained_3d_stats(),
            Some(initial_stats),
            "an unchanged resource re-decode must keep the retained 3D scene and upload"
        );

        let mut replacement_positions = frame(
            "mesh-positions",
            MeshFrameKind::Geometry,
            MeshDtype::F64LE,
            &[4, 3],
            bytes(
                &[
                    0.0_f64, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.5, 0.0, 1.0, 0.0,
                ],
                f64::to_le_bytes,
            ),
        );
        replacement_positions.generation = 2;
        cx.update_entity(&view, |view, _cx| {
            view.frames
                .borrow_mut()
                .ingest(replacement_positions)
                .expect("ingest newer geometry resource generation");
            let mut updated = spec.clone();
            updated.revision = 2;
            updated.geometry["positions"]["generation"] = Value::from(2);
            view.spec = updated;
        });
        cx.update_entity(&view, |_view, cx| cx.notify());
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("draw resource-backed Surface3d geometry patch");
        let patched_frame = cx
            .capture_screenshot(any_window)
            .expect("capture resource-backed Surface3d geometry patch");
        let patched_stats = state
            .borrow()
            .retained_3d_stats()
            .expect("geometry patch recreates a retained Surface3d scene");
        assert_ne!(
            patched_stats.scene_identity, initial_stats.scene_identity,
            "a changed geometry resource must replace the retained 3D scene"
        );
        assert_eq!(
            patched_stats.geometry_revision,
            initial_stats.geometry_revision + 1,
            "a changed geometry resource must advance the geometry domain once"
        );
        assert_eq!(
            state.borrow().field_revision,
            initial_field_revision,
            "a geometry-only resource patch must not advance the field domain"
        );
        assert_ne!(
            patched_frame.as_raw(),
            initial_frame.as_raw(),
            "the changed Surface3d geometry must affect the rendered native frame"
        );
        assert_eq!(
            cx.read_entity(&view, |view, _| {
                view.last_valid_spec
                    .as_ref()
                    .and_then(|spec| spec.geometry["positions"]["generation"].as_u64())
            }),
            Some(2),
            "the geometry patch must become the retained last-valid specification"
        );
        drop(view);
        cx.update_window(any_window, |_, window, _| window.remove_window())
            .expect("close native Surface3d resource MeshPlot window");
        cx.run_until_parked();
    }

    #[test]
    fn native_host_resource_ownership_keeps_active_plot_resources_until_release() {
        let (spec, frames) = fixture_with_store(MeshFrameStore::with_budget(204));
        let handles = mesh_plot_resource_handles(&spec).expect("fixture resource handles");
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = Rc::try_unwrap(frames)
            .expect("fixture frame store has one owner")
            .into_inner();

        showcase
            .sync_mesh_plot_resource_refs_for_spec(&spec)
            .expect("retain all resources referenced by the active plot");
        let stats = showcase.mesh_frames.stats();
        assert_eq!(stats.referenced_resources, handles.len());
        assert_eq!(stats.references, handles.len());

        for (resource_id, generation) in &handles {
            assert!(
                !showcase.mesh_frames.release(resource_id, *generation),
                "an active native plot must prevent explicit resource drop"
            );
        }

        assert!(
            showcase
                .mesh_frames
                .ingest(frame(
                    "budget-pressure",
                    MeshFrameKind::Field,
                    MeshDtype::U64LE,
                    &[1],
                    vec![0; 8],
                ))
                .is_err(),
            "a full store must reject new resources when every active plot resource is retained"
        );
        assert_eq!(
            showcase.mesh_frames.stats().referenced_resources,
            handles.len(),
            "budget pressure must not evict an active plot resource"
        );

        showcase
            .sync_mesh_plot_resource_refs(std::collections::HashMap::new())
            .expect("release resources when the plot is removed");
        let stats = showcase.mesh_frames.stats();
        assert_eq!(stats.referenced_resources, 0);
        assert_eq!(stats.references, 0);
        for (resource_id, generation) in handles {
            assert!(
                showcase.mesh_frames.release(&resource_id, generation),
                "released plot ownership must allow resource cleanup"
            );
        }
    }

    #[test]
    fn native_host_session_reset_releases_plot_state_and_allows_new_generations() {
        let (spec, frames) = fixture();
        let handles = mesh_plot_resource_handles(&spec).expect("fixture resource handles");
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = Rc::try_unwrap(frames)
            .expect("fixture frame store has one owner")
            .into_inner();
        showcase
            .sync_mesh_plot_resource_refs_for_spec(&spec)
            .expect("retain resources for the active plot");
        showcase
            .mesh_plots
            .upsert(spec)
            .expect("cache the active plot specification");
        showcase
            .session_state
            .apply_patch_revision(&Patch {
                revision: 4,
                request_id: Some("old-session".into()),
                ops: vec![PatchOp::ReplaceMeshField {
                    plot_id: "resource-pressure-plot".into(),
                    generation: 8,
                    field: serde_json::json!({"values": [1.0]}),
                }],
            })
            .expect("seed the old session revision history");
        showcase.mesh_plot_states.insert(
            "resource-pressure-plot".into(),
            Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0))),
        );
        showcase
            .mesh_plot_errors
            .insert("resource-pressure-plot".into(), "stale frame".into());

        showcase.reset_mesh_plot_runtime_state();

        assert!(showcase.mesh_plot_resource_refs.is_empty());
        assert!(showcase.mesh_plot_states.is_empty());
        assert!(showcase.mesh_plot_errors.is_empty());
        assert_eq!(showcase.mesh_plots.len(), 0);
        assert_eq!(showcase.mesh_frames.stats().resources, 0);
        assert_eq!(showcase.mesh_frames.stats().references, 0);
        assert_eq!(showcase.session_state.revision(), 0);
        assert_eq!(
            showcase
                .session_state
                .mesh_generation("resource-pressure-plot"),
            None
        );
        for (resource_id, generation) in handles {
            assert!(showcase.mesh_frames.get(&resource_id, generation).is_none());
        }

        let fresh = frame(
            "mesh-positions",
            MeshFrameKind::Geometry,
            MeshDtype::F64LE,
            &[1, 3],
            bytes(&[0.0_f64, 0.0, 0.0], f64::to_le_bytes),
        );
        assert!(matches!(
            showcase.mesh_frames.ingest(fresh),
            Ok(MeshFrameOutcome::Assembled(_))
        ));
    }

    #[test]
    fn native_host_shutdown_releases_runtime_ownership_idempotently() {
        let (spec, frames) = fixture();
        let handles = mesh_plot_resource_handles(&spec).expect("fixture resource handles");
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = Rc::try_unwrap(frames)
            .expect("fixture frame store has one owner")
            .into_inner();
        showcase
            .sync_mesh_plot_resource_refs_for_spec(&spec)
            .expect("retain resources for the active plot");
        showcase
            .mesh_plots
            .upsert(spec)
            .expect("cache the active plot specification");

        let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
        showcase
            .profiler_subscriptions
            .insert("shutdown-test".into(), cancellation.clone());
        assert_eq!(
            showcase.mesh_frames.stats().referenced_resources,
            handles.len()
        );
        assert_eq!(showcase.mesh_plots.len(), 1);

        showcase.shutdown_runtime_state();

        assert!(cancellation.load(Ordering::Acquire));
        assert!(showcase.profiler_subscriptions.is_empty());
        assert!(showcase.mesh_plot_resource_refs.is_empty());
        assert_eq!(showcase.mesh_plots.len(), 0);
        assert!(showcase.mesh_plot_states.is_empty());
        assert!(showcase.mesh_plot_errors.is_empty());
        assert_eq!(showcase.mesh_frames.stats().resources, 0);
        assert_eq!(showcase.mesh_frames.stats().references, 0);
        for (resource_id, generation) in &handles {
            assert!(showcase.mesh_frames.get(resource_id, *generation).is_none());
        }

        // Drop calls the same cleanup after this explicit shutdown; a second
        // direct call also proves that cleanup does not depend on live refs.
        showcase.shutdown_runtime_state();
        assert!(showcase.mesh_plot_resource_refs.is_empty());
        assert_eq!(showcase.mesh_plots.len(), 0);
        assert!(showcase.mesh_plot_states.is_empty());
        assert!(showcase.mesh_plot_errors.is_empty());
        assert_eq!(showcase.mesh_frames.stats().resources, 0);
        assert_eq!(showcase.mesh_frames.stats().references, 0);
    }

    #[test]
    fn native_mesh_patch_errors_are_localized_to_the_active_plot() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        let patch = gpui_python_runtime::session::Patch {
            revision: 2,
            request_id: Some("invalid-field".into()),
            ops: vec![gpui_python_runtime::session::PatchOp::ReplaceMeshField {
                plot_id: "plot".into(),
                generation: 2,
                field: Value::Null,
            }],
        };

        showcase.record_mesh_patch_error(&patch, "invalid mesh field");

        assert_eq!(showcase.load_error, None);
        assert_eq!(
            showcase.mesh_plot_errors.get("plot").map(String::as_str),
            Some("invalid mesh field")
        );
    }
}

fn validate_mesh_plot_spec_resources(
    spec: &MeshPlotSpec,
    store: &MeshFrameStore,
    patch_id: Option<&str>,
) -> Result<(), MeshPlotResourceError> {
    let refs = spec
        .resource_refs()
        .map_err(|message| MeshPlotResourceError::InvalidReference {
            plot_id: spec.id.clone(),
            role: "mesh_plot".into(),
            message,
            patch_id: patch_id.map(str::to_owned),
        })?;
    for resource in refs {
        if store
            .get(&resource.resource_id, resource.generation)
            .is_none()
        {
            return Err(MeshPlotResourceError::Unavailable {
                plot_id: spec.id.clone(),
                role: resource.role,
                resource_id: resource.resource_id,
                generation: resource.generation,
                patch_id: patch_id.map(str::to_owned),
            });
        }
    }
    let invalid = |message: String| MeshPlotResourceError::InvalidReference {
        plot_id: spec.id.clone(),
        role: "mesh_plot".into(),
        message,
        patch_id: patch_id.map(str::to_owned),
    };
    let (positions, triangles) =
        decode_mesh_geometry(&spec.geometry, store).map_err(|message| invalid(message))?;
    if matches!(
        spec.view.as_str(),
        "axisymmetric_section" | "axisymmetric_revolve"
    ) && positions.iter().any(|position| position[0] < -1.0e-12)
    {
        return Err(invalid(
            "axisymmetric mesh radial coordinates must be non-negative".into(),
        ));
    }
    let mesh_id = spec
        .geometry
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("mesh");
    let vertex_ids = decode_inline_ids(&spec.geometry, "vertex_ids", positions.len(), store)
        .map_err(|message| invalid(message))?
        .map(Arc::from);
    let cell_ids = decode_inline_ids(&spec.geometry, "cell_ids", triangles.len(), store)
        .map_err(|message| invalid(message))?
        .map(Arc::from);
    let mesh = TriangleMesh {
        id: Arc::from(mesh_id),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids,
        cell_ids,
    };
    mesh.validate()
        .map_err(|error| invalid(error.to_string()))?;
    if let Some(field) = spec.field.as_ref() {
        let (values, valid) =
            decode_mesh_field(field, store).map_err(|message| invalid(message))?;
        let association = match field.get("association").and_then(Value::as_str) {
            Some("cell") => ScalarAssociation::Cell,
            _ => ScalarAssociation::Vertex,
        };
        let scalar = ScalarField {
            id: Arc::from(field.get("id").and_then(Value::as_str).unwrap_or("field")),
            label: Arc::from(
                field
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Field"),
            ),
            unit: field.get("unit").and_then(Value::as_str).map(Arc::from),
            values: values.into(),
            association,
            valid: valid.map(Arc::from),
        };
        scalar
            .validate(&mesh)
            .map_err(|error| invalid(error.to_string()))?;
        let color_range =
            native_mesh_plot_color_range(&spec.color_range).map_err(|message| invalid(message))?;
        let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
        for (index, value) in scalar.values.iter().enumerate() {
            if scalar
                .valid
                .as_ref()
                .is_some_and(|valid| valid.get(index) != Some(&true))
            {
                continue;
            }
            min = min.min(*value);
            max = max.max(*value);
        }
        if min.is_finite() && max.is_finite() {
            color_range
                .resolve(min, max)
                .map_err(|error| invalid(error.to_string()))?;
        }
    }
    native_mesh_plot_options(spec, mesh_id).map_err(|message| invalid(message))?;
    Ok(())
}

fn validate_mesh_plot_resources(
    value: &Value,
    store: &MeshFrameStore,
    patch_id: Option<&str>,
) -> Result<(), MeshPlotResourceError> {
    if value.get("kind").and_then(Value::as_str) == Some("mesh_plot") {
        let spec_value = value.get("spec").cloned().unwrap_or(Value::Null);
        let spec = MeshPlotSpec::from_value(spec_value).map_err(|message| {
            MeshPlotResourceError::InvalidReference {
                plot_id: value
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("<unknown>")
                    .into(),
                role: "mesh_plot".into(),
                message,
                patch_id: patch_id.map(str::to_owned),
            }
        })?;
        validate_mesh_plot_spec_resources(&spec, store, patch_id)?;
    }
    if let Some(object) = value.as_object() {
        for child in object.values() {
            validate_mesh_plot_resources(child, store, patch_id)?;
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            validate_mesh_plot_resources(child, store, patch_id)?;
        }
    }
    Ok(())
}

fn mesh_plot_ids(value: &Value, ids: &mut HashSet<String>) {
    if value.get("kind").and_then(Value::as_str) == Some("mesh_plot") {
        if let Some(id) = value.get("id").and_then(Value::as_str) {
            ids.insert(id.to_owned());
        }
        if let Some(spec_id) = value
            .get("spec")
            .and_then(|spec| spec.get("id"))
            .and_then(Value::as_str)
        {
            // The UI node ID is the patch address, while the MeshPlot spec ID
            // owns retained GPU/cache state. `ui.mesh_plot(..., id=...)`
            // permits these IDs to differ, so both are live runtime aliases.
            ids.insert(spec_id.to_owned());
        }
    }
    if let Some(object) = value.as_object() {
        for child in object.values() {
            mesh_plot_ids(child, ids);
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            mesh_plot_ids(child, ids);
        }
    }
}

fn mesh_plot_spec_id_for_node(value: &Value, node_id: &str) -> Option<String> {
    if value.get("kind").and_then(Value::as_str) == Some("mesh_plot")
        && value.get("id").and_then(Value::as_str) == Some(node_id)
    {
        return value
            .get("spec")
            .and_then(|spec| spec.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
    }
    if let Some(object) = value.as_object() {
        for child in object.values() {
            if let Some(id) = mesh_plot_spec_id_for_node(child, node_id) {
                return Some(id);
            }
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            if let Some(id) = mesh_plot_spec_id_for_node(child, node_id) {
                return Some(id);
            }
        }
    }
    None
}

fn mesh_plot_resource_handles(spec: &MeshPlotSpec) -> Result<Vec<(String, u64)>, String> {
    let mut handles = spec
        .resource_refs()?
        .into_iter()
        .map(|resource| (resource.resource_id, resource.generation))
        .collect::<Vec<_>>();
    // A plot can use one resource for more than one role (for example, a
    // shared validity mask). One native plot is still one cache owner.
    handles.sort_unstable();
    handles.dedup();
    Ok(handles)
}

fn collect_mesh_plot_resource_refs(
    value: &Value,
    refs: &mut HashMap<String, Vec<(String, u64)>>,
) -> Result<(), String> {
    if value.get("kind").and_then(Value::as_str) == Some("mesh_plot") {
        let spec_value = value.get("spec").cloned().unwrap_or(Value::Null);
        let spec = MeshPlotSpec::from_value(spec_value)?;
        if refs
            .insert(spec.id.clone(), mesh_plot_resource_handles(&spec)?)
            .is_some()
        {
            return Err(format!("duplicate mesh_plot id {:?}", spec.id));
        }
    }
    if let Some(object) = value.as_object() {
        for child in object.values() {
            collect_mesh_plot_resource_refs(child, refs)?;
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            collect_mesh_plot_resource_refs(child, refs)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod runtime_resource_release_tests {
    use super::{PresentationStore, PythonIrShowcase};
    use gpui_python_runtime::audio_stream::{AudioFrame, AudioFrameKind};
    use gpui_python_runtime::mesh_frames::{MeshDtype, MeshFrame, MeshFrameKind};
    use gpui_python_runtime::meshplot::MeshPlotSpec;
    use gpui_python_runtime::session::{Patch, PatchOp};
    use gpui_python_runtime::ui_ir::{PythonAppIr, UiNode};
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    fn audio_frame(resource_id: &str) -> AudioFrame {
        AudioFrame {
            resource_id: resource_id.into(),
            generation: 1,
            sequence: 1,
            frame_kind: AudioFrameKind::Meter,
            byte_length: 4,
            shape: vec![1, 1],
            dtype: "f32".into(),
            byte_order: "little".into(),
            finite_policy: "drop_frame".into(),
            coalesce: "latest".into(),
            sample_rate: 48_000.0,
            attack_ms: None,
            release_ms: None,
            minimum_frequency: None,
            maximum_frequency: None,
            payload: 0.0_f32.to_le_bytes().to_vec(),
        }
    }

    fn mesh_frame(resource_id: &str) -> MeshFrame {
        MeshFrame {
            resource_id: resource_id.into(),
            generation: 1,
            sequence: 0,
            chunk_count: 1,
            kind: MeshFrameKind::Field,
            dtype: MeshDtype::F64LE,
            shape: vec![1],
            payload: 0.0_f64.to_le_bytes().to_vec(),
        }
    }

    fn app_with_resource_mesh(field_resource_id: &str, field_generation: u64) -> PythonAppIr {
        serde_json::from_value(serde_json::json!({
            "title": "MeshPlot patch transaction",
            "sections": [{
                "id": "main",
                "label": "Main",
                "content": {
                    "kind": "mesh_plot",
                    "id": "resource-node",
                    "spec": {
                        "schema_version": 1,
                        "id": "resource-plot",
                        "geometry": {
                            "id": "resource-mesh",
                            "positions": {"resource_id": "positions", "generation": 1, "dtype": "f64le"},
                            "triangles": {"resource_id": "triangles", "generation": 1, "dtype": "u32le"}
                        },
                        "field": {
                            "id": "pressure",
                            "label": "Pressure",
                            "unit": "Pa",
                            "resource_id": field_resource_id,
                            "generation": field_generation,
                            "association": "vertex",
                            "valid": {"resource_id": "valid", "generation": 1, "dtype": "bool_bytes"}
                        },
                        "view": "planar",
                        "mode": "scalar_fill",
                        "color_scale": "viridis",
                        "equal_aspect": true
                    }
                }
            }]
        }))
        .expect("valid resource-backed MeshPlot app")
    }

    fn resource_mesh_store() -> gpui_python_runtime::mesh_frames::MeshFrameStore {
        use gpui_python_runtime::mesh_frames::MeshFrameStore;

        let mut store = MeshFrameStore::new();
        let add = |store: &mut MeshFrameStore,
                   resource_id: &str,
                   kind: MeshFrameKind,
                   dtype: MeshDtype,
                   shape: Vec<u32>,
                   payload: Vec<u8>| {
            store
                .ingest(MeshFrame {
                    resource_id: resource_id.into(),
                    generation: 1,
                    sequence: 0,
                    chunk_count: 1,
                    kind,
                    dtype,
                    shape,
                    payload,
                })
                .expect("valid resource frame");
        };
        add(
            &mut store,
            "positions",
            MeshFrameKind::Geometry,
            MeshDtype::F64LE,
            vec![3, 3],
            [0.0_f64, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
                .into_iter()
                .flat_map(f64::to_le_bytes)
                .collect(),
        );
        add(
            &mut store,
            "triangles",
            MeshFrameKind::Geometry,
            MeshDtype::U32LE,
            vec![1, 3],
            [0_u32, 1, 2]
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect(),
        );
        add(
            &mut store,
            "pressure",
            MeshFrameKind::Field,
            MeshDtype::F64LE,
            vec![3],
            [0.0_f64, 0.5, 1.0]
                .into_iter()
                .flat_map(f64::to_le_bytes)
                .collect(),
        );
        add(
            &mut store,
            "valid",
            MeshFrameKind::Mask,
            MeshDtype::BoolBytes,
            vec![3],
            vec![1, 1, 1],
        );
        store
    }

    #[test]
    fn shared_drop_resource_releases_audio_without_mesh_error() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase
            .audio_frames
            .ingest(audio_frame("audio-only"))
            .expect("valid audio frame");

        showcase.release_runtime_resource("audio-only", 1);

        assert!(showcase.audio_frames.get("audio-only").is_none());
        assert!(showcase.mesh_frames.get("audio-only", 1).is_none());
        assert_eq!(showcase.load_error, None);
    }

    #[test]
    fn shared_drop_resource_releases_mesh_and_reports_unknown_handles() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase
            .mesh_frames
            .ingest(mesh_frame("mesh-only"))
            .expect("valid mesh frame");

        showcase.release_runtime_resource("mesh-only", 1);
        assert!(showcase.mesh_frames.get("mesh-only", 1).is_none());
        assert_eq!(showcase.load_error, None);

        showcase.release_runtime_resource("missing", 1);
        assert_eq!(
            showcase.load_error.as_deref(),
            Some("mesh resource \"missing\" generation 1 was not retained")
        );
    }

    #[test]
    fn shared_mesh_generation_has_one_reference_per_active_plot() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase
            .mesh_frames
            .ingest(mesh_frame("shared"))
            .expect("valid mesh frame");

        let mut one = HashMap::new();
        one.insert("plot-a".into(), vec![("shared".into(), 1)]);
        showcase
            .sync_mesh_plot_resource_refs(one.clone())
            .expect("retain first plot owner");
        assert_eq!(showcase.mesh_frames.stats().references, 1);

        let mut both = one;
        both.insert("plot-b".into(), vec![("shared".into(), 1)]);
        showcase
            .sync_mesh_plot_resource_refs(both)
            .expect("retain second plot owner");
        assert_eq!(showcase.mesh_frames.stats().references, 2);
        assert!(
            !showcase.mesh_frames.release("shared", 1),
            "an explicitly dropped generation remains owned by plot-b"
        );
        showcase.release_runtime_resource("shared", 1);
        assert_eq!(
            showcase.mesh_plot_errors.get("plot-a").map(String::as_str),
            Some("mesh resource \"shared\" generation 1 is still owned by an active plot")
        );

        let mut plot_b = HashMap::new();
        plot_b.insert("plot-b".into(), vec![("shared".into(), 1)]);
        showcase
            .sync_mesh_plot_resource_refs(plot_b)
            .expect("release plot-a owner");
        assert_eq!(showcase.mesh_frames.stats().references, 1);

        showcase
            .sync_mesh_plot_resource_refs(HashMap::new())
            .expect("release plot-b owner");
        assert_eq!(showcase.mesh_frames.stats().references, 0);
        assert!(showcase.mesh_frames.release("shared", 1));
    }

    #[test]
    fn unknown_mesh_generation_does_not_contaminate_other_generation_owner() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase
            .mesh_frames
            .ingest(mesh_frame("shared"))
            .expect("valid mesh frame");

        let mut refs = HashMap::new();
        refs.insert("plot-a".into(), vec![("shared".into(), 1)]);
        showcase
            .sync_mesh_plot_resource_refs(refs)
            .expect("retain generation one for plot-a");

        showcase.release_runtime_resource("shared", 2);

        assert!(showcase.mesh_plot_errors.is_empty());
        assert_eq!(
            showcase.load_error.as_deref(),
            Some("mesh resource \"shared\" generation 2 was not retained")
        );
    }

    #[test]
    fn native_patch_transaction_keeps_newest_valid_resources_and_rejects_late_updates() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = resource_mesh_store();
        showcase.app = Some(app_with_resource_mesh("pressure", 1));

        let mut initial_refs = HashMap::new();
        initial_refs.insert(
            "resource-plot".to_string(),
            vec![
                ("positions".to_string(), 1),
                ("triangles".to_string(), 1),
                ("pressure".to_string(), 1),
                ("valid".to_string(), 1),
            ],
        );
        showcase
            .sync_mesh_plot_resource_refs(initial_refs)
            .expect("retain initial app resources");
        showcase.mesh_plot_states.insert(
            "resource-plot".into(),
            Rc::new(RefCell::new(gpui_px::MeshPlotState::new(
                0.0, 1.0, 0.0, 1.0,
            ))),
        );
        assert_eq!(showcase.mesh_frames.stats().references, 4);

        let mut replacement = mesh_frame("pressure");
        replacement.generation = 2;
        replacement.shape = vec![3];
        replacement.payload = [1.0_f64, 1.5, 2.0]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect();
        showcase
            .mesh_frames
            .ingest(replacement)
            .expect("ingest newer field generation");

        showcase.apply_patch_message(Patch {
            revision: 1,
            request_id: Some("field-update".into()),
            ops: vec![
                PatchOp::ReplaceMeshField {
                    plot_id: "resource-node".into(),
                    generation: 2,
                    field: serde_json::json!({
                        "id": "pressure",
                        "label": "Pressure",
                        "unit": "Pa",
                        "resource_id": "pressure",
                        "generation": 2,
                        "association": "vertex",
                        "valid": {"resource_id": "valid", "generation": 1, "dtype": "bool_bytes"}
                    }),
                },
                PatchOp::ClearMeshPlotSelection {
                    plot_id: "resource-node".into(),
                    generation: 2,
                },
            ],
        });

        assert_eq!(showcase.session_state.revision(), 1);
        assert_eq!(
            showcase.session_state.mesh_generation("resource-node"),
            Some(2)
        );
        let UiNode::MeshPlot(committed_node) =
            &showcase.app.as_ref().expect("committed app").sections[0].content
        else {
            panic!("expected MeshPlot node");
        };
        assert_eq!(committed_node.spec["field"]["generation"], 2);
        assert_eq!(showcase.mesh_frames.stats().references, 4);
        assert!(showcase.mesh_plot_states.contains_key("resource-plot"));
        assert_eq!(
            showcase
                .mesh_plot_resource_refs
                .get("resource-plot")
                .expect("plot ownership")
                .iter()
                .filter(|(resource_id, generation)| {
                    resource_id == "pressure" && *generation == 2
                })
                .count(),
            1
        );

        let committed_app = showcase.app.clone().expect("committed app snapshot");
        showcase.apply_patch_message(Patch {
            revision: 2,
            request_id: Some("missing-field".into()),
            ops: vec![PatchOp::ReplaceMeshField {
                plot_id: "resource-node".into(),
                generation: 3,
                field: serde_json::json!({
                    "id": "pressure",
                    "label": "Pressure",
                    "unit": "Pa",
                    "resource_id": "missing-pressure",
                    "generation": 3,
                    "association": "vertex"
                }),
            }],
        });
        assert_eq!(showcase.session_state.revision(), 1);
        assert_eq!(
            showcase.session_state.mesh_generation("resource-node"),
            Some(2)
        );
        assert_eq!(showcase.app, Some(committed_app.clone()));
        assert!(
            showcase
                .mesh_plot_errors
                .get("resource-node")
                .is_some_and(|message| message.contains("missing-pressure")
                    && message.contains("missing-field"))
        );
        assert!(showcase.mesh_plot_errors.contains_key("resource-plot"));

        showcase.apply_patch_message(Patch {
            revision: 1,
            request_id: Some("late-field".into()),
            ops: vec![PatchOp::ReplaceMeshField {
                plot_id: "resource-node".into(),
                generation: 1,
                field: serde_json::json!({
                    "id": "pressure",
                    "label": "Pressure",
                    "unit": "Pa",
                    "resource_id": "pressure",
                    "generation": 1,
                    "association": "vertex",
                    "valid": {"resource_id": "valid", "generation": 1, "dtype": "bool_bytes"}
                }),
            }],
        });
        assert_eq!(showcase.session_state.revision(), 1);
        assert_eq!(showcase.app, Some(committed_app));
        assert!(
            showcase
                .mesh_plot_errors
                .get("resource-node")
                .is_some_and(|message| message.contains("stale") || message.contains("revision"))
        );
    }

    #[test]
    fn malformed_inline_mesh_patch_preserves_last_valid_commit_and_recovers() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = resource_mesh_store();
        let initial = app_with_resource_mesh("pressure", 1);
        showcase.app = Some(initial.clone());

        let mut refs = HashMap::new();
        refs.insert(
            "resource-plot".to_string(),
            vec![
                ("positions".to_string(), 1),
                ("triangles".to_string(), 1),
                ("pressure".to_string(), 1),
                ("valid".to_string(), 1),
            ],
        );
        showcase
            .sync_mesh_plot_resource_refs(refs)
            .expect("retain initial app resources");
        let committed = showcase.app.clone().expect("initial app");

        showcase.apply_patch_message(Patch {
            revision: 1,
            request_id: Some("malformed-inline-field".into()),
            ops: vec![PatchOp::ReplaceMeshField {
                plot_id: "resource-node".into(),
                generation: 2,
                field: serde_json::json!({
                    "id": "pressure",
                    "label": "Pressure",
                    "association": "vertex",
                    "values": [1.0, "not-a-number", 2.0]
                }),
            }],
        });

        assert_eq!(showcase.app, Some(committed.clone()));
        assert_eq!(showcase.session_state.revision(), 0);
        assert_eq!(
            showcase.session_state.mesh_generation("resource-node"),
            None
        );
        assert_eq!(showcase.mesh_frames.stats().references, 4);
        assert!(
            showcase
                .mesh_plot_errors
                .get("resource-node")
                .is_some_and(|message| message.contains("mesh_plot field values must be finite"))
        );

        // The rejected revision must not poison the stream: a later valid
        // patch at the same revision can commit and clear the localized error.
        showcase.apply_patch_message(Patch {
            revision: 1,
            request_id: Some("recovered-field".into()),
            ops: vec![PatchOp::ReplaceMeshField {
                plot_id: "resource-node".into(),
                generation: 2,
                field: serde_json::json!({
                    "id": "pressure",
                    "label": "Recovered pressure",
                    "unit": "Pa",
                    "resource_id": "pressure",
                    "generation": 1,
                    "association": "vertex",
                    "valid": {"resource_id": "valid", "generation": 1, "dtype": "bool_bytes"}
                }),
            }],
        });

        assert_eq!(showcase.session_state.revision(), 1);
        assert_eq!(
            showcase.session_state.mesh_generation("resource-node"),
            Some(2)
        );
        assert_eq!(showcase.mesh_frames.stats().references, 4);
        assert!(showcase.mesh_plot_errors.is_empty());
        let UiNode::MeshPlot(node) =
            &showcase.app.as_ref().expect("recovered app").sections[0].content
        else {
            panic!("expected MeshPlot node");
        };
        assert_eq!(node.spec["field"]["label"], "Recovered pressure");
    }

    #[test]
    fn native_snapshot_transaction_replaces_owners_and_preserves_last_valid_state() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = resource_mesh_store();

        let initial = app_with_resource_mesh("pressure", 1);
        showcase
            .mesh_plots
            .upsert(
                MeshPlotSpec::from_value(serde_json::json!({
                    "schema_version": 1,
                    "id": "resource-plot",
                    "geometry": {
                        "id": "resource-mesh",
                        "positions": {"resource_id": "positions", "generation": 1, "dtype": "f64le"},
                        "triangles": {"resource_id": "triangles", "generation": 1, "dtype": "u32le"}
                    },
                    "field": {
                        "id": "pressure",
                        "resource_id": "pressure",
                        "generation": 1,
                        "association": "vertex",
                        "valid": {"resource_id": "valid", "generation": 1, "dtype": "bool_bytes"}
                    },
                    "view": "planar",
                    "mode": "scalar_fill",
                    "color_scale": "viridis",
                    "equal_aspect": true
                }))
                .expect("valid cached MeshPlot spec"),
            )
            .expect("valid cached MeshPlot spec");
        showcase.apply_snapshot_message(initial.clone());
        assert_eq!(showcase.app, Some(initial));
        assert_eq!(showcase.mesh_frames.stats().references, 4);

        let mut replacement = mesh_frame("pressure");
        replacement.generation = 2;
        replacement.shape = vec![3];
        replacement.payload = [1.0_f64, 1.5, 2.0]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect();
        showcase
            .mesh_frames
            .ingest(replacement)
            .expect("ingest newer field generation");
        let updated = app_with_resource_mesh("pressure", 2);
        showcase.apply_snapshot_message(updated.clone());
        assert_eq!(showcase.app, Some(updated));
        assert_eq!(showcase.mesh_frames.stats().references, 4);
        assert_eq!(
            showcase
                .mesh_plot_resource_refs
                .get("resource-plot")
                .expect("retained snapshot resources")
                .iter()
                .filter(|(resource_id, generation)| {
                    resource_id == "pressure" && *generation == 2
                })
                .count(),
            1
        );
        assert!(
            showcase.mesh_plots.get("resource-plot").is_some(),
            "a same-id snapshot must retain the last-valid fallback cache"
        );

        let mut invalid = showcase.app.clone().expect("updated snapshot");
        let UiNode::MeshPlot(node) = &mut invalid.sections[0].content else {
            panic!("expected MeshPlot snapshot");
        };
        node.id.clear();
        let committed = showcase.app.clone();
        showcase.apply_snapshot_message(invalid);
        assert_eq!(showcase.app, committed);
        assert_eq!(showcase.mesh_frames.stats().references, 4);
        assert!(
            showcase
                .load_error
                .as_deref()
                .is_some_and(|message| message.contains("stable id"))
        );

        let missing = app_with_resource_mesh("missing-pressure", 3);
        showcase.apply_snapshot_message(missing.clone());
        assert_eq!(showcase.app, Some(missing));
        assert_eq!(showcase.mesh_frames.stats().references, 0);
        assert_eq!(showcase.load_error, None);
        assert!(
            showcase.mesh_plots.get("resource-plot").is_some(),
            "a snapshot that arrives before its frames must preserve the cached fallback"
        );
    }

    #[test]
    fn native_host_recovery_matrix_preserves_state_and_releases_plot_resources() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = resource_mesh_store();

        let initial = app_with_resource_mesh("pressure", 1);
        let UiNode::MeshPlot(node) = &initial.sections[0].content else {
            panic!("expected MeshPlot node");
        };
        showcase
            .mesh_plots
            .upsert(MeshPlotSpec::from_value(node.spec.clone()).expect("valid cached spec"))
            .expect("cache initial fallback spec");
        showcase.apply_snapshot_message(initial.clone());
        assert_eq!(showcase.app, Some(initial));
        assert_eq!(showcase.mesh_frames.stats().references, 4);

        let mut generation_two = mesh_frame("pressure");
        generation_two.generation = 2;
        generation_two.shape = vec![3];
        generation_two.payload = [1.0_f64, 1.5, 2.0]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect();
        showcase
            .mesh_frames
            .ingest(generation_two)
            .expect("ingest valid replacement field");
        showcase.apply_patch_message(Patch {
            revision: 1,
            request_id: Some("matrix-field-update".into()),
            ops: vec![PatchOp::ReplaceMeshField {
                plot_id: "resource-node".into(),
                generation: 2,
                field: serde_json::json!({
                    "id": "pressure",
                    "label": "Pressure",
                    "unit": "Pa",
                    "resource_id": "pressure",
                    "generation": 2,
                    "association": "vertex",
                    "valid": {"resource_id": "valid", "generation": 1, "dtype": "bool_bytes"}
                }),
            }],
        });
        let committed = showcase.app.clone().expect("committed field replacement");
        assert_eq!(showcase.session_state.revision(), 1);
        assert_eq!(showcase.mesh_frames.stats().references, 4);
        assert_eq!(showcase.app, Some(committed.clone()));

        // A snapshot is allowed to arrive before its next binary generation.
        // The old owners are released, but the cached generation remains
        // available as the last-valid fallback while the stream recovers.
        let pending = app_with_resource_mesh("pressure", 3);
        showcase.apply_snapshot_message(pending.clone());
        assert_eq!(showcase.app, Some(pending.clone()));
        assert_eq!(showcase.mesh_frames.stats().references, 0);
        assert!(showcase.mesh_plots.get("resource-plot").is_some());

        let mut malformed = mesh_frame("pressure");
        malformed.generation = 3;
        malformed.payload.pop();
        showcase.apply_mesh_frame_message(malformed);
        assert!(showcase.mesh_frames.get("pressure", 3).is_none());
        assert!(
            showcase
                .mesh_plot_errors
                .get("resource-plot")
                .is_some_and(
                    |message| message.contains("pressure") && message.contains("generation 3")
                )
        );

        let mut corrected = mesh_frame("pressure");
        corrected.generation = 3;
        corrected.shape = vec![3];
        corrected.payload = [2.0_f64, 2.5, 3.0]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect();
        showcase.apply_mesh_frame_message(corrected);
        assert!(showcase.mesh_plot_errors.is_empty());
        assert!(showcase.mesh_frames.get("pressure", 3).is_some());

        // A late frame from the superseded generation must be ignored after
        // recovery. It must not replace the corrected payload, clear or
        // reintroduce the plot-local diagnostic, or disturb active ownership.
        let recovered_payload = showcase
            .mesh_frames
            .get("pressure", 3)
            .expect("corrected generation is retained")
            .payload
            .clone();
        let recovered_stats = showcase.mesh_frames.stats();
        let mut stale = mesh_frame("pressure");
        stale.generation = 2;
        stale.shape = vec![3];
        stale.payload = [9.0_f64, 9.5, 10.0]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect();
        showcase.apply_mesh_frame_message(stale);
        assert_eq!(
            showcase
                .mesh_frames
                .get("pressure", 3)
                .expect("stale frame must not evict the recovered generation")
                .payload,
            recovered_payload
        );
        assert!(showcase.mesh_plot_errors.is_empty());
        assert_eq!(showcase.load_error, None);
        assert_eq!(showcase.mesh_frames.stats(), recovered_stats);

        let UiNode::MeshPlot(node) = &pending.sections[0].content else {
            panic!("expected pending MeshPlot node");
        };
        showcase
            .sync_mesh_plot_resource_refs_for_spec(
                &MeshPlotSpec::from_value(node.spec.clone()).expect("corrected spec"),
            )
            .expect("retain corrected snapshot resources");
        assert_eq!(showcase.mesh_frames.stats().references, 4);

        let committed_pending = showcase.app.clone().expect("pending snapshot");
        showcase.apply_patch_message(Patch {
            revision: 2,
            request_id: Some("matrix-missing-field".into()),
            ops: vec![PatchOp::ReplaceMeshField {
                plot_id: "resource-node".into(),
                generation: 4,
                field: serde_json::json!({
                    "id": "pressure",
                    "label": "Pressure",
                    "unit": "Pa",
                    "resource_id": "missing-pressure",
                    "generation": 4,
                    "association": "vertex"
                }),
            }],
        });
        assert_eq!(showcase.app, Some(committed_pending));
        assert_eq!(showcase.session_state.revision(), 1);
        assert!(
            showcase
                .mesh_plot_errors
                .get("resource-node")
                .is_some_and(|message| message.contains("missing-pressure"))
        );

        // Explicit release remains rejected while the corrected plot owns the
        // generation, then all ownership disappears with plot removal.
        showcase.release_runtime_resource("pressure", 3);
        assert!(showcase.mesh_plot_errors.contains_key("resource-plot"));
        let replacement_app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Recovered",
            "sections": [{
                "id": "main",
                "label": "Main",
                "content": {"kind": "text", "id": "done", "text": "recovered"}
            }]
        }))
        .expect("valid replacement app");
        showcase.apply_snapshot_message(replacement_app);
        assert!(showcase.mesh_plot_resource_refs.is_empty());
        assert!(showcase.mesh_plot_states.is_empty());
        assert!(showcase.mesh_plot_errors.is_empty());
        assert_eq!(showcase.mesh_frames.stats().references, 0);
        assert_eq!(showcase.load_error, None);
        assert_eq!(
            showcase.session_state.mesh_generation("resource-node"),
            None,
            "removing a plot must release its generation history"
        );
    }

    #[test]
    fn native_pre_frame_errors_are_localized_to_the_declared_snapshot_plot() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = gpui_python_runtime::mesh_frames::MeshFrameStore::new();
        showcase.apply_snapshot_message(app_with_resource_mesh("pressure", 2));

        let mut invalid = mesh_frame("pressure");
        invalid.generation = 2;
        invalid.payload.pop();
        showcase.apply_mesh_frame_message(invalid);

        assert_eq!(showcase.load_error, None);
        assert!(
            showcase
                .mesh_plot_errors
                .get("resource-plot")
                .is_some_and(
                    |message| message.contains("pressure") && message.contains("generation 2")
                )
        );

        let mut corrected = mesh_frame("pressure");
        corrected.generation = 2;
        corrected.shape = vec![3];
        corrected.payload = [1.0_f64, 1.5, 2.0]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect();
        showcase.apply_mesh_frame_message(corrected);

        assert!(showcase.mesh_plot_errors.is_empty());
        assert_eq!(showcase.load_error, None);
    }

    #[test]
    fn native_mesh_frame_decode_errors_are_localized_to_the_retained_plot() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = gpui_python_runtime::mesh_frames::MeshFrameStore::new();
        // The malformed frame is rejected before it can be retained. Keep
        // the declared owner in the routing index so this test exercises the
        // resource-id-plus-generation match independently of frame storage.
        showcase
            .mesh_plot_resource_refs
            .insert("resource-plot".into(), vec![("pressure".into(), 2)]);
        showcase.last_mesh_patch_id = Some("field-update".into());

        let mut invalid = mesh_frame("pressure");
        invalid.generation = 2;
        invalid.payload.pop();
        showcase.apply_mesh_frame_message(invalid);

        assert!(showcase.mesh_frames.get("pressure", 2).is_none());
        assert!(
            showcase
                .mesh_plot_errors
                .get("resource-plot")
                .is_some_and(|message| message.contains("pressure")
                    && message.contains("generation 2")
                    && message.contains("field-update"))
        );
        assert_eq!(showcase.load_error, None);

        let mut corrected = mesh_frame("pressure");
        corrected.generation = 2;
        corrected.shape = vec![3];
        corrected.payload = [1.0_f64, 1.5, 2.0]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect();
        showcase.apply_mesh_frame_message(corrected);
        assert!(showcase.mesh_frames.get("pressure", 2).is_some());
        assert!(!showcase.mesh_plot_errors.contains_key("resource-plot"));
        assert_eq!(showcase.load_error, None);
    }

    #[test]
    fn evicted_resource_patch_preserves_the_newest_valid_committed_state() {
        let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
        showcase.mesh_frames = resource_mesh_store();
        let initial = app_with_resource_mesh("pressure", 1);
        let UiNode::MeshPlot(node) = &initial.sections[0].content else {
            panic!("expected MeshPlot node");
        };
        showcase
            .mesh_plots
            .upsert(MeshPlotSpec::from_value(node.spec.clone()).expect("valid cached spec"))
            .expect("cache initial spec");
        showcase.apply_snapshot_message(initial.clone());
        assert_eq!(showcase.mesh_frames.stats().references, 4);

        // A plot removal releases ownership before the resource is evicted.
        // The cached specification remains available as the last-valid state,
        // but a later patch must not commit a handle that no longer exists.
        showcase
            .sync_mesh_plot_resource_refs(HashMap::new())
            .expect("release removed plot ownership");
        assert_eq!(showcase.mesh_frames.stats().references, 0);
        assert!(showcase.mesh_frames.release("pressure", 1));
        assert!(showcase.mesh_frames.get("pressure", 1).is_none());

        let committed = showcase.app.clone();
        showcase.apply_patch_message(Patch {
            revision: 1,
            request_id: Some("evicted-field".into()),
            ops: vec![PatchOp::ReplaceMeshField {
                plot_id: "resource-node".into(),
                generation: 1,
                field: serde_json::json!({
                    "id": "pressure",
                    "label": "Pressure",
                    "unit": "Pa",
                    "resource_id": "pressure",
                    "generation": 1,
                    "association": "vertex",
                    "valid": {"resource_id": "valid", "generation": 1, "dtype": "bool_bytes"}
                }),
            }],
        });

        assert_eq!(showcase.session_state.revision(), 0);
        assert_eq!(showcase.app, committed);
        assert!(
            showcase
                .mesh_plot_errors
                .get("resource-node")
                .is_some_and(|message| {
                    message.contains("pressure") && message.contains("evicted-field")
                })
        );
    }
}

#[cfg(test)]
mod mesh_resource_decode_tests {
    use super::*;
    use gpui_python_runtime::mesh_frames::MeshDtype;

    fn retain(
        store: &mut MeshFrameStore,
        resource_id: &str,
        kind: MeshFrameKind,
        dtype: MeshDtype,
        shape: Vec<u32>,
        payload: Vec<u8>,
    ) {
        let outcome = store
            .ingest(MeshFrame {
                resource_id: resource_id.into(),
                generation: 1,
                sequence: 0,
                chunk_count: 1,
                kind,
                dtype,
                shape,
                payload,
            })
            .expect("valid mesh resource frame");
        assert!(matches!(outcome, MeshFrameOutcome::Assembled(_)));
    }

    #[::core::prelude::v1::test]
    fn decodes_resource_backed_ids_and_both_mask_encodings() {
        let mut store = MeshFrameStore::new();
        retain(
            &mut store,
            "vertex_ids",
            MeshFrameKind::Ids,
            MeshDtype::U64LE,
            vec![3],
            [10_u64, 20, 30]
                .into_iter()
                .flat_map(u64::to_le_bytes)
                .collect(),
        );
        retain(
            &mut store,
            "cell_ids",
            MeshFrameKind::Ids,
            MeshDtype::U64LE,
            vec![1],
            99_u64.to_le_bytes().to_vec(),
        );
        retain(
            &mut store,
            "packed_mask",
            MeshFrameKind::Mask,
            MeshDtype::BoolPacked,
            vec![3],
            vec![0b0000_0101],
        );
        retain(
            &mut store,
            "byte_mask",
            MeshFrameKind::Mask,
            MeshDtype::BoolBytes,
            vec![3],
            vec![1, 0, 1],
        );

        let geometry = serde_json::json!({
            "vertex_ids": {"resource_id": "vertex_ids", "generation": 1},
            "cell_ids": {"resource_id": "cell_ids", "generation": 1}
        });
        assert_eq!(
            decode_inline_ids(&geometry, "vertex_ids", 3, &store)
                .unwrap()
                .unwrap(),
            Arc::from([10, 20, 30])
        );
        assert_eq!(
            decode_inline_ids(&geometry, "cell_ids", 1, &store)
                .unwrap()
                .unwrap(),
            Arc::from([99])
        );

        for resource_id in ["packed_mask", "byte_mask"] {
            let field = serde_json::json!({
                "values": [1.0, 2.0, 3.0],
                "valid": {"resource_id": resource_id, "generation": 1}
            });
            assert_eq!(
                decode_mesh_field(&field, &store).unwrap().1,
                Some(Arc::from([true, false, true]))
            );
        }
    }

    #[::core::prelude::v1::test]
    fn rejects_missing_and_mismatched_mesh_resources() {
        let store = MeshFrameStore::new();
        let missing = serde_json::json!({
            "vertex_ids": {"resource_id": "stale", "generation": 2}
        });
        let error = decode_inline_ids(&missing, "vertex_ids", 1, &store).unwrap_err();
        assert!(error.contains("missing geometry.vertex_ids resource"));

        let mut store = MeshFrameStore::new();
        retain(
            &mut store,
            "bad_ids",
            MeshFrameKind::Mask,
            MeshDtype::U64LE,
            vec![1],
            7_u64.to_le_bytes().to_vec(),
        );
        let wrong_kind = serde_json::json!({
            "vertex_ids": {"resource_id": "bad_ids", "generation": 1}
        });
        let error = decode_inline_ids(&wrong_kind, "vertex_ids", 1, &store).unwrap_err();
        assert!(error.contains("expected Ids"));

        retain(
            &mut store,
            "bad_mask",
            MeshFrameKind::Mask,
            MeshDtype::BoolBytes,
            vec![1, 3],
            vec![1, 0, 1],
        );
        let field = serde_json::json!({
            "values": [1.0, 2.0, 3.0],
            "valid": {"resource_id": "bad_mask", "generation": 1}
        });
        let error = decode_mesh_field(&field, &store).unwrap_err();
        assert!(error.contains("field.valid resource shape must be [value_count]"));
    }

    #[::core::prelude::v1::test]
    fn resource_backed_nan_is_masked_by_the_native_missing_value_policy() {
        let mut store = MeshFrameStore::new();
        retain(
            &mut store,
            "field-values",
            MeshFrameKind::Field,
            MeshDtype::F64LE,
            vec![3],
            [0.0_f64, f64::NAN, 2.0]
                .into_iter()
                .flat_map(f64::to_le_bytes)
                .collect(),
        );
        let field = serde_json::json!({
            "resource_id": "field-values",
            "generation": 1
        });
        let (values, valid) = decode_mesh_field(&field, &store).unwrap();
        assert!(values[1].is_nan());
        assert!(valid.is_none());

        let masked = ScalarField {
            id: "pressure".into(),
            label: "Pressure".into(),
            unit: None,
            values: Arc::from(values),
            association: ScalarAssociation::Vertex,
            valid: valid.map(Arc::from),
        }
        .mask_nan()
        .unwrap();
        assert_eq!(masked.valid.as_deref(), Some(&[true, false, true][..]));
    }

    #[::core::prelude::v1::test]
    fn resource_backed_infinity_is_rejected_even_when_nan_masking_is_requested() {
        let mut store = MeshFrameStore::new();
        retain(
            &mut store,
            "field-values",
            MeshFrameKind::Field,
            MeshDtype::F64LE,
            vec![3],
            [0.0_f64, f64::INFINITY, 2.0]
                .into_iter()
                .flat_map(f64::to_le_bytes)
                .collect(),
        );
        let field = serde_json::json!({
            "resource_id": "field-values",
            "generation": 1
        });
        let error = decode_mesh_field(&field, &store).unwrap_err();
        assert!(error.contains("field resource contains non-finite value"));
    }

    #[::core::prelude::v1::test]
    fn invalid_new_spec_selects_the_cached_last_valid_spec_without_recursion() {
        let previous = MeshPlotSpec::from_value(serde_json::json!({
            "schema_version": 1,
            "id": "last-valid",
            "revision": 1,
            "geometry": {
                "id": "mesh",
                "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                "triangles": [[0, 1, 2]]
            }
        }))
        .unwrap();
        let mut cache = GpuiMeshPlotCache::new();
        cache.upsert(previous.clone()).unwrap();

        let mut invalid_newer = previous.clone();
        invalid_newer.revision = 2;
        invalid_newer.geometry = serde_json::json!({
            "id": "mesh",
            "positions": {"resource_id": "missing", "generation": 9},
            "triangles": {"resource_id": "missing-indices", "generation": 9}
        });
        assert_eq!(
            cached_meshplot_fallback(&cache, &invalid_newer),
            Some(previous.clone())
        );
        assert!(cached_meshplot_fallback(&cache, &previous).is_none());
    }
}

fn command_domain(arguments: &Value, name: &str) -> Result<(f64, f64), String> {
    let values = arguments
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{name} must be a two-value array"))?;
    let [min, max] = values.as_slice() else {
        return Err(format!("{name} must be a two-value array"));
    };
    let min = min
        .as_f64()
        .ok_or_else(|| format!("{name} minimum must be finite"))?;
    let max = max
        .as_f64()
        .ok_or_else(|| format!("{name} maximum must be finite"))?;
    if !min.is_finite() || !max.is_finite() || min >= max {
        return Err(format!("{name} must be finite and increasing"));
    }
    Ok((min, max))
}

fn command_numbers(arguments: &Value, name: &str) -> Result<Vec<f64>, String> {
    arguments
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{name} must be an array"))?
        .iter()
        .map(|value| {
            let value = value
                .as_f64()
                .ok_or_else(|| format!("{name} values must be finite"))?;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(format!("{name} values must be finite"))
            }
        })
        .collect()
}

fn validate_chart_export_node(node: &ChartNode) -> Result<(), String> {
    if node.id.trim().is_empty() {
        return Err("chart export requires a stable chart id".into());
    }
    if !node.width.is_finite() || !node.height.is_finite() {
        return Err("chart export width and height must be finite".into());
    }
    if !(16.0..=4096.0).contains(&node.width) || !(16.0..=4096.0).contains(&node.height) {
        return Err("chart export width and height must be between 16 and 4096".into());
    }
    let finite = |field: &str, values: &[f64]| {
        if values.len() > 200_000 {
            return Err(format!(
                "chart export {field} exceeds the 200000-point limit"
            ));
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(format!("chart export {field} contains NaN or Infinity"));
        }
        Ok(())
    };
    match node.chart {
        ChartKind::Line | ChartKind::Scatter => {
            if node.series.is_empty() {
                let x = node.x.as_deref().ok_or("chart export is missing x data")?;
                let y = node.y.as_deref().ok_or("chart export is missing y data")?;
                if x.len() != y.len() {
                    return Err("chart export x and y lengths differ".into());
                }
                finite("x", x)?;
                finite("y", y)?;
            } else {
                for series in &node.series {
                    if series.id.trim().is_empty() {
                        return Err("chart export series id is empty".into());
                    }
                    if series.x.len() != series.y.len() {
                        return Err(format!(
                            "chart export series {} x and y lengths differ",
                            series.id
                        ));
                    }
                    finite("series.x", &series.x)?;
                    finite("series.y", &series.y)?;
                }
            }
        }
        _ => return Err(format!("chart export does not support {:?}", node.chart)),
    }
    Ok(())
}

fn native_chart_svg(
    node: &ChartNode,
    domains: Option<((f64, f64), (f64, f64))>,
    locally_hidden: Option<&HashSet<String>>,
) -> Result<String, String> {
    validate_chart_export_node(node)?;
    let visible_series = node
        .series
        .iter()
        .filter(|series| {
            series.visible && !locally_hidden.is_some_and(|hidden| hidden.contains(&series.id))
        })
        .collect::<Vec<_>>();
    if visible_series.is_empty() && (node.x.is_none() || node.y.is_none()) {
        return Err("chart export has no visible data series".into());
    }
    match node.chart {
        ChartKind::Line => {
            let primary = visible_series.first().copied();
            let x = primary
                .map(|series| series.x.as_slice())
                .or(node.x.as_deref())
                .unwrap_or_default();
            let y = primary
                .map(|series| series.y.as_slice())
                .or(node.y.as_deref())
                .unwrap_or_default();
            let mut chart = line(x, y)
                .title(node.title.clone())
                .color(hex_color(
                    primary
                        .and_then(|series| series.color.as_deref())
                        .or(node.color.as_deref()),
                    0xff7f0e,
                ))
                .stroke_width(
                    primary
                        .and_then(|series| series.stroke_width)
                        .unwrap_or(node.stroke_width),
                )
                .x_scale(scale_type(node.x_log))
                .y_scale(scale_type(node.y_log))
                .size(node.width, node.height)
                .curve(px_curve(&node.curve))
                .legend_position(px_legend_position(&node.legend_position))
                .annotations(px_annotations(node))
                .dash_style(&node.dash);
            if let Some(label) = &node.x_label {
                chart = chart.x_label(label.clone());
            }
            if let Some(label) = &node.y_label {
                chart = chart.y_label(label.clone());
            }
            if let Some(label) = &node.y2_label {
                chart = chart.y2_label(label.clone());
            }
            if let Some([min, max]) = node.y2_range {
                chart = chart.y2_range(min, max);
            }
            if let Some(series) = primary.filter(|series| !series.label.is_empty()) {
                chart = chart.label(series.label.clone());
            }
            for series in visible_series.iter().copied().skip(1) {
                chart = if series.secondary_y {
                    chart.add_series_y2_with_x(
                        &series.x,
                        &series.y,
                        (!series.label.is_empty()).then_some(series.label.clone()),
                        hex_color(series.color.as_deref(), 0xff7f0e),
                        series.stroke_width.unwrap_or(node.stroke_width),
                        series.opacity,
                    )
                } else {
                    chart.add_series_with_x(
                        &series.x,
                        &series.y,
                        (!series.label.is_empty()).then_some(series.label.clone()),
                        hex_color(series.color.as_deref(), 0xff7f0e),
                        series.stroke_width.unwrap_or(node.stroke_width),
                        series.opacity,
                    )
                };
                chart = chart.series_dash_style(&series.dash);
            }
            if let Some(((min, max), _)) = domains {
                chart = chart.x_range(min, max);
            } else if let Some([min, max]) = node.x_range {
                chart = chart.x_range(min, max);
            }
            if let Some((_, (min, max))) = domains {
                chart = chart.y_range(min, max);
            } else if let Some([min, max]) = node.y_range {
                chart = chart.y_range(min, max);
            }
            chart.to_svg().map_err(|error| error.to_string())
        }
        ChartKind::Scatter => {
            let primary = visible_series.first().copied();
            let x = primary
                .map(|series| series.x.as_slice())
                .or(node.x.as_deref())
                .unwrap_or_default();
            let y = primary
                .map(|series| series.y.as_slice())
                .or(node.y.as_deref())
                .unwrap_or_default();
            let mut chart = scatter(x, y)
                .title(node.title.clone())
                .color(hex_color(
                    primary
                        .and_then(|series| series.color.as_deref())
                        .or(node.color.as_deref()),
                    0x1f77b4,
                ))
                .point_radius(
                    primary
                        .and_then(|series| series.point_radius)
                        .unwrap_or(node.point_radius),
                )
                .x_scale(scale_type(node.x_log))
                .y_scale(scale_type(node.y_log))
                .legend_position(px_legend_position(&node.legend_position))
                .annotations(px_annotations(node))
                .size(node.width, node.height);
            for series in visible_series.iter().copied().skip(1) {
                chart = chart.add_series(
                    &series.x,
                    &series.y,
                    (!series.label.is_empty()).then_some(series.label.clone()),
                    hex_color(series.color.as_deref(), 0x1f77b4),
                    series.point_radius.unwrap_or(node.point_radius),
                    series.opacity,
                );
            }
            if let Some(((min, max), _)) = domains {
                chart = chart.x_range(min, max);
            } else if let Some([min, max]) = node.x_range {
                chart = chart.x_range(min, max);
            }
            if let Some((_, (min, max))) = domains {
                chart = chart.y_range(min, max);
            } else if let Some([min, max]) = node.y_range {
                chart = chart.y_range(min, max);
            }
            chart.to_svg().map_err(|error| error.to_string())
        }
        _ => unreachable!("validated chart kind"),
    }
}

#[derive(Clone, Deserialize)]
struct D3HierarchySpec {
    name: String,
    value: f64,
    #[serde(default)]
    children: Vec<D3HierarchySpec>,
}

#[derive(Clone)]
struct D3HierarchyDatum {
    name: String,
    value: f64,
}

fn d3_hierarchy_node(
    spec: D3HierarchySpec,
) -> std::rc::Rc<std::cell::RefCell<d3rs::hierarchy::HierarchyNode<D3HierarchyDatum>>> {
    let node = d3rs::hierarchy::HierarchyNode::new(D3HierarchyDatum {
        name: spec.name,
        value: spec.value,
    });
    if !spec.children.is_empty() {
        let children = spec.children.into_iter().map(d3_hierarchy_node).collect();
        node.borrow_mut().set_children(&node, children);
    }
    node
}

fn d3_algorithm_command(arguments: &Value) -> Result<Value, String> {
    let operation = arguments
        .get("operation")
        .and_then(Value::as_str)
        .ok_or_else(|| "D3 algorithm command requires operation".to_string())?;
    let number = |name: &str| {
        arguments
            .get(name)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("D3 algorithm field {name} must be finite"))
    };
    let unsigned = |name: &str| {
        arguments
            .get(name)
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("D3 algorithm field {name} must be unsigned"))
    };
    let string = |name: &str| {
        arguments
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("D3 algorithm field {name} must be a string"))
    };
    let fixed_array = |name: &str, length: usize| -> Result<Vec<f64>, String> {
        let values = command_numbers(arguments, name)?;
        if values.len() != length {
            return Err(format!(
                "D3 algorithm field {name} must contain {length} values"
            ));
        }
        Ok(values)
    };
    let points = |name: &str| -> Result<Vec<(f64, f64)>, String> {
        arguments
            .get(name)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("D3 algorithm field {name} must be a point array"))?
            .iter()
            .map(|point| {
                let point = point
                    .as_array()
                    .filter(|point| point.len() == 2)
                    .ok_or_else(|| {
                        format!("D3 algorithm field {name} points must contain two values")
                    })?;
                let x = point[0]
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| {
                        format!("D3 algorithm field {name} x coordinates must be finite")
                    })?;
                let y = point[1]
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| {
                        format!("D3 algorithm field {name} y coordinates must be finite")
                    })?;
                Ok((x, y))
            })
            .collect()
    };
    let strings = |name: &str| -> Result<Vec<String>, String> {
        arguments
            .get(name)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("D3 algorithm field {name} must be a string array"))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("D3 algorithm field {name} values must be strings"))
            })
            .collect()
    };
    let samples = || {
        arguments
            .get("count")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .min(100_000) as usize
    };
    let seed = arguments.get("seed").and_then(Value::as_u64).unwrap_or(1);

    let value = match operation {
        "color_interpolate" => {
            let start = unsigned("start")?;
            let end = unsigned("end")?;
            let color = d3rs::color::D3Color::from_hex(start as u32).interpolate(
                &d3rs::color::D3Color::from_hex(end as u32),
                number("t")? as f32,
            );
            serde_json::json!({
                "hex": color.to_hex(),
                "hex_alpha": color.to_hex_alpha(),
                "rgba": [color.r, color.g, color.b, color.a],
            })
        }
        "color_convert" => {
            let space = string("space")?;
            let components = command_numbers(arguments, "components")?;
            let color = match (space, components.as_slice()) {
                ("hex", [hex]) if *hex >= 0.0 && *hex <= u32::MAX as f64 => {
                    d3rs::color::D3Color::from_hex(*hex as u32)
                }
                ("hsl", [h, s, l]) => {
                    d3rs::color::D3Color::from_hsl(*h as f32, *s as f32, *l as f32)
                }
                ("lab", [l, a, b]) => d3rs::color::D3Color::from_lab(*l, *a, *b),
                ("hcl", [h, c, l]) => d3rs::color::D3Color::from_hcl(*h, *c, *l),
                _ => {
                    return Err(
                        "color_convert expects hex[1], hsl[3], lab[3], or hcl[3] components"
                            .to_string(),
                    );
                }
            };
            let lab = color.to_lab();
            let hcl = color.to_hcl();
            serde_json::json!({
                "hex": color.to_hex(),
                "hex_alpha": color.to_hex_alpha(),
                "rgba": [color.r, color.g, color.b, color.a],
                "lab": [lab.l, lab.a, lab.b],
                "hcl": [hcl.h, hcl.c, hcl.l],
            })
        }
        "format" => {
            let specifier = string("specifier")?;
            serde_json::json!(
                command_numbers(arguments, "values")?
                    .into_iter()
                    .map(|value| d3rs::format::format_value(specifier, value))
                    .collect::<Vec<_>>()
            )
        }
        "format_prefix" => {
            let formatter = d3rs::format::format_prefix(string("specifier")?, number("value")?);
            serde_json::json!(
                command_numbers(arguments, "values")?
                    .into_iter()
                    .map(formatter)
                    .collect::<Vec<_>>()
            )
        }
        "time_interval" => serde_json::json!(
            command_numbers(arguments, "values")?
                .into_iter()
                .map(|span| {
                    let interval = d3rs::time::TimeInterval::for_span(span.round() as i64);
                    serde_json::json!({
                        "interval": format!("{interval:?}").to_lowercase(),
                        "format": interval.format_pattern(),
                    })
                })
                .collect::<Vec<_>>()
        ),
        "time_scale" => {
            use d3rs::scale::Scale;
            let domain = fixed_array("domain", 2)?;
            let range = fixed_array("range", 2)?;
            let mut scale = d3rs::time::TimeScale::new()
                .domain(domain[0].round() as i64, domain[1].round() as i64)
                .range(range[0], range[1])
                .clamp(
                    arguments
                        .get("clamp")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                );
            if arguments
                .get("nice")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                scale = scale.nice(
                    arguments
                        .get("tick_count")
                        .and_then(Value::as_u64)
                        .map(|v| v as usize),
                );
            }
            let scaled = command_numbers(arguments, "values")?
                .into_iter()
                .map(|value| scale.scale(value.round() as i64))
                .collect::<Vec<_>>();
            let ticks = scale.time_ticks(
                arguments
                    .get("tick_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(10)
                    .max(1) as usize,
            );
            serde_json::json!({
                "values": scaled,
                "ticks": ticks,
                "interval": format!("{:?}", scale.interval()).to_lowercase(),
            })
        }
        "csv_parse" => serde_json::to_value(
            d3rs::fetch::parse_csv(string("input")?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "dsv_parse" => {
            let input = string("input")?;
            let delimiter = string("delimiter")?;
            let mut chars = delimiter.chars();
            let delimiter = chars
                .next()
                .filter(|_| chars.next().is_none())
                .ok_or_else(|| "dsv_parse delimiter must be one character".to_string())?;
            serde_json::to_value(
                d3rs::fetch::parse_dsv(input, delimiter).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?
        }
        "dsv_format" => {
            let rows: Vec<d3rs::fetch::DsvRow> = serde_json::from_value(
                arguments
                    .get("rows")
                    .cloned()
                    .ok_or_else(|| "dsv_format requires rows".to_string())?,
            )
            .map_err(|error| format!("invalid dsv_format rows: {error}"))?;
            let columns = arguments
                .get("columns")
                .and_then(Value::as_array)
                .ok_or_else(|| "dsv_format columns must be an array".to_string())?
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .ok_or_else(|| "dsv_format columns must be strings".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let delimiter = string("delimiter")?;
            match delimiter {
                "," => serde_json::json!(d3rs::fetch::format_csv(&rows, &columns)),
                "\t" => serde_json::json!(d3rs::fetch::format_tsv(&rows, &columns)),
                _ => return Err("dsv_format delimiter must be comma or tab".to_string()),
            }
        }
        "interpolate_number" => {
            let interpolate = d3rs::interpolate::interpolate(number("start")?, number("end")?);
            serde_json::json!(
                command_numbers(arguments, "values")?
                    .into_iter()
                    .map(interpolate)
                    .collect::<Vec<_>>()
            )
        }
        "interpolate_array" => {
            let interpolate = d3rs::interpolate::interpolate_number_array(
                command_numbers(arguments, "start_values")?,
                command_numbers(arguments, "end_values")?,
            );
            serde_json::json!(
                command_numbers(arguments, "values")?
                    .into_iter()
                    .map(interpolate)
                    .collect::<Vec<_>>()
            )
        }
        "interpolate_string" | "interpolate_transform_css" => {
            let start = string("start")?;
            let end = string("end")?;
            let values = command_numbers(arguments, "values")?;
            if operation == "interpolate_string" {
                let interpolate = d3rs::interpolate::interpolate_string(start, end);
                serde_json::json!(values.into_iter().map(interpolate).collect::<Vec<_>>())
            } else {
                let interpolate = d3rs::interpolate::interpolate_transform_css(start, end);
                serde_json::json!(values.into_iter().map(interpolate).collect::<Vec<_>>())
            }
        }
        "interpolate_transform_svg" => {
            let start = fixed_array("start_values", 6)?;
            let end = fixed_array("end_values", 6)?;
            let interpolate = d3rs::interpolate::interpolate_transform_svg(
                start
                    .try_into()
                    .expect("validated six-element start transform"),
                end.try_into().expect("validated six-element end transform"),
            );
            serde_json::json!(
                command_numbers(arguments, "values")?
                    .into_iter()
                    .map(interpolate)
                    .collect::<Vec<_>>()
            )
        }
        "interpolate_zoom" => {
            use d3rs::interpolate::zoom::{ZoomView, interpolate_zoom, zoom_duration};
            let start = fixed_array("start_values", 3)?;
            let end = fixed_array("end_values", 3)?;
            let start = ZoomView::new(start[0], start[1], start[2]);
            let end = ZoomView::new(end[0], end[1], end[2]);
            let interpolate = interpolate_zoom(start, end);
            let values = command_numbers(arguments, "values")?
                .into_iter()
                .map(|value| {
                    let view = interpolate(value);
                    [view.cx, view.cy, view.size]
                })
                .collect::<Vec<_>>();
            serde_json::json!({"values": values, "duration": zoom_duration(start, end)})
        }
        "ease" => {
            let kind = arguments
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("linear");
            let ease = |t: f64| -> Result<f64, String> {
                Ok(match kind {
                    "linear" => d3rs::ease::ease_linear(t),
                    "quad_in" => d3rs::ease::ease_quad_in(t),
                    "quad_out" => d3rs::ease::ease_quad_out(t),
                    "quad_in_out" => d3rs::ease::ease_quad_in_out(t),
                    "cubic_in" => d3rs::ease::ease_cubic_in(t),
                    "cubic_out" => d3rs::ease::ease_cubic_out(t),
                    "cubic_in_out" => d3rs::ease::ease_cubic_in_out(t),
                    "sin_in" => d3rs::ease::ease_sin_in(t),
                    "sin_out" => d3rs::ease::ease_sin_out(t),
                    "sin_in_out" => d3rs::ease::ease_sin_in_out(t),
                    "exp_in" => d3rs::ease::ease_exp_in(t),
                    "exp_out" => d3rs::ease::ease_exp_out(t),
                    "exp_in_out" => d3rs::ease::ease_exp_in_out(t),
                    "circle_in" => d3rs::ease::ease_circle_in(t),
                    "circle_out" => d3rs::ease::ease_circle_out(t),
                    "circle_in_out" => d3rs::ease::ease_circle_in_out(t),
                    "elastic_in" => d3rs::ease::ease_elastic_in(t),
                    "elastic_out" => d3rs::ease::ease_elastic_out(t),
                    "elastic_in_out" => d3rs::ease::ease_elastic_in_out(t),
                    "back_in" => d3rs::ease::ease_back_in(t),
                    "back_out" => d3rs::ease::ease_back_out(t),
                    "back_in_out" => d3rs::ease::ease_back_in_out(t),
                    "bounce_in" => d3rs::ease::ease_bounce_in(t),
                    "bounce_out" => d3rs::ease::ease_bounce_out(t),
                    "bounce_in_out" => d3rs::ease::ease_bounce_in_out(t),
                    _ => return Err(format!("unsupported D3 ease kind: {kind}")),
                })
            };
            serde_json::json!(
                command_numbers(arguments, "values")?
                    .into_iter()
                    .map(ease)
                    .collect::<Result<Vec<_>, _>>()?
            )
        }
        "selection_join" => {
            let keys = |name: &str| {
                arguments
                    .get(name)
                    .and_then(Value::as_array)
                    .ok_or_else(|| format!("selection_join {name} must be an array"))?
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::to_string)
                            .ok_or_else(|| format!("selection_join {name} values must be strings"))
                    })
                    .collect::<Result<Vec<_>, _>>()
            };
            let old_keys = keys("old_keys")?;
            let new_keys = keys("new_keys")?;
            let join = d3rs::selection::keyed_data_join(
                &old_keys,
                &new_keys,
                |key, _| key.clone(),
                |key, _| key.clone(),
            )
            .map_err(|error| format!("selection join failed: {error:?}"))?;
            serde_json::json!({
                "enter": join.enter().iter().map(|item| serde_json::json!({"key": item.key, "new_index": item.new_index})).collect::<Vec<_>>(),
                "update": join.update().iter().map(|item| serde_json::json!({"key": item.key, "old_index": item.old_index, "new_index": item.new_index})).collect::<Vec<_>>(),
                "exit": join.exit().iter().map(|item| serde_json::json!({"key": item.key, "old_index": item.old_index})).collect::<Vec<_>>(),
                "has_structural_changes": join.has_structural_changes(),
            })
        }
        "brush_gesture" => {
            let points = arguments
                .get("points")
                .and_then(Value::as_array)
                .ok_or_else(|| "brush_gesture points must be an array".to_string())?;
            if points.len() < 2 {
                return Err("brush_gesture requires at least two points".to_string());
            }
            let point = |value: &Value| -> Result<(f64, f64), String> {
                let values = value
                    .as_array()
                    .ok_or_else(|| "brush_gesture point must be a two-value array".to_string())?;
                if values.len() != 2 {
                    return Err("brush_gesture point must be a two-value array".to_string());
                }
                let x = values[0]
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| "brush_gesture x must be finite".to_string())?;
                let y = values[1]
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| "brush_gesture y must be finite".to_string())?;
                Ok((x, y))
            };
            let mut brush = d3rs::brush::BrushState::new();
            let start = point(&points[0])?;
            brush.start(start.0, start.1);
            for value in &points[1..] {
                let current = point(value)?;
                brush.update(current.0, current.1);
            }
            let selection = brush
                .end()
                .ok_or_else(|| "brush_gesture did not produce a selection".to_string())?;
            serde_json::json!({
                "selection": [selection.x0, selection.y0, selection.x1, selection.y1],
                "width": selection.width(),
                "height": selection.height(),
                "trivial": selection.is_trivial(arguments.get("min_size").and_then(Value::as_f64).unwrap_or(0.0)),
            })
        }
        "drag_gesture" => {
            let points = arguments
                .get("points")
                .and_then(Value::as_array)
                .ok_or_else(|| "drag_gesture points must be an array".to_string())?;
            if points.len() < 2 {
                return Err("drag_gesture requires at least two points".to_string());
            }
            let point = |value: &Value| -> Result<(f64, f64), String> {
                let values = value
                    .as_array()
                    .filter(|values| values.len() == 2)
                    .ok_or_else(|| "drag_gesture point must be a two-value array".to_string())?;
                Ok((
                    values[0]
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| "drag_gesture x must be finite".to_string())?,
                    values[1]
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| "drag_gesture y must be finite".to_string())?,
                ))
            };
            let pointer_id = arguments
                .get("pointer_id")
                .and_then(Value::as_u64)
                .unwrap_or(1);
            let config = d3rs::drag::DragConfig::default()
                .with_click_distance(
                    arguments
                        .get("click_distance")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )
                .map_err(|error| format!("{error:?}"))?;
            let mut state =
                d3rs::drag::DragState::with_config(config).map_err(|error| format!("{error:?}"))?;
            let serialize = |update: d3rs::drag::DragUpdate| {
                serde_json::json!({
                    "phase": format!("{:?}", update.phase).to_lowercase(),
                    "current": [update.current.x, update.current.y],
                    "delta": [update.delta.dx, update.delta.dy],
                    "total_delta": [update.total_delta.dx, update.total_delta.dy],
                    "distance": update.distance,
                    "exceeds_click_distance": update.exceeds_click_distance,
                })
            };
            let start = point(&points[0])?;
            let mut updates = vec![serialize(
                state
                    .start(pointer_id, start.0, start.1)
                    .map_err(|error| format!("{error:?}"))?,
            )];
            for value in &points[1..points.len() - 1] {
                let current = point(value)?;
                updates.push(serialize(
                    state
                        .drag(pointer_id, current.0, current.1)
                        .map_err(|error| format!("{error:?}"))?,
                ));
            }
            let end = point(points.last().expect("validated points"))?;
            updates.push(serialize(
                state
                    .end(pointer_id, end.0, end.1)
                    .map_err(|error| format!("{error:?}"))?,
            ));
            serde_json::json!(updates)
        }
        "transition_sample" => {
            let ease: d3rs::transition::EaseFn = match arguments
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("linear")
            {
                "linear" => d3rs::ease::ease_linear,
                "quad_in" => d3rs::ease::ease_quad_in,
                "quad_out" => d3rs::ease::ease_quad_out,
                "quad_in_out" => d3rs::ease::ease_quad_in_out,
                "cubic_in" => d3rs::ease::ease_cubic_in,
                "cubic_out" => d3rs::ease::ease_cubic_out,
                "cubic_in_out" => d3rs::ease::ease_cubic_in_out,
                "sin_in" => d3rs::ease::ease_sin_in,
                "sin_out" => d3rs::ease::ease_sin_out,
                "sin_in_out" => d3rs::ease::ease_sin_in_out,
                "exp_in" => d3rs::ease::ease_exp_in,
                "exp_out" => d3rs::ease::ease_exp_out,
                "exp_in_out" => d3rs::ease::ease_exp_in_out,
                "circle_in" => d3rs::ease::ease_circle_in,
                "circle_out" => d3rs::ease::ease_circle_out,
                "circle_in_out" => d3rs::ease::ease_circle_in_out,
                "elastic_in" => d3rs::ease::ease_elastic_in,
                "elastic_out" => d3rs::ease::ease_elastic_out,
                "elastic_in_out" => d3rs::ease::ease_elastic_in_out,
                "back_in" => d3rs::ease::ease_back_in,
                "back_out" => d3rs::ease::ease_back_out,
                "back_in_out" => d3rs::ease::ease_back_in_out,
                "bounce_in" => d3rs::ease::ease_bounce_in,
                "bounce_out" => d3rs::ease::ease_bounce_out,
                "bounce_in_out" => d3rs::ease::ease_bounce_in_out,
                kind => return Err(format!("unsupported D3 transition ease kind: {kind}")),
            };
            let mut transition = d3rs::transition::Transition::new()
                .from_to(number("start")?, number("end")?)
                .duration(number("duration_ms")?)
                .delay(
                    arguments
                        .get("delay_ms")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )
                .ease(ease);
            serde_json::json!(
                command_numbers(arguments, "delta_ms")?
                    .into_iter()
                    .map(|delta| transition.tick(delta))
                    .collect::<Vec<_>>()
            )
        }
        "contour" => {
            let width = unsigned("width")? as usize;
            let height = unsigned("height")? as usize;
            let values = command_numbers(arguments, "values")?;
            if values.len() != width.saturating_mul(height) {
                return Err("contour values must match width times height".to_string());
            }
            let contour = d3rs::contour::contour(&values, width, height, number("threshold")?);
            serde_json::json!({
                "value": contour.value,
                "rings": contour.coordinates.into_iter().map(|ring| {
                    ring.points.into_iter().map(|point| [point.x, point.y]).collect::<Vec<_>>()
                }).collect::<Vec<_>>(),
            })
        }
        "lod_m4" => {
            let x = command_numbers(arguments, "x")?;
            let y = command_numbers(arguments, "y")?;
            if x.len() != y.len() {
                return Err("lod_m4 x and y must have equal lengths".to_string());
            }
            serde_json::json!(d3rs::lod::m4_indices(
                &x,
                &y,
                number("x0")?,
                number("x1")?,
                unsigned("columns")? as usize,
            ))
        }
        "geo" => {
            let coordinates = points("coordinates")?;
            let contains = arguments
                .get("contains")
                .map(|_| {
                    points("contains").and_then(|points| {
                        let point = points
                            .first()
                            .copied()
                            .ok_or_else(|| "geo contains requires one point".to_string())?;
                        Ok(d3rs::geo::geo_contains(&coordinates, point.0, point.1))
                    })
                })
                .transpose()?;
            serde_json::json!({
                "area": d3rs::geo::geo_area(&coordinates),
                "length": d3rs::geo::geo_length(&coordinates),
                "bounds": d3rs::geo::geo_bounds(&coordinates),
                "centroid": d3rs::geo::geo_centroid(&coordinates),
                "contains": contains,
            })
        }
        "quadtree" => {
            let input = points("points")?;
            let mut tree = d3rs::quadtree::QuadTree::new();
            for (index, point) in input.iter().copied().enumerate() {
                tree.try_add(point.0, point.1, index)
                    .map_err(|error| error.to_string())?;
            }
            let matches = arguments
                .get("find")
                .map(|_| {
                    points("find").and_then(|points| {
                        let point = points
                            .first()
                            .copied()
                            .ok_or_else(|| "quadtree find requires one point".to_string())?;
                        let radius = arguments
                            .get("radius")
                            .and_then(Value::as_f64)
                            .unwrap_or(f64::INFINITY);
                        Ok(tree
                            .find_all(point.0, point.1, radius)
                            .into_iter()
                            .copied()
                            .collect::<Vec<_>>())
                    })
                })
                .transpose()?;
            serde_json::json!({"size": tree.size(), "data": tree.data(), "matches": matches})
        }
        "hierarchy_treemap" => {
            let spec: D3HierarchySpec = serde_json::from_value(
                arguments
                    .get("root")
                    .cloned()
                    .ok_or_else(|| "hierarchy_treemap requires root".to_string())?,
            )
            .map_err(|error| format!("invalid hierarchy root: {error}"))?;
            let root = d3_hierarchy_node(spec);
            d3rs::hierarchy::HierarchyNode::try_sum(root.clone(), |datum| datum.value)
                .map_err(|error| error.to_string())?;
            let size = fixed_array("size", 2)?;
            let rects = d3rs::hierarchy::TreemapLayout::new()
                .size((size[0], size[1]))
                .padding(
                    arguments
                        .get("padding")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )
                .try_layout(root)
                .map_err(|error| error.to_string())?;
            serde_json::json!(
                rects
                    .into_iter()
                    .map(|rect| {
                        let node = rect.node.borrow();
                        serde_json::json!({
                            "name": node.data.name,
                            "x0": rect.x0, "y0": rect.y0, "x1": rect.x1, "y1": rect.y1,
                            "depth": rect.depth, "value": rect.value,
                        })
                    })
                    .collect::<Vec<_>>()
            )
        }
        "force" => {
            let nodes = points("points")?
                .into_iter()
                .enumerate()
                .map(|(index, (x, y))| d3rs::force::SimulationNode::try_new(index, x, y))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            let center = fixed_array("center", 2)?;
            let many_body = d3rs::force::ForceManyBody::try_new()
                .and_then(|force| {
                    force.try_strength(
                        arguments
                            .get("strength")
                            .and_then(Value::as_f64)
                            .unwrap_or(-30.0),
                    )
                })
                .map_err(|error| error.to_string())?;
            let center_force = d3rs::force::ForceCenter::try_new(center[0], center[1])
                .map_err(|error| error.to_string())?;
            let mut simulation = d3rs::force::Simulation::try_new(nodes.clone())
                .map_err(|error| error.to_string())?
                .force(Box::new(many_body))
                .force(Box::new(center_force));
            for _ in 0..arguments
                .get("ticks")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .min(10_000)
            {
                simulation.try_tick().map_err(|error| error.to_string())?;
            }
            serde_json::json!({
                "alpha": simulation.alpha,
                "nodes": nodes.into_iter().map(|node| {
                    let node = node.borrow();
                    serde_json::json!({"index": node.index, "x": node.x, "y": node.y, "vx": node.vx, "vy": node.vy})
                }).collect::<Vec<_>>(),
            })
        }
        "chord" => {
            let matrix = arguments
                .get("matrix")
                .and_then(Value::as_array)
                .ok_or_else(|| "chord matrix must be an array".to_string())?
                .iter()
                .map(|row| {
                    row.as_array()
                        .ok_or_else(|| "chord matrix rows must be arrays".to_string())?
                        .iter()
                        .map(|value| {
                            value
                                .as_f64()
                                .filter(|value| value.is_finite())
                                .ok_or_else(|| "chord matrix values must be finite".to_string())
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            let result = d3rs::chord::ChordLayout::new()
                .pad_angle(
                    arguments
                        .get("pad_angle")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )
                .try_compute(&matrix)
                .map_err(|error| error.to_string())?;
            let subgroup = |subgroup: &d3rs::chord::ChordSubgroup| {
                serde_json::json!({
                    "index": subgroup.index,
                    "start_angle": subgroup.start_angle,
                    "end_angle": subgroup.end_angle,
                    "value": subgroup.value,
                })
            };
            serde_json::json!({
                "groups": result.groups.into_iter().map(|group| serde_json::json!({
                    "index": group.index,
                    "start_angle": group.start_angle,
                    "end_angle": group.end_angle,
                    "value": group.value,
                })).collect::<Vec<_>>(),
                "chords": result.chords.into_iter().map(|chord| serde_json::json!({
                    "source": subgroup(&chord.source),
                    "target": subgroup(&chord.target),
                })).collect::<Vec<_>>(),
            })
        }
        "sankey" => {
            let links = arguments
                .get("links")
                .and_then(Value::as_array)
                .ok_or_else(|| "sankey links must be an array".to_string())?
                .iter()
                .map(|link| {
                    Ok(d3rs::sankey::SankeyLinkInput {
                        source: link
                            .get("source")
                            .and_then(Value::as_str)
                            .ok_or_else(|| "sankey link source must be a string".to_string())?
                            .to_string(),
                        target: link
                            .get("target")
                            .and_then(Value::as_str)
                            .ok_or_else(|| "sankey link target must be a string".to_string())?
                            .to_string(),
                        value: link
                            .get("value")
                            .and_then(Value::as_f64)
                            .filter(|value| value.is_finite())
                            .ok_or_else(|| "sankey link value must be finite".to_string())?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let result = d3rs::sankey::SankeyLayout::new()
                .width(
                    arguments
                        .get("width")
                        .and_then(Value::as_f64)
                        .unwrap_or(960.0),
                )
                .height(
                    arguments
                        .get("height")
                        .and_then(Value::as_f64)
                        .unwrap_or(500.0),
                )
                .node_width(
                    arguments
                        .get("node_width")
                        .and_then(Value::as_f64)
                        .unwrap_or(24.0),
                )
                .node_padding(
                    arguments
                        .get("node_padding")
                        .and_then(Value::as_f64)
                        .unwrap_or(8.0),
                )
                .iterations(
                    arguments
                        .get("iterations")
                        .and_then(Value::as_u64)
                        .unwrap_or(6) as usize,
                )
                .try_compute(&strings("nodes")?, &links)
                .map_err(|error| error.to_string())?;
            serde_json::json!({
                "nodes": result.nodes.into_iter().map(|node| serde_json::json!({
                    "id": node.id, "index": node.index, "x0": node.x0, "x1": node.x1,
                    "y0": node.y0, "y1": node.y1, "value": node.value,
                    "depth": node.depth, "height": node.height, "layer": node.layer,
                })).collect::<Vec<_>>(),
                "links": result.links.into_iter().map(|link| serde_json::json!({
                    "source": link.source, "target": link.target, "value": link.value,
                    "y0": link.y0, "y1": link.y1, "width": link.width, "path": link.path,
                })).collect::<Vec<_>>(),
            })
        }
        "polygon" => {
            let polygon = points("points")?;
            let contains = arguments
                .get("contains")
                .map(|_| {
                    points("contains").and_then(|points| {
                        points
                            .first()
                            .copied()
                            .ok_or_else(|| "polygon contains requires one point".to_string())
                    })
                })
                .transpose()?
                .map(|point| d3rs::polygon::polygon_contains(&polygon, point));
            serde_json::json!({
                "area": d3rs::polygon::polygon_area(&polygon),
                "signed_area": d3rs::polygon::polygon_area_signed(&polygon),
                "centroid": d3rs::polygon::polygon_centroid(&polygon),
                "length": d3rs::polygon::polygon_length(&polygon),
                "hull": d3rs::polygon::polygon_hull(&polygon),
                "contains": contains,
            })
        }
        "delaunay" => {
            let triangulation = d3rs::delaunay::Delaunay::try_new(&points("points")?)
                .map_err(|error| error.to_string())?;
            let nearest = arguments
                .get("find")
                .map(|_| {
                    points("find").and_then(|points| {
                        let point = points
                            .first()
                            .copied()
                            .ok_or_else(|| "delaunay find requires one point".to_string())?;
                        triangulation
                            .try_find(point.0, point.1, None)
                            .map_err(|error| error.to_string())
                    })
                })
                .transpose()?;
            serde_json::json!({
                "triangles": triangulation.triangles().collect::<Vec<_>>(),
                "edges": triangulation.edges().collect::<Vec<_>>(),
                "hull": triangulation.hull(),
                "hull_polygon": triangulation.hull_polygon(),
                "path": triangulation.render_to_path(),
                "nearest": nearest,
            })
        }
        "hexbin" => {
            let radius = arguments
                .get("radius")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let hexbin = d3rs::hexbin::Hexbin::<(f64, f64)>::with_accessors(
                |point| point.0,
                |point| point.1,
            )
            .radius(radius);
            let bins = hexbin
                .try_bin(points("points")?)
                .map_err(|error| error.to_string())?;
            serde_json::json!({
                "bins": bins.into_iter().map(|bin| serde_json::json!({"x": bin.x, "y": bin.y, "count": bin.len(), "points": bin.points})).collect::<Vec<_>>(),
                "hexagon": hexbin.try_hexagon().map_err(|error| error.to_string())?,
            })
        }
        "tiles" => {
            let translate = fixed_array("translate", 2)?;
            let tiles = d3rs::tile::tiles_for_viewport(
                number("width")?,
                number("height")?,
                number("scale")?,
                [translate[0], translate[1]],
            )
            .map_err(|error| error.to_string())?;
            serde_json::json!({
                "zoom": tiles.zoom,
                "tile_screen_size": tiles.tile_screen_size,
                "origin": tiles.origin,
                "tiles": tiles.tiles.into_iter().map(|tile| [tile.x, tile.y, i64::from(tile.z)]).collect::<Vec<_>>(),
            })
        }
        "random_uniform" => {
            let distribution =
                d3rs::random::RandomUniform::with_seed(number("min")?, number("max")?, seed);
            serde_json::json!(
                (0..samples())
                    .map(|_| distribution.sample())
                    .collect::<Vec<_>>()
            )
        }
        "random" => {
            let kind = string("kind")?;
            let count = samples();
            match kind {
                "uniform" => {
                    let distribution = d3rs::random::RandomUniform::with_seed(
                        number("min")?,
                        number("max")?,
                        seed,
                    );
                    serde_json::json!(
                        (0..count)
                            .map(|_| distribution.sample())
                            .collect::<Vec<_>>()
                    )
                }
                "normal" => {
                    let distribution = d3rs::random::RandomNormal::with_seed(
                        number("mean")?,
                        number("deviation")?,
                        seed,
                    );
                    serde_json::json!(
                        (0..count)
                            .map(|_| distribution.sample())
                            .collect::<Vec<_>>()
                    )
                }
                "log_normal" => {
                    let distribution = d3rs::random::RandomLogNormal::with_seed(
                        number("mean")?,
                        number("deviation")?,
                        seed,
                    );
                    serde_json::json!(
                        (0..count)
                            .map(|_| distribution.sample())
                            .collect::<Vec<_>>()
                    )
                }
                "exponential" => {
                    let distribution =
                        d3rs::random::RandomExponential::with_seed(number("lambda")?, seed);
                    serde_json::json!(
                        (0..count)
                            .map(|_| distribution.sample())
                            .collect::<Vec<_>>()
                    )
                }
                "bernoulli" => {
                    let distribution =
                        d3rs::random::RandomBernoulli::with_seed(number("probability")?, seed);
                    serde_json::json!(
                        (0..count)
                            .map(|_| distribution.sample())
                            .collect::<Vec<_>>()
                    )
                }
                "poisson" => {
                    let distribution =
                        d3rs::random::RandomPoisson::with_seed(number("lambda")?, seed);
                    serde_json::json!(
                        (0..count)
                            .map(|_| distribution.sample())
                            .collect::<Vec<_>>()
                    )
                }
                "irwin_hall" => {
                    let distribution = d3rs::random::RandomIrwinHall::with_seed(
                        unsigned("summands")? as usize,
                        seed,
                    );
                    serde_json::json!(
                        (0..count)
                            .map(|_| distribution.sample())
                            .collect::<Vec<_>>()
                    )
                }
                "bates" => {
                    let summands = unsigned("summands")? as usize;
                    if summands == 0 {
                        return Err("random bates requires at least one summand".to_string());
                    }
                    let distribution = d3rs::random::RandomBates::with_seed(summands, seed);
                    serde_json::json!(
                        (0..count)
                            .map(|_| distribution.sample())
                            .collect::<Vec<_>>()
                    )
                }
                _ => return Err(format!("unsupported D3 random kind: {kind}")),
            }
        }
        "shuffle" => {
            let values = arguments
                .get("values")
                .and_then(Value::as_array)
                .ok_or_else(|| "shuffle values must be an array".to_string())?;
            serde_json::json!(d3rs::random::shuffle(
                &d3rs::random::LcgRng::new(seed),
                values
            ))
        }
        _ => return Err(format!("unsupported D3 algorithm operation: {operation}")),
    };
    Ok(serde_json::json!({"ok": true, "operation": operation, "value": value}))
}

fn d3_module_catalog() -> Value {
    let groups: &[(&[&str], &str, &str, &str)] = &[
        (
            &[
                "array",
                "scale",
                "color",
                "format",
                "time",
                "fetch",
                "interpolate",
                "ease",
                "random",
                "brush",
                "chord",
                "contour",
                "delaunay",
                "drag",
                "force",
                "geo",
                "hexbin",
                "hierarchy",
                "lod",
                "polygon",
                "quadtree",
                "sankey",
                "selection",
                "tile",
                "transition",
            ],
            "direct_command",
            "gpui_toolkit.d3.AlgorithmRequest",
            "executable renderer-independent native Rust command",
        ),
        (
            &["axis", "grid", "legend", "text", "text_layout", "shape"],
            "chart_spec",
            "gpui_toolkit.charts",
            "host-native retained chart geometry specification",
        ),
        (
            &["zoom"],
            "host_interaction",
            "gpui_toolkit.d3.ZoomRequest",
            "native zoom state plus retained GPUI chart interaction",
        ),
        (
            &["dispatch"],
            "host_interaction",
            "gpui_toolkit.events",
            "typed host event dispatch and action correlation",
        ),
        (
            &["timer"],
            "host_interaction",
            "gpui_toolkit.app.AppContext",
            "host-owned task and frame scheduling lifecycle",
        ),
        (
            &["surface", "gpu2d", "gpu3d", "sphere_gallery"],
            "scene_spec",
            "gpui_toolkit.scene3d",
            "feature-gated native GPU scene specification",
        ),
        (
            &["feature_parity"],
            "direct_command",
            "gpui_toolkit.d3.request_reports",
            "native parity and benchmark report command",
        ),
        (
            &["examples"],
            "non_consumer",
            "",
            "Rust showcase fixtures; consumer behavior is exposed by chart and scene specifications",
        ),
        (
            &["prelude"],
            "non_consumer",
            "",
            "Rust-only convenience re-exports with no distinct capability",
        ),
    ];
    let modules = groups
        .iter()
        .flat_map(|(modules, bridge, python_path, evidence)| {
            modules.iter().map(move |module| {
                serde_json::json!({
                    "module": module,
                    "bridge": bridge,
                    "python_path": python_path,
                    "evidence": evidence,
                })
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({"ok": true, "modules": modules})
}

#[cfg(test)]
mod d3_algorithm_command_tests {
    use super::{d3_algorithm_command, d3_module_catalog};
    use serde_json::{Value, json};

    fn assert_succeeds(arguments: Value) -> Value {
        let result = d3_algorithm_command(&arguments).expect("native D3 algorithm should succeed");
        assert_eq!(result["ok"], true);
        result["value"].clone()
    }

    #[::core::prelude::v1::test]
    fn covers_color_format_time_fetch_and_interpolation_operations() {
        let requests = [
            json!({"operation": "color_interpolate", "start": 0xff0000_u64, "end": 0x0000ff_u64, "t": 0.5}),
            json!({"operation": "color_convert", "space": "lab", "components": [50.0, 10.0, -5.0]}),
            json!({"operation": "format", "specifier": ".2f", "values": [1.25]}),
            json!({"operation": "format_prefix", "specifier": ".1", "value": 1000.0, "values": [1000.0]}),
            json!({"operation": "time_interval", "values": [1.0, 3600.0, 86400.0]}),
            json!({"operation": "time_scale", "domain": [0.0, 86400.0], "range": [0.0, 100.0], "values": [0.0, 43200.0, 86400.0], "tick_count": 4}),
            json!({"operation": "csv_parse", "input": "name,value\na,1"}),
            json!({"operation": "dsv_parse", "delimiter": "\t", "input": "name\tvalue\na\t1"}),
            json!({"operation": "dsv_format", "delimiter": ",", "columns": ["name", "value"], "rows": [{"name": "a", "value": "1"}]}),
            json!({"operation": "interpolate_number", "start": 0.0, "end": 10.0, "values": [0.5]}),
            json!({"operation": "interpolate_array", "start_values": [0.0, 10.0], "end_values": [10.0, 20.0], "values": [0.5]}),
            json!({"operation": "interpolate_string", "start": "0px", "end": "10px", "values": [0.5]}),
            json!({"operation": "interpolate_transform_css", "start": "translate(0px, 0px)", "end": "translate(10px, 20px)", "values": [0.5]}),
            json!({"operation": "interpolate_transform_svg", "start_values": [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], "end_values": [1.0, 0.0, 0.0, 1.0, 10.0, 20.0], "values": [0.5]}),
            json!({"operation": "interpolate_zoom", "start_values": [0.0, 0.0, 100.0], "end_values": [50.0, 50.0, 10.0], "values": [0.0, 0.5, 1.0]}),
            json!({"operation": "selection_join", "old_keys": ["a", "b"], "new_keys": ["b", "c"]}),
            json!({"operation": "brush_gesture", "points": [[10.0, 20.0], [30.0, 40.0]], "min_size": 5.0}),
            json!({"operation": "drag_gesture", "points": [[0.0, 0.0], [3.0, 4.0], [6.0, 8.0]], "click_distance": 4.0}),
            json!({"operation": "transition_sample", "start": 0.0, "end": 10.0, "duration_ms": 100.0, "delay_ms": 25.0, "delta_ms": [25.0, 25.0, 50.0], "kind": "cubic_in_out"}),
            json!({"operation": "polygon", "points": [[0.0, 0.0], [2.0, 0.0], [0.0, 2.0]], "contains": [[0.5, 0.5]]}),
            json!({"operation": "delaunay", "points": [[0.0, 0.0], [2.0, 0.0], [0.0, 2.0], [2.0, 2.0]], "find": [[1.8, 1.9]]}),
            json!({"operation": "hexbin", "points": [[0.0, 0.0], [1.0, 1.0], [20.0, 20.0]], "radius": 10.0}),
            json!({"operation": "tiles", "width": 800.0, "height": 600.0, "scale": 256.0, "translate": [400.0, 300.0]}),
            json!({"operation": "chord", "matrix": [[0.0, 2.0], [1.0, 0.0]], "pad_angle": 0.02}),
            json!({"operation": "sankey", "nodes": ["a", "b", "c"], "links": [{"source": "a", "target": "b", "value": 2.0}, {"source": "b", "target": "c", "value": 1.0}], "width": 640.0, "height": 400.0}),
            json!({"operation": "force", "points": [[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]], "center": [0.0, 0.0], "strength": -10.0, "ticks": 4}),
            json!({"operation": "hierarchy_treemap", "root": {"name": "root", "value": 0.0, "children": [{"name": "a", "value": 2.0}, {"name": "b", "value": 3.0}]}, "size": [640.0, 480.0], "padding": 2.0}),
            json!({"operation": "geo", "coordinates": [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 0.0]], "contains": [[5.0, 2.0]]}),
            json!({"operation": "quadtree", "points": [[0.0, 0.0], [5.0, 5.0], [10.0, 10.0]], "find": [[6.0, 6.0]], "radius": 3.0}),
            json!({"operation": "contour", "values": [0.0, 1.0, 1.0, 0.0], "width": 2, "height": 2, "threshold": 0.5}),
            json!({"operation": "lod_m4", "x": [0.0, 1.0, 2.0, 3.0], "y": [0.0, 2.0, 1.0, 3.0], "x0": 0.0, "x1": 3.0, "columns": 2}),
        ];
        for request in requests {
            assert_succeeds(request);
        }
    }

    #[::core::prelude::v1::test]
    fn covers_every_easing_and_seeded_random_variant() {
        let easing = [
            "linear",
            "quad_in",
            "quad_out",
            "quad_in_out",
            "cubic_in",
            "cubic_out",
            "cubic_in_out",
            "sin_in",
            "sin_out",
            "sin_in_out",
            "exp_in",
            "exp_out",
            "exp_in_out",
            "circle_in",
            "circle_out",
            "circle_in_out",
            "elastic_in",
            "elastic_out",
            "elastic_in_out",
            "back_in",
            "back_out",
            "back_in_out",
            "bounce_in",
            "bounce_out",
            "bounce_in_out",
        ];
        for kind in easing {
            assert_succeeds(json!({"operation": "ease", "kind": kind, "values": [0.0, 0.5, 1.0]}));
        }

        let distributions = [
            json!({"kind": "uniform", "min": 0.0, "max": 1.0}),
            json!({"kind": "normal", "mean": 0.0, "deviation": 1.0}),
            json!({"kind": "log_normal", "mean": 0.0, "deviation": 1.0}),
            json!({"kind": "exponential", "lambda": 2.0}),
            json!({"kind": "bernoulli", "probability": 0.5}),
            json!({"kind": "poisson", "lambda": 2.0}),
            json!({"kind": "irwin_hall", "summands": 4}),
            json!({"kind": "bates", "summands": 4}),
        ];
        for distribution in distributions {
            let mut request = distribution.as_object().expect("object").clone();
            request.insert("operation".into(), json!("random"));
            request.insert("seed".into(), json!(7));
            request.insert("count".into(), json!(4));
            assert_eq!(
                assert_succeeds(Value::Object(request))
                    .as_array()
                    .unwrap()
                    .len(),
                4
            );
        }
        assert_succeeds(
            json!({"operation": "random_uniform", "min": 0.0, "max": 1.0, "seed": 7, "count": 2}),
        );
        assert_succeeds(json!({"operation": "shuffle", "values": [1, 2, 3, 4], "seed": 7}));
    }

    #[::core::prelude::v1::test]
    fn rejects_unknown_variants_instead_of_silently_falling_back() {
        assert!(
            d3_algorithm_command(&json!({"operation": "ease", "kind": "mystery", "values": [0.5]}))
                .is_err()
        );
        assert!(
            d3_algorithm_command(&json!({"operation": "random", "kind": "mystery", "count": 1}))
                .is_err()
        );
        assert!(
            d3_algorithm_command(
                &json!({"operation": "dsv_parse", "delimiter": "::", "input": "a"})
            )
            .is_err()
        );
    }

    #[::core::prelude::v1::test]
    fn module_catalog_dispositions_match_every_public_d3_module() {
        let catalog = d3_module_catalog();
        let mut actual = catalog["modules"]
            .as_array()
            .expect("module catalog array")
            .iter()
            .map(|entry| entry["module"].as_str().unwrap())
            .collect::<Vec<_>>();
        actual.sort_unstable();
        let mut expected = vec![
            "array",
            "axis",
            "brush",
            "chord",
            "color",
            "contour",
            "delaunay",
            "dispatch",
            "drag",
            "ease",
            "examples",
            "feature_parity",
            "fetch",
            "force",
            "format",
            "geo",
            "gpu2d",
            "gpu3d",
            "grid",
            "hexbin",
            "hierarchy",
            "interpolate",
            "legend",
            "lod",
            "polygon",
            "prelude",
            "quadtree",
            "random",
            "sankey",
            "scale",
            "selection",
            "shape",
            "sphere_gallery",
            "surface",
            "text",
            "text_layout",
            "tile",
            "time",
            "timer",
            "transition",
            "zoom",
        ];
        expected.sort_unstable();
        assert_eq!(actual, expected);
        assert!(catalog["modules"].as_array().unwrap().iter().all(|entry| {
            entry["bridge"] == "non_consumer"
                || entry["python_path"]
                    .as_str()
                    .is_some_and(|path| !path.is_empty())
        }));
    }
}

struct FixedTextMeasure(f64);

impl gpui_pretext::TextMeasure for FixedTextMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * self.0
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum BuilderSizingSpec {
    Fixed {
        initial: f32,
    },
    Fractional {
        initial: f32,
        min: f32,
        max: Option<f32>,
    },
    Flex {
        min: f32,
        weight: f32,
    },
    Text {
        text: String,
        line_height: f32,
        min: f32,
    },
}

#[derive(Debug, Deserialize)]
struct BuilderDisplayTierSpec {
    name: String,
    min_size: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BuilderAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum BuilderLayoutSpec {
    Slot {
        id: String,
        sizing: BuilderSizingSpec,
        #[serde(default = "builder_default_priority")]
        priority: f32,
        #[serde(default)]
        collapsible: bool,
        #[serde(default)]
        display_tiers: Vec<BuilderDisplayTierSpec>,
        collapse_label: Option<String>,
    },
    Container {
        id: String,
        axis: BuilderAxis,
        sizing: BuilderSizingSpec,
        #[serde(default)]
        children: Vec<BuilderLayoutSpec>,
        auto_axis: Option<f32>,
        #[serde(default)]
        divider_size: f32,
    },
}

#[derive(Debug, Deserialize)]
struct BuilderRatioPreference {
    id: String,
    axis: String,
    ratio: f32,
}

#[derive(Debug, Deserialize)]
struct BuilderCollapsePreference {
    id: String,
    collapsed: bool,
}

#[derive(Debug, Deserialize)]
struct BuilderAccessibilitySpec {
    id: String,
    role: Option<String>,
    label: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BuilderViewportSpec {
    label: String,
    width: f32,
    height: f32,
}

fn builder_default_priority() -> f32 {
    1.0
}

fn builder_axis(value: BuilderAxis) -> gpui_builder::Axis {
    match value {
        BuilderAxis::Horizontal => gpui_builder::Axis::Horizontal,
        BuilderAxis::Vertical => gpui_builder::Axis::Vertical,
    }
}

fn builder_sizing<'a>(
    spec: &'a BuilderSizingSpec,
    measure: &'a FixedTextMeasure,
) -> gpui_builder::Sizing<'a> {
    match spec {
        BuilderSizingSpec::Fixed { initial } => gpui_builder::Sizing::Fixed(*initial),
        BuilderSizingSpec::Fractional { initial, min, max } => gpui_builder::Sizing::Fractional {
            initial: *initial,
            min: *min,
            max: max.unwrap_or(f32::INFINITY),
        },
        BuilderSizingSpec::Flex { min, weight } => gpui_builder::Sizing::Flex {
            min: *min,
            weight: *weight,
        },
        BuilderSizingSpec::Text {
            text,
            line_height,
            min,
        } => gpui_builder::Sizing::Text {
            text,
            measure,
            line_height: *line_height,
            min: *min,
        },
    }
}

fn with_builder_node<R>(
    spec: &BuilderLayoutSpec,
    measure: &FixedTextMeasure,
    callback: Box<dyn for<'a> FnOnce(gpui_builder::LayoutNode<'a>) -> R + '_>,
) -> R {
    match spec {
        BuilderLayoutSpec::Slot {
            id,
            sizing,
            priority,
            collapsible,
            display_tiers,
            collapse_label,
        } => {
            let tiers = display_tiers
                .iter()
                .map(|tier| gpui_builder::DisplayTier {
                    name: tier.name.as_str(),
                    min_size: tier.min_size,
                })
                .collect::<Vec<_>>();
            callback(gpui_builder::LayoutNode::Slot(gpui_builder::SlotNode {
                id,
                sizing: builder_sizing(sizing, measure),
                priority: *priority,
                collapsible: *collapsible,
                display_tiers: &tiers,
                collapse_label: collapse_label.as_deref(),
            }))
        }
        BuilderLayoutSpec::Container {
            id,
            axis,
            sizing,
            children,
            auto_axis,
            divider_size,
        } => {
            let axis = builder_axis(*axis);
            with_builder_nodes(
                children,
                measure,
                Box::new(|children| {
                    callback(gpui_builder::LayoutNode::Container(
                        gpui_builder::ContainerNode {
                            id,
                            axis,
                            auto_axis: *auto_axis,
                            sizing: builder_sizing(sizing, measure),
                            children,
                            divider_size: *divider_size,
                        },
                    ))
                }),
            )
        }
    }
}

fn with_builder_nodes<R>(
    specs: &[BuilderLayoutSpec],
    measure: &FixedTextMeasure,
    callback: Box<dyn for<'a> FnOnce(&'a [gpui_builder::LayoutNode<'a>]) -> R + '_>,
) -> R {
    fn next<R>(
        remaining: &[BuilderLayoutSpec],
        measure: &FixedTextMeasure,
        built: Vec<gpui_builder::LayoutNode<'_>>,
        callback: Box<dyn for<'a> FnOnce(&'a [gpui_builder::LayoutNode<'a>]) -> R + '_>,
    ) -> R {
        match remaining.split_first() {
            None => callback(&built),
            Some((head, tail)) => with_builder_node(
                head,
                measure,
                Box::new(|node| {
                    let mut built = built;
                    built.push(node);
                    next(tail, measure, built, callback)
                }),
            ),
        }
    }

    next(specs, measure, Vec::with_capacity(specs.len()), callback)
}

fn builder_solved_node(node: &gpui_builder::SolvedNode<'_>) -> Value {
    serde_json::json!({
        "id": node.id,
        "width": node.width,
        "height": node.height,
        "visible": node.visible,
        "active_tier": node.active_tier,
        "collapse_label": node.collapse_label,
        "resolved_axis": node.resolved_axis.map(|axis| match axis {
            gpui_builder::Axis::Horizontal => "horizontal",
            gpui_builder::Axis::Vertical => "vertical",
        }),
        "children": node.children.iter().map(builder_solved_node).collect::<Vec<_>>(),
    })
}

fn builder_issue_kind(kind: &gpui_builder::LayoutIssueKind) -> &'static str {
    use gpui_builder::LayoutIssueKind;
    match kind {
        LayoutIssueKind::EmptyId => "empty_id",
        LayoutIssueKind::DuplicateId { .. } => "duplicate_id",
        LayoutIssueKind::InvalidSizing => "invalid_sizing",
        LayoutIssueKind::InvalidAutoAxis => "invalid_auto_axis",
        LayoutIssueKind::InvalidDividerSize => "invalid_divider_size",
        LayoutIssueKind::InvalidPriority => "invalid_priority",
        LayoutIssueKind::PriorityOutOfRange => "priority_out_of_range",
        LayoutIssueKind::MissingCollapseLabel => "missing_collapse_label",
        LayoutIssueKind::EmptyCollapseLabel => "empty_collapse_label",
        LayoutIssueKind::InvalidDisplayTier => "invalid_display_tier",
        LayoutIssueKind::DuplicateDisplayTierName { .. } => "duplicate_display_tier_name",
        LayoutIssueKind::DuplicateDisplayTierThreshold { .. } => "duplicate_display_tier_threshold",
        LayoutIssueKind::DisplayTiersNotDescending => "display_tiers_not_descending",
        LayoutIssueKind::EmptyContainer => "empty_container",
    }
}

fn builder_accessibility_role(
    role: Option<&str>,
) -> Result<Option<gpui_builder::AccessibilityRole>, String> {
    match role {
        None => Ok(None),
        Some("none") => Ok(Some(gpui_builder::AccessibilityRole::None)),
        Some("group") => Ok(Some(gpui_builder::AccessibilityRole::Group)),
        Some("region") => Ok(Some(gpui_builder::AccessibilityRole::Region)),
        Some("tab") => Ok(Some(gpui_builder::AccessibilityRole::Tab)),
        Some(value) => Err(format!("unsupported builder accessibility role: {value}")),
    }
}

fn builder_accessibility_node(node: &gpui_builder::AccessibilityNode) -> Value {
    serde_json::json!({
        "id": node.id,
        "role": match node.role {
            gpui_builder::AccessibilityRole::None => "none",
            gpui_builder::AccessibilityRole::Group => "group",
            gpui_builder::AccessibilityRole::Region => "region",
            gpui_builder::AccessibilityRole::Tab => "tab",
        },
        "label": node.label,
        "description": node.description,
        "visible": node.visible,
        "collapsed": node.collapsed,
        "active_tier": node.active_tier,
        "children": node.children.iter().map(builder_accessibility_node).collect::<Vec<_>>(),
    })
}

fn builder_solved_ref(node: gpui_builder::SolvedNodeRef<'_, '_>) -> Value {
    serde_json::json!({
        "id": node.id(),
        "width": node.width(),
        "height": node.height(),
        "visible": node.visible(),
        "active_tier": node.active_tier(),
        "collapse_label": node.collapse_label(),
        "resolved_axis": node.resolved_axis().map(|axis| match axis {
            gpui_builder::Axis::Horizontal => "horizontal",
            gpui_builder::Axis::Vertical => "vertical",
        }),
        "children": node.children().map(builder_solved_ref).collect::<Vec<_>>(),
    })
}

fn builder_chassis_row(value: &Value) -> Result<gpui_builder::RowSpec, String> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "builder chassis row requires kind".to_string())?;
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "builder chassis row requires id".to_string())?
        .to_string();
    let label = || {
        value
            .get("label")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("builder chassis row {id} requires label"))
            .map(str::to_string)
    };
    match kind {
        "knob_row" => {
            let knobs = value
                .get("knobs")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("builder knob row {id} requires knobs"))?
                .iter()
                .map(|knob| {
                    let knob_id = knob
                        .get("id")
                        .and_then(Value::as_str)
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| format!("builder knob row {id} has a knob without id"))?;
                    let param_idx = knob
                        .get("param_idx")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| format!("builder knob {knob_id} requires param_idx"))?
                        as usize;
                    let label = knob
                        .get("label")
                        .and_then(Value::as_str)
                        .ok_or_else(|| format!("builder knob {knob_id} requires label"))?;
                    let size = match knob.get("size").and_then(Value::as_str).unwrap_or("md") {
                        "xs" => gpui_builder::KnobSize::Xs,
                        "sm" => gpui_builder::KnobSize::Sm,
                        "md" => gpui_builder::KnobSize::Md,
                        other => {
                            return Err(format!("unsupported builder knob size: {other}"));
                        }
                    };
                    Ok(gpui_builder::KnobSlot {
                        id: knob_id.to_string(),
                        param_idx,
                        label: label.to_string(),
                        size,
                        bipolar: knob
                            .get("bipolar")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(gpui_builder::RowSpec::KnobRow { id, knobs })
        }
        "band_toggle" => {
            let label = label()?;
            Ok(gpui_builder::RowSpec::BandToggle {
                id,
                label,
                has_toggle: value
                    .get("has_toggle")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            })
        }
        "readout_tile" => {
            let label = label()?;
            Ok(gpui_builder::RowSpec::ReadoutTile { id, label })
        }
        "toggle_group" => {
            let label = label()?;
            Ok(gpui_builder::RowSpec::ToggleGroup { id, label })
        }
        other => Err(format!("unsupported builder chassis row: {other}")),
    }
}

fn native_treemap_node(node: &ChartTreemapNode) -> gpui_px::TreemapNode {
    if node.children.is_empty() {
        gpui_px::TreemapNode::new(node.name.clone(), node.value)
    } else {
        let mut root = gpui_px::TreemapNode::new(node.name.clone(), node.value);
        for child in &node.children {
            root = root.add_child(native_treemap_node(child));
        }
        root
    }
}

fn path_event_payload(
    path: &Path,
    mode: &str,
    filters: &[gpui_python_runtime::ui_ir::PathFilter],
    must_exist: bool,
    source: &str,
) -> Value {
    let mut error = None;
    if must_exist && mode != "save_file" && !path.exists() {
        error = Some("path does not exist".to_string());
    } else if mode == "open_file" && !path.is_file() {
        error = Some("path is not a file".to_string());
    } else if mode == "directory" && !path.is_dir() {
        error = Some("path is not a directory".to_string());
    } else if mode == "open_file" && !filters.is_empty() {
        let extension = path.extension().and_then(|extension| extension.to_str());
        let matches_filter = extension.is_some_and(|extension| {
            filters.iter().any(|filter| {
                filter.extensions.iter().any(|allowed| {
                    allowed
                        .trim_start_matches('.')
                        .eq_ignore_ascii_case(extension)
                })
            })
        });
        if !matches_filter {
            error = Some("selected file does not match the declared file filters".to_string());
        }
    }

    serde_json::json!({
        "value": path.to_string_lossy(),
        "source": source,
        "valid": error.is_none(),
        "error": error,
    })
}

trait InteractiveOrbitState {
    fn begin_orbit(&mut self, position: Point<Pixels>);
    fn begin_pan(&mut self, position: Point<Pixels>);
    fn move_camera(&mut self, position: Point<Pixels>, orbit: bool, pan: bool) -> bool;
    fn end_orbit(&mut self);
    fn zoom_camera(&mut self, delta: f32);
    fn reset_camera(&mut self);
}

macro_rules! impl_interactive_orbit_state {
    ($type:ty) => {
        impl InteractiveOrbitState for $type {
            fn begin_orbit(&mut self, position: Point<Pixels>) {
                self.dragging = true;
                self.last_mouse = Some(position);
            }
            fn begin_pan(&mut self, position: Point<Pixels>) {
                self.panning = true;
                self.last_mouse = Some(position);
            }
            fn move_camera(&mut self, position: Point<Pixels>, orbit: bool, pan: bool) -> bool {
                let Some(previous) = self.last_mouse else {
                    return false;
                };
                let dx = (position.x - previous.x).as_f32();
                let dy = (position.y - previous.y).as_f32();
                if self.dragging && orbit {
                    self.controls.rotate(dx, dy);
                } else if self.panning && pan {
                    let camera = self.camera.clone();
                    self.controls.pan(dx, dy, &camera);
                } else {
                    return false;
                }
                self.update_camera();
                self.last_mouse = Some(position);
                true
            }
            fn end_orbit(&mut self) {
                self.dragging = false;
                self.panning = false;
                self.last_mouse = None;
            }
            fn zoom_camera(&mut self, delta: f32) {
                self.controls.zoom(delta);
                self.update_camera();
            }
            fn reset_camera(&mut self) {
                self.controls.reset();
                self.update_camera();
            }
        }
    };
}

impl_interactive_orbit_state!(Lines3DState);
impl_interactive_orbit_state!(Surface3DState);

fn interactive_3d_view<S: InteractiveOrbitState + 'static>(
    id: &str,
    element: impl IntoElement,
    state: Rc<RefCell<S>>,
    interactions: &[gpui_python_runtime::InteractionMode],
    theme: &Theme,
    ds: &DesignSystem,
) -> AnyElement {
    use gpui_python_runtime::InteractionMode;
    let orbit = interactions.is_empty() || interactions.contains(&InteractionMode::Orbit);
    let pan = interactions.is_empty() || interactions.contains(&InteractionMode::Pan);
    let zoom = interactions.is_empty() || interactions.contains(&InteractionMode::Zoom);
    let reset = interactions.is_empty() || interactions.contains(&InteractionMode::Reset);
    let left_state = state.clone();
    let middle_state = state.clone();
    let move_state = state.clone();
    let up_state = state.clone();
    let wheel_state = state.clone();
    let reset_state = state.clone();
    let mut viewport = div()
        .id(stable_element_id(format_args!(
            "python-scene-controls-{id}"
        )))
        .size_full()
        .relative()
        .cursor_pointer()
        .child(element)
        .on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
            if orbit {
                left_state.borrow_mut().begin_orbit(event.position);
            }
        })
        .on_mouse_down(MouseButton::Middle, move |event, _window, _cx| {
            if pan {
                middle_state.borrow_mut().begin_pan(event.position);
            }
        })
        .on_mouse_move(move |event, window, _cx| {
            if move_state
                .borrow_mut()
                .move_camera(event.position, orbit, pan)
            {
                window.refresh();
            }
        })
        .on_mouse_up(MouseButton::Left, move |_event, _window, _cx| {
            up_state.borrow_mut().end_orbit();
        })
        .on_scroll_wheel(move |event, window, _cx| {
            if zoom {
                wheel_state
                    .borrow_mut()
                    .zoom_camera(event.delta.pixel_delta(window.line_height()).y.as_f32() * 0.01);
                window.refresh();
            }
        });
    if reset {
        viewport = viewport.child(
            div()
                .id(stable_element_id(format_args!("python-scene-reset-{id}")))
                .absolute()
                .right(px(ds.spacing.grid_unit))
                .top(px(ds.spacing.grid_unit))
                .px(px(ds.spacing.grid_unit))
                .py(px(ds.spacing.grid_unit / 2.0))
                .rounded(px(ds.corners.sm))
                .bg(theme.surface_hover)
                .text_color(theme.text_primary)
                .text_size(px(ds.typography.small_size))
                .cursor_pointer()
                .child("Reset / fit")
                .on_click(move |_, window, _| {
                    reset_state.borrow_mut().reset_camera();
                    window.refresh();
                }),
        );
    }
    viewport.into_any_element()
}

fn scalar_colorbar(
    label: Option<&str>,
    range: (f64, f64),
    theme: &Theme,
    ds: &DesignSystem,
) -> AnyElement {
    let colors = [0x440154, 0x3b528b, 0x21918c, 0x5ec962, 0xfde725];
    div()
        .w(px(64.0))
        .flex()
        .flex_col()
        .gap(px(ds.spacing.grid_unit / 2.0))
        .text_size(px(ds.typography.small_size))
        .text_color(theme.text_muted)
        .child(label.unwrap_or("Scalar").to_string())
        .child(
            div().h(px(120.0)).flex().flex_col().children(
                colors
                    .into_iter()
                    .rev()
                    .map(|color| div().flex_1().bg(rgb(color))),
            ),
        )
        .child(format!("{:.4}", range.1))
        .child(format!("{:.4}", range.0))
        .into_any_element()
}

fn chart_domain(values: impl Iterator<Item = f64>, fallback: (f64, f64), log: bool) -> (f64, f64) {
    let values = values.filter(|value| value.is_finite() && (!log || *value > 0.0));
    let (mut minimum, mut maximum) = values
        .fold((f64::INFINITY, f64::NEG_INFINITY), |range, value| {
            (range.0.min(value), range.1.max(value))
        });
    if !minimum.is_finite() || !maximum.is_finite() {
        return fallback;
    }
    if minimum == maximum {
        let padding = if log {
            minimum.abs().max(1.0) * 0.1
        } else {
            minimum.abs().max(1.0) * 0.05
        };
        minimum = (minimum - padding).max(if log {
            f64::MIN_POSITIVE
        } else {
            f64::NEG_INFINITY
        });
        maximum += padding;
    }
    (minimum, maximum)
}

fn cartesian_chart_domains(node: &ChartNode) -> ((f64, f64), (f64, f64)) {
    let series = if node.series.is_empty() {
        vec![(
            node.x.as_deref().unwrap_or_default(),
            node.y.as_deref().unwrap_or_default(),
        )]
    } else {
        node.series
            .iter()
            .filter(|series| series.visible)
            .map(|series| (series.x.as_slice(), series.y.as_slice()))
            .collect()
    };
    let x_fallback = node
        .x_range
        .map(|range| (range[0], range[1]))
        .unwrap_or((0.0, 1.0));
    let y_fallback = node
        .y_range
        .map(|range| (range[0], range[1]))
        .unwrap_or((0.0, 1.0));
    let x = node
        .x_range
        .map(|range| (range[0], range[1]))
        .unwrap_or_else(|| {
            chart_domain(
                series.iter().flat_map(|(x, _)| x.iter().copied()),
                x_fallback,
                node.x_log,
            )
        });
    let y = node
        .y_range
        .map(|range| (range[0], range[1]))
        .unwrap_or_else(|| {
            chart_domain(
                series.iter().flat_map(|(_, y)| y.iter().copied()),
                y_fallback,
                node.y_log,
            )
        });
    (x, y)
}

struct ChartInspection {
    series: String,
    x: f64,
    y: f64,
    x_ratio: f32,
    y_ratio: f32,
}

fn chart_inspection(
    node: &ChartNode,
    state: &InteractiveChartState,
    locally_hidden: Option<&HashSet<String>>,
) -> Option<ChartInspection> {
    let (hover_x, hover_y) = state.interaction.borrow().hover_domain()?;
    let (x_min, x_max) = state.x_domain();
    let (y_min, y_max) = state.y_domain();
    let ratio = |value: f64, min: f64, max: f64, logarithmic: bool| {
        if logarithmic && value > 0.0 && min > 0.0 && max > min {
            ((value.ln() - min.ln()) / (max.ln() - min.ln())).clamp(0.0, 1.0)
        } else if max > min {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.5
        }
    };
    let hover_x_ratio = ratio(hover_x, x_min, x_max, node.x_log);
    let hover_y_ratio = ratio(hover_y, y_min, y_max, node.y_log);
    let mut nearest: Option<(String, f64, f64, f64)> = None;
    let mut inspect = |label: String, x: &[f64], y: &[f64]| {
        for (&point_x, &point_y) in x.iter().zip(y) {
            let dx = ratio(point_x, x_min, x_max, node.x_log) - hover_x_ratio;
            let dy = ratio(point_y, y_min, y_max, node.y_log) - hover_y_ratio;
            let distance = dx * dx + dy * dy;
            if nearest
                .as_ref()
                .is_none_or(|(_, _, _, best)| distance < *best)
            {
                nearest = Some((label.clone(), point_x, point_y, distance));
            }
        }
    };
    if node.series.is_empty() {
        inspect(
            "Series".into(),
            node.x.as_deref().unwrap_or_default(),
            node.y.as_deref().unwrap_or_default(),
        );
    } else {
        for (index, series) in node.series.iter().enumerate().filter(|(_, series)| {
            series.visible && !locally_hidden.is_some_and(|hidden| hidden.contains(&series.id))
        }) {
            inspect(
                if series.label.is_empty() {
                    format!("Series {}", index + 1)
                } else {
                    series.label.clone()
                },
                &series.x,
                &series.y,
            );
        }
    }
    nearest.map(|(series, x, y, _)| ChartInspection {
        series,
        x,
        y,
        x_ratio: ratio(x, x_min, x_max, node.x_log) as f32,
        y_ratio: ratio(y, y_min, y_max, node.y_log) as f32,
    })
}

fn chart_csv(node: &ChartNode, locally_hidden: Option<&HashSet<String>>) -> String {
    let mut csv = String::new();
    match node.chart {
        ChartKind::Scatter | ChartKind::Line | ChartKind::Area | ChartKind::BoxPlot => {
            csv.push_str("series_id,series_label,x,y\n");
            if node.series.is_empty() {
                for (x, y) in node
                    .x
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .zip(node.y.as_deref().unwrap_or_default())
                {
                    csv.push_str(&format!("default,,{x},{y}\n"));
                }
            } else {
                for series in node.series.iter().filter(|series| {
                    series.visible
                        && !locally_hidden.is_some_and(|hidden| hidden.contains(&series.id))
                }) {
                    for (x, y) in series.x.iter().zip(&series.y) {
                        csv.push_str(&format!(
                            "{},{},{x},{y}\n",
                            csv_field(&series.id),
                            csv_field(&series.label)
                        ));
                    }
                }
            }
        }
        ChartKind::Bar | ChartKind::Pie | ChartKind::Donut => {
            csv.push_str("category,value\n");
            for (category, value) in node
                .categories
                .as_deref()
                .unwrap_or_default()
                .iter()
                .zip(node.values.as_deref().unwrap_or_default())
            {
                csv.push_str(&format!("{},{}\n", csv_field(category), value));
            }
        }
        ChartKind::Heatmap | ChartKind::Contour | ChartKind::Isoline => {
            csv.push_str("x,y,value\n");
            let width = node.width_count.unwrap_or_default();
            let x = node.x.as_deref();
            let y = node.y.as_deref();
            for (index, value) in node.z.as_deref().unwrap_or_default().iter().enumerate() {
                let column = index % width;
                let row = index / width;
                let value = value.map_or_else(String::new, |value| value.to_string());
                csv.push_str(&format!(
                    "{},{},{}\n",
                    x.and_then(|values| values.get(column))
                        .copied()
                        .unwrap_or(column as f64),
                    y.and_then(|values| values.get(row))
                        .copied()
                        .unwrap_or(row as f64),
                    value
                ));
            }
        }
        ChartKind::Treemap => {
            csv.push_str("name,value\n");
            fn append(node: &ChartTreemapNode, csv: &mut String) {
                csv.push_str(&format!("{},{}\n", csv_field(&node.name), node.value));
                for child in &node.children {
                    append(child, csv);
                }
            }
            if let Some(root) = &node.treemap {
                append(root, &mut csv);
            }
        }
    }
    csv
}

fn svg_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Dependency-free portable visual export. It deliberately mirrors the
/// displayed data and active Cartesian domain rather than serializing GPUI
/// draw commands, so applications can save it on every supported platform.
fn chart_svg(
    node: &ChartNode,
    domains: Option<((f64, f64), (f64, f64))>,
    locally_hidden: Option<&HashSet<String>>,
) -> String {
    let width = node.width.max(1.0);
    let height = node.height.max(1.0);
    let left = 48.0;
    let top = 28.0;
    let plot_width = (width - left - 12.0).max(1.0);
    let plot_height = (height - top - 26.0).max(1.0);
    let ((x_min, x_max), (y_min, y_max)) = domains.unwrap_or_else(|| cartesian_chart_domains(node));
    let x_pixel = |value: f64| {
        left + ((value - x_min) / (x_max - x_min).max(f64::EPSILON)) as f32 * plot_width
    };
    let y_pixel = |value: f64| {
        top + (1.0 - ((value - y_min) / (y_max - y_min).max(f64::EPSILON)) as f32) * plot_height
    };
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/><text x=\"{left}\" y=\"18\" font-family=\"sans-serif\" font-size=\"14\">{}</text><path d=\"M {left} {top} V {} H {}\" fill=\"none\" stroke=\"#666\"/>",
        svg_escape(&node.title),
        top + plot_height,
        left + plot_width
    );
    match node.chart {
        ChartKind::Line | ChartKind::Scatter | ChartKind::Area | ChartKind::BoxPlot => {
            let fallback_x = node.x.as_deref().unwrap_or_default();
            let fallback_y = node.y.as_deref().unwrap_or_default();
            let mut series = node
                .series
                .iter()
                .filter(|series| {
                    series.visible
                        && !locally_hidden.is_some_and(|hidden| hidden.contains(&series.id))
                })
                .map(|series| {
                    (
                        series.label.as_str(),
                        series.x.as_slice(),
                        series.y.as_slice(),
                        series.color.as_deref(),
                    )
                })
                .collect::<Vec<_>>();
            if series.is_empty() {
                series.push(("Series", fallback_x, fallback_y, node.color.as_deref()));
            }
            for (index, (label, x, y, color)) in series.into_iter().enumerate() {
                let color = color.unwrap_or(if index == 0 { "#1f77b4" } else { "#ff7f0e" });
                if matches!(node.chart, ChartKind::Line) {
                    let points = x
                        .iter()
                        .zip(y)
                        .map(|(&x, &y)| format!("{:.2},{:.2}", x_pixel(x), y_pixel(y)))
                        .collect::<Vec<_>>()
                        .join(" ");
                    svg.push_str(&format!("<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"/>"));
                } else {
                    for (&x, &y) in x.iter().zip(y) {
                        svg.push_str(&format!(
                            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"3\" fill=\"{color}\"/>",
                            x_pixel(x),
                            y_pixel(y)
                        ));
                    }
                }
                if !label.is_empty() {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-size=\"10\" fill=\"{color}\">{}</text>",
                        left + plot_width - 100.0,
                        top + 14.0 + index as f32 * 12.0,
                        svg_escape(label)
                    ));
                }
            }
        }
        ChartKind::Bar | ChartKind::Pie | ChartKind::Donut => {
            let values = node.values.as_deref().unwrap_or_default();
            let max = values
                .iter()
                .copied()
                .fold(0.0_f64, f64::max)
                .max(f64::EPSILON);
            let cell = plot_width / values.len().max(1) as f32;
            for (index, value) in values.iter().enumerate() {
                let bar_height = (*value / max) as f32 * plot_height;
                svg.push_str(&format!("<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"#2ca02c\"/>", left + index as f32 * cell + 1.0, top + plot_height - bar_height, (cell - 2.0).max(1.0), bar_height));
            }
        }
        ChartKind::Heatmap | ChartKind::Contour | ChartKind::Isoline => {
            let width_count = node.width_count.unwrap_or(0);
            let height_count = node.height_count.unwrap_or(0);
            let z = node.z.as_deref().unwrap_or_default();
            let cell_width = plot_width / width_count.max(1) as f32;
            let cell_height = plot_height / height_count.max(1) as f32;
            for (index, value) in z.iter().enumerate() {
                let column = index % width_count.max(1);
                let row = index / width_count.max(1);
                let color = if value.is_none() {
                    "#9ca3af"
                } else {
                    "#1f77b4"
                };
                svg.push_str(&format!("<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{color}\"/>", left + column as f32 * cell_width, top + (height_count.saturating_sub(row + 1)) as f32 * cell_height, cell_width, cell_height));
            }
        }
        ChartKind::Treemap => {
            svg.push_str(&format!(
                "<text x=\"{left}\" y=\"{}\" font-size=\"12\">Treemap</text>",
                top + 16.0
            ));
        }
    }
    svg.push_str("</svg>");
    svg
}

/// Encode a compact, dependency-free RGB PNG. The raster deliberately follows
/// the same lightweight data contract as `chart_svg`, so exports work in a
/// bundled application without a platform image encoder or extra crate.
fn chart_png(
    node: &ChartNode,
    domains: Option<((f64, f64), (f64, f64))>,
    locally_hidden: Option<&HashSet<String>>,
) -> Vec<u8> {
    let width = node.width.clamp(160.0, 4096.0).round() as usize;
    let height = node.height.clamp(120.0, 4096.0).round() as usize;
    let mut pixels = vec![255_u8; width * height * 3];
    let set = |pixels: &mut Vec<u8>, x: i32, y: i32, color: [u8; 3]| {
        if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
            let index = (y as usize * width + x as usize) * 3;
            pixels[index..index + 3].copy_from_slice(&color);
        }
    };
    let line = |pixels: &mut Vec<u8>, from: (i32, i32), to: (i32, i32), color: [u8; 3]| {
        let steps = (from.0.abs_diff(to.0).max(from.1.abs_diff(to.1)))
            .max(1)
            .min(8192);
        for step in 0..=steps {
            let ratio = step as f32 / steps as f32;
            set(
                pixels,
                (from.0 as f32 + (to.0 - from.0) as f32 * ratio).round() as i32,
                (from.1 as f32 + (to.1 - from.1) as f32 * ratio).round() as i32,
                color,
            );
        }
    };
    let left = 48_i32;
    let top = 28_i32;
    let right = (width as i32 - 12).max(left + 1);
    let bottom = (height as i32 - 26).max(top + 1);
    line(&mut pixels, (left, top), (left, bottom), [90, 90, 90]);
    line(&mut pixels, (left, bottom), (right, bottom), [90, 90, 90]);
    let ((x_min, x_max), (y_min, y_max)) = domains.unwrap_or_else(|| cartesian_chart_domains(node));
    let ratio = |value: f64, min: f64, max: f64, log: bool| {
        if log && value > 0.0 && min > 0.0 && max > min {
            ((value.ln() - min.ln()) / (max.ln() - min.ln())).clamp(0.0, 1.0)
        } else {
            ((value - min) / (max - min).max(f64::EPSILON)).clamp(0.0, 1.0)
        }
    };
    let point = |x: f64, y: f64| {
        (
            left + (ratio(x, x_min, x_max, node.x_log) * (right - left) as f64).round() as i32,
            bottom - (ratio(y, y_min, y_max, node.y_log) * (bottom - top) as f64).round() as i32,
        )
    };
    let color = |value: Option<&str>, fallback| {
        let packed = hex_color(value, fallback);
        [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8]
    };
    match node.chart {
        ChartKind::Line | ChartKind::Scatter | ChartKind::Area | ChartKind::BoxPlot => {
            let fallback = [(
                node.x.as_deref().unwrap_or_default(),
                node.y.as_deref().unwrap_or_default(),
                node.color.as_deref(),
            )];
            let series = if node.series.is_empty() {
                fallback.into_iter().collect::<Vec<_>>()
            } else {
                node.series
                    .iter()
                    .filter(|series| {
                        series.visible
                            && !locally_hidden.is_some_and(|hidden| hidden.contains(&series.id))
                    })
                    .map(|series| {
                        (
                            series.x.as_slice(),
                            series.y.as_slice(),
                            series.color.as_deref(),
                        )
                    })
                    .collect()
            };
            for (index, (x, y, series_color)) in series.into_iter().enumerate() {
                let series_color =
                    color(series_color, if index == 0 { 0x1f77b4 } else { 0xff7f0e });
                let points = x
                    .iter()
                    .zip(y)
                    .map(|(&x, &y)| point(x, y))
                    .collect::<Vec<_>>();
                if matches!(node.chart, ChartKind::Line) {
                    for pair in points.windows(2) {
                        line(&mut pixels, pair[0], pair[1], series_color);
                    }
                } else {
                    for (x, y) in points {
                        for dy in -2..=2 {
                            for dx in -2..=2 {
                                if dx * dx + dy * dy <= 4 {
                                    set(&mut pixels, x + dx, y + dy, series_color);
                                }
                            }
                        }
                    }
                }
            }
        }
        ChartKind::Bar | ChartKind::Pie | ChartKind::Donut => {
            let values = node.values.as_deref().unwrap_or_default();
            let maximum = values
                .iter()
                .copied()
                .fold(0.0_f64, f64::max)
                .max(f64::EPSILON);
            let cell = (right - left).max(1) as f64 / values.len().max(1) as f64;
            let bar_color = color(node.color.as_deref(), 0x2ca02c);
            for (index, value) in values.iter().enumerate() {
                let bar_top = bottom
                    - ((*value / maximum).clamp(0.0, 1.0) * (bottom - top) as f64).round() as i32;
                for x in (left + (index as f64 * cell).round() as i32 + 1)
                    ..(left + ((index + 1) as f64 * cell).round() as i32 - 1)
                {
                    for y in bar_top..bottom {
                        set(&mut pixels, x, y, bar_color);
                    }
                }
            }
        }
        ChartKind::Heatmap | ChartKind::Contour | ChartKind::Isoline => {
            let columns = node.width_count.unwrap_or(0).max(1);
            let rows = node.height_count.unwrap_or(0).max(1);
            let values = node.z.as_deref().unwrap_or_default();
            let min = values
                .iter()
                .flatten()
                .copied()
                .fold(f64::INFINITY, f64::min);
            let max = values
                .iter()
                .flatten()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            for (index, value) in values.iter().enumerate() {
                let column = index % columns;
                let row = index / columns;
                let t = value
                    .map(|value| ((value - min) / (max - min).max(f64::EPSILON)).clamp(0.0, 1.0));
                let cell_color = t
                    .map(|t| {
                        [
                            (32.0 + 220.0 * t) as u8,
                            (60.0 + 120.0 * (1.0 - t)) as u8,
                            (210.0 - 160.0 * t) as u8,
                        ]
                    })
                    .unwrap_or([156, 163, 175]);
                let x0 = left + (column as i32 * (right - left) / columns as i32);
                let x1 = left + ((column + 1) as i32 * (right - left) / columns as i32);
                let y0 =
                    top + ((rows.saturating_sub(row + 1)) as i32 * (bottom - top) / rows as i32);
                let y1 = top + ((rows - row) as i32 * (bottom - top) / rows as i32);
                for y in y0..y1 {
                    for x in x0..x1 {
                        set(&mut pixels, x, y, cell_color);
                    }
                }
            }
        }
        ChartKind::Treemap => {}
    }
    png_encode(width as u32, height as u32, &pixels)
}

fn png_encode(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    fn adler32(bytes: &[u8]) -> u32 {
        let (mut a, mut b) = (1_u32, 0_u32);
        for byte in bytes {
            a = (a + *byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0_u32;
        for byte in bytes {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(crc & 1)));
            }
        }
        !crc
    }
    fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(kind);
        output.extend_from_slice(data);
        let mut crc_input = Vec::with_capacity(kind.len() + data.len());
        crc_input.extend_from_slice(kind);
        crc_input.extend_from_slice(data);
        output.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    }
    let mut raw = Vec::with_capacity((width as usize * 3 + 1) * height as usize);
    for row in pixels.chunks_exact(width as usize * 3) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut compressed = vec![0x78, 0x01];
    for (index, block) in raw.chunks(65_535).enumerate() {
        compressed.push(if (index + 1) * 65_535 >= raw.len() {
            1
        } else {
            0
        });
        let length = block.len() as u16;
        compressed.extend_from_slice(&length.to_le_bytes());
        compressed.extend_from_slice(&(!length).to_le_bytes());
        compressed.extend_from_slice(block);
    }
    compressed.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut output = Vec::new();
    output.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut output, b"IHDR", &header);
    chunk(&mut output, b"IDAT", &compressed);
    chunk(&mut output, b"IEND", &[]);
    output
}

#[cfg(test)]
mod chart_export_tests {
    use super::{ChartNode, native_chart_svg, png_encode};

    #[::core::prelude::v1::test]
    fn png_encoder_writes_a_signature_and_terminal_chunk() {
        let png = png_encode(1, 1, &[12, 34, 56]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.windows(4).any(|window| window == b"IHDR"));
        assert!(png.windows(4).any(|window| window == b"IDAT"));
        assert!(png.windows(4).any(|window| window == b"IEND"));
    }

    #[::core::prelude::v1::test]
    fn native_line_export_preserves_title_and_visible_series() {
        let node: ChartNode = serde_json::from_value(serde_json::json!({
            "id": "response",
            "chart": "line",
            "title": "Frequency response",
            "x": [100.0, 200.0, 400.0],
            "y": [80.0, 81.0, 79.5],
            "width": 640.0,
            "height": 320.0,
            "series": [
                {"id": "spl", "label": "SPL", "x": [100.0, 200.0, 400.0], "y": [80.0, 81.0, 79.5], "visible": true},
                {"id": "hidden", "label": "Hidden", "x": [100.0, 200.0, 400.0], "y": [1.0, 2.0, 3.0], "visible": false}
            ]
        }))
        .expect("valid chart fixture");
        let svg = native_chart_svg(&node, Some(((100.0, 400.0), (79.0, 82.0))), None)
            .expect("native SVG export");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Frequency response"));
        assert!(svg.contains("SPL"));
        assert!(!svg.contains("Hidden"));
    }

    #[::core::prelude::v1::test]
    fn native_export_rejects_unsupported_chart_kinds() {
        let node: ChartNode = serde_json::from_value(serde_json::json!({
            "id": "heatmap",
            "chart": "heatmap",
            "z": [1.0],
            "width_count": 1,
            "height_count": 1
        }))
        .expect("valid chart fixture");
        let error = native_chart_svg(&node, None, None).expect_err("heatmap is not supported yet");
        assert!(error.contains("does not support"));
    }
}

#[cfg(test)]
mod scene_selection_tests {
    use super::scene_selection_object_id;

    #[::core::prelude::v1::test]
    fn single_mesh_scene_uses_its_stable_geometry_id() {
        let spec = serde_json::json!({
            "id": "speaker-scene",
            "children": [{"id": "baffle"}]
        });
        assert_eq!(scene_selection_object_id("speaker-scene", &spec), "baffle");
    }

    #[::core::prelude::v1::test]
    fn compound_scene_does_not_guess_a_child() {
        let spec = serde_json::json!({
            "id": "speaker-scene",
            "children": [{"id": "baffle"}, {"id": "woofer"}]
        });
        assert_eq!(
            scene_selection_object_id("speaker-scene", &spec),
            "speaker-scene"
        );
    }
}

#[cfg(test)]
mod mesh_selection_tests {
    use super::{mesh_selection_event_payload, mesh_selection_payload};
    use gpui_px::MeshPlotPick;

    #[::core::prelude::v1::test]
    fn selection_payload_preserves_pick_identity_and_value() {
        let pick = MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "mesh".into(),
            cell_index: 7,
            cell_id: Some(42),
            nearest_vertex_index: Some(3),
            vertex_id: Some(99),
            world_position: [0.25, 0.5, 1.0],
            displayed_value: Some(91.2),
            field_id: Some("pressure".into()),
        };
        let payload = mesh_selection_payload(&pick);
        assert_eq!(payload["plot_id"], "plot");
        assert_eq!(payload["mesh_id"], "mesh");
        assert_eq!(payload["cell_index"], 7);
        assert_eq!(payload["cell_id"], 42);
        assert_eq!(payload["nearest_vertex_index"], 3);
        assert_eq!(payload["vertex_id"], 99);
        assert_eq!(
            payload["world_position"],
            serde_json::json!([0.25, 0.5, 1.0])
        );
        assert_eq!(payload["displayed_value"], 91.2);
        assert_eq!(payload["field_id"], "pressure");
    }

    #[::core::prelude::v1::test]
    fn cleared_selection_uses_a_null_payload() {
        assert_eq!(mesh_selection_event_payload(None), serde_json::Value::Null);
    }
}

#[cfg(test)]
mod builder_adapter_tests {
    use super::{BuilderLayoutSpec, FixedTextMeasure, builder_chassis_row, with_builder_node};

    #[::core::prelude::v1::test]
    fn recursive_owned_tree_is_solved_and_inspected_without_leaks() {
        let spec: BuilderLayoutSpec = serde_json::from_value(serde_json::json!({
            "kind": "container",
            "id": "root",
            "axis": "horizontal",
            "sizing": {"kind": "flex", "min": 0.0, "weight": 1.0},
            "children": [
                {
                    "kind": "slot",
                    "id": "sidebar",
                    "sizing": {"kind": "fixed", "initial": 120.0},
                    "collapsible": false
                },
                {
                    "kind": "container",
                    "id": "content",
                    "axis": "vertical",
                    "sizing": {"kind": "flex", "min": 0.0, "weight": 1.0},
                    "children": [{
                        "kind": "slot",
                        "id": "copy",
                        "sizing": {
                            "kind": "text",
                            "text": "two words",
                            "line_height": 20.0,
                            "min": 0.0
                        },
                        "display_tiers": [{"name": "full", "min_size": 20.0}]
                    }]
                }
            ]
        }))
        .unwrap();
        let measure = FixedTextMeasure(8.0);
        let result = with_builder_node(
            &spec,
            &measure,
            Box::new(|root| {
                let validation = gpui_builder::validate_layout(&root);
                let declaration = gpui_builder::inspect_layout(&root);
                let solved = gpui_builder::solve(
                    &root,
                    500.0,
                    300.0,
                    &gpui_builder::LayoutPreferences::default(),
                );
                let debug = solved.debug_report_with_source(&root);
                (
                    validation.is_clean(),
                    declaration.nodes().len(),
                    solved.find("sidebar").map(|node| node.width),
                    solved
                        .find("copy")
                        .and_then(|node| node.active_tier)
                        .map(str::to_string),
                    debug.tree().to_string(),
                )
            }),
        );
        assert!(result.0);
        assert_eq!(result.1, 4);
        assert_eq!(result.2, Some(120.0));
        assert_eq!(result.3.as_deref(), Some("full"));
        assert!(result.4.contains("copy"));
    }

    #[::core::prelude::v1::test]
    fn snapshot_and_retained_solvers_agree_across_viewports() {
        let spec: BuilderLayoutSpec = serde_json::from_value(serde_json::json!({
            "kind": "container",
            "id": "root",
            "axis": "horizontal",
            "sizing": {"kind": "flex", "min": 0.0, "weight": 1.0},
            "children": [{
                "kind": "slot",
                "id": "body",
                "sizing": {"kind": "flex", "min": 40.0, "weight": 1.0}
            }]
        }))
        .unwrap();
        let measure = FixedTextMeasure(8.0);
        let widths = with_builder_node(
            &spec,
            &measure,
            Box::new(|root| {
                let viewports = [
                    gpui_builder::LayoutViewport::new("wide", 800.0, 600.0),
                    gpui_builder::LayoutViewport::new("narrow", 320.0, 480.0),
                ];
                let preferences = gpui_builder::LayoutPreferences::default();
                let matrix = gpui_builder::solve_snapshot_matrix(&root, &viewports, &preferences);
                let expected = matrix
                    .snapshots
                    .iter()
                    .map(|snapshot| snapshot.root.width)
                    .collect::<Vec<_>>();
                let mut solver = gpui_builder::RetainedLayoutSolver::with_capacity(2);
                let retained = viewports
                    .iter()
                    .map(|viewport| {
                        solver
                            .solve(&root, viewport.width, viewport.height, &preferences)
                            .root()
                            .width()
                    })
                    .collect::<Vec<_>>();
                (expected, retained, matrix.to_markdown_table())
            }),
        );
        assert_eq!(widths.0, widths.1);
        assert_eq!(widths.0, vec![800.0, 320.0]);
        assert!(widths.2.contains("wide"));
    }

    #[::core::prelude::v1::test]
    fn full_chassis_rows_decode_to_native_specs() {
        let row = builder_chassis_row(&serde_json::json!({
            "kind": "knob_row",
            "id": "controls",
            "knobs": [{
                "id": "gain",
                "param_idx": 2,
                "label": "Gain",
                "size": "sm",
                "bipolar": true
            }]
        }))
        .unwrap();
        let gpui_builder::RowSpec::KnobRow { id, knobs } = row else {
            panic!("expected knob row")
        };
        assert_eq!(id, "controls");
        assert_eq!(knobs[0].id, "gain");
        assert_eq!(knobs[0].param_idx, 2);
        assert_eq!(knobs[0].size, gpui_builder::KnobSize::Sm);
        assert!(knobs[0].bipolar);
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

fn audio_accessibility_json(summary: &gpui_audio_kit::AudioAccessibilitySummary) -> Value {
    serde_json::json!({
        "control_type": summary.control_type,
        "label": summary.label,
        "role": format!("{:?}", summary.role).to_lowercase(),
        "value_now": summary.value_now,
        "value_min": summary.value_min,
        "value_max": summary.value_max,
        "value_text": summary.value_text,
        "unit": summary.unit,
        "normalized": summary.normalized,
        "scale": summary.scale.map(|scale| format!("{scale:?}").to_lowercase()),
        "selected": summary.selected,
        "disabled": summary.disabled,
        "muted": summary.muted,
        "peak_value": summary.peak_value,
        "description": summary.description,
    })
}

fn px_curve(value: &str) -> gpui_px::CurveType {
    match value {
        "step" => gpui_px::CurveType::Step,
        "step_before" => gpui_px::CurveType::StepBefore,
        "step_after" => gpui_px::CurveType::StepAfter,
        "basis" => gpui_px::CurveType::Basis,
        "cardinal" => gpui_px::CurveType::Cardinal,
        "catmull_rom" => gpui_px::CurveType::CatmullRom,
        "monotone_x" => gpui_px::CurveType::MonotoneX,
        "natural" => gpui_px::CurveType::Natural,
        _ => gpui_px::CurveType::Linear,
    }
}

fn px_hex_color(value: &str, fallback: u32) -> u32 {
    value
        .trim()
        .strip_prefix('#')
        .and_then(|value| u32::from_str_radix(value, 16).ok())
        .unwrap_or(fallback)
}

fn px_legend_position(value: &str) -> gpui_px::LegendPosition {
    match value {
        "right" => gpui_px::LegendPosition::Right,
        "bottom" => gpui_px::LegendPosition::Bottom,
        "top" => gpui_px::LegendPosition::Top,
        "left" => gpui_px::LegendPosition::Left,
        _ => gpui_px::LegendPosition::Right,
    }
}

fn px_annotations(node: &ChartNode) -> Vec<gpui_px::ChartAnnotation> {
    node.annotations
        .iter()
        .map(|annotation| {
            let mut result = match annotation.target.as_str() {
                "x_value" => gpui_px::ChartAnnotation::x_value(
                    &annotation.id,
                    &annotation.label,
                    annotation.x.unwrap_or_default(),
                ),
                "y_value" => gpui_px::ChartAnnotation::y_value(
                    &annotation.id,
                    &annotation.label,
                    annotation.y.unwrap_or_default(),
                ),
                "category" => gpui_px::ChartAnnotation::category(
                    &annotation.id,
                    &annotation.label,
                    annotation.category.clone().unwrap_or_default(),
                ),
                _ => gpui_px::ChartAnnotation::point(
                    &annotation.id,
                    &annotation.label,
                    annotation.x.unwrap_or_default(),
                    annotation.y.unwrap_or_default(),
                ),
            };
            if let Some(color) = annotation.color.as_deref() {
                result = result.color(px_hex_color(color, 0x1f77b4));
            }
            if let Some(index) = annotation.series_index {
                result = result.series_index(index);
            }
            result
        })
        .collect()
}

#[cfg(test)]
fn scene_selection_object_id(node_id: &str, spec: &Value) -> String {
    if let Some(children) = spec.get("children").and_then(Value::as_array) {
        let ids = children
            .iter()
            .filter_map(|child| child.get("id").and_then(Value::as_str))
            .filter(|id| !id.trim().is_empty())
            .collect::<Vec<_>>();
        // A single retained mesh has an unambiguous stable object ID. For a
        // compound scene, keep the scene ID until the host has a real depth
        // pick result rather than reporting an arbitrary child.
        if ids.len() == 1 {
            return ids[0].to_string();
        }
    }
    spec.get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .unwrap_or(node_id)
        .to_string()
}

pub(super) struct PythonIrShowcase {
    pub(super) app: Option<PythonAppIr>,
    /// JSON form of the committed app, retained for patch/resource bookkeeping.
    /// Render code continues to use the typed IR above.
    app_value: Option<Value>,
    pub(super) load_error: Option<String>,
    pub(super) current_section: String,
    pub(super) gpui_3d: Gpui3DCache,
    pub(super) mesh_plots: GpuiMeshPlotCache,
    pub(super) spec_cache: TypedSpecCache,
    pub(super) table_cells: HashMap<(usize, usize), (String, SharedString)>,
    form_focus: HashMap<String, FocusHandle>,
    color_pickers: HashMap<String, Entity<ColorPickerView>>,
    color_picker_subscriptions: HashMap<String, Subscription>,
    color_picker_actions: HashMap<String, Option<String>>,
    thinking_orbs: HashMap<String, (OrbState, Entity<ThinkingOrb>)>,
    tab_focus: HashMap<String, FocusHandle>,
    /// Retained per-chart interaction state. Re-renders rebuild the draw list
    /// from this state, so data patches do not discard a user's zoom or pan.
    chart_interactions: HashMap<String, InteractiveChartState>,
    /// Host-local legend choices. They are keyed by Python's stable series ID
    /// and intentionally survive data patches without changing Python state.
    chart_hidden_series: HashMap<String, HashSet<String>>,
    /// Latest-only decoded binary frames. Audio rendering reads these directly
    /// without routing high-rate decimal arrays through app patches.
    audio_frames: AudioFrameStore,
    mesh_frames: MeshFrameStore,
    prepared_mesh_plots: HashMap<String, CachedNativeMeshPlot>,
    /// Native MeshPlot owners keep their decoded resource generations alive
    /// until the corresponding plot is replaced or removed.
    mesh_plot_resource_refs: HashMap<String, Vec<(String, u64)>>,
    /// Mesh interaction state is host-owned and survives declarative patches.
    mesh_plot_states: HashMap<String, Rc<RefCell<MeshPlotState>>>,
    /// Recoverable resource/patch failures are kept with the affected plot so
    /// its last valid frame remains visible instead of replacing the whole
    /// application with a session error screen.
    mesh_plot_errors: HashMap<String, String>,
    last_mesh_patch_id: Option<String>,
    table_scrolls: HashMap<String, UniformListScrollHandle>,
    table_focus: HashMap<String, FocusHandle>,
    /// Anonymous legacy tables do not retain interaction state, but their
    /// element IDs must still be unique within a rendered GPUI tree.
    legacy_table_id_counter: u64,
    table_column_widths: Rc<RefCell<HashMap<(String, String), f32>>>,
    table_resize: Rc<RefCell<Option<TableResize>>>,
    job_log_scrolls: HashMap<String, UniformListScrollHandle>,
    superseded_requests: HashSet<String>,
    /// Cancellation flags for bounded host telemetry streams. The sender runs
    /// off the render thread and communicates only through the session pipe.
    profiler_subscriptions: HashMap<String, Arc<AtomicBool>>,
    applied_miniapp_shell: Option<MiniAppShellConfig>,
    observed_miniapp_theme: Option<ThemeVariant>,
    observed_miniapp_language: Option<Language>,
    pub(super) session: Option<super::python::PythonSession>,
    pub(super) session_state: SessionState,
    pub(super) jobs: JobRegistry,
    job_log_filter: Option<LogSeverity>,
    /// A paused view snapshots the visible lines but leaves the bounded live
    /// buffer intact, so following the tail later never loses diagnostics.
    paused_job_logs: HashMap<String, Vec<JobLogLine>>,
    pending_confirmation: Option<PendingConfirmation>,
    notification: Option<String>,
    presentation: PresentationStore,
    presentation_subscription: Option<Subscription>,
    content_scroll: ScrollHandle,
    close_handler_installed: bool,
    close_approved: bool,
    qa_pointer_task: Option<Task<()>>,
}

#[derive(Clone)]
struct PendingConfirmation {
    request_id: String,
    title: String,
    message: String,
    confirm_label: String,
    cancel_label: String,
}

#[derive(Clone)]
struct CachedNativeMeshPlot {
    source_address: usize,
    spec: Rc<MeshPlotSpec>,
    prepared: gpui_python_runtime::native_mesh_plot::PreparedMeshPlot,
}

/// A transient native drag; the authoritative width remains application state
/// once the corresponding resize action has been handled by Python.
#[derive(Clone)]
struct TableResize {
    table_id: String,
    column_id: String,
    start_x: f32,
    start_width: f32,
}

impl PythonIrShowcase {
    fn new_empty(presentation: PresentationStore) -> Self {
        let presentation_state = presentation.snapshot();
        let content_scroll = ScrollHandle::new();
        content_scroll.set_offset(point(px(0.0), px(-presentation_state.scroll_y)));
        Self {
            app: None,
            app_value: None,
            load_error: None,
            current_section: presentation_state.section.unwrap_or_default(),
            gpui_3d: Gpui3DCache::new(),
            mesh_plots: GpuiMeshPlotCache::new(),
            spec_cache: TypedSpecCache::new(),
            table_cells: HashMap::new(),
            form_focus: HashMap::new(),
            color_pickers: HashMap::new(),
            color_picker_subscriptions: HashMap::new(),
            color_picker_actions: HashMap::new(),
            thinking_orbs: HashMap::new(),
            tab_focus: HashMap::new(),
            chart_interactions: HashMap::new(),
            chart_hidden_series: HashMap::new(),
            audio_frames: AudioFrameStore::new(),
            mesh_frames: MeshFrameStore::new(),
            prepared_mesh_plots: HashMap::new(),
            mesh_plot_resource_refs: HashMap::new(),
            mesh_plot_states: HashMap::new(),
            mesh_plot_errors: HashMap::new(),
            last_mesh_patch_id: None,
            table_scrolls: HashMap::new(),
            table_focus: HashMap::new(),
            legacy_table_id_counter: 0,
            table_column_widths: Rc::new(RefCell::new(HashMap::new())),
            table_resize: Rc::new(RefCell::new(None)),
            job_log_scrolls: HashMap::new(),
            superseded_requests: HashSet::new(),
            profiler_subscriptions: HashMap::new(),
            applied_miniapp_shell: None,
            observed_miniapp_theme: None,
            observed_miniapp_language: None,
            session: None,
            session_state: SessionState::new(
                gpui_python_runtime::session::DEFAULT_HOST_CAPABILITIES
                    .iter()
                    .map(|capability| (*capability).into())
                    .collect(),
            ),
            // Retain the required scientific-workload history while rendering
            // only the latest 200 filtered lines below. This keeps incoming
            // log updates bounded and avoids rebuilding a 10k-row view.
            jobs: JobRegistry::new(10_000),
            job_log_filter: None,
            paused_job_logs: HashMap::new(),
            pending_confirmation: None,
            notification: None,
            presentation,
            presentation_subscription: None,
            content_scroll,
            close_handler_installed: false,
            close_approved: false,
            qa_pointer_task: None,
        }
    }

    pub(super) fn new_ready(
        cx: &mut Context<Self>,
        presentation: PresentationStore,
        app: PythonAppIr,
        session: super::python::PythonSession,
    ) -> Self {
        let mut showcase = Self::new_empty(presentation);
        showcase.install_loaded_session(app, session, cx);
        showcase
    }

    fn install_loaded_session(
        &mut self,
        app: PythonAppIr,
        session: super::python::PythonSession,
        cx: &mut Context<Self>,
    ) {
        if !app
            .sections
            .iter()
            .any(|section| section.id == self.current_section)
            && let Some(section) = app.sections.first()
        {
            self.current_section = section.id.clone();
            self.presentation.set_section(Some(section.id.clone()));
        }
        self.app_value = serde_json::to_value(&app).ok();
        self.app = Some(app);
        self.session = Some(session);
        self.start_session_updates(cx);
    }

    fn sync_mesh_plot_resource_refs(
        &mut self,
        next: HashMap<String, Vec<(String, u64)>>,
    ) -> Result<(), String> {
        let mut retained: Vec<(String, u64)> = Vec::new();
        for (plot_id, handles) in &next {
            let previous = self.mesh_plot_resource_refs.get(plot_id);
            for (resource_id, generation) in handles {
                if previous.is_some_and(|handles| {
                    handles
                        .iter()
                        .any(|handle| handle == &(resource_id.clone(), *generation))
                }) {
                    continue;
                }
                if !self.mesh_frames.retain(resource_id, *generation) {
                    for (retained_id, retained_generation) in retained {
                        self.mesh_frames
                            .release_reference(&retained_id, retained_generation);
                    }
                    return Err(format!(
                        "mesh plot {plot_id:?} resource {resource_id:?} generation {generation} disappeared while retaining"
                    ));
                }
                retained.push((resource_id.clone(), *generation));
            }
        }

        for (plot_id, handles) in &self.mesh_plot_resource_refs {
            let next_handles = next.get(plot_id);
            for (resource_id, generation) in handles {
                if next_handles.is_some_and(|handles| {
                    handles
                        .iter()
                        .any(|handle| handle == &(resource_id.clone(), *generation))
                }) {
                    continue;
                }
                self.mesh_frames.release_reference(resource_id, *generation);
            }
        }
        self.mesh_plot_resource_refs = next;
        Ok(())
    }

    fn sync_mesh_plot_resource_refs_for_spec(&mut self, spec: &MeshPlotSpec) -> Result<(), String> {
        let mut next = self.mesh_plot_resource_refs.clone();
        next.insert(spec.id.clone(), mesh_plot_resource_handles(spec)?);
        self.sync_mesh_plot_resource_refs(next)
    }

    fn release_mesh_plot_resource_refs(&mut self) {
        let refs = std::mem::take(&mut self.mesh_plot_resource_refs);
        for handles in refs.into_values() {
            for (resource_id, generation) in handles {
                self.mesh_frames.release_reference(&resource_id, generation);
            }
        }
    }

    fn reset_mesh_plot_runtime_state(&mut self) {
        self.release_mesh_plot_resource_refs();
        self.session_state.reset_for_new_session();
        // A new Python child may restart generation numbering. Use a fresh
        // frame store for a new session rather than preserving stale history
        // across unrelated producers.
        self.mesh_frames = MeshFrameStore::new();
        self.mesh_plots.retain_only(std::iter::empty::<&str>());
        self.prepared_mesh_plots.clear();
        self.mesh_plot_states.clear();
        self.mesh_plot_errors.clear();
        self.last_mesh_patch_id = None;
    }

    fn record_mesh_patch_error(
        &mut self,
        patch: &Patch,
        app_value: Option<&Value>,
        error: impl Into<String>,
    ) {
        let error = error.into();
        let mut recorded = false;
        for operation in &patch.ops {
            if let Some(plot_id) = mesh_plot_operation_id(operation) {
                self.mesh_plot_errors
                    .insert(plot_id.to_owned(), error.clone());
                if let Some(spec_id) =
                    app_value.and_then(|value| mesh_plot_spec_id_for_node(value, plot_id))
                {
                    self.mesh_plot_errors.insert(spec_id, error.clone());
                }
                recorded = true;
            }
        }
        if !recorded {
            self.load_error = Some(error);
        }
    }

    fn clear_mesh_patch_errors(&mut self, patch: &Patch, app_value: Option<&Value>) {
        for operation in &patch.ops {
            if let Some(plot_id) = mesh_plot_operation_id(operation) {
                self.mesh_plot_errors.remove(plot_id);
                if let Some(spec_id) =
                    app_value.and_then(|value| mesh_plot_spec_id_for_node(value, plot_id))
                {
                    self.mesh_plot_errors.remove(&spec_id);
                }
            }
        }
    }

    fn record_mesh_resource_error(
        &mut self,
        resource_id: &str,
        generation: u64,
        error: impl Into<String>,
    ) {
        let error = error.into();
        let mut plot_ids = Vec::new();
        let mut add_plot_id = |plot_id: &str| {
            if !plot_ids.iter().any(|existing| existing == plot_id) {
                plot_ids.push(plot_id.to_owned());
            }
        };
        for (plot_id, handles) in &self.mesh_plot_resource_refs {
            if handles.iter().any(|(id, retained_generation)| {
                id == resource_id && *retained_generation == generation
            }) {
                add_plot_id(plot_id);
            }
        }
        // A snapshot may commit before its binary frames arrive. In that
        // interval there are no retained owners yet, but the declarative app
        // still identifies the plot that should receive a resource-local
        // diagnostic rather than a global error.
        // Normal snapshot and patch commits retain this value, so resource
        // errors do not serialize the complete app again. The lazy branch
        // keeps direct host/test construction compatible.
        if self.app_value.is_none() {
            self.app_value = self
                .app
                .as_ref()
                .and_then(|app| serde_json::to_value(app).ok());
        }
        if let Some(app_value) = self.app_value.as_ref() {
            let mut declared_refs = HashMap::new();
            if collect_mesh_plot_resource_refs(app_value, &mut declared_refs).is_ok() {
                for (plot_id, handles) in declared_refs {
                    if handles.iter().any(|(id, retained_generation)| {
                        id == resource_id && *retained_generation == generation
                    }) {
                        add_plot_id(&plot_id);
                    }
                }
            }
        }
        if plot_ids.is_empty() {
            self.load_error = Some(error);
        } else {
            for plot_id in plot_ids {
                self.mesh_plot_errors.insert(plot_id, error.clone());
            }
        }
    }

    fn clear_mesh_resource_error(&mut self, resource_id: &str, generation: u64) {
        let prefix = format!("mesh resource {resource_id:?} generation {generation}");
        self.mesh_plot_errors
            .retain(|_, message| !message.starts_with(&prefix));
        if self
            .load_error
            .as_deref()
            .is_some_and(|message| message.starts_with(&prefix))
        {
            self.load_error = None;
        }
    }

    /// Commit a Python snapshot as one resource-ownership transaction.
    ///
    /// Snapshots may legally arrive before their binary MeshFrames. In that
    /// case the new declarative app is committed so the first complete frame
    /// can render it, while the previous plot cache remains available for the
    /// last-valid fallback. If a snapshot is malformed, or retaining a fully
    /// available snapshot fails, the previously committed app and ownership
    /// remain untouched.
    fn apply_snapshot_message(&mut self, app_ir: PythonAppIr) {
        if let Err(error) = app_ir.validate() {
            self.load_error = Some(error.to_string());
            return;
        }

        let app_value = serde_json::to_value(&app_ir).unwrap_or(Value::Null);
        let mut next_resource_refs = HashMap::new();
        let mut live_ids = HashSet::new();
        mesh_plot_ids(&app_value, &mut live_ids);
        if let Err(error) = collect_mesh_plot_resource_refs(&app_value, &mut next_resource_refs) {
            self.load_error = Some(error);
            return;
        }

        let resources_available =
            next_resource_refs
                .values()
                .flatten()
                .all(|(resource_id, generation)| {
                    self.mesh_frames.get(resource_id, *generation).is_some()
                });
        if resources_available {
            if let Err(error) = self.sync_mesh_plot_resource_refs(next_resource_refs) {
                self.load_error = Some(error);
                return;
            }
        } else {
            // Frames may legally arrive after a snapshot. Release the old
            // owners now and let the first successful render acquire the new
            // generations without mixing sessions or producers.
            self.release_mesh_plot_resource_refs();
        }

        self.app_value = Some(app_value);
        self.app = Some(app_ir);
        self.prune_mesh_plot_runtime_ids(&live_ids);
        self.load_error = None;
        for live_id in live_ids {
            self.mesh_plot_errors.remove(&live_id);
        }
    }

    fn release_runtime_resource(&mut self, resource_id: &str, generation: u64) {
        let released_audio = self.audio_frames.release(resource_id, generation);
        let released_mesh = self.mesh_frames.release(resource_id, generation);
        let active_plot_owns_generation =
            self.mesh_plot_resource_refs
                .values()
                .flatten()
                .any(|(id, retained_generation)| {
                    id == resource_id && *retained_generation == generation
                });
        if (!released_mesh && active_plot_owns_generation) || (!released_audio && !released_mesh) {
            let message = if active_plot_owns_generation && !released_mesh {
                format!(
                    "mesh resource {:?} generation {} is still owned by an active plot",
                    resource_id, generation
                )
            } else {
                format!(
                    "mesh resource {:?} generation {} was not retained",
                    resource_id, generation
                )
            };
            self.record_mesh_resource_error(resource_id, generation, message);
        }
    }

    fn prune_mesh_plot_runtime_ids(&mut self, live_ids: &HashSet<String>) {
        self.session_state.retain_mesh_plot_generations(live_ids);
        self.mesh_plots
            .retain_only(live_ids.iter().map(String::as_str));
        self.prepared_mesh_plots
            .retain(|id, _| live_ids.contains(id));
        self.mesh_plot_states.retain(|id, _| live_ids.contains(id));
        self.mesh_plot_errors.retain(|id, _| live_ids.contains(id));
    }

    fn load_session(&mut self, cx: &mut Context<Self>) {
        self.load_error = None;
        self.reset_mesh_plot_runtime_state();
        self.app = None;
        self.app_value = None;
        self.session = None;
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = super::python::load_python_session_async().await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok((app, session)) => {
                        this.install_loaded_session(app, session, cx);
                    }
                    Err(error) => {
                        this.load_error = Some(error.to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn select_section(&mut self, section: String) {
        self.current_section = section.clone();
        self.presentation.set_section(Some(section));
    }

    fn chart_series_is_visible(
        &self,
        chart_id: &str,
        series: &gpui_python_runtime::ui_ir::ChartSeries,
    ) -> bool {
        series.visible
            && !self
                .chart_hidden_series
                .get(chart_id)
                .is_some_and(|hidden| hidden.contains(&series.id))
    }

    fn toggle_chart_series(&mut self, chart_id: &str, series_id: &str) {
        let hidden = self
            .chart_hidden_series
            .entry(chart_id.to_string())
            .or_default();
        if !hidden.insert(series_id.to_string()) {
            hidden.remove(series_id);
        }
    }

    fn observe_presentation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.presentation_subscription.is_some() {
            return;
        }
        self.presentation_subscription =
            Some(cx.observe_window_bounds(window, |this, window, _| {
                let bounds = window.bounds();
                this.presentation
                    .set_window_size(bounds.size.width.into(), bounds.size.height.into());
            }));
    }

    fn observe_window_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_handler_installed {
            return;
        }
        self.close_handler_installed = true;
        let entity = cx.weak_entity();
        window.on_window_should_close(cx, move |_window, cx| {
            entity
                .update(cx, |this, cx| this.request_window_close(cx))
                .unwrap_or(true)
        });
    }

    fn request_window_close(&mut self, cx: &mut Context<Self>) -> bool {
        if self.close_approved || !self.jobs.has_active_jobs() {
            return true;
        }
        if self.pending_confirmation.is_none() {
            if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
                let _ = sink.dispatch(
                    "window",
                    "close_requested",
                    Some("window_close_requested".into()),
                    serde_json::json!({"active_jobs": true}),
                );
            }
            self.pending_confirmation = Some(PendingConfirmation {
                request_id: "__host_close_while_jobs_running__".into(),
                title: "Jobs are still running".into(),
                message: "Closing now stops this application session. Running jobs are not marked successful.".into(),
                confirm_label: "Close anyway".into(),
                cancel_label: "Keep running".into(),
            });
        }
        cx.notify();
        false
    }

    fn start_session_updates(&mut self, cx: &mut Context<Self>) {
        let Some(wake) = self.session.as_ref().map(|session| session.wake_handle()) else {
            return;
        };
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            loop {
                wake.clone().await;
                if this
                    .update(cx, |this, cx| {
                        this.drain_session(cx);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Schedule one explicit QA-only pointer click after the first live frame.
    ///
    /// This is intentionally opt-in: it exists so a real native host session
    /// can exercise the same GPUI event path as a user click and produce the
    /// Python selection log without relying on an external automation tool.
    fn schedule_qa_pointer_event(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.qa_pointer_task.is_some()
            || env::var("GPUI_TOOLKIT_QA_AUTO_SELECT").ok().as_deref() != Some("1")
            || self.app.is_none()
        {
            return;
        }
        let parse_coordinate = |name: &str, default: f32| {
            env::var(name)
                .ok()
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite() && *value >= 0.0)
                .unwrap_or(default)
        };
        let default_position = point(
            px(parse_coordinate("GPUI_TOOLKIT_QA_POINTER_X", 560.0)),
            px(parse_coordinate("GPUI_TOOLKIT_QA_POINTER_Y", 360.0)),
        );
        let positions = env::var("GPUI_TOOLKIT_QA_POINTER_POINTS")
            .ok()
            .into_iter()
            .flat_map(|points| {
                points
                    .split(';')
                    .filter_map(|pair| {
                        let mut values = pair.split(',');
                        let x = values.next()?.parse::<f32>().ok()?;
                        let y = values.next()?.parse::<f32>().ok()?;
                        if x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0 {
                            Some(point(px(x), px(y)))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let positions = if positions.is_empty() {
            vec![default_position]
        } else {
            positions
        };
        let delay = env::var("GPUI_TOOLKIT_QA_POINTER_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(|value| Duration::from_millis(value.clamp(100, 30_000)))
            .unwrap_or_else(|| Duration::from_millis(750));
        self.qa_pointer_task = Some(cx.spawn_in(
            window,
            async move |_this, cx: &mut AsyncWindowContext| {
                cx.background_executor().timer(delay).await;
                let total = positions.len();
                let mut completed = 0;
                let mut failure = None;
                for (index, position) in positions.iter().copied().enumerate() {
                    match cx.update(|window, app| {
                        window.dispatch_event(
                            MouseDownEvent {
                                position,
                                modifiers: Modifiers::default(),
                                button: MouseButton::Left,
                                click_count: 1,
                                first_mouse: false,
                            }
                            .to_platform_input(),
                            app,
                        );
                        window.dispatch_event(
                            MouseUpEvent {
                                position,
                                modifiers: Modifiers::default(),
                                button: MouseButton::Left,
                                click_count: 1,
                            }
                            .to_platform_input(),
                            app,
                        );
                    }) {
                        Ok(()) => completed += 1,
                        Err(error) => {
                            failure = Some(error.to_string());
                            break;
                        }
                    }
                    if index + 1 < total {
                        cx.background_executor()
                            .timer(Duration::from_millis(120))
                            .await;
                    }
                }
                write_qa_json_artifact(
                    "GPUI_TOOLKIT_QA_POINTER_TRACE",
                    &match failure {
                        None => serde_json::json!({
                            "dispatch": "completed",
                            "count": completed,
                        }),
                        Some(error) => serde_json::json!({
                            "dispatch": "failed",
                            "count": completed,
                            "error": error,
                        }),
                    },
                );
            },
        ));
    }

    pub(super) fn render_sidebar(
        &mut self,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> Div {
        let app = self.app.as_ref().expect("render_sidebar called after load");
        div()
            .w(px(240.0))
            .h_full()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.control_gap))
            .p(px(ds.spacing.card_padding))
            .bg(theme.surface)
            .border_r_1()
            .border_color(theme.border)
            .child(
                div()
                    .mb(px(ds.spacing.section_gap))
                    .flex()
                    .flex_col()
                    .gap(px(ds.spacing.grid_unit))
                    .child(
                        div()
                            .text_size(px(ds.typography.large_size))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.text_primary)
                            .child(app.sidebar_title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(ds.typography.small_size))
                            .text_color(theme.text_muted)
                            .child(app.sidebar_subtitle.clone()),
                    ),
            )
            .children(app.sections.iter().map(|section| {
                let selected = section.id == self.current_section;
                let section_id = section.id.clone();
                let bg = if selected {
                    theme.accent
                } else {
                    theme.surface
                };
                let hover_bg = if selected {
                    theme.accent_hover
                } else {
                    theme.surface_hover
                };
                let text = if selected {
                    theme.text_on_accent
                } else {
                    theme.text_primary
                };

                div()
                    .id(ElementId::Name(section_id.clone().into()))
                    .px(px(ds.spacing.control_padding_x))
                    .py(px(ds.spacing.control_padding_y))
                    .rounded(px(ds.corners.md))
                    .cursor_pointer()
                    .bg(bg)
                    .hover(move |style| style.bg(hover_bg))
                    .text_color(text)
                    .child(section.label.clone())
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.select_section(section_id.clone());
                        cx.notify();
                    }))
            }))
    }

    pub(super) fn render_content(
        &mut self,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Temporarily move the app out so a section can be borrowed while the
        // renderer mutates its independent retained caches. This avoids a
        // per-frame deep clone of potentially large IR subtrees.
        let app = self.app.take().expect("render_content called after load");
        let scrollable = app
            .miniapp
            .as_ref()
            .map_or(true, |config| config.scrollable);
        let selected_content = app
            .sections
            .iter()
            .find(|section| section.id == self.current_section)
            .or_else(|| app.sections.first())
            .map(|section| &section.content);
        let content = selected_content
            .map(|node| self.render_node(node, theme, ds, cx))
            .unwrap_or_else(|| {
                self.render_error("Python app did not define any sections", theme, ds)
            });
        self.app_value = serde_json::to_value(&app).ok();
        self.app = Some(app);

        let jobs = self.render_job_panel(theme, ds, cx);
        let scroll_handle = self.content_scroll.clone();
        let persisted_scroll = self.presentation.clone();
        let content = div()
            .id("python-showcase-content")
            .flex_1()
            .h_full()
            .bg(theme.background)
            .p(px(ds.spacing.section_gap * 1.5))
            .child(content)
            .children(jobs);
        if scrollable {
            content
                .overflow_y_scroll()
                .track_scroll(&scroll_handle)
                .on_scroll_wheel(move |event, window, _cx| {
                    let delta = event.delta.pixel_delta(window.line_height());
                    let next_y = scroll_handle.offset().y - delta.y;
                    persisted_scroll.set_scroll_y((-next_y.as_f32()).max(0.0));
                })
        } else {
            content
        }
    }

    pub(super) fn render_node(
        &mut self,
        node: &UiNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            UiNode::Vstack(node) => {
                self.render_stack(node, StackDirection::Vertical, theme, ds, cx)
            }
            UiNode::Hstack(node) => {
                self.render_stack(node, StackDirection::Horizontal, theme, ds, cx)
            }
            UiNode::Wrap(node) => self.render_stack(node, StackDirection::Wrap, theme, ds, cx),
            UiNode::Heading(node) => self.render_heading(node, theme, ds),
            UiNode::Text(node) => self.render_text(node, theme, ds),
            UiNode::Code(node) => self.render_code(node, theme, ds),
            UiNode::SectionHeader(node) => self.render_section_header(node, theme, ds),
            UiNode::Card(node) => self.render_card(node, theme, ds, cx),
            UiNode::Form(node) => self.render_form(node, theme, ds, cx),
            UiNode::Button(node) => self.render_button(node, theme, ds, cx),
            UiNode::Badge(node) => self.render_badge(node, theme, ds),
            UiNode::Metric(node) => self.render_metric(node, theme, ds),
            UiNode::Progress(node) => self.render_progress(node, theme, ds),
            UiNode::Spinner(node) => self.render_spinner(node, theme, ds),
            UiNode::ThinkingOrb(node) => self.render_thinking_orb(node, theme, ds, cx),
            UiNode::Breadcrumbs(node) => self.render_breadcrumbs(node),
            UiNode::Alert(node) => self.render_alert(node),
            UiNode::Toast(node) => self.render_toast(node),
            UiNode::Tooltip(node) => self.render_tooltip(node, theme, ds, cx),
            UiNode::EmptyState(node) => self.render_empty_state(node, theme, ds, cx),
            UiNode::Dialog(node) => self.render_dialog(node, theme, ds, cx),
            UiNode::ConfirmDialog(node) => self.render_confirm_dialog(node, cx),
            UiNode::Menu(node) => self.render_menu(node, cx),
            UiNode::MenuBar(node) => self.render_menu_bar(node, cx),
            UiNode::ContextMenu(node) => self.render_context_menu(node, cx),
            UiNode::Popover(node) => self.render_popover(node, theme, ds, cx),
            UiNode::Tabs(node) => self.render_tabs(node, theme, ds, cx),
            UiNode::Stepper(node) => self.render_stepper(node, theme, ds),
            UiNode::Accordion(node) => self.render_accordion(node, theme, ds, cx),
            UiNode::ListEditor(node) => self.render_list_editor(node, theme, ds),
            UiNode::Table(node) => self.render_table(node, theme, ds, cx),
            UiNode::Divider(node) => self.render_divider(node, theme),
            UiNode::Spacer(node) => self.render_spacer(node),
            UiNode::Chart(node) => self.render_chart(node, theme, ds, cx),
            UiNode::Scene3d(node) => self.render_scene3d(node, theme, ds, cx),
            UiNode::MeshPlot(node) => self.render_meshplot(node, theme, ds, cx),
            UiNode::TextInput(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::TextInput(node) => self.render_text_input(node, theme, ds, cx),
            UiNode::NumberInput(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::NumberInput(node) => self.render_number_input(node, theme, ds, cx),
            UiNode::Slider(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::Slider(node) => self.render_slider(node, theme, ds),
            UiNode::AudioPotentiometer(node) => self.render_audio_potentiometer(node),
            UiNode::AudioVerticalSlider(node) => self.render_audio_vertical_slider(node),
            UiNode::AudioVolumeKnob(node) => self.render_audio_volume_knob(node),
            UiNode::AudioHorizontalMeter(node) => self.render_audio_horizontal_meter(node),
            UiNode::AudioLevelMeter(node) => self.render_audio_level_meter(node),
            UiNode::AudioSpectrum(node) => self.render_audio_spectrum(node),
            UiNode::Select(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::Select(node) => self.render_select(node, theme, ds),
            UiNode::ColorPicker(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::ColorPicker(node) => self.render_color_picker(node, theme, ds, cx),
            UiNode::PathInput(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::PathInput(node) => self.render_path_input(node, theme, ds, cx),
            UiNode::Checkbox(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::Checkbox(node) => self.render_checkbox(node, theme, ds),
            UiNode::Toggle(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::Toggle(node) => self.render_toggle(node, theme, ds),
        }
    }

    fn render_color_picker(
        &mut self,
        node: &ColorPickerNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let color = Color::from_hex_string(&node.value).unwrap_or_else(|| Color::from_hex(0));
        self.color_picker_actions
            .insert(node.id.clone(), node.action.clone());
        let picker = self
            .color_pickers
            .entry(node.id.clone())
            .or_insert_with(|| {
                cx.new(|_| {
                    ColorPickerView::new(
                        node.label.clone().unwrap_or_else(|| node.id.clone()),
                        color,
                    )
                })
            })
            .clone();
        if !self.color_picker_subscriptions.contains_key(&node.id) {
            let id = node.id.clone();
            let subscription = cx.observe(&picker, move |this, picker, cx| {
                let Some(sink) = this.session.as_ref().map(|session| session.event_sink()) else {
                    return;
                };
                let color = picker.read(cx).color().to_hex_string();
                let action = this.color_picker_actions.get(&id).cloned().flatten();
                let _ = sink.dispatch(
                    id.clone(),
                    "change",
                    action,
                    serde_json::json!({ "value": color }),
                );
            });
            self.color_picker_subscriptions
                .insert(node.id.clone(), subscription);
        }
        if node.disabled {
            return self.present_form_control(
                div()
                    .text_color(theme.text_muted)
                    .child("Color picker disabled")
                    .into_any_element(),
                &node.presentation,
                theme,
                ds,
            );
        }
        self.present_form_control(picker.into_any_element(), &node.presentation, theme, ds)
    }

    pub(super) fn render_stack(
        &mut self,
        node: &StackNode,
        direction: StackDirection,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let gap = px(node.gap.unwrap_or(ds.spacing.control_gap));
        let mut element = div().flex().gap(gap).children(
            node.children
                .iter()
                .map(|child| self.render_node(child, theme, ds, cx)),
        );
        element = match direction {
            StackDirection::Vertical => element.flex_col(),
            StackDirection::Horizontal => element.flex_row(),
            StackDirection::Wrap => element.flex_row().flex_wrap(),
        };
        apply_size(element, node.width, node.height).into_any_element()
    }

    fn render_form(
        &mut self,
        node: &FormNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut form = div().flex().flex_col().gap(px(ds.spacing.control_gap));
        // Build controls first so the summary can reference their retained focus handles
        // in the same frame while still rendering above the controls.
        let children = node
            .children
            .iter()
            .map(|child| self.render_node(child, theme, ds, cx))
            .collect::<Vec<_>>();
        if let Some(label) = &node.label {
            form = form.child(
                div()
                    .text_size(px(ds.typography.large_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(label.clone()),
            );
        }
        if !node.errors.is_empty() {
            let mut summary = div()
                .flex()
                .flex_col()
                .gap(px(ds.spacing.grid_unit))
                .p(px(ds.spacing.control_padding_x))
                .rounded(px(ds.corners.sm))
                .bg(theme.alert_error_bg)
                .text_color(theme.error)
                .child(format!(
                    "{} validation issue{}",
                    node.errors.len(),
                    if node.errors.len() == 1 { "" } else { "s" }
                ));
            if let Some(handle) = node
                .errors
                .first()
                .and_then(|error| self.form_focus.get(&error.control_id))
                .cloned()
            {
                summary = summary.child(
                    div()
                        .id(stable_element_id(format_args!(
                            "python-form-focus-first-{}",
                            node.id
                        )))
                        .cursor_pointer()
                        .child("Focus first invalid control")
                        .on_click(move |_, window, cx| handle.focus(window, cx)),
                );
            }
            for (index, error) in node.errors.iter().enumerate() {
                let entry = div()
                    .id(stable_element_id(format_args!(
                        "python-form-error-{}-{index}",
                        node.id
                    )))
                    .cursor_pointer()
                    .child(format!("{}: {}", error.control_id, error.message));
                summary = if let Some(handle) = self.form_focus.get(&error.control_id).cloned() {
                    summary.child(entry.on_click(move |_, window, cx| handle.focus(window, cx)))
                } else {
                    summary.child(entry)
                };
            }
            form = form.child(summary);
        }
        form.children(children).into_any_element()
    }

    pub(super) fn render_heading(
        &self,
        node: &TextNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let size = match node.level.unwrap_or(1) {
            1 => ds.typography.large_size + 6.0,
            2 => ds.typography.large_size + 2.0,
            _ => ds.typography.large_size,
        };
        div()
            .text_size(px(size))
            .font_weight(FontWeight::BOLD)
            .text_color(theme.text_primary)
            .child(node.text.clone())
            .into_any_element()
    }

    pub(super) fn render_text(
        &self,
        node: &TextNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        div()
            .text_size(px(ds.typography.small_size))
            .text_color(tone_color(&node.tone, theme))
            .child(node.text.clone())
            .into_any_element()
    }

    pub(super) fn render_code(
        &self,
        node: &TextNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        div()
            .px(px(ds.spacing.control_padding_x))
            .py(px(ds.spacing.control_padding_y))
            .rounded(px(ds.corners.sm))
            .bg(theme.muted)
            .text_size(px(ds.typography.small_size))
            .text_color(theme.code_text)
            .child(node.text.clone())
            .into_any_element()
    }

    pub(super) fn render_section_header(
        &self,
        node: &SectionHeaderNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.grid_unit))
            .child(
                div()
                    .text_size(px(ds.typography.large_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(node.title.clone()),
            )
            .child(
                div()
                    .text_size(px(ds.typography.small_size))
                    .text_color(theme.text_secondary)
                    .child(node.subtitle.clone()),
            )
            .into_any_element()
    }

    pub(super) fn render_card(
        &mut self,
        node: &CardNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut element = div()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.control_gap))
            .p(px(ds.spacing.card_padding))
            .bg(theme.surface)
            .rounded(px(ds.corners.md))
            .border_1()
            .border_color(theme.border);

        if let Some(title) = &node.title {
            element = element.child(
                div()
                    .text_size(px(ds.typography.large_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(title.clone()),
            );
        }

        apply_size(
            element.children(
                node.children
                    .iter()
                    .map(|child| self.render_node(child, theme, ds, cx)),
            ),
            node.width,
            node.height,
        )
        .into_any_element()
    }

    pub(super) fn render_button(
        &self,
        node: &ButtonNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let bg = if node.selected {
            theme.accent
        } else {
            theme.surface_hover
        };
        let text = if node.disabled {
            theme.text_muted
        } else if node.selected {
            theme.text_on_accent
        } else {
            theme.text_primary
        };

        let element = apply_native_accessibility(
            div().id(ElementId::Name(
                node.id.clone().unwrap_or_else(|| node.label.clone()).into(),
            )),
            node.label.clone(),
            &AriaProps::with_role(AriaRole::Button)
                .maybe_state(node.disabled, AriaState::Disabled)
                .maybe_state(node.selected, AriaState::Pressed(true)),
        )
        .focusable()
        .px(px(ds.spacing.control_padding_x))
        .py(px(ds.spacing.control_padding_y))
        .rounded(px(ds.corners.md))
        .bg(bg)
        .text_color(text)
        .cursor_pointer()
        .child(node.label.clone());

        if node.disabled {
            return element.into_any_element();
        }

        if let Some(section_id) = node
            .action
            .as_deref()
            .and_then(|action| action.strip_prefix("select:"))
        {
            let section_id = section_id.to_string();
            let key_section_id = section_id.clone();
            return element
                .on_click(cx.listener(move |this, _, _window, cx| {
                    this.select_section(section_id.clone());
                    cx.notify();
                }))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.select_section(key_section_id.clone());
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .into_any_element();
        }

        if let (Some(action), Some(sink), Some(node_id)) = (
            node.action.clone(),
            self.session.as_ref().map(|session| session.event_sink()),
            node.id.clone(),
        ) {
            let key_sink = sink.clone();
            let key_node_id = node_id.clone();
            let key_action = action.clone();
            return element
                .on_click(move |_, _, _| {
                    let _ =
                        sink.dispatch(node_id.clone(), "click", Some(action.clone()), Value::Null);
                })
                .on_key_down(move |event: &KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        let _ = key_sink.dispatch(
                            key_node_id.clone(),
                            "click",
                            Some(key_action.clone()),
                            Value::Null,
                        );
                        cx.stop_propagation();
                    }
                })
                .into_any_element();
        }

        element.into_any_element()
    }

    fn render_text_input(
        &mut self,
        node: &TextInputNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = node.id.clone();
        let focus_handle = self
            .form_focus
            .entry(node.id.clone())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        let label = node.label.as_ref().map(|label| {
            if node.required {
                format!("{label} *")
            } else {
                label.clone()
            }
        });
        let mut input = Input::new(stable_element_id(format_args!("python-form-{id}")))
            .value(node.value.clone())
            .disabled(node.disabled)
            .readonly(node.read_only)
            .password(node.password)
            .focus_handle(focus_handle)
            .aria_label(label.clone().unwrap_or_else(|| id.clone()));
        if let Some(label) = label {
            input = input.label(label);
        }
        if let Some(placeholder) = &node.placeholder {
            input = input.placeholder(placeholder.clone());
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            input = input.on_text_change(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    serde_json::json!({ "value": value }),
                );
            });
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            input = input.on_edit_start(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "focus", action.clone(), Value::Null);
            });
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            input = input.on_edit_end(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "blur",
                    action.clone(),
                    serde_json::json!({ "value": value }),
                );
            });
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.commit_action.clone();
            input = input.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "commit",
                    action.clone(),
                    serde_json::json!({ "value": value }),
                );
            });
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.selection_action.clone();
            input = input.on_selection_change(move |selection, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "selection",
                    action.clone(),
                    serde_json::json!({
                        "start": selection.start,
                        "end": selection.end,
                        "reversed": selection.reversed,
                    }),
                );
            });
        }
        self.present_form_control(
            self.wrap_form_control(
                input.into_any_element(),
                node.validation.as_ref(),
                theme,
                ds,
            ),
            &node.presentation,
            theme,
            ds,
        )
    }

    fn render_number_input(
        &mut self,
        node: &NumberInputNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = node.id.clone();
        let focus_handle = self
            .form_focus
            .entry(node.id.clone())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        if let Some(raw_value) = node.value.as_str() {
            let mut input = Input::new(stable_element_id(format_args!("python-form-{id}")))
                .value(raw_value.to_string())
                .disabled(node.disabled)
                .readonly(node.read_only)
                .focus_handle(focus_handle.clone())
                .aria_label(node.label.clone().unwrap_or_else(|| id.clone()));
            if let Some(label) = &node.label {
                input = input.label(if node.required {
                    format!("{label} *")
                } else {
                    label.clone()
                });
            }
            if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
                let node_id = node.id.clone();
                let action = node.action.clone();
                input = input.on_text_change(move |value, _, _| {
                    let _ = sink.dispatch(
                        node_id.clone(),
                        "change",
                        action.clone(),
                        serde_json::json!({ "value": value, "intermediate": true }),
                    );
                });
            }
            if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
                let node_id = node.id.clone();
                let action = node.commit_action.clone();
                input = input.on_change(move |value, _, _| {
                    let _ = sink.dispatch(
                        node_id.clone(),
                        "commit",
                        action.clone(),
                        serde_json::json!({ "value": value }),
                    );
                });
            }
            let control = if let Some(unit) = &node.unit {
                div()
                    .flex()
                    .items_end()
                    .gap(px(ds.spacing.control_gap))
                    .child(input)
                    .child(
                        div()
                            .pb(px(ds.spacing.control_padding_y))
                            .text_color(theme.text_muted)
                            .child(unit.clone()),
                    )
                    .into_any_element()
            } else {
                input.into_any_element()
            };
            return self.present_form_control(
                self.wrap_form_control(control, node.validation.as_ref(), theme, ds),
                &node.presentation,
                theme,
                ds,
            );
        }
        let mut input = NumberInput::new(stable_element_id(format_args!("python-form-{id}")))
            .range(
                node.minimum.unwrap_or(f64::NEG_INFINITY),
                node.maximum.unwrap_or(f64::INFINITY),
            )
            .value(node.value.as_f64().unwrap_or_default())
            .step(node.step.unwrap_or(1.0))
            .disabled(node.disabled || node.read_only)
            .focus_handle(focus_handle)
            .aria_label(node.label.clone().unwrap_or_else(|| id.clone()));
        if let Some(label) = &node.label {
            input = input.label(label.clone());
        }
        if let Some(unit) = &node.unit {
            input = input.unit(unit.clone());
        }
        if let Some(precision) = node.precision {
            input = input.decimals(precision.into());
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            input = input.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    serde_json::json!({ "value": value }),
                );
            });
        }
        self.present_form_control(
            self.wrap_form_control(
                input.into_any_element(),
                node.validation.as_ref(),
                theme,
                ds,
            ),
            &node.presentation,
            theme,
            ds,
        )
    }

    fn render_slider(&self, node: &SliderNode, theme: &Theme, ds: &DesignSystem) -> AnyElement {
        let id = node.id.clone();
        let mut slider = Slider::new(stable_element_id(format_args!("python-slider-{id}")))
            .range(node.minimum, node.maximum)
            .value(node.value)
            .disabled(node.disabled)
            .show_value(node.show_value)
            .aria_label(node.label.clone().unwrap_or_else(|| id.clone()));
        if let Some(label) = &node.label {
            slider = slider.label(label.clone());
        }
        if let Some(step) = node.step {
            slider = slider.step(step);
        }
        if let Some(width) = node.presentation.width {
            slider = slider.width(width);
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            slider = slider.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    serde_json::json!({ "value": value }),
                );
            });
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.commit_action.clone();
            slider = slider.on_drag_end(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "commit",
                    action.clone(),
                    serde_json::json!({ "value": value }),
                );
            });
        }
        self.present_form_control(slider.into_any_element(), &node.presentation, theme, ds)
    }

    fn render_audio_potentiometer(&self, node: &AudioControlNode) -> AnyElement {
        let size = match node.size.as_str() {
            "xs" => gpui_audio_kit::PotentiometerSize::Xs,
            "sm" => gpui_audio_kit::PotentiometerSize::Sm,
            "lg" => gpui_audio_kit::PotentiometerSize::Lg,
            _ => gpui_audio_kit::PotentiometerSize::Md,
        };
        let scale = if node.scale == "logarithmic" {
            gpui_audio_kit::AudioScale::Logarithmic
        } else {
            gpui_audio_kit::AudioScale::Linear
        };
        let mut control = Potentiometer::new(stable_element_id(format_args!(
            "python-audio-potentiometer-{}",
            node.id
        )))
        .value(node.value)
        .min(node.minimum)
        .max(node.maximum)
        .label(node.label.clone())
        .unit(node.unit.clone())
        .size(size)
        .scale(scale)
        .selected(node.selected)
        .disabled(node.disabled)
        .aria_label(
            node.aria_label
                .clone()
                .unwrap_or_else(|| node.label.clone()),
        );
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let id = node.id.clone();
            control = control.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    id.clone(),
                    "preview",
                    Some(action.clone()),
                    serde_json::json!({"value": value}),
                );
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.commit_action.clone(),
        ) {
            let id = node.id.clone();
            control = control.on_commit(move |value, _, _| {
                let _ = sink.dispatch(
                    id.clone(),
                    "commit",
                    Some(action.clone()),
                    serde_json::json!({"value": value}),
                );
            });
        }
        control.into_any_element()
    }

    fn render_audio_vertical_slider(&self, node: &AudioControlNode) -> AnyElement {
        let size = match node.size.as_str() {
            "sm" | "xs" => gpui_audio_kit::VerticalSliderSize::Sm,
            "lg" => gpui_audio_kit::VerticalSliderSize::Lg,
            _ => gpui_audio_kit::VerticalSliderSize::Md,
        };
        let scale = if node.scale == "logarithmic" {
            gpui_audio_kit::AudioScale::Logarithmic
        } else {
            gpui_audio_kit::AudioScale::Linear
        };
        let mut control = VerticalSlider::new(stable_element_id(format_args!(
            "python-audio-vertical-slider-{}",
            node.id
        )))
        .value(node.value)
        .min(node.minimum)
        .max(node.maximum)
        .label(node.label.clone())
        .unit(node.unit.clone())
        .size(size)
        .scale(scale)
        .selected(node.selected)
        .disabled(node.disabled)
        .peak(node.peak)
        .aria_label(
            node.aria_label
                .clone()
                .unwrap_or_else(|| node.label.clone()),
        );
        if node.with_ticks {
            control = control.with_ticks();
        }
        if let Some(height) = node.height {
            control = control.height(height);
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let id = node.id.clone();
            control = control.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    id.clone(),
                    "preview",
                    Some(action.clone()),
                    serde_json::json!({"value": value}),
                );
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.commit_action.clone(),
        ) {
            let id = node.id.clone();
            control = control.on_commit(move |value, _, _| {
                let _ = sink.dispatch(
                    id.clone(),
                    "commit",
                    Some(action.clone()),
                    serde_json::json!({"value": value}),
                );
            });
        }
        control.into_any_element()
    }

    fn render_audio_volume_knob(&self, node: &AudioControlNode) -> AnyElement {
        let mut control = VolumeKnob::new()
            .id(stable_element_id(format_args!(
                "python-audio-volume-knob-{}",
                node.id
            )))
            .value(node.value as f32)
            .label(node.label.clone())
            .muted(node.muted)
            .size(px(node.width.unwrap_or(80.0)))
            .aria_label(
                node.aria_label
                    .clone()
                    .unwrap_or_else(|| node.label.clone()),
            );
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let id = node.id.clone();
            control = control.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    id.clone(),
                    "preview",
                    Some(action.clone()),
                    serde_json::json!({"value": value}),
                );
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.commit_action.clone(),
        ) {
            let id = node.id.clone();
            control = control.on_commit(move |value, _, _| {
                let _ = sink.dispatch(
                    id.clone(),
                    "commit",
                    Some(action.clone()),
                    serde_json::json!({"value": value}),
                );
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.mute_action.clone(),
        ) {
            let id = node.id.clone();
            control = control.on_mute_toggle(move |muted, _, _| {
                let _ = sink.dispatch(
                    id.clone(),
                    "mute_toggle",
                    Some(action.clone()),
                    serde_json::json!({"muted": muted}),
                );
            });
        }
        control.into_any_element()
    }

    fn render_audio_horizontal_meter(&self, node: &AudioMeterNode) -> AnyElement {
        let tick_config = gpui_audio_kit::TickConfig::db_linear(-60.0, 0.0);
        let streamed = node
            .stream_id
            .as_deref()
            .and_then(|id| self.audio_frames.get(id))
            .filter(|frame| frame.frame_kind == AudioFrameKind::Meter)
            .and_then(|frame| frame.meter_levels())
            .map(|levels| {
                levels
                    .iter()
                    .map(|value| f64::from(*value))
                    .collect::<Vec<_>>()
            });
        let levels = streamed.as_deref().unwrap_or(&node.levels);
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .children(levels.iter().enumerate().map(|(index, level)| {
                let label = node
                    .channel_names
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| format!("Channel {}", index + 1));
                gpui_audio_kit::render_horizontal_meter_bar(
                    label,
                    *level,
                    &tick_config,
                    gpui_audio_kit::HorizontalMeterTheme::default(),
                )
            }))
            .into_any_element()
    }

    fn render_audio_level_meter(&self, node: &AudioMeterNode) -> AnyElement {
        let stream = node
            .stream_id
            .as_deref()
            .and_then(|id| self.audio_frames.get(id))
            .filter(|frame| frame.frame_kind == AudioFrameKind::Meter);
        let streamed_levels = stream.and_then(|frame| frame.meter_levels()).map(|levels| {
            levels
                .iter()
                .map(|value| f64::from(*value))
                .collect::<Vec<_>>()
        });
        let streamed_peaks = stream.and_then(|frame| frame.meter_peaks()).map(|peaks| {
            peaks
                .iter()
                .map(|value| f64::from(*value))
                .collect::<Vec<_>>()
        });
        let levels = streamed_levels.as_deref().unwrap_or(&node.levels);
        let peaks = streamed_peaks.as_deref().unwrap_or(&node.peaks);
        div()
            .flex()
            .gap(px(8.0))
            .children(levels.iter().enumerate().map(|(index, level)| {
                let label = node
                    .channel_names
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| format!("Channel {}", index + 1));
                let mut meter =
                    LevelMeterElement::new(*level, label).width(px(node.width.unwrap_or(16.0)));
                if let Some(peak) = peaks.get(index) {
                    meter = meter.peak(*peak);
                }
                meter
            }))
            .into_any_element()
    }

    fn render_audio_spectrum(&self, node: &AudioSpectrumNode) -> AnyElement {
        let stream = node
            .stream_id
            .as_deref()
            .and_then(|id| self.audio_frames.get(id))
            .filter(|frame| frame.frame_kind == AudioFrameKind::Spectrum);
        let magnitudes = stream
            .map(|frame| frame.values.clone())
            .unwrap_or_else(|| node.magnitudes.clone());
        let minimum_frequency = stream
            .and_then(|frame| frame.minimum_frequency)
            .map(|value| value as f32)
            .unwrap_or(node.minimum_frequency);
        let maximum_frequency = stream
            .and_then(|frame| frame.maximum_frequency)
            .map(|value| value as f32)
            .unwrap_or(node.maximum_frequency);
        let mut spectrum = SpectrumElement::new(Arc::<[f32]>::from(magnitudes))
            .frequency_range(minimum_frequency, maximum_frequency)
            .smoothing(node.smoothing);
        if !node.previous.is_empty() {
            spectrum = spectrum.previous(Arc::<[f32]>::from(node.previous.clone()));
        }
        if let Some(height) = node.height {
            spectrum = spectrum.height(px(height));
        }
        if let Some(gap) = node.bar_gap {
            spectrum = spectrum.bar_gap(px(gap));
        }
        spectrum.into_any_element()
    }

    fn render_select(&self, node: &SelectNode, theme: &Theme, ds: &DesignSystem) -> AnyElement {
        let id = node.id.clone();
        let choices: Vec<(String, Value)> = node
            .options
            .iter()
            .map(|option| (select_wire_value(&option.value), option.value.clone()))
            .collect();
        let options = node
            .options
            .iter()
            .map(|option| {
                gpui_ui_kit::select::SelectOption::new(
                    select_wire_value(&option.value),
                    option.label.clone(),
                )
                .disabled(option.disabled)
            })
            .collect();
        let mut select = Select::new(stable_element_id(format_args!("python-form-{id}")))
            .options(options)
            .selected(select_wire_value(&node.value))
            .disabled(node.disabled)
            .aria_label(node.label.clone().unwrap_or_else(|| id.clone()));
        if let Some(label) = &node.label {
            select = select.label(label.clone());
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            select = select.on_change(move |value, _, _| {
                let selected = choices
                    .iter()
                    .find(|(wire_value, _)| wire_value == value.as_ref())
                    .map(|(_, value)| value.clone())
                    .unwrap_or_else(|| Value::String(value.to_string()));
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    serde_json::json!({ "value": selected }),
                );
            });
        }
        self.present_form_control(
            self.wrap_form_control(select.into_any_element(), None, theme, ds),
            &node.presentation,
            theme,
            ds,
        )
    }

    fn render_path_input(
        &self,
        node: &PathInputNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = node.id.clone();
        let label = node.label.as_ref().map(|label| {
            if node.required {
                format!("{label} *")
            } else {
                label.clone()
            }
        });
        let mut input = Input::new(stable_element_id(format_args!("python-path-{id}")))
            .value(node.value.clone())
            .disabled(node.disabled)
            .readonly(node.read_only)
            .aria_label(label.clone().unwrap_or_else(|| id.clone()));
        if let Some(label) = label {
            input = input.label(label);
        }
        if let Some(placeholder) = &node.placeholder {
            input = input.placeholder(placeholder.clone());
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            let mode = node.mode.clone();
            let filters = node.filters.clone();
            let must_exist = node.must_exist;
            input = input.on_text_change(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    path_event_payload(Path::new(&value), &mode, &filters, must_exist, "manual"),
                );
            });
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.commit_action.clone();
            let mode = node.mode.clone();
            let filters = node.filters.clone();
            let must_exist = node.must_exist;
            input = input.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "commit",
                    action.clone(),
                    path_event_payload(Path::new(value), &mode, &filters, must_exist, "manual"),
                );
            });
        }

        let mut row = div()
            .flex()
            .items_end()
            .gap(px(ds.spacing.control_gap))
            .child(input);
        if !node.disabled {
            let node_id = node.id.clone();
            let mode = node.mode.clone();
            let action = node.action.clone();
            let filters = node.filters.clone();
            let must_exist = node.must_exist;
            let initial_path = node.value.clone();
            let sink = self.session.as_ref().map(|session| session.event_sink());
            row = row.child(
                div()
                    .id(stable_element_id(format_args!(
                        "python-path-browse-{node_id}"
                    )))
                    .px(px(ds.spacing.control_padding_x))
                    .py(px(ds.spacing.control_padding_y))
                    .rounded(px(ds.corners.md))
                    .bg(theme.surface_hover)
                    .text_color(theme.text_primary)
                    .cursor_pointer()
                    .child("Browse…")
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let Some(sink) = sink.clone() else { return };
                        let picked_mode = mode.clone();
                        let picked_filters = filters.clone();
                        let picked_node_id = node_id.clone();
                        let picked_action = action.clone();
                        let picked_initial_path = initial_path.clone();
                        let receiver = if picked_mode == "save_file" {
                            let initial = PathBuf::from(&picked_initial_path);
                            let directory = if initial.is_dir() {
                                initial
                            } else {
                                initial
                                    .parent()
                                    .map(Path::to_path_buf)
                                    .unwrap_or_else(|| PathBuf::from("."))
                            };
                            let suggested_name = Path::new(&picked_initial_path)
                                .file_name()
                                .and_then(|name| name.to_str());
                            cx.prompt_for_new_path(&directory, suggested_name)
                        } else {
                            let receiver = cx.prompt_for_paths(PathPromptOptions {
                                files: picked_mode == "open_file",
                                directories: picked_mode == "directory",
                                multiple: false,
                                prompt: None,
                                initial_directory: Path::new(&picked_initial_path)
                                    .parent()
                                    .filter(|path| path.is_dir())
                                    .map(Path::to_path_buf),
                                extensions: picked_filters
                                    .iter()
                                    .flat_map(|filter| filter.extensions.iter())
                                    .map(|extension| {
                                        SharedString::from(extension.trim_start_matches('.'))
                                    })
                                    .collect(),
                            });
                            cx.spawn(async move |_, _| {
                                let payload = match receiver.await {
                                    Ok(Ok(Some(paths))) => paths
                                        .first()
                                        .map(|path| {
                                            path_event_payload(
                                                path,
                                                &picked_mode,
                                                &picked_filters,
                                                must_exist,
                                                "browse",
                                            )
                                        })
                                        .unwrap_or_else(|| serde_json::json!({"cancelled": true})),
                                    Ok(Ok(None)) => serde_json::json!({"cancelled": true}),
                                    Ok(Err(error)) => {
                                        serde_json::json!({"error": error.to_string()})
                                    }
                                    Err(error) => serde_json::json!({"error": error.to_string()}),
                                };
                                let event = if payload.get("valid") == Some(&Value::Bool(false)) {
                                    "browse_rejected"
                                } else if payload.get("cancelled") == Some(&Value::Bool(true)) {
                                    "browse_cancelled"
                                } else {
                                    "change"
                                };
                                let _ =
                                    sink.dispatch(picked_node_id, event, picked_action, payload);
                            })
                            .detach();
                            return;
                        };
                        cx.spawn(async move |_, _| {
                            let payload = match receiver.await {
                                Ok(Ok(Some(path))) => path_event_payload(
                                    &path,
                                    &picked_mode,
                                    &picked_filters,
                                    must_exist,
                                    "browse",
                                ),
                                Ok(Ok(None)) => serde_json::json!({"cancelled": true}),
                                Ok(Err(error)) => serde_json::json!({"error": error.to_string()}),
                                Err(error) => serde_json::json!({"error": error.to_string()}),
                            };
                            let event = if payload.get("valid") == Some(&Value::Bool(false)) {
                                "browse_rejected"
                            } else if payload.get("cancelled") == Some(&Value::Bool(true)) {
                                "browse_cancelled"
                            } else {
                                "change"
                            };
                            let _ = sink.dispatch(picked_node_id, event, picked_action, payload);
                        })
                        .detach();
                    })),
            );
        }

        let mut field = div()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.grid_unit))
            .child(row);
        if !node.recent_values.is_empty() {
            let action = node.action.clone();
            let node_id = node.id.clone();
            let sink = self.session.as_ref().map(|session| session.event_sink());
            field = field.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(ds.spacing.grid_unit))
                    .children(node.recent_values.iter().enumerate().map(|(index, path)| {
                        let path = path.clone();
                        let sink = sink.clone();
                        let action = action.clone();
                        let node_id = node_id.clone();
                        div()
                            .id(stable_element_id(format_args!(
                                "python-path-recent-{node_id}-{index}"
                            )))
                            .px(px(ds.spacing.grid_unit))
                            .py(px(ds.spacing.grid_unit / 2.0))
                            .rounded(px(ds.corners.sm))
                            .bg(theme.surface_hover)
                            .text_color(theme.text_secondary)
                            .cursor_pointer()
                            .child(path.clone())
                            .on_click(move |_, _, _| {
                                if let Some(sink) = &sink {
                                    let _ = sink.dispatch(
                                        node_id.clone(),
                                        "change",
                                        action.clone(),
                                        serde_json::json!({"value": path, "source": "recent"}),
                                    );
                                }
                            })
                    })),
            );
        }
        self.present_form_control(
            self.wrap_form_control(
                field.into_any_element(),
                node.validation.as_ref(),
                theme,
                ds,
            ),
            &node.presentation,
            theme,
            ds,
        )
    }

    fn render_checkbox(
        &self,
        node: &BooleanInputNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let mut checkbox =
            Checkbox::new(stable_element_id(format_args!("python-form-{}", node.id)))
                .checked(node.value)
                .indeterminate(node.indeterminate)
                .label(node.label.clone())
                .disabled(node.disabled)
                .aria_label(node.label.clone());
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            checkbox = checkbox.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    serde_json::json!({ "value": value }),
                );
            });
        }
        self.present_form_control(checkbox.into_any_element(), &node.presentation, theme, ds)
    }

    fn render_toggle(
        &self,
        node: &BooleanInputNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let mut toggle = Toggle::new(stable_element_id(format_args!("python-form-{}", node.id)))
            .checked(node.value)
            .label(node.label.clone())
            .disabled(node.disabled)
            .aria_label(node.label.clone());
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            toggle = toggle.on_change(move |value, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    serde_json::json!({ "value": value }),
                );
            });
        }
        self.present_form_control(toggle.into_any_element(), &node.presentation, theme, ds)
    }

    fn wrap_form_control(
        &self,
        control: AnyElement,
        validation: Option<&gpui_python_runtime::ui_ir::ValidationState>,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let mut field = div()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.grid_unit))
            .child(control);
        if let Some(validation) = validation {
            let color = if validation.severity.eq_ignore_ascii_case("error") {
                theme.error
            } else {
                theme.text_muted
            };
            field = field.child(
                div()
                    .text_size(px(ds.typography.small_size))
                    .text_color(color)
                    .child(validation.message.clone()),
            );
        }
        field.into_any_element()
    }

    fn present_form_control(
        &self,
        control: AnyElement,
        presentation: &gpui_python_runtime::ui_ir::FormControlProps,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let mut field = div().flex().flex_col().gap(px(ds.spacing.grid_unit));
        if let Some(width) = presentation.width {
            field = field.w(px(width));
        }
        field = field.child(control);
        if let Some(help) = &presentation.help {
            field = field.child(
                div()
                    .text_size(px(ds.typography.small_size))
                    .text_color(theme.text_muted)
                    .child(help.clone()),
            );
        }
        field.into_any_element()
    }

    fn render_job_panel(
        &mut self,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let mut jobs = self.jobs.iter().cloned().collect::<Vec<_>>();
        jobs.sort_by(|left, right| left.id.cmp(&right.id));
        if jobs.is_empty() {
            return None;
        }

        let filter_label = match self.job_log_filter {
            None => "All",
            Some(LogSeverity::Error) => "Errors",
            Some(LogSeverity::Warn) => "Warnings",
            Some(LogSeverity::Info) => "Info",
            Some(LogSeverity::Debug) => "Debug",
            Some(LogSeverity::Trace) => "Trace",
        };
        let filter_button = |label: &'static str, filter: Option<LogSeverity>| {
            let selected = self.job_log_filter == filter;
            div()
                .id(stable_element_id(format_args!(
                    "python-job-log-filter-{label}"
                )))
                .px(px(ds.spacing.grid_unit))
                .py(px(ds.spacing.grid_unit / 2.0))
                .rounded(px(ds.corners.sm))
                .bg(if selected {
                    theme.accent
                } else {
                    theme.surface_hover
                })
                .text_color(if selected {
                    theme.text_on_accent
                } else {
                    theme.text_secondary
                })
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.job_log_filter = filter.clone();
                    cx.notify();
                }))
        };

        Some(
            div()
                .mt(px(ds.spacing.section_gap))
                .p(px(ds.spacing.card_padding))
                .flex()
                .flex_col()
                .gap(px(ds.spacing.control_gap))
                .bg(theme.surface)
                .rounded(px(ds.corners.md))
                .border_1()
                .border_color(theme.border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_color(theme.text_primary)
                                .child("Simulation jobs"),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(ds.spacing.grid_unit))
                                .text_size(px(ds.typography.small_size))
                                .text_color(theme.text_muted)
                                .child(format!("Log filter: {filter_label}")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(ds.spacing.grid_unit))
                        .child(filter_button("All", None))
                        .child(filter_button("Errors", Some(LogSeverity::Error)))
                        .child(filter_button("Warnings", Some(LogSeverity::Warn)))
                        .child(filter_button("Info", Some(LogSeverity::Info))),
                )
                .children(jobs.into_iter().map(|job| {
                    let status = job
                        .message
                        .clone()
                        .unwrap_or_else(|| format!("{:?}", job.state).to_lowercase());
                    let progress = match (job.completed, job.total) {
                        (Some(completed), Some(total)) if total > 0 => {
                            format!("{completed} / {total}")
                        }
                        _ => "working…".into(),
                    };
                    let cancel = !job.state.is_terminal() && job.state != JobState::Cancelling;
                    let job_id = job.id.clone();
                    let is_paused = self.paused_job_logs.contains_key(&job.id);
                    let visible_logs = self
                        .paused_job_logs
                        .get(&job.id)
                        .cloned()
                        .unwrap_or_else(|| job.logs().cloned().collect::<Vec<_>>());
                    let visible_logs = visible_logs
                        .into_iter()
                        .filter(|line| {
                            self.job_log_filter
                                .as_ref()
                                .is_none_or(|filter| line.severity == *filter)
                        })
                        .collect::<Vec<_>>();
                    let copied_logs = visible_logs
                        .iter()
                        .map(|line| format!("[{:?}] {}", line.severity, line.message))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let log_scroll = self
                        .job_log_scrolls
                        .entry(job_id.clone())
                        .or_insert_with(UniformListScrollHandle::new)
                        .clone();
                    if !is_paused {
                        log_scroll.scroll_to_bottom();
                    }
                    let log_lines = visible_logs.clone();
                    let log_text_size = ds.typography.small_size;
                    let log_error = theme.error;
                    let log_text = theme.text_muted;
                    let pause_job_id = job_id.clone();
                    let mut row = div()
                        .flex()
                        .flex_col()
                        .gap(px(ds.spacing.grid_unit))
                        .p(px(ds.spacing.control_padding_y))
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text_primary)
                                        .child(job.id.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(ds.typography.small_size))
                                        .text_color(theme.text_secondary)
                                        .child(progress),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(ds.typography.small_size))
                                .text_color(theme.text_secondary)
                                .child(status),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(px(ds.spacing.grid_unit))
                                .child(
                                    div()
                                        .id(stable_element_id(format_args!(
                                            "python-job-log-pause-{job_id}"
                                        )))
                                        .px(px(ds.spacing.grid_unit))
                                        .py(px(ds.spacing.grid_unit / 2.0))
                                        .rounded(px(ds.corners.sm))
                                        .bg(theme.surface_hover)
                                        .text_color(theme.text_secondary)
                                        .cursor_pointer()
                                        .child(if is_paused { "Follow tail" } else { "Pause" })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this.paused_job_logs.remove(&pause_job_id).is_none()
                                            {
                                                if let Some(job) = this.jobs.get(&pause_job_id) {
                                                    this.paused_job_logs.insert(
                                                        pause_job_id.clone(),
                                                        job.logs().cloned().collect(),
                                                    );
                                                }
                                            }
                                            cx.notify();
                                        })),
                                )
                                .child({
                                    let copied_logs = copied_logs.clone();
                                    div()
                                        .id(stable_element_id(format_args!(
                                            "python-job-log-copy-{}",
                                            job.id
                                        )))
                                        .px(px(ds.spacing.grid_unit))
                                        .py(px(ds.spacing.grid_unit / 2.0))
                                        .rounded(px(ds.corners.sm))
                                        .bg(theme.surface_hover)
                                        .text_color(theme.text_secondary)
                                        .cursor_pointer()
                                        .child("Copy")
                                        .on_click(move |_, _, cx| {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                copied_logs.clone(),
                                            ))
                                        })
                                })
                                .child({
                                    let job_id = job.id.clone();
                                    div()
                                        .id(stable_element_id(format_args!(
                                            "python-job-log-clear-{job_id}"
                                        )))
                                        .px(px(ds.spacing.grid_unit))
                                        .py(px(ds.spacing.grid_unit / 2.0))
                                        .rounded(px(ds.corners.sm))
                                        .bg(theme.surface_hover)
                                        .text_color(theme.text_secondary)
                                        .cursor_pointer()
                                        .child("Clear")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Err(error) = this.jobs.clear_logs(&job_id) {
                                                this.load_error = Some(error.to_string());
                                            }
                                            this.paused_job_logs.remove(&job_id);
                                            cx.notify();
                                        }))
                                })
                                .child({
                                    let copied_logs = copied_logs.clone();
                                    div()
                                        .id(stable_element_id(format_args!(
                                            "python-job-log-export-{}",
                                            job.id
                                        )))
                                        .px(px(ds.spacing.grid_unit))
                                        .py(px(ds.spacing.grid_unit / 2.0))
                                        .rounded(px(ds.corners.sm))
                                        .bg(theme.surface_hover)
                                        .text_color(theme.text_secondary)
                                        .cursor_pointer()
                                        .child("Export…")
                                        .on_click(cx.listener(move |_, _, _, cx| {
                                            let receiver = cx.prompt_for_new_path(
                                                Path::new("."),
                                                Some("simulation-job.log"),
                                            );
                                            let copied_logs = copied_logs.clone();
                                            cx.spawn(async move |_, _| {
                                                if let Ok(Ok(Some(path))) = receiver.await {
                                                    std::thread::spawn(move || {
                                                        let _ = std::fs::write(path, copied_logs);
                                                    });
                                                }
                                            })
                                            .detach();
                                        }))
                                }),
                        )
                        .child(
                            uniform_list(
                                stable_element_id(format_args!("python-job-log-lines-{job_id}")),
                                log_lines.len(),
                                move |range, _, _| {
                                    range
                                        .map(|index| {
                                            let line = &log_lines[index];
                                            div()
                                                .h(px(20.0))
                                                .text_size(px(log_text_size))
                                                .text_color(
                                                    if matches!(line.severity, LogSeverity::Error) {
                                                        log_error
                                                    } else {
                                                        log_text
                                                    },
                                                )
                                                .child(line.message.clone())
                                        })
                                        .collect::<Vec<_>>()
                                },
                            )
                            .h(px(180.0))
                            .w_full()
                            .track_scroll(&log_scroll),
                        );
                    if cancel {
                        row = row.child(
                            div()
                                .id(stable_element_id(format_args!("python-cancel-{}", job_id)))
                                .self_start()
                                .px(px(ds.spacing.control_padding_x))
                                .py(px(ds.spacing.grid_unit))
                                .rounded(px(ds.corners.sm))
                                .bg(theme.alert_error_bg)
                                .text_color(theme.error)
                                .cursor_pointer()
                                .child("Cancel")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(session) = &this.session {
                                        if let Err(error) = session.send(&HostMessage::Cancel {
                                            request_id: job_id.clone(),
                                        }) {
                                            this.load_error = Some(error.to_string());
                                        } else if let Err(error) = this.jobs.update(JobUpdate {
                                            id: job_id.clone(),
                                            state: JobState::Cancelling,
                                            completed: None,
                                            total: None,
                                            message: Some("Cancellation requested".into()),
                                        }) {
                                            this.load_error = Some(error.to_string());
                                        }
                                    }
                                    cx.notify();
                                })),
                        );
                    }
                    row
                }))
                .into_any_element(),
        )
    }

    pub(super) fn render_badge(
        &self,
        node: &BadgeNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let (bg, text) = badge_colors(&node.tone, theme);
        div()
            .px(px(ds.spacing.control_padding_x))
            .py(px(ds.spacing.grid_unit))
            .rounded(px(ds.corners.sm))
            .bg(bg)
            .text_color(text)
            .text_size(px(ds.typography.small_size))
            .child(node.label.clone())
            .into_any_element()
    }

    pub(super) fn render_metric(
        &self,
        node: &gpui_python_runtime::ui_ir::MetricNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        div()
            .w(px(180.0))
            .p(px(ds.spacing.card_padding))
            .bg(theme.surface)
            .rounded(px(ds.corners.md))
            .border_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .gap(px(ds.spacing.grid_unit))
            .child(
                div()
                    .text_size(px(ds.typography.large_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(node.value.clone()),
            )
            .child(
                div()
                    .text_size(px(ds.typography.small_size))
                    .text_color(theme.text_muted)
                    .child(node.label.clone()),
            )
            .into_any_element()
    }

    pub(super) fn render_progress(
        &self,
        node: &ProgressNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let value = node.value.clamp(0.0, 1.0);
        div()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.grid_unit))
            .children(node.label.as_ref().map(|label| {
                div()
                    .text_size(px(ds.typography.small_size))
                    .text_color(theme.text_secondary)
                    .child(label.clone())
            }))
            .child(
                div()
                    .w(px(260.0))
                    .h(px(8.0))
                    .rounded(px(4.0))
                    .bg(theme.muted)
                    .overflow_hidden()
                    .child(
                        div()
                            .w(px(260.0 * value))
                            .h_full()
                            .rounded(px(4.0))
                            .bg(theme.accent),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn render_spinner(
        &self,
        node: &SpinnerNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap(px(ds.spacing.control_gap))
            .child(
                div()
                    .w(px(10.0))
                    .h(px(10.0))
                    .rounded(px(5.0))
                    .bg(theme.accent),
            )
            .children(node.label.as_ref().map(|label| {
                div()
                    .text_size(px(ds.typography.small_size))
                    .text_color(theme.text_secondary)
                    .child(label.clone())
            }))
            .into_any_element()
    }

    fn render_thinking_orb(
        &mut self,
        node: &ThinkingOrbNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = match node.state.parse::<OrbState>() {
            Ok(state) => state,
            Err(()) => {
                return self.render_error(
                    &format!("Invalid thinking orb state {:?}", node.state),
                    theme,
                    ds,
                );
            }
        };
        let color = match Color::from_hex_string(&node.dot_color) {
            Some(color) => color.to_rgba(),
            None => {
                return self.render_error(
                    &format!("Invalid thinking orb color {:?}", node.dot_color),
                    theme,
                    ds,
                );
            }
        };
        let preset_size = if node.size <= 40.0 {
            OrbSize::Px20
        } else {
            OrbSize::Px64
        };
        let resolved = thinking_orb_presets::resolve_preset(state, preset_size);
        let base_count =
            thinking_orb_engine::frame(resolved.mode, f64::from(node.size), 0.0, &resolved.opts)
                .dots
                .len()
                .max(1);
        let count_scale = f64::from(node.points_per_sphere) / base_count as f64;
        let size = px(node.size);

        if self
            .thinking_orbs
            .get(&node.id)
            .is_none_or(|(current_state, _)| *current_state != state)
        {
            let aria_label = node.aria_label.clone();
            let speed = node.speed;
            let dot_scale = node.dot_scale;
            let paused = node.paused;
            let entity = cx.new(move |cx| {
                let mut orb = ThinkingOrb::new(state, size, cx)
                    .speed(speed)
                    .count_scale(count_scale)
                    .dot_scale(dot_scale)
                    .dot_color(color)
                    .paused(paused);
                if let Some(label) = aria_label {
                    orb = orb.aria_label(label);
                }
                orb
            });
            self.thinking_orbs.insert(node.id.clone(), (state, entity));
        }

        let orb = self
            .thinking_orbs
            .get(&node.id)
            .expect("thinking orb cache populated")
            .1
            .clone();
        orb.update(cx, |orb, cx| {
            orb.set_size(size, cx);
            orb.set_speed(node.speed, cx);
            orb.set_count_scale(count_scale, cx);
            orb.set_dot_scale(node.dot_scale, cx);
            orb.set_dot_color(color, cx);
            orb.set_paused(node.paused, cx);
        });

        div().w(size).h(size).child(orb).into_any_element()
    }

    fn render_breadcrumbs(&self, node: &BreadcrumbsNode) -> AnyElement {
        let separator = match node.separator.as_str() {
            "chevron" => BreadcrumbSeparator::Chevron,
            "dot" => BreadcrumbSeparator::Dot,
            _ => BreadcrumbSeparator::Slash,
        };
        let items = node
            .items
            .iter()
            .map(|source| {
                let mut item = BreadcrumbItem::new(source.id.clone(), source.label.clone());
                if let Some(icon) = &source.icon {
                    item = item.icon(icon.clone());
                }
                if let Some(href) = &source.href {
                    item = item.href(href.clone());
                }
                item
            })
            .collect();
        let mut breadcrumbs = Breadcrumbs::new().items(items).separator(separator);
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let node_id = node.id.clone();
            breadcrumbs = breadcrumbs.on_click(move |item_id, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    Some(action.clone()),
                    serde_json::json!({"item_id": item_id.as_ref()}),
                );
            });
        }
        breadcrumbs.into_any_element()
    }

    fn render_alert(&self, node: &AlertNode) -> AnyElement {
        let variant = match node.variant.as_str() {
            "success" => AlertVariant::Success,
            "warning" => AlertVariant::Warning,
            "error" => AlertVariant::Error,
            _ => AlertVariant::Info,
        };
        let mut alert = Alert::new(
            stable_element_id(format_args!("python-alert-{}", node.id)),
            node.message.clone(),
        )
        .variant(variant)
        .closeable(node.closeable);
        if let Some(title) = &node.title {
            alert = alert.title(title.clone());
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let node_id = node.id.clone();
            alert = alert.on_close(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "close", Some(action.clone()), Value::Null);
            });
        }
        alert.into_any_element()
    }

    fn render_toast(&self, node: &ToastNode) -> AnyElement {
        let variant = match node.variant.as_str() {
            "success" => ToastVariant::Success,
            "warning" => ToastVariant::Warning,
            "error" => ToastVariant::Error,
            _ => ToastVariant::Info,
        };
        let mut toast = Toast::new(
            stable_element_id(format_args!("python-toast-{}", node.id)),
            node.message.clone(),
        )
        .variant(variant)
        .closeable(node.closeable)
        .duration_secs(node.duration_secs);
        if let Some(title) = &node.title {
            toast = toast.title(title.clone());
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let node_id = node.id.clone();
            toast = toast.on_close(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "close", Some(action.clone()), Value::Null);
            });
        }
        toast.into_any_element()
    }

    fn render_tooltip(
        &mut self,
        node: &TooltipNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let placement = match node.placement.as_str() {
            "bottom" => TooltipPlacement::Bottom,
            "left" => TooltipPlacement::Left,
            "right" => TooltipPlacement::Right,
            _ => TooltipPlacement::Top,
        };
        let mut tooltip = WithTooltip::new(
            self.render_node(&node.child, theme, ds, cx),
            node.content.clone(),
        )
        .id(stable_element_id(format_args!(
            "python-tooltip-{}",
            node.id
        )))
        .placement(placement)
        .delay(node.delay_ms);
        if let Some(show) = node.show {
            tooltip = tooltip.show(show);
        }
        tooltip.into_any_element()
    }

    fn render_empty_state(
        &mut self,
        node: &EmptyStateNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut empty = EmptyState::new(node.title.clone());
        if let Some(description) = &node.description {
            empty = empty.description(description.clone());
        }
        if let Some(icon) = &node.icon {
            empty = empty.icon(icon.clone());
        }
        if let Some(action) = &node.action {
            empty = empty.action(self.render_node(action, theme, ds, cx));
        }
        empty.into_any_element()
    }

    fn render_dialog(
        &mut self,
        node: &DialogNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let size = match node.size.as_str() {
            "sm" => DialogSize::Sm,
            "lg" => DialogSize::Lg,
            "xl" => DialogSize::Xl,
            "full" => DialogSize::Full,
            _ => DialogSize::Md,
        };
        let content = div().flex().flex_col().children(
            node.content
                .iter()
                .map(|child| self.render_node(child, theme, ds, cx)),
        );
        let footer = div()
            .flex()
            .items_center()
            .gap(px(ds.spacing.control_gap))
            .children(
                node.footer
                    .iter()
                    .map(|child| self.render_node(child, theme, ds, cx)),
            );
        let mut dialog = Dialog::new(stable_element_id(format_args!("python-dialog-{}", node.id)))
            .size(size)
            .content(content)
            .footer(footer)
            .show_close_button(node.show_close_button)
            .close_on_backdrop(node.close_on_backdrop);
        if let Some(title) = &node.title {
            dialog = dialog.title(title.clone());
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.close_action.clone(),
        ) {
            let node_id = node.id.clone();
            dialog = dialog.on_close(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "close", Some(action.clone()), Value::Null);
            });
        }
        dialog.into_any_element()
    }

    fn render_confirm_dialog(
        &self,
        node: &ConfirmDialogNode,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let variant = match node.variant.as_str() {
            "destructive" => ConfirmDialogVariant::Destructive,
            "warning" => ConfirmDialogVariant::Warning,
            _ => ConfirmDialogVariant::Default,
        };
        let mut dialog = ConfirmDialog::new(stable_element_id(format_args!(
            "python-confirm-dialog-{}",
            node.id
        )))
        .message(node.message.clone())
        .variant(variant)
        .confirm_label(node.confirm_label.clone())
        .cancel_label(node.cancel_label.clone())
        .focus_handle(cx.focus_handle());
        if let Some(title) = &node.title {
            dialog = dialog.title(title.clone());
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.confirm_action.clone(),
        ) {
            let node_id = node.id.clone();
            dialog = dialog.on_confirm(move |_, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "confirm",
                    Some(action.clone()),
                    Value::Null,
                );
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.cancel_action.clone(),
        ) {
            let node_id = node.id.clone();
            dialog = dialog.on_cancel(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "cancel", Some(action.clone()), Value::Null);
            });
        }
        dialog.into_any_element()
    }

    fn menu_items(items: &[MenuItemNode]) -> Vec<MenuItem> {
        items
            .iter()
            .map(|item| {
                if item.separator {
                    return MenuItem::separator();
                }
                let mut rendered = if item.checkbox {
                    MenuItem::checkbox(item.id.clone(), item.label.clone(), item.checked)
                } else {
                    MenuItem::new(item.id.clone(), item.label.clone())
                }
                .disabled(item.disabled);
                if let Some(shortcut) = &item.shortcut {
                    rendered = rendered.with_shortcut(shortcut.clone());
                }
                if item.danger {
                    rendered = rendered.danger();
                }
                if !item.children.is_empty() {
                    rendered = rendered.with_children(Self::menu_items(&item.children));
                }
                rendered
            })
            .collect()
    }

    fn render_context_menu(&self, node: &ContextMenuNode, cx: &mut Context<Self>) -> AnyElement {
        let mut menu = ContextMenu::new(
            stable_element_id(format_args!("python-context-menu-{}", node.id)),
            Self::menu_items(&node.items),
        )
        .position(point(px(node.position[0]), px(node.position[1])))
        .min_width(px(node.min_width));
        menu = menu.focus_handle(cx.focus_handle());
        if let Some(index) = node.focused_index {
            menu = menu.focused_index(index);
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_select(move |item_id, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "select",
                    Some(action.clone()),
                    serde_json::json!({"item_id": item_id.as_ref()}),
                );
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.close_action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_close(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "close", Some(action.clone()), Value::Null);
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.focus_action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_focus_change(move |index, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "focus",
                    Some(action.clone()),
                    serde_json::json!({"index": index}),
                );
            });
        }
        menu.into_any_element()
    }

    fn render_menu(&self, node: &MenuNode, cx: &mut Context<Self>) -> AnyElement {
        let mut menu = Menu::new(
            stable_element_id(format_args!("python-menu-{}", node.id)),
            Self::menu_items(&node.items),
        )
        .min_width(px(node.min_width))
        .focus_handle(cx.focus_handle());
        if let Some(index) = node.focused_index {
            menu = menu.focused_index(index);
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_select(move |item_id, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "select",
                    Some(action.clone()),
                    serde_json::json!({"item_id": item_id.as_ref()}),
                );
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.close_action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_close(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "close", Some(action.clone()), Value::Null);
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.focus_action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_focus_change(move |index, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "focus",
                    Some(action.clone()),
                    serde_json::json!({"index": index}),
                );
            });
        }
        menu.into_any_element()
    }

    fn render_menu_bar(&self, node: &MenuBarNode, cx: &mut Context<Self>) -> AnyElement {
        let bar_items = node
            .items
            .iter()
            .map(|item| {
                MenuBarItem::new(item.id.clone(), item.label.clone())
                    .with_items(Self::menu_items(&item.items))
            })
            .collect();
        let mut bar = MenuBar::new(bar_items).active_menu(node.active_menu.clone().map(Into::into));
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.toggle_action.clone(),
        ) {
            let node_id = node.id.clone();
            bar = bar.on_menu_toggle(move |menu_id, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "toggle",
                    Some(action.clone()),
                    serde_json::json!({"menu_id": menu_id.map(|id| id.as_ref())}),
                );
            });
        }
        let mut rendered = div().relative().child(bar);
        if let Some(active_id) = &node.active_menu
            && let Some(active) = node.items.iter().find(|item| &item.id == active_id)
        {
            let mut menu = Menu::new(
                stable_element_id(format_args!("python-menu-bar-{}-{active_id}", node.id)),
                Self::menu_items(&active.items),
            )
            .focus_handle(cx.focus_handle());
            if let (Some(sink), Some(action)) = (
                self.session.as_ref().map(|session| session.event_sink()),
                node.action.clone(),
            ) {
                let node_id = node.id.clone();
                menu = menu.on_select(move |item_id, _, _| {
                    let _ = sink.dispatch(
                        node_id.clone(),
                        "select",
                        Some(action.clone()),
                        serde_json::json!({"item_id": item_id.as_ref()}),
                    );
                });
            }
            if let (Some(sink), Some(action)) = (
                self.session.as_ref().map(|session| session.event_sink()),
                node.toggle_action.clone(),
            ) {
                let node_id = node.id.clone();
                menu = menu.on_close(move |_, _| {
                    let _ = sink.dispatch(
                        node_id.clone(),
                        "toggle",
                        Some(action.clone()),
                        serde_json::json!({"menu_id": Value::Null}),
                    );
                });
            }
            rendered = rendered.child(div().absolute().top_full().left_0().mt_1().child(menu));
        }
        rendered.into_any_element()
    }

    fn render_popover(
        &mut self,
        node: &PopoverNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let placement = match node.placement.as_str() {
            "top" => PopoverPlacement::Top,
            "left" => PopoverPlacement::Left,
            "right" => PopoverPlacement::Right,
            "top_start" => PopoverPlacement::TopStart,
            "top_end" => PopoverPlacement::TopEnd,
            "bottom_start" => PopoverPlacement::BottomStart,
            "bottom_end" => PopoverPlacement::BottomEnd,
            _ => PopoverPlacement::Bottom,
        };
        let content = div().flex().flex_col().children(
            node.content
                .iter()
                .map(|child| self.render_node(child, theme, ds, cx)),
        );
        let mut popover = Popover::new(stable_element_id(format_args!(
            "python-popover-{}",
            node.id
        )))
        .placement(placement)
        .content(content)
        .show_backdrop(node.show_backdrop)
        .focus_handle(cx.focus_handle());
        if let Some(width) = node.width {
            popover = popover.width(px(width));
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.close_action.clone(),
        ) {
            let node_id = node.id.clone();
            popover = popover.on_close(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "close", Some(action.clone()), Value::Null);
            });
        }
        div()
            .relative()
            .child(self.render_node(&node.trigger, theme, ds, cx))
            .child(popover)
            .into_any_element()
    }

    pub(super) fn render_tabs(
        &mut self,
        node: &TabsNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tab_id = node.id.clone().unwrap_or_else(|| "static".into());
        let focus_handle = self
            .tab_focus
            .entry(tab_id.clone())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        let action = node.action.clone();
        let sink = self.session.as_ref().map(|session| session.event_sink());
        let node_id = node.id.clone();
        let items = node.items.clone();
        let active = node.active;
        apply_native_accessibility(
            div().id(stable_element_id(format_args!("python-tablist-{tab_id}"))),
            format!("Tabs {tab_id}"),
            &AriaProps::with_role(AriaRole::Tablist),
        )
        .track_focus(&focus_handle)
        .focusable()
        .flex()
        .gap(px(ds.spacing.grid_unit))
        .children(node.items.iter().enumerate().map(|(index, item)| {
            let active = index == node.active;
            let tab = apply_native_accessibility(
                div()
                    .id(stable_element_id(format_args!(
                        "python-tab-{}-{index}",
                        node.id.as_deref().unwrap_or("unbound")
                    )))
                    .px(px(ds.spacing.control_padding_x))
                    .py(px(ds.spacing.control_padding_y))
                    .rounded(px(ds.corners.md))
                    .bg(if active {
                        theme.accent
                    } else {
                        theme.surface_hover
                    })
                    .text_color(if active {
                        theme.text_on_accent
                    } else {
                        theme.text_primary
                    })
                    .child(item.clone()),
                item.clone(),
                &AriaProps::with_role(AriaRole::Tab).maybe_state(active, AriaState::Selected(true)),
            );
            if let (Some(sink), Some(node_id)) = (sink.clone(), node_id.clone()) {
                let action = node.action.clone();
                let item = item.clone();
                let click_focus = focus_handle.clone();
                tab.cursor_pointer().on_click(move |_, window, cx| {
                    click_focus.focus(window, cx);
                    let _ = sink.dispatch(
                        node_id.clone(),
                        "change",
                        action.clone(),
                        serde_json::json!({"index": index, "item": item}),
                    );
                })
            } else {
                tab
            }
        }))
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if !focus_handle.is_focused(window) || items.is_empty() {
                return;
            }
            let next = match event.keystroke.key.as_str() {
                "left" => active.saturating_sub(1),
                "right" => (active + 1).min(items.len() - 1),
                "home" => 0,
                "end" => items.len() - 1,
                _ => return,
            };
            if let (Some(sink), Some(node_id)) = (&sink, node_id.as_ref()) {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    serde_json::json!({"index": next, "item": items[next].clone()}),
                );
                cx.stop_propagation();
            }
        })
        .into_any_element()
    }

    fn render_stepper(&self, node: &StepperNode, theme: &Theme, ds: &DesignSystem) -> AnyElement {
        div()
            .flex()
            .gap(px(ds.spacing.grid_unit))
            .children(node.steps.iter().enumerate().map(|(index, step)| {
                let active = index == node.active;
                let disabled = node.disabled_steps.contains(&index);
                let mut item = div()
                    .id(stable_element_id(format_args!(
                        "python-stepper-{}-{index}",
                        node.id
                    )))
                    .flex()
                    .items_center()
                    .gap(px(ds.spacing.grid_unit))
                    .px(px(ds.spacing.control_padding_x))
                    .py(px(ds.spacing.control_padding_y))
                    .rounded(px(ds.corners.md))
                    .bg(if active {
                        theme.accent
                    } else {
                        theme.surface_hover
                    })
                    .text_color(if active {
                        theme.text_on_accent
                    } else if disabled {
                        theme.text_muted
                    } else {
                        theme.text_primary
                    })
                    .child(format!("{}  {}", index + 1, step));
                if disabled {
                    item = item.cursor_not_allowed();
                } else if let Some(sink) = self.session.as_ref().map(|session| session.event_sink())
                {
                    let node_id = node.id.clone();
                    let action = node.action.clone();
                    let step = step.clone();
                    item = item.cursor_pointer().on_click(move |_, _, _| {
                        let _ = sink.dispatch(
                            node_id.clone(),
                            "change",
                            action.clone(),
                            serde_json::json!({"index": index, "step": step}),
                        );
                    });
                }
                item
            }))
            .into_any_element()
    }

    fn render_accordion(
        &mut self,
        node: &AccordionNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let items = node
            .items
            .iter()
            .map(|item| {
                let content = div().flex().flex_col().children(
                    item.children
                        .iter()
                        .map(|child| self.render_node(child, theme, ds, cx)),
                );
                let mut native =
                    AccordionItem::new(item.id.clone(), item.title.clone()).content(content);
                if item.disabled {
                    native = native.disabled(true);
                }
                if let Some(trailing) = &item.trailing {
                    native = native.trailing(trailing.clone());
                }
                native
            })
            .collect();
        let mut accordion = Accordion::new().items(items).expanded(
            node.expanded
                .iter()
                .cloned()
                .map(SharedString::from)
                .collect(),
        );
        if node.multiple {
            accordion = accordion.mode(AccordionMode::Multiple);
        }
        if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
            let node_id = node.id.clone();
            let action = node.action.clone();
            accordion = accordion.on_change(move |item_id, expanded, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "change",
                    action.clone(),
                    serde_json::json!({"item_id": item_id.as_ref(), "expanded": expanded}),
                );
            });
        }
        accordion.into_any_element()
    }

    fn render_list_editor(
        &self,
        node: &ListEditorNode,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let mut items = Vec::with_capacity(node.rows.len());
        for row in &node.rows {
            let row_id = row.id.clone();
            let row_value = row.value.clone();
            let remove = if node.disabled || row.disabled || node.remove_action.is_none() {
                div()
                    .id(stable_element_id(format_args!(
                        "python-list-remove-{}-{}",
                        node.id, row.id
                    )))
                    .text_color(theme.text_muted)
                    .child("Remove")
            } else if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
                let list_id = node.id.clone();
                let action = node.remove_action.clone();
                div()
                    .id(stable_element_id(format_args!(
                        "python-list-remove-{}-{}",
                        node.id, row.id
                    )))
                    .px(px(ds.spacing.grid_unit))
                    .py(px(ds.spacing.grid_unit / 2.0))
                    .rounded(px(ds.corners.sm))
                    .bg(theme.surface_hover)
                    .text_color(theme.text_primary)
                    .cursor_pointer()
                    .child("Remove")
                    .on_click(move |_, _, _| {
                        let _ = sink.dispatch(
                            list_id.clone(),
                            "remove",
                            action.clone(),
                            serde_json::json!({"row_id": row_id, "value": row_value}),
                        );
                    })
            } else {
                div()
                    .id(stable_element_id(format_args!(
                        "python-list-remove-{}-{}",
                        node.id, row.id
                    )))
                    .text_color(theme.text_muted)
                    .child("Remove")
            };
            let mut content = div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(ds.spacing.control_gap))
                .child(div().flex().flex_col().child(row.label.clone()).children(
                    row.validation.as_ref().map(|validation| {
                        div()
                            .text_size(px(ds.typography.small_size))
                            .text_color(if validation.severity == "error" {
                                theme.error
                            } else {
                                theme.warning
                            })
                            .child(validation.message.clone())
                    }),
                ))
                .child(remove);
            if row.disabled {
                content = content.opacity(0.5);
            }
            items.push(DragItem::new(row.id.clone(), content));
        }
        let mut list = DragList::new(
            stable_element_id(format_args!("python-list-editor-{}", node.id)),
            items,
        )
        .show_handles(!node.disabled)
        .gap(px(ds.spacing.grid_unit));
        if !node.disabled {
            if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
                let list_id = node.id.clone();
                let action = node.reorder_action.clone();
                let row_ids = node
                    .rows
                    .iter()
                    .map(|row| row.id.clone())
                    .collect::<Vec<_>>();
                list = list.on_reorder(move |from, to, _, _| {
                    let _ = sink.dispatch(
                        list_id.clone(),
                        "reorder",
                        action.clone(),
                        serde_json::json!({
                            "from_index": from,
                            "to_index": to,
                            "row_id": row_ids.get(from),
                            "before_row_id": row_ids.get(to),
                        }),
                    );
                });
            }
        }
        let mut editor = div().flex().flex_col().gap(px(ds.spacing.control_gap));
        if let Some(label) = &node.label {
            editor = editor.child(
                div()
                    .text_size(px(ds.typography.small_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(label.clone()),
            );
        }
        editor = editor.child(list);
        if !node.disabled {
            if let (Some(action), Some(sink)) = (
                node.add_action.clone(),
                self.session.as_ref().map(|session| session.event_sink()),
            ) {
                let list_id = node.id.clone();
                editor = editor.child(
                    div()
                        .id(stable_element_id(format_args!(
                            "python-list-add-{}",
                            node.id
                        )))
                        .px(px(ds.spacing.control_padding_x))
                        .py(px(ds.spacing.control_padding_y))
                        .rounded(px(ds.corners.sm))
                        .bg(theme.surface_hover)
                        .text_color(theme.text_primary)
                        .cursor_pointer()
                        .child(node.add_label.clone().unwrap_or_else(|| "Add row".into()))
                        .on_click(move |_, _, _| {
                            let _ = sink.dispatch(
                                list_id.clone(),
                                "add",
                                Some(action.clone()),
                                Value::Null,
                            );
                        }),
                );
            }
        }
        editor.into_any_element()
    }

    pub(super) fn render_table(
        &mut self,
        node: &TableNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dom_id = node.id.clone().unwrap_or_else(|| {
            self.legacy_table_id_counter += 1;
            format!("legacy-{}", self.legacy_table_id_counter)
        });
        let mut table = apply_native_accessibility(
            div().id(stable_element_id(format_args!("python-table-{dom_id}"))),
            format!("Table {dom_id}"),
            &AriaProps::with_role(AriaRole::Table),
        )
        .flex()
        .flex_col()
        .rounded(px(ds.corners.md))
        .border_1()
        .border_color(theme.border)
        .overflow_hidden();

        if node.columns.is_empty() {
            if !node.headers.is_empty() {
                table = table.child(self.render_table_row(&node.headers, 0, true, &[], theme, ds));
            }
        } else {
            table = table.child(self.render_table_header(node, theme, ds, cx.entity().clone()));
        }

        let offset = node.row_offset;
        let total_rows = if node.typed_rows.is_empty() {
            node.rows.len()
        } else {
            node.typed_rows.len()
        };
        let available = total_rows.saturating_sub(offset);
        // A supplied row limit remains useful for remote/application-windowed
        // tables. Otherwise UniformList virtualizes the whole retained data
        // set, materializing only rows inside the native viewport.
        let visible_count = node.row_limit.unwrap_or(available).min(available);
        if let Some(table_id) = node.id.as_ref().filter(|id| !id.is_empty()) {
            let rows = if node.typed_rows.is_empty() {
                node.rows
                    .iter()
                    .enumerate()
                    .skip(offset)
                    .take(visible_count)
                    .map(|(index, cells)| {
                        let cells = cells
                            .iter()
                            .map(|cell| Value::String(cell.clone()))
                            .collect::<Vec<_>>();
                        (format!("row-{index}"), cells, false)
                    })
                    .collect::<Vec<_>>()
            } else {
                node.typed_rows
                    .iter()
                    .skip(offset)
                    .take(visible_count)
                    .map(|row| {
                        (
                            row.id.clone(),
                            row.cells.clone(),
                            node.selected_row.as_deref() == Some(row.id.as_str()),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let scroll = self
                .table_scrolls
                .entry(table_id.clone())
                .or_insert_with(UniformListScrollHandle::new)
                .clone();
            let focus_handle = self
                .table_focus
                .entry(table_id.clone())
                .or_insert_with(|| cx.focus_handle())
                .clone();
            let mut columns = node.columns.iter().cloned().enumerate().collect::<Vec<_>>();
            columns.sort_by_key(|(_, column)| !column.pinned);
            let column_widths = self.table_column_widths.clone();
            let table_id = table_id.clone();
            let action = node
                .selection_action
                .clone()
                .or_else(|| node.row_action.clone());
            let sink = self.session.as_ref().map(|session| session.event_sink());
            let keyboard_rows = rows
                .iter()
                .map(|(row_id, _, _)| row_id.clone())
                .collect::<Vec<_>>();
            let list_table_id = table_id.clone();
            let list_action = action.clone();
            let list_sink = sink.clone();
            let list_focus = focus_handle.clone();
            let row_height = 34.0;
            let table_surface = theme.surface;
            let table_accent = theme.accent;
            let table_border = theme.border;
            let table_text = theme.text_secondary;
            let table_selected_text = theme.text_on_accent;
            let table_padding_x = ds.spacing.control_padding_x;
            let table_small_text = ds.typography.small_size;
            table = table.child(
                uniform_list(
                    stable_element_id(format_args!("python-table-virtual-{table_id}")),
                    rows.len(),
                    move |range, _, _| {
                        range
                            .map(|index| {
                                let (row_id, values, selected) = &rows[index];
                                let display = values.iter().map(table_cell_text).collect::<Vec<_>>();
                                let row_id_for_event = row_id.clone();
                                let values_for_event = values.clone();
                                let table_id_for_event = list_table_id.clone();
                                let action = list_action.clone();
                                let sink = list_sink.clone();
                                let focus_handle = list_focus.clone();
                                div()
                                    .id(stable_element_id(format_args!("python-table-row-{list_table_id}-{row_id}")))
                                    .h(px(row_height))
                                    .flex()
                                    .items_center()
                                    .bg(if *selected { table_accent } else { table_surface })
                                    .border_b_1()
                                    .border_color(table_border)
                                    .cursor_pointer()
                                    .children(columns.iter().map(|(source, column)| {
                                        let width = column_widths
                                            .borrow()
                                            .get(&(list_table_id.clone(), column.id.clone()))
                                            .copied()
                                            .unwrap_or_else(|| column.width.unwrap_or(180.0));
                                        div()
                                            .w(px(width))
                                            .px(px(table_padding_x))
                                            .text_size(px(table_small_text))
                                            .text_color(if *selected { table_selected_text } else { table_text })
                                            .child(display.get(*source).cloned().unwrap_or_default())
                                    }))
                                    .on_click(move |_, window, cx| {
                                        focus_handle.focus(window, cx);
                                        if let Some(sink) = &sink {
                                            let _ = sink.dispatch(
                                                table_id_for_event.clone(),
                                                "select",
                                                action.clone(),
                                                serde_json::json!({"row_id": row_id_for_event, "cells": values_for_event}),
                                            );
                                        }
                                    })
                            })
                            .collect::<Vec<_>>()
                    },
                )
                .h(px(360.0))
                .w_full()
                .track_scroll(&scroll),
            );
            if action.is_some() {
                let action = action.clone();
                let sink = sink.clone();
                let table_id = table_id.clone();
                let selected_index = node.selected_row.as_ref().and_then(|selected| {
                    keyboard_rows.iter().position(|row_id| row_id == selected)
                });
                let key_focus = focus_handle.clone();
                table = table.track_focus(&focus_handle).focusable().on_key_down(
                    move |event: &KeyDownEvent, window, cx| {
                        if !key_focus.is_focused(window) {
                            return;
                        }
                        let Some(navigation) =
                            DataNavigationAction::from_key(event.keystroke.key.as_str())
                        else {
                            return;
                        };
                        let next = match navigation {
                            DataNavigationAction::Previous
                            | DataNavigationAction::Next
                            | DataNavigationAction::First
                            | DataNavigationAction::Last => {
                                DataNavigationState::new(keyboard_rows.len())
                                    .selected_index(selected_index)
                                    .move_selection(navigation)
                            }
                            DataNavigationAction::Activate => selected_index,
                            _ => None,
                        };
                        let Some(index) = next else { return };
                        let Some(row_id) = keyboard_rows.get(index) else {
                            return;
                        };
                        if let Some(sink) = &sink {
                            let _ = sink.dispatch(
                                table_id.clone(),
                                "select",
                                action.clone(),
                                serde_json::json!({"row_id": row_id, "source": "keyboard"}),
                            );
                            cx.stop_propagation();
                        }
                    },
                );
            }
            if total_rows > visible_count {
                table = table.child(
                    div()
                        .px(px(ds.spacing.control_padding_x))
                        .py(px(ds.spacing.grid_unit))
                        .text_size(px(ds.typography.small_size))
                        .text_color(theme.text_muted)
                        .child(format!(
                            "Virtualized rows {}–{} of {total_rows}",
                            offset + 1,
                            offset + visible_count
                        )),
                );
            }
        } else {
            // Legacy tables without stable IDs cannot safely preserve native
            // scroll/selection state, so retain the bounded static renderer.
            if node.typed_rows.is_empty() {
                for (index, row) in node
                    .rows
                    .iter()
                    .enumerate()
                    .skip(offset)
                    .take(visible_count)
                {
                    table = table.child(self.render_table_row(
                        row,
                        index + 1,
                        false,
                        &node.columns,
                        theme,
                        ds,
                    ));
                }
            } else {
                for row in node.typed_rows.iter().skip(offset).take(visible_count) {
                    let cells = row.cells.iter().map(table_cell_text).collect::<Vec<_>>();
                    table = table.child(self.render_table_row(
                        &cells,
                        0,
                        false,
                        &node.columns,
                        theme,
                        ds,
                    ));
                }
            }
        }

        table.into_any_element()
    }

    fn render_table_header(
        &self,
        node: &TableNode,
        theme: &Theme,
        ds: &DesignSystem,
        entity: Entity<Self>,
    ) -> Div {
        let mut columns = node.columns.iter().collect::<Vec<_>>();
        columns.sort_by_key(|column| !column.pinned);
        let widths = self.table_column_widths.clone();
        let resize = self.table_resize.clone();
        div()
            .flex()
            .bg(theme.muted)
            .border_b_1()
            .border_color(theme.border)
            .children(columns.into_iter().map(|column| {
                let active = node.sort_column.as_deref() == Some(column.id.as_str());
                let next_direction = if active && node.sort_direction == "ascending" {
                    "descending"
                } else {
                    "ascending"
                };
                let label = if active {
                    format!("{} {}", column.label, if node.sort_direction == "ascending" { "↑" } else { "↓" })
                } else {
                    column.label.clone()
                };
                let width = node
                    .id
                    .as_ref()
                    .and_then(|table_id| widths.borrow().get(&(table_id.clone(), column.id.clone())).copied())
                    .unwrap_or_else(|| column.width.unwrap_or(180.0));
                let mut cell = div()
                    .id(stable_element_id(format_args!("python-table-header-{}", column.id)))
                    .relative()
                    .w(px(width))
                    .px(px(ds.spacing.control_padding_x))
                    .py(px(ds.spacing.control_padding_y))
                    .text_size(px(ds.typography.small_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(label);
                if let (Some(table_id), Some(action)) = (node.id.clone(), node.resize_action.clone()) {
                    let resize_on_down = resize.clone();
                    let resize_on_move = resize.clone();
                    let resize_on_up = resize.clone();
                    let widths_on_move = widths.clone();
                    let widths_on_up = widths.clone();
                    let notify_entity = entity.clone();
                    let sink = self.session.as_ref().map(|session| session.event_sink());
                    let column_id = column.id.clone();
                    let initial_width = width;
                    let grip = div()
                        .id(stable_element_id(format_args!("python-table-resize-{table_id}-{column_id}")))
                        .absolute()
                        .right_0()
                        .top_0()
                        .bottom_0()
                        .w(px(8.0))
                        .cursor_col_resize()
                        .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                            *resize_on_down.borrow_mut() = Some(TableResize {
                                table_id: table_id.clone(),
                                column_id: column_id.clone(),
                                start_x: event.position.x.as_f32(),
                                start_width: initial_width,
                            });
                            cx.stop_propagation();
                        })
                        .on_mouse_move(move |event, _window, cx| {
                            if let Some(drag) = resize_on_move.borrow().clone() {
                                let width = (drag.start_width + event.position.x.as_f32() - drag.start_x)
                                    .clamp(64.0, 960.0);
                                widths_on_move
                                    .borrow_mut()
                                    .insert((drag.table_id, drag.column_id), width);
                                notify_entity.update(cx, |_this, cx| cx.notify());
                                cx.stop_propagation();
                            }
                        })
                        .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                            if let Some(drag) = resize_on_up.borrow_mut().take() {
                                let width = widths_on_up
                                    .borrow()
                                    .get(&(drag.table_id.clone(), drag.column_id.clone()))
                                    .copied()
                                    .unwrap_or(drag.start_width);
                                if let Some(sink) = &sink {
                                    let _ = sink.dispatch(
                                        drag.table_id,
                                        "resize",
                                        Some(action.clone()),
                                        serde_json::json!({"column_id": drag.column_id, "width": width}),
                                    );
                                }
                            }
                            cx.stop_propagation();
                        });
                    cell = cell.child(grip);
                }
                if column.sortable {
                    if let (Some(table_id), Some(action), Some(sink)) = (
                        node.id.clone(),
                        node.sort_action.clone(),
                        self.session.as_ref().map(|session| session.event_sink()),
                    ) {
                        let column_id = column.id.clone();
                        cell.cursor_pointer().on_click(move |_, _, _| {
                            let _ = sink.dispatch(
                                table_id.clone(),
                                "sort",
                                Some(action.clone()),
                                serde_json::json!({"column_id": column_id, "direction": next_direction}),
                            );
                        })
                    } else {
                        cell
                    }
                } else {
                    cell
                }
            }))
    }

    pub(super) fn render_table_row(
        &mut self,
        row: &[String],
        row_index: usize,
        header: bool,
        columns: &[gpui_python_runtime::ui_ir::TableColumn],
        theme: &Theme,
        ds: &DesignSystem,
    ) -> Div {
        let mut column_order = columns.iter().enumerate().collect::<Vec<_>>();
        column_order.sort_by_key(|(_, column)| !column.pinned);
        div()
            .flex()
            .bg(if header { theme.muted } else { theme.surface })
            .border_b_1()
            .border_color(theme.border)
            .children(column_order.into_iter().map(|(col, column)| {
                let cell = row.get(col).cloned().unwrap_or_default();
                let cached = self
                    .table_cells
                    .entry((row_index, col))
                    .or_insert_with(|| (cell.clone(), SharedString::from(cell.clone())));
                if cached.0 != cell {
                    *cached = (cell.clone(), SharedString::from(cell.clone()));
                }
                div()
                    .w(px(column.width.unwrap_or(180.0)))
                    .px(px(ds.spacing.control_padding_x))
                    .py(px(ds.spacing.control_padding_y))
                    .text_size(px(ds.typography.small_size))
                    .font_weight(if header {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if header {
                        theme.text_primary
                    } else {
                        theme.text_secondary
                    })
                    .child(cached.1.clone())
            }))
    }

    pub(super) fn render_divider(&self, node: &SimpleNode, theme: &Theme) -> AnyElement {
        apply_size(
            div()
                .h(px(node.height.unwrap_or(1.0)))
                .w_full()
                .bg(theme.border),
            node.width,
            node.height,
        )
        .into_any_element()
    }

    pub(super) fn render_spacer(&self, node: &SimpleNode) -> AnyElement {
        apply_size(div(), node.width.or(Some(1.0)), node.height.or(Some(1.0))).into_any_element()
    }

    pub(super) fn render_chart(
        &mut self,
        node: &ChartNode,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let interaction = match node.chart {
            ChartKind::Scatter | ChartKind::Line => {
                let ((x_min, x_max), (y_min, y_max)) = cartesian_chart_domains(node);
                let entity = cx.weak_entity();
                Some(
                    self.chart_interactions
                        .entry(node.id.clone())
                        .or_insert_with(|| {
                            InteractiveChartState::new(x_min, x_max, y_min, y_max)
                                .with_log_x(node.x_log)
                                .with_log_y(node.y_log)
                                .with_size(node.width, node.height)
                                .on_interaction_change(move |cx| {
                                    let _ = entity.update(cx, |_, cx| cx.notify());
                                })
                        })
                        .clone(),
                )
            }
            ChartKind::Bar
            | ChartKind::Heatmap
            | ChartKind::Area
            | ChartKind::BoxPlot
            | ChartKind::Contour
            | ChartKind::Isoline
            | ChartKind::Pie
            | ChartKind::Donut
            | ChartKind::Treemap => None,
        };
        let active_domains = interaction
            .as_ref()
            .map(|state| (state.x_domain(), state.y_domain()));
        let visible_series = node
            .series
            .iter()
            .filter(|series| self.chart_series_is_visible(&node.id, series))
            .collect::<Vec<_>>();
        let result = match node.chart {
            ChartKind::Scatter => {
                let primary = visible_series.first().copied();
                let x = primary
                    .map(|series| series.x.as_slice())
                    .or(node.x.as_deref())
                    .unwrap_or_default();
                let y = primary
                    .map(|series| series.y.as_slice())
                    .or(node.y.as_deref())
                    .unwrap_or_default();
                let mut chart = scatter(x, y)
                    .title(node.title.clone())
                    .color(hex_color(
                        primary
                            .and_then(|series| series.color.as_deref())
                            .or(node.color.as_deref()),
                        0x1f77b4,
                    ))
                    .point_radius(
                        primary
                            .and_then(|series| series.point_radius)
                            .unwrap_or(node.point_radius),
                    )
                    .x_scale(scale_type(node.x_log))
                    .y_scale(scale_type(node.y_log))
                    .legend_position(px_legend_position(&node.legend_position))
                    .annotations(px_annotations(node))
                    .size(node.width, node.height);
                for series in visible_series.iter().copied().skip(1) {
                    chart = chart.add_series(
                        &series.x,
                        &series.y,
                        (!series.label.is_empty()).then_some(series.label.clone()),
                        hex_color(series.color.as_deref(), 0x1f77b4),
                        series.point_radius.unwrap_or(node.point_radius),
                        series.opacity,
                    );
                }
                if let Some(((min, max), _)) = active_domains {
                    chart = chart.x_range(min, max);
                }
                if let Some((_, (min, max))) = active_domains {
                    chart = chart.y_range(min, max);
                }
                chart.build().map(IntoElement::into_any_element)
            }
            ChartKind::Line => {
                let primary = visible_series.first().copied();
                let x = primary
                    .map(|series| series.x.as_slice())
                    .or(node.x.as_deref())
                    .unwrap_or_default();
                let y = primary
                    .map(|series| series.y.as_slice())
                    .or(node.y.as_deref())
                    .unwrap_or_default();
                let mut chart = line(x, y)
                    .title(node.title.clone())
                    .color(hex_color(
                        primary
                            .and_then(|series| series.color.as_deref())
                            .or(node.color.as_deref()),
                        0xff7f0e,
                    ))
                    .stroke_width(
                        primary
                            .and_then(|series| series.stroke_width)
                            .unwrap_or(node.stroke_width),
                    )
                    .x_scale(scale_type(node.x_log))
                    .y_scale(scale_type(node.y_log))
                    .size(node.width, node.height);
                chart = chart
                    .curve(px_curve(&node.curve))
                    .legend_position(px_legend_position(&node.legend_position))
                    .annotations(px_annotations(node));
                chart = chart.dash_style(&node.dash);
                if let Some(label) = &node.y2_label {
                    chart = chart.y2_label(label.clone());
                }
                if let Some([min, max]) = node.y2_range {
                    chart = chart.y2_range(min, max);
                }
                if let Some(label) = &node.x_label {
                    chart = chart.x_label(label.clone());
                }
                if let Some(label) = &node.y_label {
                    chart = chart.y_label(label.clone());
                }
                if let Some(series) = primary.filter(|series| !series.label.is_empty()) {
                    chart = chart.label(series.label.clone());
                }
                for series in visible_series.iter().copied().skip(1) {
                    chart = if series.secondary_y {
                        chart.add_series_y2_with_x(
                            &series.x,
                            &series.y,
                            (!series.label.is_empty()).then_some(series.label.clone()),
                            hex_color(series.color.as_deref(), 0xff7f0e),
                            series.stroke_width.unwrap_or(node.stroke_width),
                            series.opacity,
                        )
                    } else {
                        chart.add_series_with_x(
                            &series.x,
                            &series.y,
                            (!series.label.is_empty()).then_some(series.label.clone()),
                            hex_color(series.color.as_deref(), 0xff7f0e),
                            series.stroke_width.unwrap_or(node.stroke_width),
                            series.opacity,
                        )
                    };
                    chart = chart.series_dash_style(&series.dash);
                }
                if let Some(((min, max), _)) = active_domains {
                    chart = chart.x_range(min, max);
                }
                if let Some((_, (min, max))) = active_domains {
                    chart = chart.y_range(min, max);
                }
                chart.build().map(IntoElement::into_any_element)
            }
            ChartKind::Bar => {
                let categories = node.categories.as_deref().unwrap_or_default();
                let values = visible_series
                    .first()
                    .map(|series| series.y.as_slice())
                    .or(node.values.as_deref())
                    .unwrap_or_default();
                let mut chart = bar(categories, values);
                for series in visible_series.iter().copied().skip(1) {
                    chart = chart.add_series(
                        &series.y,
                        (!series.label.is_empty()).then_some(series.label.clone()),
                        px_hex_color(series.color.as_deref().unwrap_or(""), 0x2ca02c),
                        series.opacity,
                    );
                }
                chart = chart
                    .title(node.title.clone())
                    .color(hex_color(node.color.as_deref(), 0x2ca02c))
                    .legend_position(px_legend_position(&node.legend_position))
                    .annotations(px_annotations(node))
                    .size(node.width, node.height);
                chart.build().map(IntoElement::into_any_element)
            }
            ChartKind::Heatmap => {
                let raw_z = node.z.as_deref().unwrap_or_default();
                let missing_count = raw_z.iter().filter(|value| value.is_none()).count();
                let fallback = raw_z
                    .iter()
                    .flatten()
                    .copied()
                    .fold(f64::INFINITY, f64::min);
                let z = raw_z
                    .iter()
                    .map(|value| value.unwrap_or(fallback))
                    .collect::<Vec<_>>();
                let mut chart = heatmap(
                    &z,
                    node.width_count.unwrap_or_default(),
                    node.height_count.unwrap_or_default(),
                )
                .title(node.title.clone())
                .color_scale(color_scale(&node.color_scale))
                .x_scale(scale_type(node.x_log))
                .y_scale(scale_type(node.y_log))
                .size(node.width, node.height);
                if let Some(x) = &node.x {
                    chart = chart.x(x);
                }
                if let Some(y) = &node.y {
                    chart = chart.y(y);
                }
                if let Some([min, max]) = node.x_range {
                    chart = chart.x_range(min, max);
                }
                if let Some([min, max]) = node.y_range {
                    chart = chart.y_range(min, max);
                }
                if let Some(aspect_ratio) = node.aspect_ratio {
                    chart = chart.aspect_ratio(aspect_ratio);
                }
                chart.build().map(|element| {
                    let width = node.width_count.unwrap_or_default();
                    let height = node.height_count.unwrap_or_default();
                    let mut heatmap_element = div()
                        .relative()
                        .w(px(node.width))
                        .h(px(node.height))
                        .child(element);
                    if missing_count > 0 && width > 0 && height > 0 {
                        // gpui-px correctly rejects NaN cells. Render finite values through
                        // it, then cover null cells with a neutral overlay at their grid slot.
                        let left = 50.0;
                        let top = if node.title.is_empty() { 10.0 } else { 34.0 };
                        let cell_width = ((node.width - left - 20.0) / width as f32).max(1.0);
                        let cell_height = ((node.height - top - 30.0) / height as f32).max(1.0);
                        for (index, _value) in raw_z
                            .iter()
                            .enumerate()
                            .filter(|(_, value)| value.is_none())
                        {
                            let column = index % width;
                            let row = index / width;
                            heatmap_element = heatmap_element.child(
                                div()
                                    .absolute()
                                    .left(px(left + column as f32 * cell_width))
                                    .top(px(top + (height - row - 1) as f32 * cell_height))
                                    .w(px(cell_width))
                                    .h(px(cell_height))
                                    .bg(theme.muted),
                            );
                        }
                    }
                    let mut container = div()
                        .flex()
                        .flex_col()
                        .gap(px(ds.spacing.grid_unit))
                        .child(heatmap_element);
                    if let Some(label) = &node.color_label {
                        let unit = node
                            .color_unit
                            .as_deref()
                            .map(|unit| format!(" ({unit})"))
                            .unwrap_or_default();
                        let range = node
                            .color_range
                            .map(|[min, max]| format!(": {min:.4}–{max:.4}"))
                            .unwrap_or_default();
                        container = container.child(
                            div()
                                .text_size(px(ds.typography.small_size))
                                .text_color(theme.text_muted)
                                .child(format!("Color: {label}{unit}{range}")),
                        );
                    }
                    if missing_count > 0 {
                        container = container.child(
                            div()
                                .text_size(px(ds.typography.small_size))
                                .text_color(theme.text_muted)
                                .child(format!(
                                    "{missing_count} missing cell{} shown in neutral gray",
                                    if missing_count == 1 { "" } else { "s" }
                                )),
                        );
                    }
                    container.into_any_element()
                })
            }
            ChartKind::Area => {
                let x = node.x.as_deref().unwrap_or_default();
                let y = node.y.as_deref().unwrap_or_default();
                let mut chart = area(x, y)
                    .title(node.title.clone())
                    .color(hex_color(node.color.as_deref(), 0x1f77b4))
                    .opacity(node.opacity)
                    .x_scale(scale_type(node.x_log))
                    .y_scale(scale_type(node.y_log))
                    .size(node.width, node.height);
                if let Some(y0) = &node.y0 {
                    chart = chart.y0(y0);
                }
                if let Some(aspect_ratio) = node.aspect_ratio {
                    chart = chart.aspect_ratio(aspect_ratio);
                }
                chart.build().map(IntoElement::into_any_element)
            }
            ChartKind::BoxPlot => {
                let mut chart = boxplot(
                    node.x.as_deref().unwrap_or_default(),
                    node.y.as_deref().unwrap_or_default(),
                )
                .title(node.title.clone())
                .box_color(hex_color(node.color.as_deref(), 0xdddddd))
                .box_opacity(node.opacity)
                .stroke_width(node.stroke_width)
                .outlier_radius(node.point_radius)
                .x_scale(scale_type(node.x_log))
                .y_scale(scale_type(node.y_log))
                .size(node.width, node.height);
                if let Some(num_bins) = node.num_bins {
                    chart = chart.bins(num_bins);
                }
                if let Some(aspect_ratio) = node.aspect_ratio {
                    chart = chart.aspect_ratio(aspect_ratio);
                }
                chart.build().map(IntoElement::into_any_element)
            }
            ChartKind::Contour => {
                let z = node
                    .z
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|value| value.unwrap_or(0.0))
                    .collect::<Vec<_>>();
                let mut chart = contour(
                    &z,
                    node.width_count.unwrap_or_default(),
                    node.height_count.unwrap_or_default(),
                )
                .title(node.title.clone())
                .color_scale(color_scale(&node.color_scale))
                .opacity(node.opacity)
                .x_scale(scale_type(node.x_log))
                .y_scale(scale_type(node.y_log))
                .size(node.width, node.height);
                if let Some(x) = &node.x {
                    chart = chart.x(x);
                }
                if let Some(y) = &node.y {
                    chart = chart.y(y);
                }
                if let Some(thresholds) = &node.thresholds {
                    chart = chart.thresholds(thresholds.clone());
                }
                if let Some([min, max]) = node.x_range {
                    chart = chart.x_range(min, max);
                }
                if let Some([min, max]) = node.y_range {
                    chart = chart.y_range(min, max);
                }
                chart.build().map(IntoElement::into_any_element)
            }
            ChartKind::Isoline => {
                let z = node
                    .z
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|value| value.unwrap_or(0.0))
                    .collect::<Vec<_>>();
                let mut chart = isoline(
                    &z,
                    node.width_count.unwrap_or_default(),
                    node.height_count.unwrap_or_default(),
                )
                .title(node.title.clone())
                .color(hex_color(node.color.as_deref(), 0x1f77b4))
                .stroke_width(node.stroke_width)
                .opacity(node.opacity)
                .x_scale(scale_type(node.x_log))
                .y_scale(scale_type(node.y_log))
                .size(node.width, node.height);
                if let Some(x) = &node.x {
                    chart = chart.x(x);
                }
                if let Some(y) = &node.y {
                    chart = chart.y(y);
                }
                if let Some(levels) = &node.levels {
                    chart = chart.levels(levels.clone());
                }
                if let Some([min, max]) = node.x_range {
                    chart = chart.x_range(min, max);
                }
                if let Some([min, max]) = node.y_range {
                    chart = chart.y_range(min, max);
                }
                chart.build().map(IntoElement::into_any_element)
            }
            ChartKind::Pie | ChartKind::Donut => {
                let values = node.values.as_deref().unwrap_or_default();
                let mut chart = if node.chart == ChartKind::Donut {
                    donut(values)
                } else {
                    pie(values)
                }
                .title(node.title.clone())
                .hole(node.inner_radius)
                .size(node.width, node.height);
                if let Some(labels) = &node.categories {
                    chart = chart.labels(labels);
                }
                if let Some(aspect_ratio) = node.aspect_ratio {
                    chart = chart.aspect_ratio(aspect_ratio);
                }
                chart.build().map(IntoElement::into_any_element)
            }
            ChartKind::Treemap => {
                let root = native_treemap_node(node.treemap.as_ref().expect("validated treemap"));
                let method = match node.tiling_method.as_str() {
                    "binary" => gpui_px::TilingMethod::Binary,
                    "slice" => gpui_px::TilingMethod::Slice,
                    "dice" => gpui_px::TilingMethod::Dice,
                    "slice_dice" => gpui_px::TilingMethod::SliceDice,
                    _ => gpui_px::TilingMethod::Squarify,
                };
                let mut chart = treemap(&root)
                    .title(node.title.clone())
                    .tiling_method(method)
                    .padding(node.padding)
                    .size(node.width, node.height);
                if let Some(aspect_ratio) = node.aspect_ratio {
                    chart = chart.aspect_ratio(aspect_ratio);
                }
                chart.build().map(IntoElement::into_any_element)
            }
        };

        let chart = result.unwrap_or_else(|error| {
            self.render_error(&format!("chart {}: {error}", node.id), theme, ds)
        });
        let inspection = interaction.as_ref().and_then(|state| {
            chart_inspection(node, state, self.chart_hidden_series.get(&node.id))
        });
        let chart = match interaction {
            Some(state) => interactive(
                stable_element_id(format_args!("python-chart-{}", node.id)),
                chart,
                state,
            )
            .build()
            .into_any_element(),
            None => chart,
        };
        let chart = if let Some(inspection) = inspection {
            let left_margin = 50.0;
            let top_margin = 30.0;
            let plot_width = (node.width - left_margin).max(1.0);
            let plot_height = (node.height - top_margin).max(1.0);
            let cross_x = left_margin + inspection.x_ratio * plot_width;
            let cross_y = top_margin + (1.0 - inspection.y_ratio) * plot_height;
            div()
                .relative()
                .w(px(node.width))
                .h(px(node.height))
                .child(chart)
                .child(
                    div()
                        .absolute()
                        .left(px(cross_x))
                        .top(px(top_margin))
                        .w(px(1.0))
                        .h(px(plot_height))
                        .bg(theme.accent.opacity(0.65)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(left_margin))
                        .top(px(cross_y))
                        .w(px(plot_width))
                        .h(px(1.0))
                        .bg(theme.accent.opacity(0.65)),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(ds.spacing.grid_unit))
                        .top(px(ds.spacing.grid_unit))
                        .px(px(ds.spacing.grid_unit))
                        .py(px(ds.spacing.grid_unit / 2.0))
                        .rounded(px(ds.corners.sm))
                        .bg(theme.surface)
                        .border_1()
                        .border_color(theme.border)
                        .text_size(px(ds.typography.small_size))
                        .text_color(theme.text_primary)
                        .child(format!(
                            "{}: x={:.5}, y={:.5}",
                            inspection.series, inspection.x, inspection.y
                        )),
                )
                .into_any_element()
        } else {
            chart
        };
        let locally_hidden = self.chart_hidden_series.get(&node.id);
        let csv = chart_csv(node, locally_hidden);
        let svg = chart_svg(node, active_domains, locally_hidden);
        let png = chart_png(node, active_domains, locally_hidden);
        let legend = matches!(node.chart, ChartKind::Scatter | ChartKind::Line).then(|| {
            div()
                .flex()
                .flex_wrap()
                .gap(px(ds.spacing.grid_unit))
                .children(node.series.iter().map(|series| {
                    let chart_id = node.id.clone();
                    let series_id = series.id.clone();
                    let selected = self.chart_series_is_visible(&chart_id, series);
                    let color = rgb(hex_color(
                        series.color.as_deref(),
                        if matches!(node.chart, ChartKind::Line) {
                            0xff7f0e
                        } else {
                            0x1f77b4
                        },
                    ));
                    div()
                        .id(stable_element_id(format_args!(
                            "python-chart-legend-{chart_id}-{series_id}"
                        )))
                        .flex()
                        .items_center()
                        .gap(px(ds.spacing.grid_unit / 2.0))
                        .px(px(ds.spacing.grid_unit))
                        .py(px(ds.spacing.grid_unit / 2.0))
                        .rounded(px(ds.corners.sm))
                        .cursor_pointer()
                        .bg(if selected {
                            theme.surface_hover
                        } else {
                            theme.muted
                        })
                        .text_color(if selected {
                            theme.text_primary
                        } else {
                            theme.text_muted
                        })
                        .child(div().w(px(10.0)).h(px(10.0)).rounded_full().bg(color))
                        .child(if series.label.is_empty() {
                            series.id.clone()
                        } else {
                            series.label.clone()
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_chart_series(&chart_id, &series_id);
                            cx.notify();
                        }))
                }))
        });
        div()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.grid_unit))
            .child(chart)
            .children(legend)
            .child(
                div()
                    .id(stable_element_id(format_args!(
                        "python-chart-export-{}",
                        node.id
                    )))
                    .self_start()
                    .px(px(ds.spacing.grid_unit))
                    .py(px(ds.spacing.grid_unit / 2.0))
                    .rounded(px(ds.corners.sm))
                    .bg(theme.surface_hover)
                    .text_color(theme.text_secondary)
                    .text_size(px(ds.typography.small_size))
                    .cursor_pointer()
                    .child("Export CSV…")
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let receiver = cx.prompt_for_new_path(Path::new("."), Some("chart.csv"));
                        let csv = csv.clone();
                        cx.spawn(async move |_, _| {
                            if let Ok(Ok(Some(path))) = receiver.await {
                                std::thread::spawn(move || {
                                    let _ = std::fs::write(path, csv);
                                });
                            }
                        })
                        .detach();
                    })),
            )
            .child(
                div()
                    .id(stable_element_id(format_args!(
                        "python-chart-export-svg-{}",
                        node.id
                    )))
                    .self_start()
                    .px(px(ds.spacing.grid_unit))
                    .py(px(ds.spacing.grid_unit / 2.0))
                    .rounded(px(ds.corners.sm))
                    .bg(theme.surface_hover)
                    .text_color(theme.text_secondary)
                    .text_size(px(ds.typography.small_size))
                    .cursor_pointer()
                    .child("Export SVG…")
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let receiver = cx.prompt_for_new_path(Path::new("."), Some("chart.svg"));
                        let svg = svg.clone();
                        cx.spawn(async move |_, _| {
                            if let Ok(Ok(Some(path))) = receiver.await {
                                std::thread::spawn(move || {
                                    let _ = std::fs::write(path, svg);
                                });
                            }
                        })
                        .detach();
                    })),
            )
            .child(
                div()
                    .id(stable_element_id(format_args!(
                        "python-chart-export-png-{}",
                        node.id
                    )))
                    .self_start()
                    .px(px(ds.spacing.grid_unit))
                    .py(px(ds.spacing.grid_unit / 2.0))
                    .rounded(px(ds.corners.sm))
                    .bg(theme.surface_hover)
                    .text_color(theme.text_secondary)
                    .text_size(px(ds.typography.small_size))
                    .cursor_pointer()
                    .child("Export PNG…")
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let receiver = cx.prompt_for_new_path(Path::new("."), Some("chart.png"));
                        let png = png.clone();
                        cx.spawn(async move |_, _| {
                            if let Ok(Ok(Some(path))) = receiver.await {
                                std::thread::spawn(move || {
                                    let _ = std::fs::write(path, png);
                                });
                            }
                        })
                        .detach();
                    })),
            )
            .into_any_element()
    }

    pub(super) fn render_scene3d(
        &mut self,
        node: &Scene3dNode,
        theme: &Theme,
        ds: &DesignSystem,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let width = node.width.unwrap_or(560.0);
        let height = node.height.unwrap_or(360.0);
        let element = match node.spec.get("kind").and_then(Value::as_str) {
            Some("surface") => self.render_surface_spec(&node.id, &node.spec, theme, ds),
            Some("lines") => self.render_lines_spec(&node.id, &node.spec, theme, ds),
            Some("mesh") => self.render_mesh_summary(&node.id, &node.spec, theme, ds),
            Some("light") => self.render_error("light nodes render inside scene specs", theme, ds),
            Some(kind) => {
                self.render_error(&format!("unsupported scene3d kind: {kind}"), theme, ds)
            }
            None if node.spec.get("children").is_some() => {
                self.render_scene_summary(&node.id, &node.spec, theme, ds)
            }
            None => self.render_error("scene3d spec is missing kind or children", theme, ds),
        };

        let container = div()
            .id(stable_element_id(format_args!("python-scene-{}", node.id)))
            .w(px(width))
            .h(px(height))
            .rounded(px(ds.corners.md))
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            .child(element);
        container.into_any_element()
    }

    fn render_meshplot_error_or_last_valid(
        &mut self,
        node: &MeshPlotNode,
        requested: &MeshPlotSpec,
        error: &str,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let previous = cached_meshplot_fallback(&self.mesh_plots, requested);
        if let Some(previous) = previous
            && let Ok(spec) = serde_json::to_value(previous)
        {
            let mut fallback = node.clone();
            fallback.spec = spec;
            return self.render_meshplot(&fallback, theme, ds, cx);
        }
        self.render_error(error, theme, ds)
    }

    /// Render the host-owned mesh-plot surface and dispatch selection only.
    ///
    /// Resource validation and native plot construction happen before the
    /// retained cache is committed, so a malformed patch leaves the previous
    /// frame visible. Hover is deliberately not sent across the Python pipe.
    pub(super) fn render_meshplot(
        &mut self,
        node: &MeshPlotNode,
        theme: &Theme,
        ds: &DesignSystem,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let source_address = (&node.spec as *const Value) as usize;
        let cached = self.prepared_mesh_plots.get(&node.id).cloned();
        let (spec, prepared, unchanged_source) = if let Some(cached) = cached.as_ref()
            && cached.source_address == source_address
        {
            (Rc::clone(&cached.spec), cached.prepared.clone(), true)
        } else {
            let spec = match MeshPlotSpec::from_value(node.spec.clone()) {
                Ok(spec) => Rc::new(spec),
                Err(error) => return self.render_error(&error, theme, ds),
            };
            if self
                .mesh_plots
                .get(&spec.id)
                .is_some_and(|previous| spec.revision < previous.revision)
            {
                return self.render_meshplot_error_or_last_valid(
                    node,
                    &spec,
                    "stale mesh_plot revision",
                    theme,
                    ds,
                    _cx,
                );
            }
            if let Err(error) = validate_mesh_plot_spec_resources(
                &spec,
                &self.mesh_frames,
                self.last_mesh_patch_id.as_deref(),
            ) {
                return self.render_meshplot_error_or_last_valid(
                    node,
                    &spec,
                    &error.to_string(),
                    theme,
                    ds,
                    _cx,
                );
            }
            let prepared =
                match gpui_python_runtime::native_mesh_plot::prepare(&spec, &self.mesh_frames) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        return self.render_meshplot_error_or_last_valid(
                            node, &spec, &error, theme, ds, _cx,
                        );
                    }
                };
            self.prepared_mesh_plots.insert(
                node.id.clone(),
                CachedNativeMeshPlot {
                    source_address,
                    spec: Rc::clone(&spec),
                    prepared: prepared.clone(),
                },
            );
            (spec, prepared, false)
        };
        let positions = prepared.mesh().positions.len();
        let triangles = prepared.mesh().triangles.len();
        let field_values = prepared.field().map_or(0, |field| field.values.len());
        let width = node.width.or(spec.width).unwrap_or(560.0);
        let height = node.height.or(spec.height).unwrap_or(360.0);
        let mesh_id = spec
            .geometry
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("mesh");

        let geometry_changed = self
            .mesh_plots
            .get(&spec.id)
            .is_none_or(|previous| previous.geometry != spec.geometry);
        let field_changed = self
            .mesh_plots
            .get(&spec.id)
            .is_some_and(|previous| previous.field != spec.field);
        // Retain the same live owner across both field and geometry patches.
        // Geometry replacement advances its dirty domain but deliberately
        // preserves the last complete renderer/camera while an expensive new
        // revolve is prepared; replacing this Rc here would defeat that
        // retained-frame contract before MeshPlot can apply it.
        let retained_state = self.mesh_plot_states.get(&spec.id).cloned();
        let host_selection_callback: Option<Rc<dyn Fn(Option<MeshPlotPick>)>> = match (
            node.selection_action.clone(),
            self.session.as_ref().map(|session| session.event_sink()),
        ) {
            (Some(action), Some(sink)) => {
                let node_id = node.id.clone();
                Some(Rc::new(move |selection| {
                    let payload = mesh_selection_event_payload(selection.as_ref());
                    write_qa_json_artifact(
                        "GPUI_TOOLKIT_QA_HOST_SELECTION_LOG",
                        &serde_json::json!({
                            "event": "host_selection_callback",
                            "action": action.clone(),
                            "node_id": node_id.clone(),
                            "payload": payload.clone(),
                        }),
                    );
                    let _ = sink.dispatch(node_id.clone(), "select", Some(action.clone()), payload);
                }))
            }
            _ => None,
        };
        let (live_plot, live_state) = match Self::build_native_mesh_plot(
            &spec,
            &prepared,
            retained_state.clone(),
            host_selection_callback,
        ) {
            Ok((element, state)) => (Some(element), Some(state)),
            Err(error) => {
                return self
                    .render_meshplot_error_or_last_valid(node, &spec, &error, theme, ds, _cx);
            }
        };
        if let Some(state) = retained_state.as_ref() {
            // Commit dirty domains only after all native option/resource
            // validation succeeded. The builder returns a deferred GPUI
            // element, so this still happens before its next render while an
            // invalid patch leaves the prior live owner untouched.
            state
                .borrow_mut()
                .mark_resources_changed(geometry_changed, field_changed);
        }
        write_qa_json_artifact(
            "GPUI_TOOLKIT_QA_RENDER_TRACE",
            &serde_json::json!({
                "plot_id": spec.id.clone(),
                "mesh_id": mesh_id,
                "positions": positions,
                "triangles": triangles,
                "field_values": field_values,
                "selection_action": node.selection_action.is_some(),
                "session": self.session.is_some(),
                "live_plot": live_plot.is_some(),
            }),
        );
        if !unchanged_source {
            if let Err(error) = self.sync_mesh_plot_resource_refs_for_spec(&spec) {
                return self
                    .render_meshplot_error_or_last_valid(node, &spec, &error, theme, ds, _cx);
            }
            if let Err(error) = self.mesh_plots.upsert((*spec).clone()) {
                return self.render_error(&error, theme, ds);
            }
        }
        if let Some(state) = live_state {
            self.mesh_plot_states.insert(spec.id.clone(), state);
        }
        let plot_error = self.mesh_plot_errors.get(&spec.id).cloned();
        let mut plot_container = div()
            .relative()
            .flex_1()
            .size_full()
            .rounded(px(ds.corners.sm))
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface_hover)
            .child(live_plot.unwrap_or_else(|| {
                div()
                    .text_color(theme.text_secondary)
                    .child(format!(
                        "{} · {} · {}",
                        mesh_id, spec.mode, spec.color_scale
                    ))
                    .into_any_element()
            }));
        if let Some(error) = plot_error {
            plot_container = plot_container.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .p(px(ds.spacing.grid_unit))
                    .bg(theme.alert_error_bg)
                    .text_color(theme.error)
                    .text_size(px(ds.typography.small_size))
                    .child(format!("MeshPlot update rejected: {error}")),
            );
        }
        if env::var_os("GPUI_TOOLKIT_QA_INNER_HIT_TRACE").is_some() {
            let trace_node_id = node.id.clone();
            plot_container = plot_container.on_mouse_down(
                MouseButton::Left,
                move |event: &MouseDownEvent, _window, _cx| {
                    write_qa_json_artifact(
                        "GPUI_TOOLKIT_QA_INNER_HIT_TRACE",
                        &serde_json::json!({
                            "hit": true,
                            "node_id": trace_node_id,
                            "position": [event.position.x.as_f32(), event.position.y.as_f32()],
                        }),
                    );
                },
            );
        }

        let mut container = div()
            .id(stable_element_id(format_args!(
                "python-mesh-plot-{}",
                node.id
            )))
            .w(px(width))
            .h(px(height))
            .flex()
            .flex_col()
            .gap(px(ds.spacing.control_gap))
            .p(px(ds.spacing.card_padding))
            .rounded(px(ds.corners.md))
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .child(
                div()
                    .text_color(theme.text_primary)
                    .font_weight(FontWeight::BOLD)
                    .child(spec.title.clone().unwrap_or_else(|| "Mesh plot".into())),
            )
            .child(div().text_color(theme.text_secondary).child(format!(
                "{} · {} vertices · {} triangles · {} field values",
                spec.view, positions, triangles, field_values
            )))
            .child(plot_container);
        if env::var_os("GPUI_TOOLKIT_QA_HIT_TRACE").is_some() {
            let trace_node_id = node.id.clone();
            container = container.on_mouse_down(
                MouseButton::Left,
                move |event: &MouseDownEvent, _window, _cx| {
                    write_qa_json_artifact(
                        "GPUI_TOOLKIT_QA_HIT_TRACE",
                        &serde_json::json!({
                            "hit": true,
                            "node_id": trace_node_id,
                            "position": [event.position.x.as_f32(), event.position.y.as_f32()],
                        }),
                    );
                },
            );
        }
        container.into_any_element()
    }

    #[allow(unreachable_code)] // Retained temporarily while the legacy builder is removed.
    fn build_native_mesh_plot(
        spec: &MeshPlotSpec,
        prepared: &gpui_python_runtime::native_mesh_plot::PreparedMeshPlot,
        retained_state: Option<Rc<RefCell<MeshPlotState>>>,
        selection_callback: Option<Rc<dyn Fn(Option<MeshPlotPick>)>>,
    ) -> Result<(gpui::AnyElement, Rc<RefCell<MeshPlotState>>), String> {
        gpui_python_runtime::native_mesh_plot::build_prepared(
            spec,
            prepared,
            retained_state,
            selection_callback,
        )
    }

    pub(super) fn render_surface_spec(
        &mut self,
        node_id: &str,
        value: &Value,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let spec = match self.spec_cache.parse_surface(node_id, value) {
            Ok(spec) => spec.clone(),
            Err(error) => return self.render_error(&error, theme, ds),
        };

        match self.gpui_3d.surface_element(&spec) {
            Ok(element) => {
                let range = spec
                    .z_range
                    .map(|range| (range.min, range.max))
                    .unwrap_or_else(|| {
                        spec.z
                            .values
                            .iter()
                            .copied()
                            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                                (min.min(value), max.max(value))
                            })
                    });
                div()
                    .size_full()
                    .flex()
                    .child(div().flex_1().child(interactive_3d_view(
                        &spec.id,
                        element.clone(),
                        element.state(),
                        &spec.interactions,
                        theme,
                        ds,
                    )))
                    .child(scalar_colorbar(spec.labels.z.as_deref(), range, theme, ds))
                    .into_any_element()
            }
            Err(error) => self.render_error(&error.to_string(), theme, ds),
        }
    }

    pub(super) fn render_lines_spec(
        &mut self,
        node_id: &str,
        value: &Value,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let spec = match self.spec_cache.parse_lines(node_id, value) {
            Ok(spec) => spec.clone(),
            Err(error) => return self.render_error(&error, theme, ds),
        };

        match self.gpui_3d.lines_element(&spec) {
            Ok(element) => self
                .gpui_3d
                .lines_state(&spec.id)
                .map(|state| {
                    interactive_3d_view(
                        &spec.id,
                        element.clone(),
                        state,
                        &spec.interactions,
                        theme,
                        ds,
                    )
                })
                .unwrap_or_else(|| element.into_any_element()),
            Err(error) => self.render_error(&error.to_string(), theme, ds),
        }
    }

    pub(super) fn render_mesh_summary(
        &mut self,
        node_id: &str,
        value: &Value,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let spec = match self.spec_cache.parse_mesh(node_id, value) {
            Ok(spec) => spec.clone(),
            Err(error) => return self.render_error(&error, theme, ds),
        };
        match self.gpui_3d.mesh_element(&spec) {
            Ok(element) => {
                let viewport = self
                    .gpui_3d
                    .mesh_state(&spec.id)
                    .map(|state| {
                        interactive_3d_view(&spec.id, element.clone(), state, &[], theme, ds)
                    })
                    .unwrap_or_else(|| element.into_any_element());
                if let Some(field) = &spec.scalar_field {
                    let range =
                        field
                            .range
                            .map(|range| (range.min, range.max))
                            .unwrap_or_else(|| {
                                field.values.iter().copied().fold(
                                    (f64::INFINITY, f64::NEG_INFINITY),
                                    |(min, max), value| (min.min(value), max.max(value)),
                                )
                            });
                    div()
                        .size_full()
                        .flex()
                        .child(div().flex_1().child(viewport))
                        .child(scalar_colorbar(Some("Scalar"), range, theme, ds))
                        .into_any_element()
                } else {
                    viewport
                }
            }
            Err(error) => self.render_error(&error.to_string(), theme, ds),
        }
    }

    pub(super) fn render_scene_summary(
        &mut self,
        node_id: &str,
        value: &Value,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        let spec = match self.spec_cache.parse_scene(node_id, value) {
            Ok(spec) => spec.clone(),
            Err(error) => return self.render_error(&error, theme, ds),
        };
        match self.gpui_3d.scene_element(&spec) {
            Ok(element) => self
                .gpui_3d
                .scene_state(&spec.id)
                .map(|state| {
                    interactive_3d_view(
                        &spec.id,
                        element.clone(),
                        state,
                        &spec.interactions,
                        theme,
                        ds,
                    )
                })
                .unwrap_or_else(|| element.into_any_element()),
            Err(error) => self.render_error(&error.to_string(), theme, ds),
        }
    }

    pub(super) fn render_error(
        &self,
        message: &str,
        theme: &Theme,
        ds: &DesignSystem,
    ) -> AnyElement {
        div()
            .p(px(ds.spacing.card_padding))
            .bg(theme.alert_error_bg)
            .text_color(theme.error)
            .text_size(px(ds.typography.small_size))
            .child(message.to_string())
            .into_any_element()
    }

    fn send_effect_result(&mut self, request_id: String, result: Value) {
        if let Some(session) = &self.session
            && let Err(error) = session.send(&HostMessage::EffectResult { request_id, result })
        {
            self.load_error = Some(error.to_string());
        }
    }

    fn send_command_result(&mut self, request_id: String, result: Value) {
        if let Some(session) = &self.session
            && let Err(error) = session.send(&HostMessage::CommandResult { request_id, result })
        {
            self.load_error = Some(format!("failed to send command result: {error}"));
        }
    }

    fn apply_editor_theme(&mut self, editor: &gpui_themes::EditorTheme, cx: &mut Context<Self>) {
        // gpui-themes owns the complete editor/audio palette. GPUI widgets use
        // this shared core palette, so map the corresponding tokens rather
        // than reimplementing community-theme parsing in Python.
        let mut theme = (*cx.theme()).clone();
        theme.background = editor.background.to_rgba();
        theme.surface = editor.surface.to_rgba();
        theme.surface_hover = editor.surface_hover.to_rgba();
        theme.muted = editor.background_secondary.to_rgba();
        theme.text_primary = editor.text_primary.to_rgba();
        theme.text_secondary = editor.text_secondary.to_rgba();
        theme.text_muted = editor.text_muted.to_rgba();
        theme.text_on_accent = editor.text_on_accent.to_rgba();
        theme.border = editor.border.to_rgba();
        theme.border_hover = editor.border_focused.to_rgba();
        theme.accent = editor.accent.to_rgba();
        theme.accent_hover = editor.accent_hover.to_rgba();
        theme.accent_muted = editor.accent_muted.to_rgba();
        theme.success = editor.success.to_rgba();
        theme.warning = editor.warning.to_rgba();
        theme.error = editor.error.to_rgba();
        theme.info = editor.info.to_rgba();
        cx.set_global(ThemeState {
            theme: Arc::new(theme),
        });
        cx.refresh_windows();
    }

    fn handle_command(
        &mut self,
        request_id: String,
        command: String,
        arguments: Value,
        cx: &mut Context<Self>,
    ) {
        match command.as_str() {
            "runtime.capabilities" => self.send_command_result(
                request_id,
                serde_json::json!({
                    "ok": true,
                    "session_version": gpui_python_runtime::session::PYTHON_APP_SESSION_VERSION,
                    "capabilities": gpui_python_runtime::session::DEFAULT_HOST_CAPABILITIES,
                }),
            ),
            "chart.reset_view" => {
                let result = (|| -> Result<Value, String> {
                    let chart_id = arguments
                        .get("chart_id")
                        .and_then(Value::as_str)
                        .filter(|id| !id.trim().is_empty())
                        .ok_or_else(|| "chart.reset_view requires chart_id".to_string())?;
                    let state = self
                        .chart_interactions
                        .get(chart_id)
                        .ok_or_else(|| format!("chart {chart_id:?} has no interactive state"))?;
                    state.reset_zoom();
                    Ok(serde_json::json!({
                        "ok": true,
                        "chart_id": chart_id,
                        "x": [state.x_domain().0, state.x_domain().1],
                        "y": [state.y_domain().0, state.y_domain().1],
                    }))
                })();
                match result {
                    Ok(result) => {
                        self.send_command_result(request_id, result);
                        cx.notify();
                    }
                    Err(error) => self.send_command_result(
                        request_id,
                        serde_json::json!({"ok": false, "error": error}),
                    ),
                }
            }
            "chart.export_svg" => {
                let result = (|| -> Result<Value, String> {
                    let chart_value = arguments
                        .get("chart")
                        .ok_or_else(|| "chart.export_svg requires chart".to_string())?;
                    let node: ChartNode = serde_json::from_value(chart_value.clone())
                        .map_err(|error| format!("invalid chart export payload: {error}"))?;
                    let active_domains = self
                        .chart_interactions
                        .get(&node.id)
                        .map(|state| (state.x_domain(), state.y_domain()));
                    let hidden = self.chart_hidden_series.get(&node.id);
                    let svg = native_chart_svg(&node, active_domains, hidden)?;
                    if svg.len() > 4 * 1024 * 1024 {
                        return Err("chart SVG exceeds the 4 MiB safety limit".into());
                    }
                    Ok(serde_json::json!({
                        "ok": true,
                        "chart_id": node.id,
                        "format": "svg",
                        "svg": svg,
                    }))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(
                        request_id,
                        serde_json::json!({"ok": false, "error": error}),
                    ),
                }
            }
            "d3.zoom" => {
                let result = (|| -> Result<Value, String> {
                    let original_x = command_domain(&arguments, "original_x")?;
                    let original_y = command_domain(&arguments, "original_y")?;
                    let mut zoom = d3rs::zoom::ZoomState::new(
                        original_x.0, original_x.1, original_y.0, original_y.1,
                    )
                    .with_log_x(arguments.get("log_x").and_then(Value::as_bool).unwrap_or(false))
                    .with_log_y(arguments.get("log_y").and_then(Value::as_bool).unwrap_or(false));
                    let mut back_results = Vec::new();
                    for operation in arguments.get("operations").and_then(Value::as_array).into_iter().flatten() {
                        let kind = operation.get("kind").and_then(Value::as_str)
                            .ok_or_else(|| "zoom operation requires kind".to_string())?;
                        match kind {
                            "zoom_to" => {
                                let x = command_domain(operation, "x")?;
                                let y = command_domain(operation, "y")?;
                                zoom.zoom_to(x.0, x.1, y.0, y.1);
                            }
                            "reset" => zoom.reset(),
                            "back" => back_results.push(zoom.zoom_back()),
                            _ => return Err(format!("unsupported zoom operation: {kind}")),
                        }
                    }
                    let x = zoom.x_domain();
                    let y = zoom.y_domain();
                    Ok(serde_json::json!({
                        "ok": true, "x": [x.0, x.1], "y": [y.0, y.1],
                        "zoomed": zoom.is_zoomed(), "level": zoom.zoom_level(),
                        "back_results": back_results,
                    }))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "d3.array" => {
                let result = (|| -> Result<Value, String> {
                    let data = command_numbers(&arguments, "data")?;
                    let operation = arguments.get("operation").and_then(Value::as_str)
                        .ok_or_else(|| "array command requires operation".to_string())?;
                    let value = match operation {
                        "bisect_left" => {
                            let needle = arguments.get("value").and_then(Value::as_f64)
                                .filter(|value| value.is_finite())
                                .ok_or_else(|| "bisect requires a finite value".to_string())?;
                            serde_json::json!(d3rs::array::bisect_left_f64(&data, needle))
                        }
                        "bisect_right" => {
                            let needle = arguments.get("value").and_then(Value::as_f64)
                                .filter(|value| value.is_finite())
                                .ok_or_else(|| "bisect requires a finite value".to_string())?;
                            serde_json::json!(d3rs::array::bisect_right_f64(&data, needle))
                        }
                        "quantile" => {
                            let percentile = arguments.get("percentile").and_then(Value::as_f64)
                                .ok_or_else(|| "quantile requires percentile".to_string())?;
                            let mut sorted = data.clone();
                            serde_json::json!(d3rs::array::quantile(&mut sorted, percentile))
                        }
                        _ => return Err(format!("unsupported array operation: {operation}")),
                    };
                    Ok(serde_json::json!({"ok": true, "value": value}))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "d3.scale" => {
                let result = (|| -> Result<Value, String> {
                    use d3rs::scale::Scale;
                    let kind = arguments
                        .get("kind")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "scale requires kind".to_string())?;
                    let numeric_values = || command_numbers(&arguments, "values");
                    let pair = |name: &str| -> Result<(f64, f64), String> {
                        let values = command_numbers(&arguments, name)?;
                        match values.as_slice() {
                            [minimum, maximum] => Ok((*minimum, *maximum)),
                            _ => Err(format!("{name} must contain two finite numbers")),
                        }
                    };
                    let strings = |name: &str| -> Result<Vec<String>, String> {
                        arguments
                            .get(name)
                            .and_then(Value::as_array)
                            .ok_or_else(|| format!("{name} must be an array"))?
                            .iter()
                            .map(|value| {
                                value
                                    .as_str()
                                    .map(str::to_string)
                                    .ok_or_else(|| format!("{name} values must be strings"))
                            })
                            .collect()
                    };
                    let clamped = arguments
                        .get("clamp")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let count = arguments
                        .get("tick_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(10) as usize;
                    let output = match kind {
                        "linear" => {
                            let (d0, d1) = pair("domain")?;
                            let (r0, r1) = pair("range")?;
                            let scale = d3rs::scale::LinearScale::new()
                                .domain(d0, d1)
                                .range(r0, r1)
                                .clamp(clamped);
                            serde_json::json!({"values": numeric_values()?.into_iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "ticks": scale.ticks(count)})
                        }
                        "log" => {
                            let (d0, d1) = pair("domain")?;
                            let (r0, r1) = pair("range")?;
                            let scale = d3rs::scale::LogScale::new()
                                .domain(d0, d1)
                                .range(r0, r1)
                                .base(arguments.get("base").and_then(Value::as_f64).unwrap_or(10.0))
                                .clamp(clamped);
                            serde_json::json!({"values": numeric_values()?.into_iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "ticks": scale.ticks(count)})
                        }
                        "power" | "sqrt" => {
                            let (d0, d1) = pair("domain")?;
                            let (r0, r1) = pair("range")?;
                            let exponent = if kind == "sqrt" { 0.5 } else { arguments.get("exponent").and_then(Value::as_f64).unwrap_or(1.0) };
                            let scale = d3rs::scale::PowScale::new().domain(d0, d1).range(r0, r1).exponent(exponent).clamp(clamped);
                            serde_json::json!({"values": numeric_values()?.into_iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "ticks": scale.ticks(count)})
                        }
                        "symlog" => {
                            let (d0, d1) = pair("domain")?;
                            let (r0, r1) = pair("range")?;
                            let scale = d3rs::scale::SymlogScale::new().domain(d0, d1).range(r0, r1).constant(arguments.get("constant").and_then(Value::as_f64).unwrap_or(1.0)).clamp(clamped);
                            serde_json::json!({"values": numeric_values()?.into_iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "ticks": scale.ticks(count)})
                        }
                        "quantize" => {
                            let (d0, d1) = pair("domain")?;
                            let scale = d3rs::scale::QuantizeScale::with_range(strings("range")?).domain(d0, d1);
                            serde_json::json!({"values": numeric_values()?.into_iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "thresholds": scale.thresholds()})
                        }
                        "quantile" => {
                            let scale = d3rs::scale::QuantileScale::with_range(strings("range")?).domain(command_numbers(&arguments, "domain")?);
                            serde_json::json!({"values": numeric_values()?.into_iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "thresholds": scale.quantiles()})
                        }
                        "threshold" => {
                            let scale = d3rs::scale::ThresholdScale::with_range(strings("range")?).domain(command_numbers(&arguments, "domain")?);
                            serde_json::json!({"values": numeric_values()?.into_iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "thresholds": scale.thresholds()})
                        }
                        "ordinal" => {
                            let scale = d3rs::scale::OrdinalScale::new().domain(strings("domain")?).range(strings("range")?);
                            serde_json::json!({"values": strings("values")?.iter().map(|value| scale.scale(value)).collect::<Vec<_>>()})
                        }
                        "band" => {
                            let (r0, r1) = pair("range")?;
                            let scale = d3rs::scale::BandScale::new().domain(strings("domain")?).range(r0, r1).padding_inner(arguments.get("padding_inner").and_then(Value::as_f64).unwrap_or(0.0)).padding_outer(arguments.get("padding_outer").and_then(Value::as_f64).unwrap_or(0.0)).align(arguments.get("align").and_then(Value::as_f64).unwrap_or(0.5)).round(arguments.get("round").and_then(Value::as_bool).unwrap_or(false));
                            serde_json::json!({"values": strings("values")?.iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "bandwidth": scale.bandwidth(), "step": scale.step()})
                        }
                        "point" => {
                            let (r0, r1) = pair("range")?;
                            let scale = d3rs::scale::PointScale::new().domain(strings("domain")?).range(r0, r1).padding(arguments.get("padding_outer").and_then(Value::as_f64).unwrap_or(0.0)).align(arguments.get("align").and_then(Value::as_f64).unwrap_or(0.5)).round(arguments.get("round").and_then(Value::as_bool).unwrap_or(false));
                            serde_json::json!({"values": strings("values")?.iter().map(|value| scale.scale(value)).collect::<Vec<_>>(), "step": scale.step()})
                        }
                        _ => return Err(format!("unsupported scale kind: {kind}")),
                    };
                    Ok(serde_json::json!({"ok": true, "output": output}))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "d3.statistics" => {
                let result = (|| -> Result<Value, String> {
                    let data = command_numbers(&arguments, "data")?;
                    let operation = arguments
                        .get("operation")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "statistics requires operation".to_string())?;
                    let value = match operation {
                        "sum" => serde_json::json!(d3rs::array::sum(&data)),
                        "mean" => serde_json::json!(d3rs::array::mean(&data)),
                        "median" => {
                            let mut values = data.clone();
                            serde_json::json!(d3rs::array::median(&mut values))
                        }
                        "variance" => serde_json::json!(d3rs::array::variance(&data)),
                        "deviation" => serde_json::json!(d3rs::array::deviation(&data)),
                        "quantile" => {
                            let percentile = arguments
                                .get("percentile")
                                .and_then(Value::as_f64)
                                .filter(|value| (0.0..=1.0).contains(value))
                                .ok_or_else(|| {
                                    "quantile requires percentile in [0, 1]".to_string()
                                })?;
                            let mut values = data.clone();
                            serde_json::json!(d3rs::array::quantile(&mut values, percentile))
                        }
                        "extent" => {
                            let minimum = data.iter().copied().reduce(f64::min);
                            let maximum = data.iter().copied().reduce(f64::max);
                            serde_json::json!(minimum.zip(maximum))
                        }
                        "cumsum" => serde_json::json!(d3rs::array::cumsum(&data)),
                        _ => return Err(format!("unsupported statistics operation: {operation}")),
                    };
                    Ok(serde_json::json!({"ok": true, "value": value}))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(
                        request_id,
                        serde_json::json!({"ok": false, "error": error}),
                    ),
                }
            }
            "d3.ticks" => {
                let result = (|| -> Result<Value, String> {
                    let operation = arguments
                        .get("operation")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "ticks requires operation".to_string())?;
                    let start = arguments
                        .get("start")
                        .and_then(Value::as_f64)
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| "ticks requires finite start".to_string())?;
                    let stop = arguments
                        .get("stop")
                        .and_then(Value::as_f64)
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| "ticks requires finite stop".to_string())?;
                    let count = arguments
                        .get("count")
                        .and_then(Value::as_u64)
                        .unwrap_or(10) as usize;
                    let value = match operation {
                        "ticks" => serde_json::json!(d3rs::array::ticks(start, stop, count)),
                        "tick_step" => {
                            serde_json::json!(d3rs::array::tick_step(start, stop, count))
                        }
                        "tick_increment" => {
                            serde_json::json!(d3rs::array::tick_increment(start, stop, count))
                        }
                        "nice" => serde_json::json!(d3rs::array::nice(start, stop, count)),
                        "time_ticks" => {
                            serde_json::json!(d3rs::array::time_ticks(start, stop, count))
                        }
                        "interval" => {
                            let interval = arguments
                                .get("interval")
                                .and_then(Value::as_f64)
                                .filter(|value| value.is_finite() && *value > 0.0)
                                .ok_or_else(|| {
                                    "interval ticks require positive finite interval".to_string()
                                })?;
                            serde_json::json!(d3rs::array::ticks_interval(start, stop, interval))
                        }
                        "log" => {
                            let base = arguments
                                .get("base")
                                .and_then(Value::as_f64)
                                .unwrap_or(10.0);
                            if !base.is_finite() || base <= 1.0 {
                                return Err("log ticks require finite base > 1".into());
                            }
                            serde_json::json!(d3rs::array::log_ticks(
                                start,
                                stop,
                                base,
                                arguments
                                    .get("subdivisions")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(true),
                            ))
                        }
                        _ => return Err(format!("unsupported ticks operation: {operation}")),
                    };
                    Ok(serde_json::json!({"ok": true, "value": value}))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(
                        request_id,
                        serde_json::json!({"ok": false, "error": error}),
                    ),
                }
            }
            "d3.algorithms" => {
                let result = d3_algorithm_command(&arguments);
                match result {
                    Ok(value) => self.send_command_result(request_id, value),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "d3.modules" => {
                let groups: &[(&[&str], &str, &str, &str)] = &[
                    (
                        &["array", "scale", "color", "format", "time", "fetch", "interpolate", "ease", "random"],
                        "direct_command",
                        "gpui_toolkit.d3",
                        "renderer-independent native Rust algorithms",
                    ),
                    (
                        &["axis", "grid", "legend", "text", "shape", "contour", "chord", "force", "hierarchy", "sankey", "hexbin", "delaunay", "polygon", "quadtree", "tile", "lod"],
                        "chart_spec",
                        "gpui_toolkit.charts",
                        "host-native retained chart and geometry specifications",
                    ),
                    (
                        &["brush", "zoom", "dispatch", "drag", "selection", "timer", "transition"],
                        "host_interaction",
                        "gpui_toolkit.events",
                        "host-owned GPUI event and interaction lifecycle",
                    ),
                    (
                        &["surface", "gpu2d", "gpu3d", "sphere_gallery"],
                        "scene_spec",
                        "gpui_toolkit.scene3d",
                        "feature-gated native GPU scene specifications",
                    ),
                ];
                let _modules = groups
                    .iter()
                    .flat_map(|(modules, bridge, python_path, evidence)| {
                        modules.iter().map(move |module| {
                            serde_json::json!({
                                "module": module,
                                "bridge": bridge,
                                "python_path": python_path,
                                "evidence": evidence,
                            })
                        })
                    })
                    .collect::<Vec<_>>();
                self.send_command_result(request_id, d3_module_catalog());
            }
            "d3.reports" => {
                let parity = d3rs::feature_parity::feature_parity_report();
                let benchmark = d3rs::feature_parity::d3_benchmark_coverage_report();
                self.send_command_result(
                    request_id,
                    serde_json::json!({
                        "ok": true,
                        "parity": {
                            "schema_version": parity.schema_version,
                            "report_type": parity.report_type,
                            "reviewed_on": parity.reviewed_on,
                            "entries": parity.entries.iter().map(|entry| serde_json::json!({
                                "id": entry.id,
                                "d3_area": entry.d3_area,
                                "gpui_d3rs_modules": entry.gpui_d3rs_modules,
                                "status": entry.status.as_str(),
                                "evidence": entry.evidence,
                                "release_requirement": entry.release_requirement,
                            })).collect::<Vec<_>>(),
                            "markdown": parity.to_markdown_table(),
                        },
                        "benchmark": {
                            "schema_version": benchmark.schema_version,
                            "report_type": benchmark.report_type,
                            "reviewed_on": benchmark.reviewed_on,
                            "command": benchmark.command,
                            "baseline_policy": benchmark.baseline_policy,
                            "case_count": benchmark.case_count(),
                            "cases": benchmark.cases.iter().map(|case| serde_json::json!({
                                "id": case.id,
                                "module": case.module,
                                "bench_target": case.bench_target,
                                "benchmark_group": case.benchmark_group,
                                "benchmark_id": case.benchmark_id,
                                "dataset_scale": case.dataset_scale,
                                "evidence": case.evidence,
                            })).collect::<Vec<_>>(),
                            "markdown": benchmark.to_markdown_table(),
                        },
                    }),
                );
            }
            "text.rich" => {
                let result = (|| -> Result<Value, String> {
                    let text = arguments.get("text").and_then(Value::as_str).ok_or_else(|| "text.rich requires text".to_string())?;
                    let spans = gpui_pretext::parse_inline_markdown(text);
                    let runs = gpui_pretext::accessibility_runs_for_spans(&spans);
                    let mut segment_starts = Vec::with_capacity(spans.len());
                    let mut offset = 0;
                    for span in &spans { segment_starts.push(offset); offset += span.text.len(); }
                    let bidi_levels = gpui_pretext::bidi::compute_segment_levels(text, &segment_starts);
                    let mut settings = gpui_pretext::FontVariationSettings::default();
                    let mut axes = Vec::new();
                    for axis in arguments.get("axes").and_then(Value::as_array).into_iter().flatten() {
                        let tag = axis.get("tag").and_then(Value::as_str).ok_or_else(|| "variable axis requires tag".to_string())?;
                        let minimum = axis.get("min").and_then(Value::as_f64).ok_or_else(|| "variable axis requires min".to_string())? as f32;
                        let default = axis.get("default").and_then(Value::as_f64).ok_or_else(|| "variable axis requires default".to_string())? as f32;
                        let maximum = axis.get("max").and_then(Value::as_f64).ok_or_else(|| "variable axis requires max".to_string())? as f32;
                        let descriptor = gpui_pretext::VariableFontAxis::new(tag, minimum, default, maximum)?;
                        let value = axis.get("value").and_then(Value::as_f64).unwrap_or(f64::from(default)) as f32;
                        if !value.is_finite() || value < minimum || value > maximum { return Err(format!("variable axis {tag} value is out of range")); }
                        settings = settings.set(tag, value);
                        axes.push(serde_json::json!({"tag": descriptor.tag, "min": descriptor.min, "default": descriptor.default, "max": descriptor.max, "value": value}));
                    }
                    let spans = spans.iter().map(|span| serde_json::json!({"text": span.text, "style": {"bold": span.style.bold, "italic": span.style.italic, "code": span.style.code, "link": span.style.link}})).collect::<Vec<_>>();
                    let runs = runs.iter().map(|run| serde_json::json!({"byte_start": run.byte_range.start, "byte_end": run.byte_range.end, "label": run.label, "role": format!("{:?}", run.role).to_lowercase()})).collect::<Vec<_>>();
                    Ok(serde_json::json!({"ok": true, "spans": spans, "accessibility_runs": runs, "bidi_levels": bidi_levels, "axes": axes, "css_settings": settings.css_settings()}))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "text.reports" => {
                let language = gpui_pretext::language_support_report();
                let locale = gpui_pretext::locale_golden_report();
                let benchmark = gpui_pretext::benchmark_baseline_report();
                let language_notes = language.notes.iter().map(|note| serde_json::json!({"category":note.category,"level":note.level.as_str(),"summary":note.summary,"recommendation":note.recommendation})).collect::<Vec<_>>();
                let locale_cases = locale.cases.iter().map(|case| serde_json::json!({"id":case.id,"locale":case.locale,"category":case.category,"text":case.text,"white_space":format!("{:?}",case.white_space).to_lowercase(),"max_width":case.max_width,"line_height":case.line_height,"expected_lines":case.expected_lines,"note":case.note})).collect::<Vec<_>>();
                let benchmark_cases = benchmark.cases.iter().map(|case| serde_json::json!({"id":case.id,"benchmark_id":case.benchmark_id,"focus":case.focus,"baseline_artifact":case.baseline_artifact,"comparator_artifact":case.comparator_artifact,"release_requirement":case.release_requirement})).collect::<Vec<_>>();
                let comparators = benchmark.comparators.iter().map(|value| serde_json::json!({"id":value.id,"platform":value.platform,"backend":value.backend,"artifact":value.artifact,"requirement":value.requirement})).collect::<Vec<_>>();
                self.send_command_result(request_id, serde_json::json!({"ok":true,
                    "language":{"schema_version":language.schema_version,"report_type":language.report_type,"notes":language_notes},
                    "locale":{"schema_version":locale.schema_version,"report_type":locale.report_type,"cases":locale_cases,"markdown":locale.to_markdown()},
                    "benchmark":{"schema_version":benchmark.schema_version,"report_type":benchmark.report_type,"criterion_command":benchmark.criterion_command,"baseline_policy":benchmark.baseline_policy,"cases":benchmark_cases,"comparators":comparators,"locale_case_ids":benchmark.locale_case_ids,"markdown":benchmark.to_markdown()},
                }));
            }
            "text.prepare_layout" => {
                let result = (|| -> Result<Value, String> {
                    let text = arguments.get("text").and_then(Value::as_str)
                        .ok_or_else(|| "text layout requires text".to_string())?;
                    let max_width = arguments.get("max_width").and_then(Value::as_f64)
                        .filter(|value| value.is_finite() && *value > 0.0)
                        .ok_or_else(|| "text layout requires positive finite max_width".to_string())?;
                    let line_height = arguments.get("line_height").and_then(Value::as_f64).unwrap_or(16.0);
                    let char_width = arguments.get("char_width").and_then(Value::as_f64).unwrap_or(8.0);
                    if !line_height.is_finite() || line_height <= 0.0 || !char_width.is_finite() || char_width <= 0.0 {
                        return Err("text layout line_height and char_width must be positive finite".into());
                    }
                    let measure = FixedTextMeasure(char_width);
                    let mut profile = gpui_pretext::EngineProfile::default();
                    if let Some(value) = arguments.get("profile").and_then(Value::as_object) {
                        profile.line_fit_epsilon = value.get("line_fit_epsilon").and_then(Value::as_f64).unwrap_or(profile.line_fit_epsilon);
                        profile.carry_cjk_after_closing_quote = value.get("carry_cjk_after_closing_quote").and_then(Value::as_bool).unwrap_or(profile.carry_cjk_after_closing_quote);
                        profile.prefer_prefix_widths_for_breakable_runs = value.get("prefer_prefix_widths_for_breakable_runs").and_then(Value::as_bool).unwrap_or(profile.prefer_prefix_widths_for_breakable_runs);
                        profile.prefer_early_soft_hyphen_break = value.get("prefer_early_soft_hyphen_break").and_then(Value::as_bool).unwrap_or(profile.prefer_early_soft_hyphen_break);
                    }
                    if !profile.line_fit_epsilon.is_finite() || profile.line_fit_epsilon < 0.0 {
                        return Err("text layout profile line_fit_epsilon must be finite and non-negative".into());
                    }
                    let white_space = match arguments.get("options").and_then(|value| value.get("white_space")).and_then(Value::as_str).unwrap_or("normal") {
                        "normal" => gpui_pretext::WhiteSpaceMode::Normal,
                        "pre_wrap" => gpui_pretext::WhiteSpaceMode::PreWrap,
                        _ => return Err("text layout white_space must be normal or pre_wrap".into()),
                    };
                    let mut options = gpui_pretext::PrepareOptions::default();
                    options.white_space = white_space;
                    let budget_value = arguments.get("budget").and_then(Value::as_object);
                    let budget = gpui_pretext::TextBudget::new(
                        budget_value.and_then(|value| value.get("max_input_bytes")).and_then(Value::as_u64).unwrap_or(16 * 1024 * 1024) as usize,
                        budget_value.and_then(|value| value.get("max_graphemes")).and_then(Value::as_u64).unwrap_or(4_000_000) as usize,
                        budget_value.and_then(|value| value.get("max_segments")).and_then(Value::as_u64).unwrap_or(1_000_000) as usize,
                    );
                    let prepared = gpui_pretext::prepare_with_segments_with_budget(
                        text, &measure, &profile, &options, budget,
                    ).map_err(|error| error.to_string())?;
                    let mut kp = gpui_pretext::KnuthPlassParams::default();
                    if let Some(value) = arguments.get("knuth_plass").and_then(Value::as_object) {
                        kp.line_penalty = value.get("line_penalty").and_then(Value::as_f64).unwrap_or(kp.line_penalty);
                        kp.hyphen_penalty = value.get("hyphen_penalty").and_then(Value::as_f64).unwrap_or(kp.hyphen_penalty);
                        kp.flagged_demerits = value.get("flagged_demerits").and_then(Value::as_f64).unwrap_or(kp.flagged_demerits);
                        kp.fitness_demerits = value.get("fitness_demerits").and_then(Value::as_f64).unwrap_or(kp.fitness_demerits);
                        kp.tolerance = value.get("tolerance").and_then(Value::as_f64).unwrap_or(kp.tolerance);
                        kp.looseness_recovery = value.get("looseness_recovery").and_then(Value::as_bool).unwrap_or(kp.looseness_recovery);
                    }
                    if ![kp.line_penalty, kp.hyphen_penalty, kp.flagged_demerits, kp.fitness_demerits, kp.tolerance].into_iter().all(f64::is_finite) || kp.tolerance < 0.0 {
                        return Err("text layout Knuth-Plass parameters must be finite with non-negative tolerance".into());
                    }
                    let strategy = match arguments.get("strategy").and_then(Value::as_str).unwrap_or("greedy") {
                        "greedy" => gpui_pretext::LineBreakStrategy::Greedy,
                        "optimal" => gpui_pretext::LineBreakStrategy::Optimal,
                        _ => return Err("text layout strategy must be greedy or optimal".into()),
                    };
                    let layout = gpui_pretext::layout_with_lines_and_strategy(
                        &prepared, max_width, line_height, &profile, strategy, &kp,
                    );
                    let lines = layout.lines.into_iter().map(|line| serde_json::json!({
                        "text": line.text, "width": line.width,
                        "start": {"segment_index": line.start.segment_index, "grapheme_index": line.start.grapheme_index},
                        "end": {"segment_index": line.end.segment_index, "grapheme_index": line.end.grapheme_index},
                    })).collect::<Vec<_>>();
                    Ok(serde_json::json!({
                        "ok": true, "line_count": layout.line_count, "height": layout.height,
                        "lines": lines, "segments": prepared.segments,
                    }))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "audio.accessibility" => {
                let result = arguments.get("node").cloned()
                    .ok_or_else(|| "audio.accessibility requires node".to_string())
                    .and_then(|value| serde_json::from_value::<UiNode>(value).map_err(|error| error.to_string()))
                    .and_then(|node| match node {
                        UiNode::AudioPotentiometer(node) => {
                            let scale = if node.scale == "logarithmic" { gpui_audio_kit::AudioScale::Logarithmic } else { gpui_audio_kit::AudioScale::Linear };
                            let summary = Potentiometer::new("audio-accessibility-potentiometer").value(node.value).min(node.minimum).max(node.maximum).label(node.label).unit(node.unit).scale(scale).selected(node.selected).disabled(node.disabled).aria_label(node.aria_label.unwrap_or(node.id)).accessibility_summary();
                            Ok(vec![audio_accessibility_json(&summary)])
                        }
                        UiNode::AudioVerticalSlider(node) => {
                            let scale = if node.scale == "logarithmic" { gpui_audio_kit::AudioScale::Logarithmic } else { gpui_audio_kit::AudioScale::Linear };
                            let summary = VerticalSlider::new("audio-accessibility-slider").value(node.value).min(node.minimum).max(node.maximum).label(node.label).unit(node.unit).scale(scale).selected(node.selected).disabled(node.disabled).peak(node.peak).aria_label(node.aria_label.unwrap_or(node.id)).accessibility_summary();
                            Ok(vec![audio_accessibility_json(&summary)])
                        }
                        UiNode::AudioVolumeKnob(node) => {
                            let summary = VolumeKnob::new().value(node.value as f32).label(node.label).muted(node.muted).aria_label(node.aria_label.unwrap_or(node.id)).accessibility_summary();
                            Ok(vec![audio_accessibility_json(&summary)])
                        }
                        UiNode::AudioHorizontalMeter(node) => {
                            let streamed = node.stream_id.as_deref().and_then(|id| self.audio_frames.get(id)).and_then(|frame| frame.meter_levels()).map(|values| values.iter().map(|value| f64::from(*value)).collect::<Vec<_>>());
                            let levels = streamed.as_deref().unwrap_or(&node.levels);
                            Ok(levels.iter().enumerate().map(|(index, level)| {
                                let label = node.channel_names.get(index).cloned().unwrap_or_else(|| format!("Channel {}", index + 1));
                                let summary = gpui_audio_kit::horizontal_meter_accessibility_summary(label, *level, &gpui_audio_kit::TickConfig::db_linear(-60.0, 0.0));
                                audio_accessibility_json(&summary)
                            }).collect())
                        }
                        UiNode::AudioLevelMeter(node) => {
                            let stream = node.stream_id.as_deref().and_then(|id| self.audio_frames.get(id));
                            let streamed_levels = stream.and_then(|frame| frame.meter_levels()).map(|values| values.iter().map(|value| f64::from(*value)).collect::<Vec<_>>());
                            let streamed_peaks = stream.and_then(|frame| frame.meter_peaks()).map(|values| values.iter().map(|value| f64::from(*value)).collect::<Vec<_>>());
                            let levels = streamed_levels.as_deref().unwrap_or(&node.levels);
                            let peaks = streamed_peaks.as_deref().unwrap_or(&node.peaks);
                            Ok(levels.iter().enumerate().map(|(index, level)| {
                                let label = node.channel_names.get(index).cloned().unwrap_or_else(|| format!("Channel {}", index + 1));
                                let mut meter = LevelMeterElement::new(*level, label);
                                if let Some(peak) = peaks.get(index) { meter = meter.peak(*peak); }
                                audio_accessibility_json(&meter.accessibility_summary())
                            }).collect())
                        }
                        UiNode::AudioSpectrum(node) => {
                            let stream = node.stream_id.as_deref().and_then(|id| self.audio_frames.get(id)).filter(|frame| frame.frame_kind == AudioFrameKind::Spectrum);
                            let bins = stream.map(|frame| frame.values.len()).unwrap_or(node.magnitudes.len());
                            let minimum_frequency = stream.and_then(|frame| frame.minimum_frequency).unwrap_or(f64::from(node.minimum_frequency));
                            let maximum_frequency = stream.and_then(|frame| frame.maximum_frequency).unwrap_or(f64::from(node.maximum_frequency));
                            Ok(vec![serde_json::json!({
                            "control_type":"spectrum", "label":node.id, "role":"img",
                            "value_now":null, "value_min":null, "value_max":null, "value_text":null,
                            "unit":"dB", "normalized":null, "scale":"logarithmic", "selected":false,
                            "disabled":false, "muted":false, "peak_value":null,
                            "description":format!("Audio spectrum with {} bins from {:.0} to {:.0} Hz", bins, minimum_frequency, maximum_frequency),
                        })])
                        }
                        _ => Err("node is not an audio declaration".into()),
                    });
                match result {
                    Ok(summaries) => self.send_command_result(request_id, serde_json::json!({"ok": true, "summaries": summaries})),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "audio.reports" => {
                let automation = gpui_audio_kit::audio_automation_pattern_report();
                let visual = gpui_audio_kit::audio_visual_regression_manifest();
                let tokens = gpui_audio_kit::AudioDesignTokens::default();
                self.send_command_result(request_id, serde_json::json!({
                    "ok": true,
                    "automation": {
                        "schema_version": automation.schema_version,
                        "report_type": automation.report_type,
                        "unique_ids": automation.validate_unique_ids(),
                        "patterns": automation.patterns.iter().map(|pattern| serde_json::json!({
                            "id": pattern.id,
                            "parameter_family": pattern.parameter_family,
                            "recommended_control": pattern.recommended_control,
                            "scale": pattern.scale,
                            "automation_sources": pattern.automation_sources,
                            "expected_interactions": pattern.expected_interactions,
                            "accessibility_summary_contract": pattern.accessibility_summary_contract,
                            "release_evidence": pattern.release_evidence,
                            "status": pattern.status.label(),
                        })).collect::<Vec<_>>(),
                        "markdown": automation.to_markdown(),
                    },
                    "visual": {
                        "schema_version": visual.schema_version,
                        "report_type": visual.report_type,
                        "crate_name": visual.crate_name,
                        "crate_version": visual.crate_version,
                        "capture_count": visual.capture_count(),
                        "expected_capture_count": visual.expected_capture_count(),
                        "unique_capture_ids": visual.validate_unique_capture_ids(),
                        "components": visual.components().into_iter().collect::<Vec<_>>(),
                        "markdown": visual.to_markdown_table(),
                    },
                    "design_tokens": {
                        "knob_arc_start_deg": tokens.knob_arc_start_deg,
                        "knob_arc_sweep_deg": tokens.knob_arc_sweep_deg,
                        "knob_arc_widths": tokens.knob_arc_widths,
                        "knob_arc_track_widths": tokens.knob_arc_track_widths,
                        "knob_arc_glow": tokens.knob_arc_glow,
                        "knob_arc_segments": tokens.knob_arc_segments,
                        "knob_border_width": tokens.knob_border_width,
                        "knob_label_style": tokens.knob_label_style,
                        "knob_indicator_style": tokens.knob_indicator_style,
                        "slider_track_widths": tokens.slider_track_widths,
                        "meter_label_style": tokens.meter_label_style,
                        "meter_use_gradient": tokens.meter_use_gradient,
                        "meter_corner_radius": tokens.meter_corner_radius,
                        "meter_glow": tokens.meter_glow,
                        "toggle_variant": tokens.toggle_variant,
                        "corner_radius": tokens.corner_radius,
                        "min_touch_target": tokens.min_touch_target,
                        "control_padding_x": tokens.control_padding_x,
                        "control_padding_y": tokens.control_padding_y,
                        "animation_duration_ms": tokens.animation_duration_ms,
                        "prefer_spring": tokens.prefer_spring,
                        "spring_stiffness": tokens.spring_stiffness,
                        "spring_damping": tokens.spring_damping,
                    },
                }));
            }
            "px.reports" => {
                let capability = gpui_px::chart_capability_report();
                let visual = gpui_px::chart_visual_regression_manifest();
                let result = serde_json::json!({
                    "ok": true,
                    "capability": {
                        "schema_version": capability.schema_version,
                        "report_type": capability.report_type,
                        "reviewed_on": capability.reviewed_on,
                        "all_release_ready": capability.all_release_ready(),
                        "entries": capability.entries.iter().map(|entry| serde_json::json!({
                            "id": entry.id,
                            "capability": entry.capability,
                            "chart_families": entry.chart_families,
                            "story_ids": entry.story_ids,
                            "test_contracts": entry.test_contracts,
                            "status": entry.status.as_str(),
                            "evidence": entry.evidence,
                            "release_requirement": entry.release_requirement,
                        })).collect::<Vec<_>>(),
                        "markdown": capability.to_markdown_table(),
                    },
                    "visual": {
                        "schema_version": visual.schema_version,
                        "report_type": visual.report_type,
                        "crate_name": visual.crate_name,
                        "crate_version": visual.crate_version,
                        "capture_count": visual.capture_count(),
                        "expected_capture_count": visual.expected_capture_count(),
                        "unique_capture_ids": visual.validate_unique_capture_ids(),
                        "chart_families": visual.chart_families().into_iter().collect::<Vec<_>>(),
                        "markdown": visual.to_markdown_table(),
                    },
                });
                self.send_command_result(request_id, result);
            }
            "builder.solve_matrix" => {
                let result = (|| -> Result<Value, String> {
                    let root: BuilderLayoutSpec = serde_json::from_value(
                        arguments
                            .get("root")
                            .cloned()
                            .ok_or_else(|| "builder matrix requires root".to_string())?,
                    )
                    .map_err(|error| format!("invalid builder root: {error}"))?;
                    let viewports = serde_json::from_value::<Vec<BuilderViewportSpec>>(
                        arguments
                            .get("viewports")
                            .cloned()
                            .ok_or_else(|| "builder matrix requires viewports".to_string())?,
                    )
                    .map_err(|error| format!("invalid builder viewports: {error}"))?;
                    for viewport in &viewports {
                        if viewport.label.is_empty()
                            || !viewport.width.is_finite()
                            || viewport.width < 0.0
                            || !viewport.height.is_finite()
                            || viewport.height < 0.0
                        {
                            return Err(
                                "builder viewports require labels and finite non-negative sizes"
                                    .into(),
                            );
                        }
                    }
                    let preference_value = arguments.get("preferences");
                    let ratios = preference_value
                        .and_then(|value| value.get("ratios"))
                        .cloned()
                        .map(serde_json::from_value::<Vec<BuilderRatioPreference>>)
                        .transpose()
                        .map_err(|error| format!("invalid builder ratio preferences: {error}"))?
                        .unwrap_or_default();
                    let collapsed = preference_value
                        .and_then(|value| value.get("collapsed"))
                        .cloned()
                        .map(serde_json::from_value::<Vec<BuilderCollapsePreference>>)
                        .transpose()
                        .map_err(|error| {
                            format!("invalid builder collapse preferences: {error}")
                        })?
                        .unwrap_or_default();
                    let ratio_values = ratios
                        .iter()
                        .map(|value| {
                            if !value.ratio.is_finite() {
                                return Err(format!(
                                    "builder ratio for {} must be finite",
                                    value.id
                                ));
                            }
                            let axis = match value.axis.as_str() {
                                "horizontal" => gpui_builder::Axis::Horizontal,
                                "vertical" => gpui_builder::Axis::Vertical,
                                other => {
                                    return Err(format!(
                                        "unsupported builder preference axis: {other}"
                                    ));
                                }
                            };
                            Ok((value.id.as_str(), axis, value.ratio))
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    let collapsed_values = collapsed
                        .iter()
                        .map(|value| (value.id.as_str(), value.collapsed))
                        .collect::<Vec<_>>();
                    let preferences =
                        gpui_builder::LayoutPreferences::new(&ratio_values, &collapsed_values);
                    let char_width = arguments
                        .get("char_width")
                        .and_then(Value::as_f64)
                        .unwrap_or(8.0);
                    if !char_width.is_finite() || char_width <= 0.0 {
                        return Err(
                            "builder matrix char_width must be positive and finite".into(),
                        );
                    }
                    let include_retained = arguments
                        .get("include_retained")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                    let measure = FixedTextMeasure(char_width);
                    with_builder_node(
                        &root,
                        &measure,
                        Box::new(|root| {
                            let native_viewports = viewports
                                .iter()
                                .map(|viewport| gpui_builder::LayoutViewport::new(
                                    &viewport.label,
                                    viewport.width,
                                    viewport.height,
                                ))
                                .collect::<Vec<_>>();
                            let matrix = gpui_builder::solve_snapshot_matrix(
                                &root,
                                &native_viewports,
                                &preferences,
                            );
                            let snapshots = matrix
                                .snapshots
                                .iter()
                                .map(|snapshot| {
                                    serde_json::json!({
                                        "label": snapshot.label,
                                        "width": snapshot.width,
                                        "height": snapshot.height,
                                        "root": builder_solved_node(&snapshot.root),
                                        "visible_ids": snapshot.visible_ids(),
                                        "collapsed_labels": snapshot.collapsed_labels(),
                                        "active_tiers": snapshot.active_tiers(),
                                        "resolved_axes": snapshot.resolved_axes(),
                                    })
                                })
                                .collect::<Vec<_>>();
                            let retained_snapshots = if include_retained {
                                let mut solver = gpui_builder::RetainedLayoutSolver::with_capacity(
                                    gpui_builder::inspect_layout(&root).nodes().len(),
                                );
                                native_viewports
                                    .iter()
                                    .map(|viewport| {
                                        let tree = solver.solve(
                                            &root,
                                            viewport.width,
                                            viewport.height,
                                            &preferences,
                                        );
                                        serde_json::json!({
                                            "label": viewport.label,
                                            "width": viewport.width,
                                            "height": viewport.height,
                                            "root": builder_solved_ref(tree.root()),
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                Vec::new()
                            };
                            Ok(serde_json::json!({
                                "ok": true,
                                "snapshots": snapshots,
                                "retained_snapshots": retained_snapshots,
                                "report": matrix.to_text(),
                                "markdown": matrix.to_markdown_table(),
                            }))
                        }),
                    )
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(
                        request_id,
                        serde_json::json!({"ok": false, "error": error}),
                    ),
                }
            }
            "builder.solve" => {
                let result = (|| -> Result<Value, String> {
                    let width = arguments
                        .get("width")
                        .and_then(Value::as_f64)
                        .filter(|value| value.is_finite() && *value >= 0.0)
                        .ok_or_else(|| {
                            "builder solve requires finite non-negative width".to_string()
                        })? as f32;
                    let height = arguments
                        .get("height")
                        .and_then(Value::as_f64)
                        .filter(|value| value.is_finite() && *value >= 0.0)
                        .ok_or_else(|| {
                            "builder solve requires finite non-negative height".to_string()
                        })? as f32;
                    let char_width = arguments
                        .get("char_width")
                        .and_then(Value::as_f64)
                        .unwrap_or(8.0);
                    if !char_width.is_finite() || char_width <= 0.0 {
                        return Err("builder solve char_width must be positive and finite".into());
                    }
                    let root: BuilderLayoutSpec = serde_json::from_value(
                        arguments
                            .get("root")
                            .cloned()
                            .ok_or_else(|| "builder solve requires root".to_string())?,
                    )
                    .map_err(|error| format!("invalid builder root: {error}"))?;
                    let preferences = arguments.get("preferences");
                    let ratios = preferences
                        .and_then(|value| value.get("ratios"))
                        .cloned()
                        .map(serde_json::from_value::<Vec<BuilderRatioPreference>>)
                        .transpose()
                        .map_err(|error| format!("invalid builder ratio preferences: {error}"))?
                        .unwrap_or_default();
                    let collapsed = preferences
                        .and_then(|value| value.get("collapsed"))
                        .cloned()
                        .map(serde_json::from_value::<Vec<BuilderCollapsePreference>>)
                        .transpose()
                        .map_err(|error| {
                            format!("invalid builder collapse preferences: {error}")
                        })?
                        .unwrap_or_default();
                    let accessibility = arguments
                        .get("accessibility")
                        .cloned()
                        .map(serde_json::from_value::<Vec<BuilderAccessibilitySpec>>)
                        .transpose()
                        .map_err(|error| format!("invalid builder accessibility metadata: {error}"))?
                        .unwrap_or_default();
                    let ratio_values = ratios
                        .iter()
                        .map(|value| {
                            if !value.ratio.is_finite() {
                                return Err(format!(
                                    "builder ratio for {} must be finite",
                                    value.id
                                ));
                            }
                            Ok((value.id.as_str(), builder_axis(match value.axis.as_str() {
                                "horizontal" => BuilderAxis::Horizontal,
                                "vertical" => BuilderAxis::Vertical,
                                other => {
                                    return Err(format!(
                                        "unsupported builder preference axis: {other}"
                                    ));
                                }
                            }), value.ratio))
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    let collapsed_values = collapsed
                        .iter()
                        .map(|value| (value.id.as_str(), value.collapsed))
                        .collect::<Vec<_>>();
                    let preferences =
                        gpui_builder::LayoutPreferences::new(&ratio_values, &collapsed_values);
                    let measure = FixedTextMeasure(char_width);
                    with_builder_node(
                        &root,
                        &measure,
                        Box::new(|root| {
                            let validation = gpui_builder::validate_layout(&root);
                            let declaration_inspection = gpui_builder::inspect_layout(&root);
                            let solved =
                                gpui_builder::solve(&root, width, height, &preferences);
                            let solved_inspection = gpui_builder::inspect_solved(&solved);
                            let debug = solved.debug_report_with_source(&root);
                            let accessibility_metadata = accessibility
                                .iter()
                                .map(|value| {
                                    Ok((
                                        value.id.as_str(),
                                        gpui_builder::AccessibilityMetadata {
                                            role: builder_accessibility_role(value.role.as_deref())?,
                                            label: value.label.as_deref(),
                                            description: value.description.as_deref(),
                                        },
                                    ))
                                })
                                .collect::<Result<Vec<_>, String>>()?;
                            let accessibility = gpui_builder::accessibility_tree_from_solved(
                                &solved,
                                &accessibility_metadata,
                            );
                            let issues = validation
                                .issues()
                                .iter()
                                .map(|issue| {
                                    serde_json::json!({
                                        "severity": match issue.severity {
                                            gpui_builder::LayoutIssueSeverity::Error => "error",
                                            gpui_builder::LayoutIssueSeverity::Warning => "warning",
                                        },
                                        "kind": builder_issue_kind(&issue.kind),
                                        "node_id": issue.node_id,
                                        "path": issue.path,
                                        "message": issue.message,
                                    })
                                })
                                .collect::<Vec<_>>();
                            let warnings = debug
                                .warnings()
                                .iter()
                                .map(|warning| {
                                    serde_json::json!({
                                        "code": warning.code(),
                                        "node_id": warning.node_id,
                                        "message": warning.to_string(),
                                        "remediation": warning.remediation(),
                                    })
                                })
                                .collect::<Vec<_>>();
                            Ok(serde_json::json!({
                                "ok": true,
                                "solved": builder_solved_node(&solved),
                                "collapsed_tabs": solved.collapsed_tabs().into_iter().map(|(id, label)| serde_json::json!({"id": id, "label": label})).collect::<Vec<_>>(),
                                "accessibility": builder_accessibility_node(&accessibility.root),
                                "validation": {
                                    "clean": validation.is_clean(),
                                    "error_count": validation.error_count(),
                                    "warning_count": validation.warning_count(),
                                    "issues": issues,
                                    "report": validation.to_text(),
                                },
                                "inspection": {
                                    "declaration_report": declaration_inspection.to_text(),
                                    "solved_report": solved_inspection.to_text(),
                                },
                                "debug": {
                                    "report": debug.to_string(),
                                    "warnings": warnings,
                                },
                            }))
                        }),
                    )
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(
                        request_id,
                        serde_json::json!({"ok": false, "error": error}),
                    ),
                }
            }
            "builder.solve_chassis" => {
                let result = (|| -> Result<Value, String> {
                    let width = arguments.get("width").and_then(Value::as_f64)
                        .filter(|value| value.is_finite() && *value >= 0.0)
                        .ok_or_else(|| "builder chassis requires finite non-negative width".to_string())? as f32;
                    let sections = arguments.get("sections").and_then(Value::as_array)
                        .ok_or_else(|| "builder chassis requires sections".to_string())?
                        .iter().map(|section| {
                            let id = section.get("id").and_then(Value::as_str).filter(|value| !value.is_empty())
                                .ok_or_else(|| "builder section requires id".to_string())?.to_string();
                            let min_width = section.get("min_width").and_then(Value::as_f64)
                                .filter(|value| value.is_finite() && *value >= 0.0)
                                .ok_or_else(|| format!("builder section {id} requires min_width"))? as f32;
                            let preferred_width = section.get("preferred_width").and_then(Value::as_f64)
                                .filter(|value| value.is_finite() && *value >= min_width as f64)
                                .ok_or_else(|| format!("builder section {id} requires preferred_width >= min_width"))? as f32;
                            let priority = section.get("priority").and_then(Value::as_f64).unwrap_or(1.0);
                            if !priority.is_finite() { return Err(format!("builder section {id} priority must be finite")); }
                            Ok(gpui_builder::plugin_chassis::SectionSpec {
                                id,
                                eyebrow: section.get("eyebrow").and_then(Value::as_str).unwrap_or("").to_string(),
                                title: section.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
                                caption: section.get("caption").and_then(Value::as_str).map(str::to_string),
                                rows: section.get("rows").and_then(Value::as_array).map(|rows| rows.iter().map(builder_chassis_row).collect::<Result<Vec<_>, String>>()).transpose()?.unwrap_or_default(),
                                min_width, preferred_width, priority: priority as f32,
                            })
                        }).collect::<Result<Vec<_>, String>>()?;
                    let header = arguments.get("header");
                    let header = gpui_builder::plugin_chassis::HeaderSpec {
                        brand_mark: header.and_then(|value| value.get("brand_mark")).and_then(Value::as_str).unwrap_or("").to_string(),
                        title: header.and_then(|value| value.get("title")).and_then(Value::as_str).unwrap_or("").to_string(),
                        subtitle: header.and_then(|value| value.get("subtitle")).and_then(Value::as_str).unwrap_or("").to_string(),
                    };
                    let mut chassis = gpui_builder::plugin_chassis::ChassisLayout::new(header, sections);
                    if let Some(footer) = arguments.get("footer").filter(|value| !value.is_null()) {
                        let ticks = footer.get("ticks").and_then(Value::as_array).ok_or_else(|| "builder chassis footer requires ticks".to_string())?.iter().map(|tick| tick.as_str().map(str::to_string).ok_or_else(|| "builder chassis footer ticks must be strings".to_string())).collect::<Result<Vec<_>, String>>()?;
                        chassis = chassis.with_footer(gpui_builder::plugin_chassis::FooterSpec {
                            ticks,
                            serial: footer.get("serial").and_then(Value::as_str).unwrap_or("").to_string(),
                        });
                    }
                    let solved = chassis.solve(width);
                    Ok(serde_json::json!({"ok": true, "total_width": solved.total_width, "sections": solved.sections.into_iter().map(|section| serde_json::json!({"id": section.id, "width": section.width, "visible": section.visible})).collect::<Vec<_>>() }))
                })();
                match result { Ok(result) => self.send_command_result(request_id, result), Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})) }
            }
            "design.tokens" => {
                let result = (|| -> Result<Value, String> {
                    let format = arguments.get("format").and_then(Value::as_str)
                        .ok_or_else(|| "design-token command requires format".to_string())?;
                    let format = gpui_design_tools::DesignTokenFormat::parse(format)
                        .map_err(|error| error.to_string())?;
                    let operation = arguments.get("operation").and_then(Value::as_str)
                        .ok_or_else(|| "design-token command requires operation".to_string())?;
                    match operation {
                        "export" => Ok(serde_json::json!({
                            "ok": true,
                            "output": gpui_design_tools::export_design_tokens(format)
                                .map_err(|error| error.to_string())?,
                        })),
                        "import" => {
                            let input = arguments.get("input").and_then(Value::as_str)
                                .ok_or_else(|| "design-token import requires input".to_string())?;
                            let imported = gpui_design_tools::import_design_tokens(input, format)
                                .map_err(|error| error.to_string())?;
                            Ok(serde_json::json!({
                                "ok": true, "preset_count": imported.preset_count,
                                "token_count": imported.token_count, "raw": imported.raw,
                            }))
                        }
                        "validate" => {
                            let input = arguments.get("input").and_then(Value::as_str)
                                .ok_or_else(|| "design-token validation requires input".to_string())?;
                            let report = gpui_design_tools::validate_design_tokens(
                                input, format,
                                arguments.get("render_markdown").and_then(Value::as_bool).unwrap_or(false),
                            ).map_err(|error| error.to_string())?;
                            let report = serde_json::to_value(report).map_err(|error| error.to_string())?;
                            Ok(serde_json::json!({"ok": true, "report": report}))
                        }
                        "handoff" => {
                            let report = gpui_design_tools::design_tooling_handoff_report();
                            let report = serde_json::to_value(report).map_err(|error| error.to_string())?;
                            Ok(serde_json::json!({"ok": true, "report": report}))
                        }
                        _ => Err(format!("unsupported design-token operation: {operation}")),
                    }
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "design.reports" => {
                let result = (|| -> Result<Value, String> {
                    let tokens = serde_json::to_value(gpui_design::DesignTokenExport::for_all_presets()).map_err(|error| error.to_string())?;
                    let documentation = serde_json::to_value(gpui_design::DesignDocumentationReport::for_all_presets()).map_err(|error| error.to_string())?;
                    let release = serde_json::to_value(gpui_design::DesignReleasePresentation::for_all_presets()).map_err(|error| error.to_string())?;
                    Ok(serde_json::json!({"ok": true, "tokens": tokens, "documentation": documentation, "release": release}))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "scaffolder.preview" | "scaffolder.write" => {
                let result = (|| -> Result<Value, String> {
                    let name = arguments.get("name").and_then(Value::as_str)
                        .ok_or_else(|| "scaffolder command requires name".to_string())?;
                    let output_dir = arguments.get("output_dir").and_then(Value::as_str)
                        .ok_or_else(|| "scaffolder command requires output_dir".to_string())?;
                    let options = gpui_scaffolder::ScaffoldOptions {
                        name: name.into(), output_dir: PathBuf::from(output_dir),
                        force: arguments.get("force").and_then(Value::as_bool).unwrap_or(false),
                        dry_run: arguments.get("dry_run").and_then(Value::as_bool).unwrap_or(false),
                    };
                    if command == "scaffolder.preview" {
                        let preview = gpui_scaffolder::preview_scaffold(&options).map_err(|error| error.to_string())?;
                        Ok(serde_json::json!({
                            "ok": true, "app_dir": preview.app.app_dir, "package_name": preview.app.package_name,
                            "title": preview.app.title, "files": preview.files,
                        }))
                    } else {
                        let app = gpui_scaffolder::scaffold_app(&options).map_err(|error| error.to_string())?;
                        Ok(serde_json::json!({"ok": true, "app_dir": app.app_dir, "package_name": app.package_name, "title": app.title}))
                    }
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "ui.reports" => {
                let accessibility = gpui_ui_kit::accessibility_readiness_report();
                let focus = gpui_ui_kit::focus_integration_report();
                let behavior = gpui_ui_kit::component_behavior_report();
                self.send_command_result(
                    request_id,
                    serde_json::json!({
                        "ok": true,
                        "accessibility": {
                            "schema_version": accessibility.schema_version,
                            "report_type": accessibility.report_type,
                            "reviewed_on": accessibility.reviewed_on,
                            "entry_count": accessibility.entries.len(),
                            "all_release_ready": accessibility.all_release_ready(),
                            "markdown": accessibility.to_markdown_table(),
                        },
                        "focus": {
                            "schema_version": focus.schema_version,
                            "report_type": focus.report_type,
                            "reviewed_on": focus.reviewed_on,
                            "entry_count": focus.entries.len(),
                            "all_release_ready": focus.all_release_ready(),
                            "markdown": focus.to_markdown_table(),
                        },
                        "behavior": {
                            "schema_version": behavior.schema_version,
                            "report_type": behavior.report_type,
                            "reviewed_on": behavior.reviewed_on,
                            "entry_count": behavior.entries.len(),
                            "all_release_ready": behavior.all_release_ready(),
                            "markdown": behavior.to_markdown_table(),
                        },
                    }),
                );
            }
            "themes.gallery" => {
                let gallery = gpui_themes::ThemeGallery::from_built_ins();
                let entries = gallery
                    .entries
                    .into_iter()
                    .map(|entry| {
                        serde_json::json!({
                            "id": entry.id,
                            "display_name": entry.display_name,
                            "tags": entry.tags,
                            "accessibility": entry.accessibility,
                            "appearance": entry.appearance,
                        })
                    })
                    .collect::<Vec<_>>();
                self.send_command_result(
                    request_id,
                    serde_json::json!({"ok": true, "entries": entries}),
                );
            }
            "themes.community_validate" => {
                let result = (|| -> Result<Value, String> {
                    let input = arguments.get("input").and_then(Value::as_str)
                        .ok_or_else(|| "community-theme validation requires input".to_string())?;
                    let bundle = gpui_themes::CommunityThemeBundle::from_json(input)
                        .map_err(|error| error.to_string())?;
                    bundle.validate()?;
                    let gallery = gpui_themes::ThemeGallery::from_built_ins().with_community_bundle(&bundle);
                    let entry = gallery.entries.into_iter().find(|entry| entry.id == bundle.manifest.id)
                        .ok_or_else(|| "validated community theme was not added to gallery".to_string())?;
                    Ok(serde_json::json!({
                        "ok": true, "id": entry.id, "display_name": entry.display_name,
                        "tags": entry.tags, "accessibility": entry.accessibility,
                        "appearance": entry.appearance,
                    }))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "themes.community_activate" => {
                let result = (|| -> Result<Value, String> {
                    let input = arguments.get("input").and_then(Value::as_str)
                        .ok_or_else(|| "community-theme activation requires input".to_string())?;
                    let bundle = gpui_themes::CommunityThemeBundle::from_json(input)
                        .map_err(|error| error.to_string())?;
                    bundle.validate()?;
                    let gallery = gpui_themes::ThemeGallery::from_built_ins().with_community_bundle(&bundle);
                    let entry = gallery.entries.into_iter().find(|entry| entry.id == bundle.manifest.id)
                        .ok_or_else(|| "validated community theme was not added to gallery".to_string())?;
                    self.apply_editor_theme(&bundle.theme, cx);
                    Ok(serde_json::json!({
                        "ok": true, "id": entry.id, "display_name": entry.display_name,
                        "tags": entry.tags, "accessibility": entry.accessibility,
                        "appearance": entry.appearance, "active": true,
                    }))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "profiler.snapshot" => {
                let sample = gpui_profiler::AllocSnapshot::now();
                self.send_command_result(request_id, serde_json::json!({
                    "ok": true, "mode": "counting_allocator", "bytes": sample.bytes, "count": sample.count,
                }));
            }
            "profiler.subscribe" => {
                let result = (|| -> Result<Value, String> {
                    let subscription_id = arguments.get("subscription_id").and_then(Value::as_str)
                        .filter(|id| !id.trim().is_empty())
                        .ok_or_else(|| "profiler subscription requires subscription_id".to_string())?
                        .to_string();
                    let interval_ms = arguments.get("interval_ms").and_then(Value::as_u64).unwrap_or(1_000);
                    if !(50..=60_000).contains(&interval_ms) {
                        return Err("profiler interval_ms must be between 50 and 60000".into());
                    }
                    let sink = self.session.as_ref()
                        .ok_or_else(|| "profiler subscription requires an active Python session".to_string())?
                        .event_sink();
                    if let Some(previous) = self.profiler_subscriptions.remove(&subscription_id) {
                        previous.store(true, Ordering::Release);
                    }
                    let cancelled = Arc::new(AtomicBool::new(false));
                    self.profiler_subscriptions.insert(subscription_id.clone(), cancelled.clone());
                    let stream_id = subscription_id.clone();
                    std::thread::spawn(move || {
                        let mut sequence = 0_u64;
                        while !cancelled.load(Ordering::Acquire) {
                            std::thread::sleep(Duration::from_millis(interval_ms));
                            if cancelled.load(Ordering::Acquire) { break; }
                            sequence = sequence.saturating_add(1);
                            let snapshot = gpui_profiler::AllocSnapshot::now();
                            let message = HostMessage::ProfilerSample {
                                subscription_id: stream_id.clone(), sequence,
                                sample: serde_json::json!({
                                    "mode": "counting_allocator", "bytes": snapshot.bytes, "count": snapshot.count,
                                }),
                            };
                            if sink.send(&message).is_err() { break; }
                        }
                    });
                    Ok(serde_json::json!({
                        "ok": true, "subscription_id": subscription_id,
                        "interval_ms": interval_ms, "mode": "counting_allocator",
                    }))
                })();
                match result {
                    Ok(result) => self.send_command_result(request_id, result),
                    Err(error) => self.send_command_result(request_id, serde_json::json!({"ok": false, "error": error})),
                }
            }
            "profiler.unsubscribe" => {
                let subscription_id = arguments.get("subscription_id").and_then(Value::as_str).unwrap_or("");
                let cancelled = self.profiler_subscriptions.remove(subscription_id).is_some_and(|flag| {
                    flag.store(true, Ordering::Release);
                    true
                });
                self.send_command_result(request_id, serde_json::json!({
                    "ok": true, "subscription_id": subscription_id, "cancelled": cancelled,
                }));
            }
            _ => self.send_command_result(
                request_id,
                serde_json::json!({"ok": false, "unsupported": true, "error": format!("unsupported command: {command}")}),
            ),
        }
    }

    fn handle_effect(
        &mut self,
        request_id: String,
        effect: String,
        arguments: Value,
        cx: &mut Context<Self>,
    ) {
        match effect.as_str() {
            "notification" => {
                let message = arguments
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Notification")
                    .to_string();
                self.notification = Some(message);
                self.send_effect_result(request_id, serde_json::json!({"ok": true}));
            }
            "confirm" => {
                self.pending_confirmation = Some(PendingConfirmation {
                    request_id,
                    title: arguments
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("Confirm")
                        .to_string(),
                    message: arguments
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    confirm_label: arguments
                        .get("confirm_label")
                        .and_then(Value::as_str)
                        .unwrap_or("Confirm")
                        .to_string(),
                    cancel_label: arguments
                        .get("cancel_label")
                        .and_then(Value::as_str)
                        .unwrap_or("Cancel")
                        .to_string(),
                });
            }
            "clipboard_write" => match arguments.get("text").and_then(Value::as_str) {
                Some(text) => {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.into()));
                    self.send_effect_result(request_id, serde_json::json!({"ok": true}));
                }
                None => self.send_effect_result(
                    request_id,
                    serde_json::json!({"ok": false, "error": "clipboard_write requires text"}),
                ),
            },
            "clipboard_read" => {
                let result = cx
                    .read_from_clipboard()
                    .and_then(|item| item.text())
                    .map(|text| serde_json::json!({"ok": true, "text": text}))
                    .unwrap_or_else(|| serde_json::json!({"ok": true, "empty": true}));
                self.send_effect_result(request_id, result);
            }
            "open_with_system" | "reveal_path" => {
                let Some(raw_path) = arguments.get("path").and_then(Value::as_str) else {
                    self.send_effect_result(
                        request_id,
                        serde_json::json!({"ok": false, "error": "path effect requires path"}),
                    );
                    return;
                };
                let path = PathBuf::from(raw_path);
                if raw_path.trim().is_empty() || raw_path.contains('\0') {
                    self.send_effect_result(
                        request_id,
                        serde_json::json!({"ok": false, "error": "path effect path is invalid"}),
                    );
                    return;
                }
                if effect == "open_with_system" {
                    cx.open_with_system(&path);
                } else {
                    cx.reveal_path(&path);
                }
                self.send_effect_result(request_id, serde_json::json!({"ok": true}));
            }
            "credential_store" => match super::credentials::handle(&arguments) {
                Ok(result) => self.send_effect_result(request_id, result),
                Err(error) => self.send_effect_result(
                    request_id,
                    serde_json::json!({"ok": false, "error": error}),
                ),
            },
            "open_url" => match arguments.get("url").and_then(Value::as_str) {
                Some(url) => {
                    cx.open_url(url);
                    self.send_effect_result(request_id, serde_json::json!({"ok": true}));
                }
                None => self.send_effect_result(
                    request_id,
                    serde_json::json!({"ok": false, "error": "open_url requires url"}),
                ),
            },
            "open_file" | "open_directory" => {
                let prompt = arguments
                    .get("prompt")
                    .and_then(Value::as_str)
                    .map(SharedString::from);
                let receiver = cx.prompt_for_paths(PathPromptOptions {
                    files: effect == "open_file",
                    directories: effect == "open_directory",
                    multiple: arguments
                        .get("multiple")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    prompt,
                    initial_directory: arguments
                        .get("initial_directory")
                        .and_then(Value::as_str)
                        .map(PathBuf::from),
                    extensions: arguments
                        .get("filters")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(|extension| SharedString::from(extension.trim_start_matches('.')))
                        .collect(),
                });
                let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) else {
                    return;
                };
                cx.spawn(async move |_, _| {
                    let result = match receiver.await {
                        Ok(Ok(Some(paths))) => serde_json::json!({
                            "ok": true,
                            "paths": paths.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>(),
                        }),
                        Ok(Ok(None)) => serde_json::json!({"ok": true, "cancelled": true}),
                        Ok(Err(error)) => serde_json::json!({"ok": false, "error": error.to_string()}),
                        Err(error) => serde_json::json!({"ok": false, "error": error.to_string()}),
                    };
                    let _ = sink.send(&HostMessage::EffectResult { request_id, result });
                })
                .detach();
            }
            "save_file" => {
                let directory = arguments
                    .get("initial_directory")
                    .and_then(Value::as_str)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."));
                let suggested_name = arguments.get("suggested_name").and_then(Value::as_str);
                let receiver = cx.prompt_for_new_path(Path::new(&directory), suggested_name);
                let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) else {
                    return;
                };
                cx.spawn(async move |_, _| {
                    let result = match receiver.await {
                        Ok(Ok(Some(path))) => {
                            serde_json::json!({"ok": true, "path": path.to_string_lossy()})
                        }
                        Ok(Ok(None)) => serde_json::json!({"ok": true, "cancelled": true}),
                        Ok(Err(error)) => {
                            serde_json::json!({"ok": false, "error": error.to_string()})
                        }
                        Err(error) => serde_json::json!({"ok": false, "error": error.to_string()}),
                    };
                    let _ = sink.send(&HostMessage::EffectResult { request_id, result });
                })
                .detach();
            }
            "close_window" => {
                self.send_effect_result(request_id, serde_json::json!({"ok": true}));
                self.close_approved = true;
                cx.quit();
            }
            _ => self.send_effect_result(
                request_id,
                serde_json::json!({"ok": false, "error": format!("unsupported effect: {effect}")}),
            ),
        }
    }

    fn render_effect_ui(
        &mut self,
        theme: &Theme,
        ds: &DesignSystem,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::new();
        if let Some(notification) = &self.notification {
            elements.push(
                div()
                    .m(px(ds.spacing.control_gap))
                    .p(px(ds.spacing.control_padding_y))
                    .rounded(px(ds.corners.md))
                    .bg(theme.surface_hover)
                    .text_color(theme.text_primary)
                    .child(notification.clone())
                    .into_any_element(),
            );
        }
        if let Some(confirmation) = self.pending_confirmation.clone() {
            let request_id = confirmation.request_id.clone();
            let confirm_id = request_id.clone();
            let cancel_id = request_id.clone();
            let confirm_button = div()
                .id(stable_element_id(format_args!(
                    "python-confirm-{confirm_id}"
                )))
                .px(px(ds.spacing.control_padding_x))
                .py(px(ds.spacing.control_padding_y))
                .rounded(px(ds.corners.md))
                .bg(theme.accent)
                .text_color(theme.text_on_accent)
                .cursor_pointer()
                .child(confirmation.confirm_label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(pending) = this.pending_confirmation.take() {
                        if pending.request_id == "__host_close_while_jobs_running__" {
                            this.close_approved = true;
                            cx.quit();
                        } else {
                            this.send_effect_result(
                                pending.request_id,
                                serde_json::json!({"ok": true, "confirmed": true}),
                            );
                        }
                    }
                    cx.notify();
                }));
            let cancel_button = div()
                .id(stable_element_id(format_args!(
                    "python-cancel-confirm-{cancel_id}"
                )))
                .px(px(ds.spacing.control_padding_x))
                .py(px(ds.spacing.control_padding_y))
                .rounded(px(ds.corners.md))
                .bg(theme.surface_hover)
                .text_color(theme.text_primary)
                .cursor_pointer()
                .child(confirmation.cancel_label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(pending) = this.pending_confirmation.take() {
                        this.send_effect_result(
                            pending.request_id,
                            serde_json::json!({"ok": true, "confirmed": false, "cancelled": true}),
                        );
                    }
                    cx.notify();
                }));
            elements.push(
                div()
                    .id(stable_element_id(format_args!(
                        "python-confirm-overlay-{request_id}"
                    )))
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x00000099))
                    .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {})
                    .child(
                        div()
                            .w(px(420.0))
                            .p(px(ds.spacing.card_padding))
                            .flex()
                            .flex_col()
                            .gap(px(ds.spacing.control_gap))
                            .bg(theme.surface)
                            .rounded(px(ds.corners.md))
                            .border_1()
                            .border_color(theme.border)
                            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {})
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.text_primary)
                                    .child(confirmation.title),
                            )
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .child(confirmation.message),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(ds.spacing.control_gap))
                                    .child(confirm_button)
                                    .child(cancel_button),
                            ),
                    )
                    .into_any_element(),
            );
        }
        elements
    }

    /// Apply a revisioned UI patch after validating both the generic session
    /// ordering and all MeshPlot resource references. Keeping this transaction
    /// separate from the GPUI message-drain loop makes the ownership and
    /// last-valid-frame contract testable without starting a native window.
    fn apply_patch_message(&mut self, patch: Patch) {
        let app_value = self
            .app
            .as_ref()
            .map(|app| serde_json::to_value(app).unwrap_or(Value::Null));
        let mut next_state = self.session_state.clone();
        if let Err(error) = next_state.apply_patch_revision(&patch) {
            self.record_mesh_patch_error(&patch, app_value.as_ref(), error.to_string());
        } else if patch
            .request_id
            .as_ref()
            .is_some_and(|request_id| self.superseded_requests.remove(request_id))
        {
            // Consume the revision without mutating the UI. The handler
            // completed after a newer event superseded it.
            self.session_state = next_state;
        } else if let Some(mut next_app_value) = app_value {
            if let Err(error) =
                PythonAppIr::apply_patch_ops_to_value(&mut next_app_value, &patch.ops)
            {
                self.record_mesh_patch_error(&patch, Some(&next_app_value), error.to_string());
            } else if let Err(error) = validate_mesh_plot_resources(
                &next_app_value,
                &self.mesh_frames,
                patch.request_id.as_deref(),
            ) {
                // Resource-backed patches are committed only when every
                // referenced generation is already retained; the previous
                // valid frame remains visible while a sender recovers from a
                // stale or evicted handle.
                self.record_mesh_patch_error(&patch, Some(&next_app_value), error.to_string());
            } else {
                let mut next_resource_refs = HashMap::new();
                if let Err(error) =
                    collect_mesh_plot_resource_refs(&next_app_value, &mut next_resource_refs)
                {
                    self.record_mesh_patch_error(&patch, Some(&next_app_value), error);
                } else if let Err(error) = self.sync_mesh_plot_resource_refs(next_resource_refs) {
                    self.record_mesh_patch_error(&patch, Some(&next_app_value), error);
                } else {
                    let next_app = match PythonAppIr::from_patched_value(&next_app_value) {
                        Ok(app) => app,
                        Err(error) => {
                            self.record_mesh_patch_error(
                                &patch,
                                Some(&next_app_value),
                                error.to_string(),
                            );
                            return;
                        }
                    };
                    self.app = Some(next_app);
                    self.last_mesh_patch_id = patch.request_id.clone();
                    self.session_state = next_state;
                    self.clear_mesh_patch_errors(&patch, Some(&next_app_value));
                    for operation in &patch.ops {
                        match operation {
                            PatchOp::ClearMeshPlotSelection { plot_id, .. } => {
                                let runtime_id =
                                    mesh_plot_spec_id_for_node(&next_app_value, plot_id)
                                        .unwrap_or_else(|| plot_id.clone());
                                if let Some(state) = self.mesh_plot_states.get(&runtime_id) {
                                    state.borrow_mut().clear_selection();
                                }
                            }
                            PatchOp::ResetMeshPlotViewport { plot_id, .. } => {
                                let runtime_id =
                                    mesh_plot_spec_id_for_node(&next_app_value, plot_id)
                                        .unwrap_or_else(|| plot_id.clone());
                                if let Some(state) = self.mesh_plot_states.get(&runtime_id) {
                                    state.borrow_mut().interaction.reset_zoom();
                                }
                            }
                            #[cfg(feature = "gpu-3d")]
                            PatchOp::ResetMeshPlotCamera { plot_id, .. } => {
                                let runtime_id =
                                    mesh_plot_spec_id_for_node(&next_app_value, plot_id)
                                        .unwrap_or_else(|| plot_id.clone());
                                if let Some(state) = self.mesh_plot_states.get(&runtime_id) {
                                    state.borrow_mut().orbit_reset();
                                }
                            }
                            _ => {}
                        }
                    }
                    let mut live_ids = HashSet::new();
                    mesh_plot_ids(&next_app_value, &mut live_ids);
                    self.prune_mesh_plot_runtime_ids(&live_ids);
                    self.app_value = Some(next_app_value);
                }
            }
        } else {
            self.load_error = Some("patch before snapshot".into());
        }
    }

    fn apply_mesh_frame_message(&mut self, frame: MeshFrame) {
        let resource_id = frame.resource_id.clone();
        let generation = frame.generation;
        match self.mesh_frames.ingest(frame) {
            Ok(MeshFrameOutcome::Assembled(_)) => {
                self.clear_mesh_resource_error(&resource_id, generation);
            }
            Ok(MeshFrameOutcome::Incomplete) | Ok(MeshFrameOutcome::DroppedStale) => {}
            Err(error) => {
                let patch_id = self.last_mesh_patch_id.as_deref().unwrap_or("<stream>");
                self.record_mesh_resource_error(
                    &resource_id,
                    generation,
                    format!(
                        "mesh resource {:?} generation {} (patch {}) failed: {}",
                        resource_id, generation, patch_id, error
                    ),
                );
            }
        }
    }

    fn drain_session(&mut self, cx: &mut Context<Self>) {
        let mut messages = Vec::new();
        if let Some(session) = &self.session {
            while let Some(message) = session.try_recv() {
                messages.push(message);
            }
        }
        for message in messages {
            match message {
                Ok(PythonMessage::Patch(patch)) => self.apply_patch_message(patch),
                Ok(PythonMessage::Snapshot { app_ir }) => self.apply_snapshot_message(app_ir),
                Ok(PythonMessage::Job(update)) => {
                    if let Err(error) = self.jobs.update(update) {
                        self.load_error = Some(error.to_string());
                    }
                }
                Ok(PythonMessage::JobLog(log)) => {
                    if let Err(error) = self.jobs.append_log(&log.id, log.line) {
                        self.load_error = Some(error.to_string());
                    }
                }
                Ok(PythonMessage::ResourceFrame(frame)) => {
                    if let Err(error) = self.audio_frames.ingest(frame) {
                        self.load_error = Some(error.to_string());
                    }
                }
                Ok(PythonMessage::MeshFrame(frame)) => {
                    self.apply_mesh_frame_message(frame);
                }
                Ok(PythonMessage::DropResource {
                    resource_id,
                    generation,
                }) => {
                    self.release_runtime_resource(&resource_id, generation);
                }
                Ok(PythonMessage::Effect {
                    request_id,
                    effect,
                    arguments,
                }) => self.handle_effect(request_id, effect, arguments, cx),
                Ok(PythonMessage::Command {
                    request_id,
                    command,
                    arguments,
                }) => self.handle_command(request_id, command, arguments, cx),
                Ok(PythonMessage::Rejected(error)) => {
                    if !error
                        .request_id
                        .as_ref()
                        .is_some_and(|request_id| self.superseded_requests.remove(request_id))
                    {
                        self.load_error = Some(format!("{}: {}", error.code, error.message))
                    }
                }
                Ok(PythonMessage::Superseded(outcome)) => {
                    self.superseded_requests.insert(outcome.request_id);
                }
                Ok(PythonMessage::Error(error)) => {
                    self.load_error = Some(format!("{}: {}", error.code, error.message))
                }
                Err(error) => {
                    let diagnostics = self
                        .session
                        .as_ref()
                        .map(|session| session.stderr_diagnostics())
                        .filter(|diagnostics| !diagnostics.is_empty());
                    self.load_error = Some(match diagnostics {
                        Some(diagnostics) => {
                            format!("{error}\n\nPython diagnostics:\n{diagnostics}")
                        }
                        None => error,
                    });
                }
                _ => {}
            }
        }
    }

    fn apply_miniapp_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(app) = self.app.as_ref() else {
            return;
        };
        let Some(config) = app.miniapp.clone() else {
            return;
        };
        if self.applied_miniapp_shell.as_ref() == Some(&config) {
            return;
        }
        window.set_window_title(&config.title);
        window.resize(size(px(config.width), px(config.height)));
        self.presentation
            .set_window_size(config.width, config.height);
        if config.with_theme {
            let variant = match config.initial_theme.to_ascii_lowercase().as_str() {
                "light" => ThemeVariant::Light,
                "midnight" => ThemeVariant::Midnight,
                "forest" => ThemeVariant::Forest,
                "black_and_white" => ThemeVariant::BlackAndWhite,
                "onyx" => ThemeVariant::Onyx,
                "carbon_white" => ThemeVariant::CarbonWhite,
                "carbon_gray_10" => ThemeVariant::CarbonGray10,
                "carbon_gray_90" => ThemeVariant::CarbonGray90,
                "carbon_gray_100" => ThemeVariant::CarbonGray100,
                _ => ThemeVariant::Dark,
            };
            cx.set_global(ThemeState::with_variant(variant));
            self.observed_miniapp_theme = Some(variant);
        }
        if config.with_i18n {
            let language = match config.initial_language.to_ascii_lowercase().as_str() {
                "french" => Language::French,
                "german" => Language::German,
                "spanish" => Language::Spanish,
                "japanese" => Language::Japanese,
                _ => Language::English,
            };
            let mut i18n = I18nState::new();
            i18n.set_language(language);
            cx.set_global(i18n);
            self.observed_miniapp_language = Some(language);
        }
        self.applied_miniapp_shell = Some(config);
    }

    fn observe_miniapp_shell_state(&mut self, cx: &mut Context<Self>) {
        let Some(config) = self.app.as_ref().and_then(|app| app.miniapp.as_ref()) else {
            return;
        };
        let sink = self.session.as_ref().map(|session| session.event_sink());
        if config.with_theme {
            if let Some(theme) = cx
                .try_global::<ThemeState>()
                .map(|state| state.theme.variant)
                && self
                    .observed_miniapp_theme
                    .replace(theme)
                    .is_some_and(|previous| previous != theme)
                && let Some(sink) = &sink
            {
                let _ = sink.dispatch(
                    "miniapp",
                    "theme_changed",
                    Some("miniapp_theme_changed".into()),
                    serde_json::json!({"theme": theme.name()}),
                );
            }
        }
        if config.with_i18n {
            if let Some(language) = cx.try_global::<I18nState>().map(|state| state.language)
                && self
                    .observed_miniapp_language
                    .replace(language)
                    .is_some_and(|previous| previous != language)
                && let Some(sink) = &sink
            {
                let _ = sink.dispatch(
                    "miniapp",
                    "language_changed",
                    Some("miniapp_language_changed".into()),
                    serde_json::json!({"language": language.code()}),
                );
            }
        }
    }
}

impl Drop for PythonIrShowcase {
    fn drop(&mut self) {
        self.shutdown_runtime_state();
    }
}

impl PythonIrShowcase {
    fn shutdown_runtime_state(&mut self) {
        for cancellation in self.profiler_subscriptions.values() {
            cancellation.store(true, Ordering::Release);
        }
        self.profiler_subscriptions.clear();
        self.audio_frames.clear();
        // Shutdown must release both decoded resources and the retained
        // MeshPlot cache/state that can otherwise keep GPU-facing owners
        // alive until the entity is finally dropped. Reuse the session-reset
        // path so explicit cleanup and Drop remain idempotent and consistent.
        self.reset_mesh_plot_runtime_state();
    }
}

fn mesh_plot_operation_id(operation: &PatchOp) -> Option<&str> {
    match operation {
        PatchOp::ReplaceMeshGeometry { plot_id, .. }
        | PatchOp::ReplaceMeshField { plot_id, .. }
        | PatchOp::SetMeshPlotProp { plot_id, .. }
        | PatchOp::SetMeshPlotSelection { plot_id, .. }
        | PatchOp::ClearMeshPlotSelection { plot_id, .. }
        | PatchOp::SetMeshPlotCamera { plot_id, .. }
        | PatchOp::ResetMeshPlotCamera { plot_id, .. }
        | PatchOp::SetMeshPlotViewport { plot_id, .. }
        | PatchOp::ResetMeshPlotViewport { plot_id, .. } => Some(plot_id),
        _ => None,
    }
}

impl Render for PythonIrShowcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.observe_presentation(window, cx);
        self.observe_window_close(window, cx);
        self.drain_session(cx);
        self.apply_miniapp_shell(window, cx);
        self.observe_miniapp_shell_state(cx);
        let theme = cx.theme();
        let ds = cx.design();

        if let Some(error) = self.load_error.clone() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.background)
                .p(px(ds.spacing.card_padding))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(ds.spacing.control_gap))
                        .child(self.render_error(&error, &theme, &ds))
                        .child(
                            apply_native_accessibility(
                                div().id("python-session-restart"),
                                "Restart Python application",
                                &AriaProps::with_role(AriaRole::Button),
                            )
                            .focusable()
                            .px(px(ds.spacing.control_padding_x))
                            .py(px(ds.spacing.control_padding_y))
                            .rounded(px(ds.corners.md))
                            .bg(theme.accent)
                            .text_color(theme.text_on_accent)
                            .cursor_pointer()
                            .child("Restart Python application")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load_session(cx);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(
                                |this, event: &KeyDownEvent, _, cx| {
                                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                        this.load_session(cx);
                                        cx.stop_propagation();
                                        cx.notify();
                                    }
                                },
                            )),
                        ),
                );
        }

        if self.app.is_none() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.background)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(ds.spacing.control_gap))
                        .child(
                            div()
                                .w(px(24.0))
                                .h(px(24.0))
                                .rounded(px(12.0))
                                .bg(theme.accent),
                        )
                        .child(
                            div()
                                .text_size(px(ds.typography.small_size))
                                .text_color(theme.text_secondary)
                                .child("Loading Python app..."),
                        ),
                );
        }

        self.schedule_qa_pointer_event(window, cx);

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(self.render_sidebar(&theme, &ds, cx))
                    .child(self.render_content(&theme, &ds, cx)),
            )
            .children(self.render_effect_ui(&theme, &ds, cx))
    }
}
