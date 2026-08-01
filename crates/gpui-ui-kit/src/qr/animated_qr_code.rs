use super::misc::MIN_MODULE_PX;
use super::misc::QUIET_ZONE;
use super::misc::clamped_scroll_range;
use super::misc::ease_in_out_cubic;
use super::paint::paint_qr_static;
use super::{QrCodeError, QrCodeLimits};
use crate::theme::ThemeExt;
use gpui::prelude::{Context, IntoElement, Render, Styled};
use gpui::{
    BorderStyle, Bounds, Corners, Edges, PaintQuad, Pixels, Rgba, WeakEntity, Window, canvas,
    point, px, size,
};
use qrcode::QrCode as QrMatrix;
use qrcode::types::Color as QrColor;
use std::time::{Duration, Instant};

/// An animated QR code that pans a zoomed viewport when the display size is
/// too small for modules to be individually legible.
///
/// When the QR fits comfortably, it renders identically to [`crate::QrCode`].
///
/// # Example
///
/// ```ignore
/// // In a Context<Parent>:
/// let qr = cx.new(|cx| AnimatedQrCode::new("https://example.com", px(60.0), cx));
/// // In render:
/// parent.child(qr)
/// ```
pub struct AnimatedQrCode {
    /// Encoded QR matrix (None on encode failure).
    pub(super) matrix: Option<QrMatrix>,
    /// Pre-computed module colors (empty on encode failure).
    pub(super) colors: Vec<QrColor>,
    /// Number of modules on one side of the QR.
    pub(super) modules: usize,
    /// Display size in pixels.
    pub(super) size: Pixels,
    /// Foreground color.
    pub(super) fg: Option<Rgba>,
    /// Background color.
    pub(super) bg: Option<Rgba>,
    /// Whether animation is needed.
    pub(super) needs_animation: bool,
    /// Animation start time.
    pub(super) start: Instant,
    /// Total cycle duration for one full pan traversal.
    pub(super) cycle_duration: Duration,
    /// Zoom factor: how many times larger we render modules vs the tiny size.
    pub(super) zoom: f32,
}

impl AnimatedQrCode {
    /// Create a new animated QR code.
    ///
    /// If the given `size` is too small for modules to be legible, a panning
    /// animation starts automatically. Otherwise it renders statically.
    pub fn new(data: impl AsRef<[u8]>, size: Pixels, cx: &mut Context<Self>) -> Self {
        Self::try_new(data, size, cx, QrCodeLimits::default()).unwrap_or_else(|_| Self {
            matrix: None,
            colors: Vec::new(),
            modules: 0,
            size,
            fg: None,
            bg: None,
            needs_animation: false,
            start: Instant::now(),
            cycle_duration: Duration::ZERO,
            zoom: 1.0,
        })
    }

