//! Treemap - Plotly Express style API for hierarchical data visualization.
//!
//! Treemaps display hierarchical data as nested rectangles. Each rectangle's area
//! is proportional to the value it represents. Multiple tiling algorithms are supported
//! for different visual layouts.
//!
//! # Example
//! ```ignore
//! use gpui_px::{treemap, TilingMethod};
//!
//! let root = TreemapNode::new("root", 100.0)
//!     .add_child(TreemapNode::new("A", 30.0))
//!     .add_child(TreemapNode::new("B", 70.0));
//!
//! let chart = treemap(&root)
//!     .title("Sales by Region")
//!     .tiling_method(TilingMethod::Squarify)
//!     .padding(2.0)
//!     .build()
//!     .unwrap();
//! ```

use crate::error::ChartError;
use crate::{
    ChartAccessibilitySummary, ChartSize, DEFAULT_TITLE_FONT_SIZE, TITLE_AREA_HEIGHT,
    apply_chart_size, default_design, finite_range_owned, format_range, resolved_chart_dimensions,
    validate_dimensions,
};
use d3rs::color::{ColorScheme, D3Color};
use d3rs::render2d::{Renderer2D, VelloBackend};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{
    Bounds, IntoElement, MouseButton, PathBuilder, Pixels, Rgba, canvas, div, hsla, point, px, rgb,
};
use gpui_design::DesignSystem;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(test)]
mod tests;
mod tile;
mod tiling_method;
mod treemap_node;
mod treemap_rect;
mod types;

pub use tiling_method::*;
pub use treemap_node::*;
pub use types::*;

use tiling_method::compute_treemap;

/// Treemap chart builder.
#[allow(clippy::type_complexity)]
pub struct Treemap {
    root: TreemapNode,
    title: Option<String>,
    tiling_method: TilingMethod,
    padding: f64,
    width: f32,
    height: f32,
    chart_size: ChartSize,
    color_scheme: Option<ColorScheme>,
    on_click: Option<Rc<dyn Fn(&str, f64) + 'static>>,
    hover_enabled: bool,
    design: Option<Arc<DesignSystem>>,
    renderer_2d: Renderer2D,
    vello_backend: VelloBackend,
}

#[derive(Clone, Debug)]
struct RectDrawData {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    name: String,
    value: f64,
    fill: Rgba,
    border: Rgba,
}

impl Treemap {
    /// Select the high-level 2D renderer. Vello is the default when enabled.
    pub fn renderer_2d(mut self, renderer: Renderer2D) -> Self {
        self.renderer_2d = renderer;
        self
    }

    /// Select the Vello WGPU/CPU backend.
    pub fn vello_backend(mut self, backend: VelloBackend) -> Self {
        self.vello_backend = backend;
        self
    }

    /// Export this treemap as deterministic SVG.
    pub fn to_svg(&self) -> Result<String, ChartError> {
        self.to_svg_with_options(crate::StaticSvgOptions::new(self.width, self.height))
    }

