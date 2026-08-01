use super::paint::paint_qr_full_from_colors;
use super::{QrCodeError, QrCodeLimits};
use crate::theme::ThemeExt;
use gpui::prelude::{IntoElement, RenderOnce, Styled};
use gpui::{App, Pixels, Rgba, Window, canvas, px};
use qrcode::QrCode as QrMatrix;

/// A QR code display component.
///
/// Encodes a string at Medium error-correction level and renders each dark
/// module as a filled rectangle scaled to the requested pixel size.
///
/// # Example
///
/// ```ignore
/// QrCode::new("https://example.com")
///     .size(px(200.0))
/// ```
pub struct QrCode {
    /// Pre-computed module colors (empty on encode failure).
    pub(super) colors: Vec<qrcode::types::Color>,
    /// Number of modules on one side of the QR.
    pub(super) modules: usize,
    /// Rendered size in pixels (width and height; the code is always square).
    pub(super) size: Pixels,
    /// Foreground (dark module) color. Defaults to theme's `text_primary`.
    pub(super) fg: Option<Rgba>,
    /// Background color. Defaults to transparent.
    pub(super) bg: Option<Rgba>,
}

impl QrCode {
    /// Create a new QR code component that encodes `data`.
    pub fn new(data: impl AsRef<[u8]>) -> Self {
        Self::try_new(data, QrCodeLimits::default()).unwrap_or_else(|_| Self {
            colors: Vec::new(),
            modules: 0,
            size: px(200.0),
            fg: None,
            bg: None,
        })
    }

    /// Create a QR code while enforcing input and matrix-size limits.
    pub fn try_new(data: impl AsRef<[u8]>, limits: QrCodeLimits) -> Result<Self, QrCodeError> {
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
        Ok(Self {
            colors,
            modules,
            size: px(200.0),
            fg: None,
            bg: None,
        })
    }

    /// Bounded constructor with a cooperative cancellation check.
    pub fn try_new_with_cancel<F: FnMut() -> bool>(
        data: impl AsRef<[u8]>,
        limits: QrCodeLimits,
        mut cancelled: F,
    ) -> Result<Self, QrCodeError> {
        if cancelled() {
            return Err(QrCodeError::Cancelled);
        }
        let qr = Self::try_new(data, limits)?;
        if cancelled() {
            return Err(QrCodeError::Cancelled);
        }
        Ok(qr)
    }

    /// Set the rendered size (both width and height) in pixels.
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
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

    /// Build the canvas element with explicit colors.
    pub(super) fn build(self, fg_color: Rgba, bg_color: Rgba) -> impl IntoElement {
        let requested_size = self.size;
        let size_f32: f32 = requested_size.into();
        let colors = self.colors;
        let modules = self.modules;

        canvas(
            move |_bounds, _window, _cx| (colors, modules),
            move |bounds, (colors, modules), window, _cx| {
                paint_qr_full_from_colors(
                    bounds, &colors, modules, size_f32, fg_color, bg_color, window,
                );
            },
        )
        .w(requested_size)
        .h(requested_size)
    }
}

impl RenderOnce for QrCode {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let fg_color = self.fg.unwrap_or(theme.text_primary);
        let bg_color = self.bg.unwrap_or(theme.transparent);
        self.build(fg_color, bg_color)
    }
}

impl IntoElement for QrCode {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{QrCode, QrCodeError, QrCodeLimits};

    #[test]
    fn bounded_qr_rejects_large_input() {
        let error = QrCode::try_new(
            "hello",
            QrCodeLimits {
                max_input_bytes: 4,
                max_modules: 177,
            },
        )
        .unwrap_err();
        assert!(matches!(error, QrCodeError::InputTooLarge { .. }));
    }

    #[test]
    fn bounded_qr_supports_cancellation() {
        let error = QrCode::try_new_with_cancel(
            "hello",
            QrCodeLimits::default(),
            || true,
        )
        .unwrap_err();
        assert_eq!(error, QrCodeError::Cancelled);
    }
}
