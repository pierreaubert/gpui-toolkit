use super::host_state::PresentationStore;
use super::misc::apply_size;
use super::misc::badge_colors;
use super::misc::color_scale;
use super::misc::hex_color;
use super::misc::scale_type;
use super::misc::tone_color;
use super::types::StackDirection;
use d3rs::gpu3d::{Lines3DElement, Lines3DState, Surface3DElement, Surface3DState};
use gpui::prelude::*;
use gpui::*;
use gpui_design::{DesignExt, DesignSystem};
use gpui_px::interaction::{InteractiveChartState, interactive};
use gpui_px::{bar, heatmap, line, scatter};
use gpui_python_runtime::gpui_adapter::Gpui3DCache;
use gpui_python_runtime::session::{
    HostMessage, JobLogLine, JobRegistry, JobState, JobUpdate, LogSeverity, PythonMessage,
    SessionState,
};
use gpui_python_runtime::spec_cache::TypedSpecCache;
use gpui_python_runtime::ui_ir::{
    AccordionNode, AlertNode, BadgeNode, BooleanInputNode, BreadcrumbsNode, ButtonNode, CardNode,
    ChartKind, ChartNode, ColorPickerNode, ConfirmDialogNode, ContextMenuNode, DialogNode, EmptyStateNode, FormNode, ListEditorNode, MenuBarNode, MenuItemNode, MenuNode, NumberInputNode,
    PathInputNode, ProgressNode, PythonAppIr, Scene3dNode, SectionHeaderNode, SelectNode,
    SimpleNode, SliderNode, SpinnerNode, StackNode, StepperNode, TableNode, TabsNode, TextInputNode,
    PopoverNode, TextNode, ToastNode, TooltipNode, UiNode, MiniAppShellConfig,
};
use gpui_ui_kit::color::Color;
use gpui_ui_kit::data_navigation::{DataNavigationAction, DataNavigationState};
use gpui_ui_kit::theme::{Theme, ThemeExt, ThemeState, ThemeVariant};
use gpui_ui_kit::{
    Alert, AlertVariant, BreadcrumbItem, BreadcrumbSeparator, Breadcrumbs, ColorPickerView, Toast,
    ToastVariant, TooltipPlacement, WithTooltip,
    ConfirmDialog, ConfirmDialogVariant, ContextMenu, Dialog, DialogSize, EmptyState, Menu, MenuBar, MenuBarItem, MenuItem, Popover, PopoverPlacement,
    DragItem, DragList,
    accordion::{Accordion, AccordionItem, AccordionMode},
    checkbox::Checkbox,
    input::Input,
    number_input::NumberInput,
    select::Select,
    slider::Slider,
    toggle::Toggle, I18nState, Language,
};
use gpui_ui_kit::{AriaProps, AriaRole, AriaState, apply_native_accessibility};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;

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

fn command_domain(arguments: &Value, name: &str) -> Result<(f64, f64), String> {
    let values = arguments
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{name} must be a two-value array"))?;
    let [min, max] = values.as_slice() else {
        return Err(format!("{name} must be a two-value array"));
    };
    let min = min.as_f64().ok_or_else(|| format!("{name} minimum must be finite"))?;
    let max = max.as_f64().ok_or_else(|| format!("{name} maximum must be finite"))?;
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
            let value = value.as_f64().ok_or_else(|| format!("{name} values must be finite"))?;
            if value.is_finite() { Ok(value) } else { Err(format!("{name} values must be finite")) }
        })
        .collect()
}

struct FixedTextMeasure(f64);