    /// Export this treemap as deterministic SVG with explicit export options.
    pub fn to_svg_with_options(
        &self,
        options: crate::StaticSvgOptions,
    ) -> Result<String, ChartError> {
        validate_dimensions(options.width, options.height)?;
        crate::static_export::validate_plot_area(options)?;

        let total_value = self.root.total_value();
        if !total_value.is_finite() || total_value <= 0.0 {
            return Err(ChartError::InvalidData {
                field: "root",
                reason: "Total value must be positive and finite",
            });
        }

        if !Self::validate_values(&self.root) {
            return Err(ChartError::InvalidData {
                field: "node",
                reason: "All node values must be finite and non-negative",
            });
        }

        let plot_width = (options.width - options.margin_left - options.margin_right) as f64;
        let plot_height = (options.height - options.margin_top - options.margin_bottom) as f64;
        if plot_width <= 0.0 || plot_height <= 0.0 {
            return Err(ChartError::InvalidDimension {
                field: "width",
                value: options.width,
            });
        }

        let mut rects = Vec::new();
        compute_treemap(
            &self.root,
            0.0,
            0.0,
            plot_width,
            plot_height,
            self.tiling_method,
            self.padding,
            0,
            0,
            &mut rects,
        );

        let color_scheme = self
            .color_scheme
            .clone()
            .unwrap_or_else(ColorScheme::tableau10);
        let mut svg = crate::static_export::svg_header(options);
        crate::static_export::draw_title(&mut svg, self.title.as_deref(), options.width);
        svg.push_str("<g class=\"gpui-px-treemap\">\n");

        for rect in rects {
            let x = options.margin_left + rect.x0 as f32;
            let y = options.margin_top + rect.y0 as f32;
            let width = (rect.x1 - rect.x0).max(0.0) as f32;
            let height = (rect.y1 - rect.y0).max(0.0) as f32;
            if width <= 0.0 || height <= 0.0 {
                continue;
            }

            let color = color_scheme.color(rect.category_index);
            let fill = d3_color_to_hex(color);
            let stroke = d3_color_to_hex(darken_d3_color(color, 0.7));
            let escaped_name = crate::static_export::escape_xml(&rect.name);
            let _ = writeln!(
                svg,
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" fill=\"{fill}\" opacity=\"0.800\" stroke=\"{stroke}\" stroke-width=\"1\"><title>{escaped_name}: {:.3}</title></rect>",
                rect.value
            );

            if width > 30.0 && height > 15.0 {
                let luminance = 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
                let text_color = if luminance > 0.5 {
                    "#1a1a1a"
                } else {
                    "#f2f2f2"
                };
                let font_size = (height * 0.2).clamp(8.0, 12.0);
                let _ = writeln!(
                    svg,
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"system-ui, sans-serif\" font-size=\"{font_size:.2}\" fill=\"{text_color}\">{escaped_name}</text>",
                    x + width / 2.0,
                    y + height / 2.0,
                );
            }
        }

        svg.push_str("</g>\n</svg>\n");
        Ok(svg)
    }

    /// Return structured accessibility metadata for this chart.
    pub fn accessibility_summary(&self) -> ChartAccessibilitySummary {
        fn visit(
            node: &TreemapNode,
            node_count: &mut usize,
            leaf_count: &mut usize,
            values: &mut Vec<f64>,
            labels: &mut Vec<String>,
        ) {
            *node_count += 1;
            if node.is_leaf() {
                *leaf_count += 1;
                values.push(node.value);
                labels.push(node.name.clone());
            }

            for child in &node.children {
                visit(child, node_count, leaf_count, values, labels);
            }
        }

        let mut node_count = 0;
        let mut leaf_count = 0;
        let mut values = Vec::new();
        let mut series_labels = Vec::new();
        visit(
            &self.root,
            &mut node_count,
            &mut leaf_count,
            &mut values,
            &mut series_labels,
        );

        let value_range = finite_range_owned(values);
        let title = self.title.clone();
        let name = title.as_deref().unwrap_or("Treemap");
        let description = format!(
            "{name}: treemap with {node_count} nodes and {leaf_count} leaves using {:?} tiling. Total value {:.3}. {}.",
            self.tiling_method,
            self.root.total_value(),
            format_range("Leaf value", value_range)
        );

        ChartAccessibilitySummary {
            chart_type: "treemap",
            title,
            series_count: 1,
            datum_count: node_count,
            x_range: None,
            y_range: None,
            value_range,
            x_scale: None,
            y_scale: None,
            series_labels,
            description,
        }
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the tiling algorithm.
    ///
    /// Default: `TilingMethod::Squarify`
    pub fn tiling_method(mut self, method: TilingMethod) -> Self {
        self.tiling_method = method;
        self
    }

    /// Set the padding between rectangles in pixels.
    ///
    /// Default: 1.0
    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    /// Set the chart size in pixels.
    ///
    /// Default: 600 x 400
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self.chart_size = ChartSize::fixed(width, height);
        self
    }

    /// Fill the parent using the current minimum chart dimensions.
    pub fn fill(mut self) -> Self {
        self.chart_size = ChartSize::fill().min_size(self.width, self.height);
        self
    }

