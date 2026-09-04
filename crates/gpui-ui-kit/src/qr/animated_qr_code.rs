use super::misc::MIN_MODULE_PX;
use super::misc::QUIET_ZONE;
use super::misc::clamped_scroll_range;
use super::misc::ease_in_out_cubic;
use super::paint::cached_rasterize_qr_image;
use super::{QrCodeError, QrCodeLimits};
use crate::theme::ThemeExt;
use gpui::prelude::{Context, IntoElement, Render, Styled};
use gpui::{
    Bounds, Corners, Pixels, RenderImage, Rgba, WeakEntity, Window, canvas, point, px, size,
};
use qrcode::QrCode as QrMatrix;
use qrcode::types::Color as QrColor;
use std::sync::Arc;
use std::time::Duration;
// `std::time::Instant` panics on wasm32-unknown-unknown ("time not implemented
// on this platform"); web-time aliases std on native targets.
use web_time::Instant;

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
    pub(super) colors: Arc<[QrColor]>,
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
            colors: Arc::from([]),
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
        let colors: Arc<[QrColor]> = Arc::from(matrix.to_colors());
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
                    #[cfg(not(target_family = "wasm"))]
                    smol::Timer::after(Duration::from_millis(33)).await;
                    #[cfg(target_family = "wasm")]
                    cx.background_executor()
                        .timer(Duration::from_millis(33))
                        .await;
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

    /// Theme-colored module raster from the shared content-keyed cache.
    /// The matrix never changes after construction, so any color change
    /// addresses a different cache entry and rebuilds exactly once.
    fn raster_image(&self, fg: Rgba, bg: Rgba) -> Option<Arc<RenderImage>> {
        cached_rasterize_qr_image(&self.colors, self.modules, fg, bg)
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
            let image = self.raster_image(fg_color, bg_color);

            return canvas(
                move |_bounds, _window, _cx| image,
                move |bounds, image, window, _cx| {
                    if let Some(image) = image {
                        let _ = window.paint_image(bounds, Corners::default(), image, 0, false);
                    }
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

        let image = self.raster_image(fg_color, bg_color);

        canvas(
            move |_bounds, _window, _cx| image,
            move |bounds, image, window, _cx| {
                let Some(image) = image else { return };

                // Paint the complete raster at the zoomed size, then shift it
                // under this overflow-hidden canvas. The parent clip supplies
                // the moving viewport without rebuilding module primitives.
                let pixel_offset = offset_modules * zoomed_module_px;
                let full_size = size_f32 * zoom;
                let _ = window.paint_image(
                    Bounds {
                        origin: point(
                            bounds.origin.x - px(pixel_offset),
                            bounds.origin.y - px(pixel_offset),
                        ),
                        size: size(px(full_size), px(full_size)),
                    },
                    Corners::default(),
                    image,
                    0,
                    false,
                );
            },
        )
        .w(requested_size)
        .h(requested_size)
        .overflow_hidden()
        .into_any_element()
    }
}