    /// Create an animated QR code while enforcing input and matrix limits.
    pub fn try_new(
        data: impl AsRef<[u8]>,
        size: Pixels,
        cx: &mut Context<Self>,
        limits: QrCodeLimits,
    ) -> Result<Self, QrCodeError> {
        let data = data.as_ref();
        if data.len() > limits.max_input_bytes {
            return Err(QrCodeError::InputTooLarge {
                limit: limits.max_input_bytes,
                actual: data.len(),
            });
        }
        let matrix = QrMatrix::new(data).map_err(|_| QrCodeError::MatrixTooLarge {
            limit: limits.max_modules,
            actual: limits.max_modules.saturating_add(1),
        })?;
        let modules = matrix.width();
        if modules > limits.max_modules {
            return Err(QrCodeError::MatrixTooLarge {
                limit: limits.max_modules,
                actual: modules,
            });
        }
        let colors = matrix.to_colors();
        let size_f32: f32 = size.into();
        let total_modules = modules + QUIET_ZONE * 2;
        let module_px = if total_modules > 0 {
            size_f32 / total_modules as f32
        } else {
            0.0
        };

        let needs_animation = modules > 0 && module_px < MIN_MODULE_PX;

        // Compute zoom so that each module is at least MIN_MODULE_PX * 2 for
        // comfortable readability in the zoomed viewport.
        let zoom = if needs_animation {
            (MIN_MODULE_PX * 2.0 / module_px).max(1.0)
        } else {
            1.0
        };

        // Cycle duration scales with QR complexity: more modules → longer pan.
        // A full raster scan at comfortable speed.
        let rows_in_view = (size_f32 / (module_px * zoom)).ceil() as usize;
        let pan_rows = if modules > rows_in_view {
            modules - rows_in_view
        } else {
            1
        };
        let cycle_duration = Duration::from_millis((pan_rows as u64 * 400).max(2000));

        if needs_animation {
            cx.spawn(async move |this: WeakEntity<Self>, cx| {
                loop {
                    smol::Timer::after(Duration::from_millis(33)).await;
                    let alive = this.update(cx, |_this, cx| {
                        cx.notify();
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }

        Ok(Self {
            matrix: Some(matrix),
            colors,
            modules,
            size,
            fg: None,
            bg: None,
            needs_animation,
            start: Instant::now(),
            cycle_duration,
            zoom,
        })
    }

    /// Override the foreground (dark module) color.
    pub fn fg(mut self, color: Rgba) -> Self {
        self.fg = Some(color);
        self
    }

    /// Override the background color.
    pub fn bg(mut self, color: Rgba) -> Self {
        self.bg = Some(color);
        self
    }
}

impl Render for AnimatedQrCode {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let fg_color = self.fg.unwrap_or(theme.text_primary);
        let bg_color = self.bg.unwrap_or(theme.transparent);
        let requested_size = self.size;
        let size_f32: f32 = requested_size.into();

        if !self.needs_animation || self.matrix.is_none() {
            // Static render — same as QrCode
            let colors = self.colors.clone();
            let modules = self.modules;

            return canvas(
                move |_bounds, _window, _cx| (colors, modules),
                move |bounds, (colors, modules), window, _cx| {
                    paint_qr_static(
                        bounds, &colors, modules, size_f32, fg_color, bg_color, window,
                    );
                },
            )
            .w(requested_size)
            .h(requested_size)
            .into_any_element();
        }

        // Animated render: compute pan offset from elapsed time
        let elapsed = self.start.elapsed();
        let modules = self.modules;
        let zoom = self.zoom;
        let total_modules = modules + QUIET_ZONE * 2;
        let base_module_px = size_f32 / total_modules as f32;
        let zoomed_module_px = base_module_px * zoom;

        // How many modules fit in the viewport at the zoomed scale
        let viewport_modules = (size_f32 / zoomed_module_px).floor();
        // Total scrollable range in modules (including quiet zones)
        let scroll_range = clamped_scroll_range(total_modules, viewport_modules);

        // Ping-pong progress: 0→1→0 over cycle_duration
        let cycle_secs = self.cycle_duration.as_secs_f32();
        let raw_t = (elapsed.as_secs_f32() % (cycle_secs * 2.0)) / cycle_secs;
        let t = if raw_t <= 1.0 { raw_t } else { 2.0 - raw_t };

        // Ease the progress for smooth motion
        let eased = ease_in_out_cubic(t);

        // Scroll both axes together (diagonal pan)
        let offset_modules = eased * scroll_range;

        let colors = self.colors.clone();

        canvas(
            move |_bounds, _window, _cx| colors,
            move |bounds, colors, window, _cx| {
                // Background
                if bg_color.a > 0.0 {
                    window.paint_quad(PaintQuad {
                        bounds,
                        corner_radii: Corners::default(),
                        background: bg_color.into(),
                        border_widths: Edges::default(),
                        border_color: bg_color.into(),
                        border_style: BorderStyle::default(),
                    });
                }

                if modules == 0 {
                    return;
                }

                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();

                // Pixel offset of the viewport into the full zoomed QR
                let pixel_offset = offset_modules * zoomed_module_px;

                for row in 0..modules {
                    for col in 0..modules {
                        if colors[row * modules + col] == QrColor::Dark {
                            let x = (col + QUIET_ZONE) as f32 * zoomed_module_px - pixel_offset;
                            let y = (row + QUIET_ZONE) as f32 * zoomed_module_px - pixel_offset;

                            // Clip to viewport
                            if x + zoomed_module_px < 0.0
                                || x > size_f32
                                || y + zoomed_module_px < 0.0
                                || y > size_f32
                            {
                                continue;
                            }

                            // Clamp to viewport edges
                            let draw_x = x.max(0.0);
                            let draw_y = y.max(0.0);
                            let draw_w = (x + zoomed_module_px).min(size_f32) - draw_x;
                            let draw_h = (y + zoomed_module_px).min(size_f32) - draw_y;

                            if draw_w > 0.0 && draw_h > 0.0 {
                                window.paint_quad(PaintQuad {
                                    bounds: Bounds {
                                        origin: point(px(origin_x + draw_x), px(origin_y + draw_y)),
                                        size: size(px(draw_w), px(draw_h)),
                                    },
                                    corner_radii: Corners::default(),
                                    background: fg_color.into(),
                                    border_widths: Edges::default(),
                                    border_color: fg_color.into(),
                                    border_style: BorderStyle::default(),
                                });
                            }
                        }
                    }
                }
            },
        )
        .w(requested_size)
        .h(requested_size)
        .into_any_element()
    }
}
