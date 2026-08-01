//! QR Code display component
//!
//! Renders an encoded QR code as a matrix of filled squares using GPUI's
//! low-level paint API. Suitable for sharing URLs, wallet addresses, or any
//! string data.
//!
//! Two variants are provided:
//!
//! - [`QrCode`] — stateless `RenderOnce` for sizes large enough to display
//!   all modules legibly.
//! - [`AnimatedQrCode`] — stateful `Entity` that automatically pans a zoomed
//!   viewport across the QR when the display size is too small for modules to
//!   be individually distinguishable, then settles to show the full code.

mod animated_qr_code;
mod misc;
mod paint;
mod qr_code;

pub use animated_qr_code::AnimatedQrCode;
pub use qr_code::QrCode;

/// Bounds applied before QR matrix allocation and rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QrCodeLimits {
    pub max_input_bytes: usize,
    pub max_modules: usize,
}

impl Default for QrCodeLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 4096,
            max_modules: 177,
        }
    }
}

/// Failure returned by the bounded QR constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrCodeError {
    InputTooLarge { limit: usize, actual: usize },
    MatrixTooLarge { limit: usize, actual: usize },
    Cancelled,
}

impl std::fmt::Display for QrCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputTooLarge { limit, actual } => {
                write!(f, "QR input is too large: {actual} bytes > {limit}")
            }
            Self::MatrixTooLarge { limit, actual } => {
                write!(f, "QR matrix is too large: {actual} modules > {limit}")
            }
            Self::Cancelled => write!(f, "QR encoding cancelled"),
        }
    }
}

impl std::error::Error for QrCodeError {}
