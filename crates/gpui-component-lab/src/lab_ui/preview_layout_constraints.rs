use super::misc::clamp_f32;
use super::preview_align::PreviewAlign;
use super::preview_overflow::PreviewOverflow;
use super::preview_sizing::PreviewSizing;
use super::preview_surface::PreviewSurface;
use crate::ViewportPreset;
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PreviewLayoutConstraints {
    pub(super) sizing: PreviewSizing,
    pub(super) min_width: f32,
    pub(super) min_height: f32,
    pub(super) aspect_ratio: f32,
    pub(super) padding: f32,
    pub(super) horizontal_align: PreviewAlign,
    pub(super) vertical_align: PreviewAlign,
    pub(super) overflow: PreviewOverflow,
    pub(super) surface: PreviewSurface,
    pub(super) gap: f32,
    pub(super) border: bool,
}

impl Default for PreviewLayoutConstraints {
    fn default() -> Self {
        Self {
            sizing: PreviewSizing::Fill,
            min_width: 560.0,
            min_height: 340.0,
            aspect_ratio: 1.6,
            padding: 24.0,
            horizontal_align: PreviewAlign::Center,
            vertical_align: PreviewAlign::Center,
            overflow: PreviewOverflow::Hidden,
            surface: PreviewSurface::Background,
            gap: 0.0,
            border: true,
        }
    }
}

impl PreviewLayoutConstraints {
    pub(super) fn from_layout(layout: &Value) -> Self {
        let mut constraints = Self::default();
        let Some(raw) = layout.get("constraints") else {
            return constraints;
        };

        if let Some(sizing) = raw.get("sizing").and_then(Value::as_str) {
            constraints.sizing = PreviewSizing::parse(sizing);
        }
        if let Some(min_width) = raw.get("min_width").and_then(Value::as_f64) {
            constraints.min_width = clamp_f32(min_width, 160.0, 1600.0);
        }
        if let Some(min_height) = raw.get("min_height").and_then(Value::as_f64) {
            constraints.min_height = clamp_f32(min_height, 120.0, 1200.0);
        }
        if let Some(aspect_ratio) = raw.get("aspect_ratio").and_then(Value::as_f64) {
            constraints.aspect_ratio = clamp_f32(aspect_ratio, 0.5, 3.0);
        }
        if let Some(padding) = raw.get("padding").and_then(Value::as_f64) {
            constraints.padding = clamp_f32(padding, 0.0, 80.0);
        }

        if let Some(builder) = layout.get("builder").and_then(Value::as_object) {
            if let Some(align) = builder.get("horizontal_align").and_then(Value::as_str) {
                constraints.horizontal_align = PreviewAlign::parse(align);
            }
            if let Some(align) = builder.get("vertical_align").and_then(Value::as_str) {
                constraints.vertical_align = PreviewAlign::parse(align);
            }
            if let Some(overflow) = builder.get("overflow").and_then(Value::as_str) {
                constraints.overflow = PreviewOverflow::parse(overflow);
            }
            if let Some(surface) = builder.get("surface").and_then(Value::as_str) {
                constraints.surface = PreviewSurface::parse(surface);
            }
            if let Some(gap) = builder.get("gap").and_then(Value::as_f64) {
                constraints.gap = clamp_f32(gap, 0.0, 80.0);
            }
            if let Some(border) = builder.get("border").and_then(Value::as_bool) {
                constraints.border = border;
            }
        }

        constraints
    }

    pub(super) fn as_json(self) -> Value {
        json!({
            "sizing": self.sizing.as_str(),
            "min_width": self.min_width,
            "min_height": self.min_height,
            "aspect_ratio": self.aspect_ratio,
            "padding": self.padding,
        })
    }

    pub(super) fn builder_json(self) -> Value {
        json!({
            "horizontal_align": self.horizontal_align.as_str(),
            "vertical_align": self.vertical_align.as_str(),
            "overflow": self.overflow.as_str(),
            "surface": self.surface.as_str(),
            "gap": self.gap,
            "border": self.border,
        })
    }

    pub(super) fn frame_dimensions(self, viewport: &ViewportPreset) -> (f32, f32) {
        let viewport_width = viewport.width.clamp(240.0, 980.0);
        let viewport_height = viewport.height.clamp(180.0, 620.0);
        match self.sizing {
            PreviewSizing::Fixed => (self.min_width, self.min_height),
            PreviewSizing::Fit => {
                let width = self.min_width.min(viewport_width);
                let height = if self.aspect_ratio > 0.0 {
                    (width / self.aspect_ratio).max(self.min_height.min(viewport_height))
                } else {
                    self.min_height.min(viewport_height)
                };
                (width, height.min(viewport_height))
            }
            PreviewSizing::Fill => {
                let width = viewport_width.max(self.min_width).min(980.0);
                let height = if self.aspect_ratio > 0.0 {
                    (width / self.aspect_ratio).max(self.min_height)
                } else {
                    viewport_height.max(self.min_height)
                };
                (width, height.min(620.0).max(self.min_height.min(620.0)))
            }
        }
    }
}