    /// Set minimum dimensions for responsive fill sizing.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self.chart_size = self.chart_size.min_size(width, height);
        self
    }

    /// Set preferred fill-layout aspect ratio.
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.chart_size = self.chart_size.aspect_ratio(ratio);
        self
    }

    /// Override the design system used for chart defaults.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set a custom color scheme.
    ///
    /// Default: ColorScheme::tableau10()
    pub fn color_scheme(mut self, scheme: ColorScheme) -> Self {
        self.color_scheme = Some(scheme);
        self
    }

    /// Set a click handler for treemap nodes.
    ///
    /// The handler receives the node name and value when clicked.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&str, f64) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Recursively validate that all node values are finite and non-negative.
    fn validate_values(node: &TreemapNode) -> bool {
        if !node.value.is_finite() || node.value < 0.0 {
            return false;
        }
        node.children.iter().all(Self::validate_values)
    }

    /// Enable hover highlighting (default: true).
    pub fn hover(mut self, enabled: bool) -> Self {
        self.hover_enabled = enabled;
        self
    }

    /// Build the treemap chart.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        let design = self.design.clone().unwrap_or_else(default_design);
        let (layout_width, layout_height) = resolved_chart_dimensions(self.chart_size);

        // Validate
        validate_dimensions(layout_width, layout_height)?;

        let total_value = self.root.total_value();
        if !total_value.is_finite() || total_value <= 0.0 {
            return Err(ChartError::InvalidData {
                field: "root",
                reason: "Total value must be positive and finite",
            });
        }

        // Validate all node values are finite
        if !Self::validate_values(&self.root) {
            return Err(ChartError::InvalidData {
                field: "node",
                reason: "All node values must be finite and non-negative",
            });
        }

        // Calculate layout
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };

        let margin = design.spacing.control_gap as f64;
        let plot_width = (layout_width as f64 - 2.0 * margin).max(0.0);
        let plot_height = (layout_height as f64 - title_height as f64 - 2.0 * margin).max(0.0);

        // Compute treemap layout
        let mut rects = Vec::new();
        compute_treemap(
            &self.root,
            0.0,
            0.0,
            plot_width,
            plot_height,
            self.tiling_method,
            self.padding,
            0,
            0,
            &mut rects,
        );

        let color_scheme = self.color_scheme.unwrap_or_else(ColorScheme::tableau10);

        // Precompute per-rect colors and group by fill color so the canvas paint
        // callback can emit one path per color group instead of one quad per rect.
        let mut draw_data: Vec<RectDrawData> = Vec::with_capacity(rects.len());
        let mut groups: BTreeMap<(u32, u32, u32, u32), Vec<usize>> = BTreeMap::new();

        for rect in rects {
            let color = color_scheme.color(rect.category_index);
            let rgba = Rgba {
                r: color.r / 255.0,
                g: color.g / 255.0,
                b: color.b / 255.0,
                a: 0.8,
            };
            let border = Rgba {
                r: rgba.r * 0.7,
                g: rgba.g * 0.7,
                b: rgba.b * 0.7,
                a: 1.0,
            };
            let key = (
                color.r.to_bits(),
                color.g.to_bits(),
                color.b.to_bits(),
                color.a.to_bits(),
            );
            let idx = draw_data.len();
            draw_data.push(RectDrawData {
                x0: rect.x0,
                y0: rect.y0,
                x1: rect.x1,
                y1: rect.y1,
                name: rect.name,
                value: rect.value,
                fill: rgba,
                border,
            });
            groups.entry(key).or_default().push(idx);
        }

        // Labels are emitted only for rects large enough to be legible.
        let mut label_elements = Vec::new();
        for rect in &draw_data {
            let width = rect.x1 - rect.x0;
            let height = rect.y1 - rect.y0;
            if width > 30.0 && height > 15.0 {
                let font_size = (height * 0.2).clamp(8.0, 12.0);
                let luminance = 0.2126 * rect.fill.r + 0.7152 * rect.fill.g + 0.0722 * rect.fill.b;
                let text_color = if luminance > 0.5 {
                    hsla(0.0, 0.0, 0.1, 1.0)
                } else {
                    hsla(0.0, 0.0, 0.95, 1.0)
                };
                let font_config = GlyphTextConfig::horizontal(font_size as f32, text_color);
                label_elements.push(
                    div()
                        .absolute()
                        .left(px(rect.x0 as f32))
                        .top(px(rect.y0 as f32))
                        .w(px(width as f32))
                        .h(px(height as f32))
                        .flex()
                        .flex_col()
                        .justify_center()
                        .items_center()
                        .overflow_hidden()
                        .text_ellipsis()
                        .px_1()
                        .child(render_glyph_text(&rect.name, &font_config)),
                );
            }
        }

        let draw_data = Rc::new(draw_data);
        let groups = Rc::new(groups);

        // Shared bounds updated by the canvas paint callback; used by the click
        // handler to map mouse coordinates back to treemap rects.
        let bounds: Rc<RefCell<Option<Bounds<Pixels>>>> = Rc::new(RefCell::new(None));
        let bounds_for_paint = bounds.clone();
        let bounds_for_click = bounds.clone();
        let draw_data_for_click = Rc::clone(&draw_data);
        let on_click = self.on_click;
        let renderer_2d = self.renderer_2d;
        let vello_backend = self.vello_backend;
        #[cfg(feature = "vello")]
        let vello_draw_data = (renderer_2d == Renderer2D::Vello).then(|| Rc::clone(&draw_data));

        let legacy_canvas_element = canvas(
            move |_bounds, _window, _cx| (Rc::clone(&draw_data), Rc::clone(&groups)),
            move |bounds, (draw_data, groups), window, _cx| {
                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();

                // Cache bounds for hit testing in event handlers.
                // We only need to do this once per paint.
                let _ = bounds_for_paint.borrow_mut().replace(bounds);

                for (_, indices) in groups.iter() {
                    if indices.is_empty() {
                        continue;
                    }

                    let mut fill_builder = PathBuilder::fill();
                    let mut stroke_builder = PathBuilder::stroke(px(1.0));

                    for &idx in indices {
                        let rect = &draw_data[idx];
                        let x = origin_x + rect.x0 as f32;
                        let y = origin_y + rect.y0 as f32;
                        let w = (rect.x1 - rect.x0) as f32;
                        let h = (rect.y1 - rect.y0) as f32;

                        add_rect_to_path(&mut fill_builder, x, y, w, h);
                        add_rect_to_path(&mut stroke_builder, x, y, w, h);
                    }

                    let first = &draw_data[indices[0]];
                    if let Ok(path) = fill_builder.build() {
                        window.paint_path(path, first.fill);
                    }
                    if let Ok(path) = stroke_builder.build() {
                        window.paint_path(path, first.border);
                    }
                }
            },
        )
        .w(px(plot_width as f32))
        .h(px(plot_height as f32))
        .absolute();

        let canvas_element: gpui::AnyElement = {
            #[cfg(feature = "vello")]
            if renderer_2d == Renderer2D::Vello {
                let vello_draw_data = vello_draw_data
                    .expect("Vello draw data is retained when the Vello renderer is selected");
                let mut cache_key = d3rs::vello2d::SceneCacheKey::new();
                cache_key.add_f64(plot_width).add_f64(plot_height);
                for rect in vello_draw_data.iter() {
                    cache_key
                        .add_f64(rect.x0)
                        .add_f64(rect.y0)
                        .add_f64(rect.x1)
                        .add_f64(rect.y1)
                        .add_f32(rect.fill.r)
                        .add_f32(rect.fill.g)
                        .add_f32(rect.fill.b)
                        .add_f32(rect.fill.a)
                        .add_f32(rect.border.r)
                        .add_f32(rect.border.g)
                        .add_f32(rect.border.b)
                        .add_f32(rect.border.a);
                }
                let cache_key = cache_key.finish();
                d3rs::vello2d::VelloChartElement::with_builder(move |width, height| {
                    let rects: Vec<_> = vello_draw_data
                        .iter()
                        .map(|rect| (rect.x0, rect.y0, rect.x1, rect.y1, rect.fill, rect.border))
                        .collect();
                    treemap_chart_scene(&rects, plot_width, plot_height, width, height)
                })
                .cache_key(cache_key)
                .backend(vello_backend)
                .absolute()
                .into_any_element()
            } else {
                legacy_canvas_element.into_any_element()
            }
            #[cfg(not(feature = "vello"))]
            {
                let _ = (renderer_2d, vello_backend);
                legacy_canvas_element.into_any_element()
            }
        };

        let mut plot_content = div()
            .w(px(plot_width as f32))
            .h(px(plot_height as f32))
            .relative()
            .bg(rgb(0xffffff))
            .child(canvas_element);

        if self.hover_enabled {
            plot_content = plot_content.hover(|style| style.cursor_pointer());
        }

        for label in label_elements {
            plot_content = plot_content.child(label);
        }

        // Click handler maps the mouse position to the treemap rect under the cursor.
        if let Some(handler) = on_click {
            let handler = Rc::clone(&handler);
            let bounds_for_click_down = bounds_for_click.clone();
            plot_content =
                plot_content.on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
                    if let Some(bounds) = *bounds_for_click_down.borrow() {
                        let origin_x: f32 = bounds.origin.x.into();
                        let origin_y: f32 = bounds.origin.y.into();
                        let local_x = f32::from(event.position.x) - origin_x;
                        let local_y = f32::from(event.position.y) - origin_y;

                        for rect in draw_data_for_click.iter() {
                            if local_x >= rect.x0 as f32
                                && local_x <= rect.x1 as f32
                                && local_y >= rect.y0 as f32
                                && local_y <= rect.y1 as f32
                            {
                                handler(&rect.name, rect.value);
                                break;
                            }
                        }
                    }
                });
        }

        // Build container
        let mut container = apply_chart_size(div(), self.chart_size)
            .flex()
            .flex_col()
            .bg(rgb(0xffffff));

        // Add title if present
        if let Some(title) = &self.title {
            let font_config = GlyphTextConfig::horizontal(
                design.typography.large_size.max(DEFAULT_TITLE_FONT_SIZE),
                hsla(0.0, 0.0, 0.2, 1.0),
            );
            container = container.child(
                div()
                    .w_full()
                    .h(px(title_height))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(render_glyph_text(title, &font_config)),
            );
        }

        // Add plot
        container = container.child(
            div()
                .flex()
                .justify_center()
                .items_center()
                .flex_1()
                .child(plot_content),
        );

        Ok(container)
    }
}