impl gpui_pretext::TextMeasure for FixedTextMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * self.0
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
        .id(ElementId::Name(
            format!("python-scene-controls-{id}").into(),
        ))
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
                .id(ElementId::Name(format!("python-scene-reset-{id}").into()))
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
        ChartKind::Scatter | ChartKind::Line => {
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
        ChartKind::Bar => {
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
        ChartKind::Heatmap => {
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
        ChartKind::Line | ChartKind::Scatter => {
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
        ChartKind::Bar => {
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
        ChartKind::Heatmap => {
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
        ChartKind::Line | ChartKind::Scatter => {
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
        ChartKind::Bar => {
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
        ChartKind::Heatmap => {
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
    use super::png_encode;

    #[test]
    fn png_encoder_writes_a_signature_and_terminal_chunk() {
        let png = png_encode(1, 1, &[12, 34, 56]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.windows(4).any(|window| window == b"IHDR"));
        assert!(png.windows(4).any(|window| window == b"IDAT"));
        assert!(png.windows(4).any(|window| window == b"IEND"));
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

pub(super) struct PythonIrShowcase {
    pub(super) app: Option<PythonAppIr>,
    pub(super) load_error: Option<String>,
    pub(super) current_section: String,
    pub(super) gpui_3d: Gpui3DCache,
    pub(super) spec_cache: TypedSpecCache,
    pub(super) table_cells: HashMap<(usize, usize), (String, SharedString)>,
    form_focus: HashMap<String, FocusHandle>,
    color_pickers: HashMap<String, Entity<ColorPickerView>>,
    color_picker_subscriptions: HashMap<String, Subscription>,
    color_picker_actions: HashMap<String, Option<String>>,
    tab_focus: HashMap<String, FocusHandle>,
    /// Retained per-chart interaction state. Re-renders rebuild the draw list
    /// from this state, so data patches do not discard a user's zoom or pan.
    chart_interactions: HashMap<String, InteractiveChartState>,
    /// Host-local legend choices. They are keyed by Python's stable series ID
    /// and intentionally survive data patches without changing Python state.
    chart_hidden_series: HashMap<String, HashSet<String>>,
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
}

#[derive(Clone)]
struct PendingConfirmation {
    request_id: String,
    title: String,
    message: String,
    confirm_label: String,
    cancel_label: String,
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
            load_error: None,
            current_section: presentation_state.section.unwrap_or_default(),
            gpui_3d: Gpui3DCache::new(),
            spec_cache: TypedSpecCache::new(),
            table_cells: HashMap::new(),
            form_focus: HashMap::new(),
            color_pickers: HashMap::new(),
            color_picker_subscriptions: HashMap::new(),
            color_picker_actions: HashMap::new(),
            tab_focus: HashMap::new(),
            chart_interactions: HashMap::new(),
            chart_hidden_series: HashMap::new(),
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
                    .iter().map(|capability| (*capability).into()).collect(),
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
        if !app.sections.iter().any(|section| section.id == self.current_section)
            && let Some(section) = app.sections.first()
        {
            self.current_section = section.id.clone();
            self.presentation.set_section(Some(section.id.clone()));
        }
        self.app = Some(app);
        self.session = Some(session);
        self.start_session_updates(cx);
    }

    fn load_session(&mut self, cx: &mut Context<Self>) {
        self.load_error = None;
        self.app = None;
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
        let (content, scrollable) = {
            let app = self.app.as_ref().expect("render_content called after load");
            (
                app.sections
                    .iter()
                    .find(|section| section.id == self.current_section)
                    .or_else(|| app.sections.first())
                    .map(|section| section.content.clone()),
                app.miniapp.as_ref().map_or(true, |config| config.scrollable),
            )
        };

        let content = content
            .as_ref()
            .map(|node| self.render_node(node, theme, ds, cx))
            .unwrap_or_else(|| {
                self.render_error("Python app did not define any sections", theme, ds)
            });

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
            UiNode::TextInput(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::TextInput(node) => self.render_text_input(node, theme, ds, cx),
            UiNode::NumberInput(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::NumberInput(node) => self.render_number_input(node, theme, ds, cx),
            UiNode::Slider(node) if !node.presentation.visible => div().into_any_element(),
            UiNode::Slider(node) => self.render_slider(node, theme, ds),
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
                        .id(ElementId::Name(
                            format!("python-form-focus-first-{}", node.id).into(),
                        ))
                        .cursor_pointer()
                        .child("Focus first invalid control")
                        .on_click(move |_, window, cx| handle.focus(window, cx)),
                );
            }
            for (index, error) in node.errors.iter().enumerate() {
                let entry = div()
                    .id(ElementId::Name(
                        format!("python-form-error-{}-{index}", node.id).into(),
                    ))
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
        let mut input = Input::new(ElementId::Name(format!("python-form-{id}").into()))
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
            let mut input = Input::new(ElementId::Name(format!("python-form-{id}").into()))
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
        let mut input = NumberInput::new(ElementId::Name(format!("python-form-{id}").into()))
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
        let mut slider = Slider::new(ElementId::Name(format!("python-slider-{id}").into()))
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
        let mut select = Select::new(ElementId::Name(format!("python-form-{id}").into()))
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
        let mut input = Input::new(ElementId::Name(format!("python-path-{id}").into()))
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
                    .id(ElementId::Name(
                        format!("python-path-browse-{node_id}").into(),
                    ))
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
                            .id(ElementId::Name(
                                format!("python-path-recent-{node_id}-{index}").into(),
                            ))
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
            Checkbox::new(ElementId::Name(format!("python-form-{}", node.id).into()))
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
        let mut toggle = Toggle::new(ElementId::Name(format!("python-form-{}", node.id).into()))
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
                .id(ElementId::Name(
                    format!("python-job-log-filter-{label}").into(),
                ))
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
                                        .id(ElementId::Name(
                                            format!("python-job-log-pause-{job_id}").into(),
                                        ))
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
                                        .id(ElementId::Name(
                                            format!("python-job-log-copy-{}", job.id).into(),
                                        ))
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
                                        .id(ElementId::Name(
                                            format!("python-job-log-clear-{job_id}").into(),
                                        ))
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
                                        .id(ElementId::Name(
                                            format!("python-job-log-export-{}", job.id).into(),
                                        ))
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
                                ElementId::Name(format!("python-job-log-lines-{job_id}").into()),
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
                                .id(ElementId::Name(format!("python-cancel-{}", job_id).into()))
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
            ElementId::Name(format!("python-alert-{}", node.id).into()),
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
            ElementId::Name(format!("python-toast-{}", node.id).into()),
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
        .id(ElementId::Name(format!("python-tooltip-{}", node.id).into()))
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
            node.content.iter().map(|child| self.render_node(child, theme, ds, cx)),
        );
        let footer = div().flex().items_center().gap(px(ds.spacing.control_gap)).children(
            node.footer.iter().map(|child| self.render_node(child, theme, ds, cx)),
        );
        let mut dialog = Dialog::new(ElementId::Name(format!("python-dialog-{}", node.id).into()))
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

    fn render_confirm_dialog(&self, node: &ConfirmDialogNode, cx: &mut Context<Self>) -> AnyElement {
        let variant = match node.variant.as_str() {
            "destructive" => ConfirmDialogVariant::Destructive,
            "warning" => ConfirmDialogVariant::Warning,
            _ => ConfirmDialogVariant::Default,
        };
        let mut dialog = ConfirmDialog::new(ElementId::Name(format!("python-confirm-dialog-{}", node.id).into()))
            .message(node.message.clone())
            .variant(variant)
            .confirm_label(node.confirm_label.clone())
            .cancel_label(node.cancel_label.clone())
            .focus_handle(cx.focus_handle());
        if let Some(title) = &node.title { dialog = dialog.title(title.clone()); }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()), node.confirm_action.clone(),
        ) {
            let node_id = node.id.clone();
            dialog = dialog.on_confirm(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "confirm", Some(action.clone()), Value::Null);
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()), node.cancel_action.clone(),
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
            ElementId::Name(format!("python-context-menu-{}", node.id).into()),
            Self::menu_items(&node.items),
        )
        .position(point(px(node.position[0]), px(node.position[1])))
        .min_width(px(node.min_width));
        menu = menu.focus_handle(cx.focus_handle());
        if let Some(index) = node.focused_index { menu = menu.focused_index(index); }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()),
            node.action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_select(move |item_id, _, _| {
                let _ = sink.dispatch(node_id.clone(), "select", Some(action.clone()), serde_json::json!({"item_id": item_id.as_ref()}));
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
                let _ = sink.dispatch(node_id.clone(), "focus", Some(action.clone()), serde_json::json!({"index": index}));
            });
        }
        menu.into_any_element()
    }

    fn render_menu(&self, node: &MenuNode, cx: &mut Context<Self>) -> AnyElement {
        let mut menu = Menu::new(
            ElementId::Name(format!("python-menu-{}", node.id).into()),
            Self::menu_items(&node.items),
        )
        .min_width(px(node.min_width))
        .focus_handle(cx.focus_handle());
        if let Some(index) = node.focused_index { menu = menu.focused_index(index); }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()), node.action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_select(move |item_id, _, _| {
                let _ = sink.dispatch(node_id.clone(), "select", Some(action.clone()), serde_json::json!({"item_id": item_id.as_ref()}));
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()), node.close_action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_close(move |_, _| {
                let _ = sink.dispatch(node_id.clone(), "close", Some(action.clone()), Value::Null);
            });
        }
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()), node.focus_action.clone(),
        ) {
            let node_id = node.id.clone();
            menu = menu.on_focus_change(move |index, _, _| {
                let _ = sink.dispatch(node_id.clone(), "focus", Some(action.clone()), serde_json::json!({"index": index}));
            });
        }
        menu.into_any_element()
    }

    fn render_menu_bar(&self, node: &MenuBarNode, cx: &mut Context<Self>) -> AnyElement {
        let bar_items = node.items.iter().map(|item| {
            MenuBarItem::new(item.id.clone(), item.label.clone()).with_items(Self::menu_items(&item.items))
        }).collect();
        let mut bar = MenuBar::new(bar_items).active_menu(node.active_menu.clone().map(Into::into));
        if let (Some(sink), Some(action)) = (
            self.session.as_ref().map(|session| session.event_sink()), node.toggle_action.clone(),
        ) {
            let node_id = node.id.clone();
            bar = bar.on_menu_toggle(move |menu_id, _, _| {
                let _ = sink.dispatch(node_id.clone(), "toggle", Some(action.clone()), serde_json::json!({"menu_id": menu_id.map(|id| id.as_ref())}));
            });
        }
        let mut rendered = div().relative().child(bar);
        if let Some(active_id) = &node.active_menu
            && let Some(active) = node.items.iter().find(|item| &item.id == active_id)
        {
            let mut menu = Menu::new(
                ElementId::Name(format!("python-menu-bar-{}-{active_id}", node.id).into()),
                Self::menu_items(&active.items),
            )
            .focus_handle(cx.focus_handle());
            if let (Some(sink), Some(action)) = (
                self.session.as_ref().map(|session| session.event_sink()), node.action.clone(),
            ) {
                let node_id = node.id.clone();
                menu = menu.on_select(move |item_id, _, _| {
                    let _ = sink.dispatch(node_id.clone(), "select", Some(action.clone()), serde_json::json!({"item_id": item_id.as_ref()}));
                });
            }
            if let (Some(sink), Some(action)) = (
                self.session.as_ref().map(|session| session.event_sink()), node.toggle_action.clone(),
            ) {
                let node_id = node.id.clone();
                menu = menu.on_close(move |_, _| {
                    let _ = sink.dispatch(node_id.clone(), "toggle", Some(action.clone()), serde_json::json!({"menu_id": Value::Null}));
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
            node.content.iter().map(|child| self.render_node(child, theme, ds, cx)),
        );
        let mut popover = Popover::new(ElementId::Name(format!("python-popover-{}", node.id).into()))
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
            div().id(ElementId::Name(format!("python-tablist-{tab_id}").into())),
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
                    .id(ElementId::Name(
                        format!(
                            "python-tab-{}-{index}",
                            node.id.as_deref().unwrap_or("unbound")
                        )
                        .into(),
                    ))
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
                    .id(ElementId::Name(
                        format!("python-stepper-{}-{index}", node.id).into(),
                    ))
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
                    .id(ElementId::Name(
                        format!("python-list-remove-{}-{}", node.id, row.id).into(),
                    ))
                    .text_color(theme.text_muted)
                    .child("Remove")
            } else if let Some(sink) = self.session.as_ref().map(|session| session.event_sink()) {
                let list_id = node.id.clone();
                let action = node.remove_action.clone();
                div()
                    .id(ElementId::Name(
                        format!("python-list-remove-{}-{}", node.id, row.id).into(),
                    ))
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
                    .id(ElementId::Name(
                        format!("python-list-remove-{}-{}", node.id, row.id).into(),
                    ))
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
            ElementId::Name(format!("python-list-editor-{}", node.id).into()),
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
                        .id(ElementId::Name(
                            format!("python-list-add-{}", node.id).into(),
                        ))
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
            div().id(ElementId::Name(format!("python-table-{dom_id}").into())),
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
                    ElementId::Name(format!("python-table-virtual-{table_id}").into()),
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
                                    .id(ElementId::Name(
                                        format!("python-table-row-{list_table_id}-{row_id}").into(),
                                    ))
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
                    .id(ElementId::Name(format!("python-table-header-{}", column.id).into()))
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
                        .id(ElementId::Name(format!("python-table-resize-{table_id}-{column_id}").into()))
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
                Some(
                    self.chart_interactions
                        .entry(node.id.clone())
                        .or_insert_with(|| {
                            InteractiveChartState::new(x_min, x_max, y_min, y_max)
                                .with_log_x(node.x_log)
                                .with_log_y(node.y_log)
                                .with_size(node.width, node.height)
                        })
                        .clone(),
                )
            }
            ChartKind::Bar | ChartKind::Heatmap => None,
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
                    .size(node.width, node.height);
                for series in visible_series.iter().copied().skip(1) {
                    chart = chart.add_series(
                        &series.x,
                        &series.y,
                        (!series.label.is_empty()).then_some(series.label.clone()),
                        hex_color(series.color.as_deref(), 0x1f77b4),
                        series.point_radius.unwrap_or(node.point_radius),
                        1.0,
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
                    chart = chart.add_series_with_x(
                        &series.x,
                        &series.y,
                        (!series.label.is_empty()).then_some(series.label.clone()),
                        hex_color(series.color.as_deref(), 0xff7f0e),
                        series.stroke_width.unwrap_or(node.stroke_width),
                        1.0,
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
            ChartKind::Bar => {
                let categories = node.categories.as_deref().unwrap_or_default();
                let values = node.values.as_deref().unwrap_or_default();
                bar(categories, values)
                    .title(node.title.clone())
                    .color(hex_color(node.color.as_deref(), 0x2ca02c))
                    .size(node.width, node.height)
                    .build()
                    .map(IntoElement::into_any_element)
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
        };

        let chart = result.unwrap_or_else(|error| {
            self.render_error(&format!("chart {}: {error}", node.id), theme, ds)
        });
        let inspection = interaction.as_ref().and_then(|state| {
            chart_inspection(node, state, self.chart_hidden_series.get(&node.id))
        });
        let chart = match interaction {
            Some(state) => interactive(
                ElementId::Name(format!("python-chart-{}", node.id).into()),
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
                        .id(ElementId::Name(
                            format!("python-chart-legend-{chart_id}-{series_id}").into(),
                        ))
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
                    .id(ElementId::Name(
                        format!("python-chart-export-{}", node.id).into(),
                    ))
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
                    .id(ElementId::Name(
                        format!("python-chart-export-svg-{}", node.id).into(),
                    ))
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
                    .id(ElementId::Name(
                        format!("python-chart-export-png-{}", node.id).into(),
                    ))
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

        let mut container = div()
            .id(ElementId::Name(format!("python-scene-{}", node.id).into()))
            .w(px(width))
            .h(px(height))
            .rounded(px(ds.corners.md))
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            .child(element);
        if let (Some(action), Some(sink)) = (
            node.selection_action.clone(),
            self.session.as_ref().map(|session| session.event_sink()),
        ) {
            let node_id = node.id.clone();
            let object_id = node
                .spec
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or(&node.id)
                .to_string();
            container = container.cursor_pointer().on_click(move |_, _, _| {
                let _ = sink.dispatch(
                    node_id.clone(),
                    "select",
                    Some(action.clone()),
                    serde_json::json!({"object_id": object_id}),
                );
            });
        }
        container.into_any_element()
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
        cx.set_global(ThemeState { theme: Arc::new(theme) });
        cx.refresh_windows();
    }

    fn handle_command(&mut self, request_id: String, command: String, arguments: Value, cx: &mut Context<Self>) {
        match command.as_str() {
            "runtime.capabilities" => self.send_command_result(
                request_id,
                serde_json::json!({
                    "ok": true,
                    "session_version": gpui_python_runtime::session::PYTHON_APP_SESSION_VERSION,
                    "capabilities": gpui_python_runtime::session::DEFAULT_HOST_CAPABILITIES,
                }),
            ),
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
                                id, eyebrow: String::new(), title: String::new(), caption: None, rows: Vec::new(),
                                min_width, preferred_width, priority: priority as f32,
                            })
                        }).collect::<Result<Vec<_>, String>>()?;
                    let chassis = gpui_builder::plugin_chassis::ChassisLayout::new(
                        gpui_builder::plugin_chassis::HeaderSpec { brand_mark: String::new(), title: String::new(), subtitle: String::new() },
                        sections,
                    );
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
                .id(ElementId::Name(
                    format!("python-confirm-{confirm_id}").into(),
                ))
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
                .id(ElementId::Name(
                    format!("python-cancel-confirm-{cancel_id}").into(),
                ))
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
                    .id(ElementId::Name(
                        format!("python-confirm-overlay-{request_id}").into(),
                    ))
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

    fn drain_session(&mut self, cx: &mut Context<Self>) {
        let mut messages = Vec::new();
        if let Some(session) = &self.session {
            while let Some(message) = session.try_recv() {
                messages.push(message);
            }
        }
        for message in messages {
            match message {
                Ok(PythonMessage::Patch(patch)) => {
                    let mut next_state = self.session_state.clone();
                    if let Err(error) = next_state.apply_patch_revision(&patch) {
                        self.load_error = Some(error.to_string());
                    } else if patch
                        .request_id
                        .as_ref()
                        .is_some_and(|request_id| self.superseded_requests.contains(request_id))
                    {
                        // Consume the revision without mutating the UI. The
                        // handler completed after a newer event superseded it.
                        self.session_state = next_state;
                    } else if let Some(app) = self.app.as_mut() {
                        if let Err(error) = app.apply_patch_ops(&patch.ops) {
                            self.load_error = Some(error.to_string());
                        } else {
                            self.session_state = next_state;
                        }
                    } else {
                        self.load_error = Some("patch before snapshot".into());
                    }
                }
                Ok(PythonMessage::Snapshot { app_ir }) => {
                    if let Err(error) = app_ir.validate() {
                        self.load_error = Some(error.to_string());
                    } else {
                        self.app = Some(app_ir);
                    }
                }
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
                        .is_some_and(|request_id| self.superseded_requests.contains(request_id))
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
        let Some(app) = self.app.as_ref() else { return; };
        let Some(config) = app.miniapp.clone() else { return; };
        if self.applied_miniapp_shell.as_ref() == Some(&config) { return; }
        window.set_window_title(&config.title);
        window.resize(size(px(config.width), px(config.height)));
        self.presentation.set_window_size(config.width, config.height);
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
            if let Some(theme) = cx.try_global::<ThemeState>().map(|state| state.theme.variant)
                && self.observed_miniapp_theme.replace(theme).is_some_and(|previous| previous != theme)
                && let Some(sink) = &sink
            {
                let _ = sink.dispatch(
                    "miniapp", "theme_changed", Some("miniapp_theme_changed".into()),
                    serde_json::json!({"theme": theme.name()}),
                );
            }
        }
        if config.with_i18n {
            if let Some(language) = cx.try_global::<I18nState>().map(|state| state.language)
                && self.observed_miniapp_language.replace(language).is_some_and(|previous| previous != language)
                && let Some(sink) = &sink
            {
                let _ = sink.dispatch(
                    "miniapp", "language_changed", Some("miniapp_language_changed".into()),
                    serde_json::json!({"language": language.code()}),
                );
            }
        }
    }
}

impl Drop for PythonIrShowcase {
    fn drop(&mut self) {
        for cancellation in self.profiler_subscriptions.values() {
            cancellation.store(true, Ordering::Release);
        }
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
