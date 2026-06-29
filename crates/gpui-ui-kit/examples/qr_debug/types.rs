#[derive(Debug, Clone)]
pub(super) struct CameraPreview {
    pub(super) rgb_pixels: Vec<u8>,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) qr_bounds: Option<[(f32, f32); 4]>,
}

#[derive(Debug, Clone)]
pub(super) enum QrScanResult {
    Success(String, CameraPreview),
    Scanning(CameraPreview),
    Detected(CameraPreview),
    Error(String),
}

/// Camera permission / availability state
#[derive(Debug, Clone, PartialEq)]
pub(super) enum CameraState {
    /// Haven't tried yet
    Idle,
    /// Camera is active and streaming
    Active,
    /// Camera access denied or unavailable
    Denied(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tab {
    Generate,
    Scan,
}