/// Build a backend-neutral Vello scene for prepared treemap rectangles.
#[cfg(feature = "vello")]
pub fn treemap_chart_scene(
    rectangles: &[(f64, f64, f64, f64, Rgba, Rgba)],
    source_width: f64,
    source_height: f64,
    width: f32,
    height: f32,
) -> d3rs::vello2d::ChartScene {
    use d3rs::vello2d::kurbo::{Rect, Shape, Stroke};
    use d3rs::vello2d::peniko::{Brush, Color};

    let sx = if source_width > 0.0 {
        width as f64 / source_width
    } else {
        1.0
    };
    let sy = if source_height > 0.0 {
        height as f64 / source_height
    } else {
        1.0
    };
    let mut scene = d3rs::vello2d::ChartScene::new();
    for &(x0, y0, x1, y1, fill, border) in rectangles {
        let rect = Rect::new(x0 * sx, y0 * sy, x1 * sx, y1 * sy);
        scene.fill_rect(
            rect,
            Brush::Solid(Color::new([fill.r, fill.g, fill.b, fill.a])),
        );
        scene.stroke_path(
            rect.to_path(0.1),
            Stroke::new(1.0),
            Brush::Solid(Color::new([border.r, border.g, border.b, border.a])),
        );
    }
    scene
}

/// Append a rectangle outline to a GPUI path builder.
pub(crate) fn add_rect_to_path(builder: &mut PathBuilder, x: f32, y: f32, width: f32, height: f32) {
    builder.move_to(point(px(x), px(y)));
    builder.line_to(point(px(x + width), px(y)));
    builder.line_to(point(px(x + width), px(y + height)));
    builder.line_to(point(px(x), px(y + height)));
    builder.close();
}

fn d3_color_to_hex(color: D3Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        d3_channel_to_u8(color.r),
        d3_channel_to_u8(color.g),
        d3_channel_to_u8(color.b)
    )
}

fn d3_channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn darken_d3_color(color: D3Color, factor: f32) -> D3Color {
    D3Color {
        r: color.r * factor,
        g: color.g * factor,
        b: color.b * factor,
        a: color.a,
    }
}
